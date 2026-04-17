//! Regression coverage for the atomic rate-limit decrement (B7 / 0.3).
//!
//! Two concurrent `consume_rate_limit` calls must never both succeed when
//! only enough quota exists for one. The previous implementation did a
//! read-modify-write in two round trips and leaked quota under load; this
//! suite pins the new atomic contract in place.
//!
//! Semantics: the SQL predicate uses `>=`, so the last slot is
//! consumable — a key with `daily=N` allows exactly `N` consumes of
//! usage 1 per window; the `N+1`th is refused.

#![cfg(feature = "ssr")]

mod common;

use chrono::{Duration, Utc};
use common::{a_key, fresh_pool};
use koentji::rate_limit::{consume_rate_limit, ConsumeResult};

#[tokio::test]
async fn sequential_consume_decrements_by_usage() {
    let pool = fresh_pool().await;
    let seeded = a_key()
        .with_device("seq-dev")
        .with_rate_limit(100)
        .insert(&pool)
        .await;

    let r = consume_rate_limit(&pool, &seeded.key, &seeded.device_id, 1, Utc::now())
        .await
        .expect("first consume succeeds");
    let remaining = expect_allowed(r);
    assert_eq!(remaining, 99, "remaining drops by exactly one");

    let r = consume_rate_limit(&pool, &seeded.key, &seeded.device_id, 5, Utc::now())
        .await
        .expect("second consume succeeds");
    let remaining = expect_allowed(r);
    assert_eq!(remaining, 94, "remaining drops by requested usage");
}

#[tokio::test]
async fn consume_returns_rate_limited_when_window_open_and_empty() {
    let pool = fresh_pool().await;
    let seeded = a_key()
        .with_device("exhaust-dev")
        .with_rate_limit(3)
        .with_remaining(3)
        .insert(&pool)
        .await;

    // With daily=3 the full quota is usable: three consumes drop
    // remaining to 2, 1, then 0.
    for expected_remaining in [2, 1, 0] {
        let r = consume_rate_limit(&pool, &seeded.key, &seeded.device_id, 1, Utc::now())
            .await
            .unwrap();
        assert_eq!(expect_allowed(r), expected_remaining);
    }

    // Fourth consume: remaining=0 < usage=1 → predicate fails → rate-limited.
    let r = consume_rate_limit(&pool, &seeded.key, &seeded.device_id, 1, Utc::now())
        .await
        .unwrap();
    assert!(matches!(r, ConsumeResult::RateLimitExceeded));
}

#[tokio::test]
async fn concurrent_consume_never_exceeds_quota() {
    // The canary test: fire 11 concurrent consumes at a key with daily=10,
    // remaining=10. With the `>=` predicate exactly 10 must succeed —
    // the 11th finds remaining=0 and is refused. The old
    // read-modify-write path could let all requests win.
    let pool = fresh_pool().await;
    let seeded = a_key()
        .with_device("race-dev")
        .with_rate_limit(10)
        .with_remaining(10)
        .insert(&pool)
        .await;

    // Reset updated_at so the window is open (not in reset territory).
    sqlx::query("UPDATE authentication_keys SET rate_limit_updated_at = NOW() WHERE id = $1")
        .bind(seeded.id)
        .execute(&pool)
        .await
        .unwrap();

    let mut handles = Vec::new();
    for _ in 0..11 {
        let pool = pool.clone();
        let key = seeded.key.clone();
        let device = seeded.device_id.clone();
        handles.push(tokio::spawn(async move {
            consume_rate_limit(&pool, &key, &device, 1, Utc::now())
                .await
                .unwrap()
        }));
    }

    let mut allowed = 0usize;
    let mut limited = 0usize;
    for h in handles {
        match h.await.unwrap() {
            ConsumeResult::Allowed { .. } => allowed += 1,
            ConsumeResult::RateLimitExceeded => limited += 1,
        }
    }

    assert_eq!(allowed, 10, "exactly `daily` consumes win");
    assert_eq!(limited, 1, "the surplus request is rejected");

    // DB ground truth: predicate `remaining >= usage` clamps at 0,
    // never races into a negative counter.
    let final_remaining: i32 =
        sqlx::query_scalar("SELECT rate_limit_remaining FROM authentication_keys WHERE id = $1")
            .bind(seeded.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(final_remaining, 0);
}

#[tokio::test]
async fn window_elapsed_resets_remaining_to_daily_minus_usage() {
    let pool = fresh_pool().await;
    let seeded = a_key()
        .with_device("reset-dev")
        .with_rate_limit(50)
        .with_remaining(0) // exhausted
        .insert(&pool)
        .await;

    // Pretend the last update was two days ago — the daily window has
    // elapsed, so the next consume must reset instead of refusing.
    sqlx::query("UPDATE authentication_keys SET rate_limit_updated_at = $1 WHERE id = $2")
        .bind(Utc::now() - Duration::days(2))
        .bind(seeded.id)
        .execute(&pool)
        .await
        .unwrap();

    let r = consume_rate_limit(&pool, &seeded.key, &seeded.device_id, 1, Utc::now())
        .await
        .unwrap();
    assert_eq!(expect_allowed(r), 49, "reset yields daily-minus-usage");
}

#[tokio::test]
async fn unknown_key_returns_rate_limited() {
    let pool = fresh_pool().await;
    let r = consume_rate_limit(&pool, "no_such_key", "no_such_device", 1, Utc::now())
        .await
        .unwrap();
    assert!(matches!(r, ConsumeResult::RateLimitExceeded));
}

#[tokio::test]
async fn usage_exceeding_daily_is_rejected() {
    let pool = fresh_pool().await;
    let seeded = a_key()
        .with_device("huge-usage")
        .with_rate_limit(10)
        .insert(&pool)
        .await;

    let r = consume_rate_limit(&pool, &seeded.key, &seeded.device_id, 100, Utc::now())
        .await
        .unwrap();
    assert!(matches!(r, ConsumeResult::RateLimitExceeded));
}

fn expect_allowed(r: ConsumeResult) -> i32 {
    match r {
        ConsumeResult::Allowed { remaining, .. } => remaining,
        ConsumeResult::RateLimitExceeded => panic!("expected Allowed, got RateLimitExceeded"),
    }
}
