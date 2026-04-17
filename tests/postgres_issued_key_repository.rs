//! Integration tests for
//! [`koentji::infrastructure::postgres::PostgresIssuedKeyRepository`].
//!
//! These hit a real Postgres (the shared test DB the harness sets up)
//! — mocking the DB would defeat the point. Every test calls
//! `reset(&pool)` first so cross-test pollution is impossible.
//!
//! Coverage:
//! - `find` hydrates username/email/window_seconds/expired_at/deleted_at.
//! - `consume_quota` atomically decrements and refuses to go negative.
//! - `consume_quota` under concurrency never oversells.
//! - `claim_free_trial` inserts on the FREE_TRIAL marker.
//! - `claim_free_trial` rebinds a pre-issued `device_id = '-'` row.
//! - `claim_free_trial` returns `None` for a plain unknown key.

#![cfg(feature = "ssr")]

mod common;

use chrono::{DateTime, Duration, Utc};
use koentji::domain::authentication::{
    AuthKey, ConsumeOutcome, DeviceId, FreeTrialConfig, IssuedKeyRepository, RateLimitUsage,
};
use koentji::infrastructure::postgres::PostgresIssuedKeyRepository;
use std::sync::Arc;

use common::{a_key, fresh_pool};

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn repo(pool: sqlx::PgPool) -> PostgresIssuedKeyRepository {
    PostgresIssuedKeyRepository::new(pool)
}

fn auth_key(s: &str) -> AuthKey {
    AuthKey::parse(s.to_string()).expect("valid test key")
}

fn device(s: &str) -> DeviceId {
    DeviceId::parse(s.to_string()).expect("valid test device")
}

fn usage(n: i32) -> RateLimitUsage {
    RateLimitUsage::new(n).expect("non-negative usage")
}

/// The default `a_key()` insert leaves `rate_limit_updated_at` NULL,
/// which the consume SQL treats as "window has elapsed" → reset. For
/// tests that want the within-window branch, stamp a recent timestamp.
async fn stamp_updated_now(pool: &sqlx::PgPool, id: i32) {
    sqlx::query("UPDATE authentication_keys SET rate_limit_updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .expect("stamp updated_at");
}

#[tokio::test]
async fn find_returns_none_for_an_unknown_key() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());

    let found = r
        .find(&auth_key("no_such_key"), &device("no_such_device"))
        .await
        .expect("find must not error");
    assert!(found.is_none());
}

#[tokio::test]
async fn find_hydrates_identity_and_ledger_fields() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_find_id")
        .with_device("dev-find")
        .with_rate_limit(100)
        .with_remaining(37)
        .with_username("alice")
        .with_email("alice@example.com")
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let snap = r
        .find(&auth_key(&inserted.key), &device(&inserted.device_id))
        .await
        .expect("find must not error")
        .expect("key exists");

    assert_eq!(snap.key.as_str(), "klab_find_id");
    assert_eq!(snap.device_id.as_str(), "dev-find");
    assert_eq!(snap.rate_limit.daily.value(), 100);
    assert_eq!(snap.rate_limit.remaining.value(), 37);
    assert_eq!(snap.username.as_deref(), Some("alice"));
    assert_eq!(snap.email.as_deref(), Some("alice@example.com"));
    assert!(snap.revoked_at.is_none());
}

#[tokio::test]
async fn find_treats_a_revoked_key_as_revoked_not_missing() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_revoked")
        .with_device("dev-revoked")
        .revoked()
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let snap = r
        .find(&auth_key(&inserted.key), &device(&inserted.device_id))
        .await
        .expect("find must not error")
        .expect("revoked row still hydrates");
    assert!(snap.revoked_at.is_some());
}

#[tokio::test]
async fn find_uses_daily_window_when_no_interval_linked() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_default_window")
        .with_device("dev-window")
        .with_interval("daily")
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let snap = r
        .find(&auth_key(&inserted.key), &device(&inserted.device_id))
        .await
        .expect("find must not error")
        .expect("key exists");
    assert_eq!(snap.rate_limit.window.as_seconds(), 86_400);
}

#[tokio::test]
async fn consume_quota_allows_and_decrements_within_quota() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_consume_1")
        .with_device("dev-consume-1")
        .with_rate_limit(100)
        .with_remaining(10)
        .insert(&pool)
        .await;
    stamp_updated_now(&pool, inserted.id).await;

    let r = repo(pool.clone());
    let out = r
        .consume_quota(
            &auth_key(&inserted.key),
            &device(&inserted.device_id),
            usage(1),
            now(),
        )
        .await
        .expect("consume must not error");

    match out {
        ConsumeOutcome::Allowed { remaining, .. } => assert_eq!(remaining.value(), 9),
        ConsumeOutcome::RateLimitExceeded => panic!("expected Allowed"),
    }
}

#[tokio::test]
async fn consume_quota_denies_at_the_legacy_off_by_one_boundary() {
    // remaining == usage is denied — legacy `>` not `>=`.
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_consume_off_by_one")
        .with_device("dev-off-by-one")
        .with_rate_limit(10)
        .with_remaining(1)
        .insert(&pool)
        .await;
    stamp_updated_now(&pool, inserted.id).await;

    let r = repo(pool.clone());
    let out = r
        .consume_quota(
            &auth_key(&inserted.key),
            &device(&inserted.device_id),
            usage(1),
            now(),
        )
        .await
        .expect("consume must not error");

    assert!(matches!(out, ConsumeOutcome::RateLimitExceeded));
}

#[tokio::test]
async fn consume_quota_resets_when_the_window_has_elapsed() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_consume_reset")
        .with_device("dev-reset")
        .with_rate_limit(10)
        .with_remaining(0)
        .insert(&pool)
        .await;

    sqlx::query("UPDATE authentication_keys SET rate_limit_updated_at = $1 WHERE id = $2")
        .bind(Utc::now() - Duration::days(2))
        .bind(inserted.id)
        .execute(&pool)
        .await
        .expect("nudge timestamp");

    let r = repo(pool.clone());
    let out = r
        .consume_quota(
            &auth_key(&inserted.key),
            &device(&inserted.device_id),
            usage(1),
            now(),
        )
        .await
        .expect("consume must not error");

    match out {
        ConsumeOutcome::Allowed { remaining, .. } => assert_eq!(remaining.value(), 9),
        ConsumeOutcome::RateLimitExceeded => panic!("expected reset + Allowed"),
    }
}

#[tokio::test]
async fn consume_quota_never_oversells_under_concurrency() {
    // A classic read-modify-write leak would let the 20 spawned
    // consumers race past remaining==20 — the atomic UPDATE ensures
    // exactly 19 Allowed (legacy off-by-one) and the rest refused.
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_consume_race")
        .with_device("dev-race")
        .with_rate_limit(20)
        .with_remaining(20)
        .insert(&pool)
        .await;
    stamp_updated_now(&pool, inserted.id).await;
    let r = Arc::new(repo(pool.clone()));

    let mut handles = Vec::new();
    for _ in 0..20 {
        let r = r.clone();
        let k = inserted.key.clone();
        let d = inserted.device_id.clone();
        handles.push(tokio::spawn(async move {
            r.consume_quota(&auth_key(&k), &device(&d), usage(1), now())
                .await
                .expect("consume must not error")
        }));
    }

    let mut allowed = 0;
    for h in handles {
        if matches!(h.await.unwrap(), ConsumeOutcome::Allowed { .. }) {
            allowed += 1;
        }
    }
    // Legacy off-by-one: 20 starting remaining, predicate `>`, so the
    // 20th attempt (remaining==1 at that point) is refused.
    assert_eq!(allowed, 19);

    let (final_remaining,): (i32,) =
        sqlx::query_as("SELECT rate_limit_remaining FROM authentication_keys WHERE id = $1")
            .bind(inserted.id)
            .fetch_one(&pool)
            .await
            .expect("read back");
    assert_eq!(final_remaining, 1);
}

#[tokio::test]
async fn consume_quota_denies_when_daily_is_not_greater_than_usage() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_consume_usage_gte_daily")
        .with_device("dev-usage-gte-daily")
        .with_rate_limit(5)
        .insert(&pool)
        .await;
    stamp_updated_now(&pool, inserted.id).await;

    let r = repo(pool.clone());
    let out = r
        .consume_quota(
            &auth_key(&inserted.key),
            &device(&inserted.device_id),
            usage(5),
            now(),
        )
        .await
        .expect("consume must not error");

    assert!(matches!(out, ConsumeOutcome::RateLimitExceeded));
}

#[tokio::test]
async fn claim_free_trial_inserts_on_marker_match() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());
    let config = FreeTrialConfig::new("FREE_TRIAL", "free");

    let out = r
        .claim_free_trial(&auth_key("FREE_TRIAL"), &device("dev-new-trial"), &config)
        .await
        .expect("claim must not error")
        .expect("row was created");

    assert_eq!(out.key.as_str(), "FREE_TRIAL");
    assert_eq!(out.device_id.as_str(), "dev-new-trial");
    assert!(out.expired_at.is_some());
    // Free-trial rows carry the "free" subscription by default.
    assert_eq!(out.subscription.as_ref().map(|s| s.as_str()), Some("free"));
    assert!(out.is_free_trial);
}

#[tokio::test]
async fn claim_free_trial_rebinds_a_pre_issued_key() {
    // Admin-issued "pre-bound" row sits with device_id='-'; the first
    // device to call in claims it by rebinding.
    let pool = fresh_pool().await;
    let _ = a_key()
        .with_key("klab_preissued_42")
        .with_device("-")
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let config = FreeTrialConfig::new("FREE_TRIAL", "free");
    let out = r
        .claim_free_trial(
            &auth_key("klab_preissued_42"),
            &device("dev-new-owner"),
            &config,
        )
        .await
        .expect("claim must not error")
        .expect("rebind returns a fresh snapshot");

    assert_eq!(out.device_id.as_str(), "dev-new-owner");

    // Only one row for that key — device_id='-' is gone.
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM authentication_keys WHERE key = $1")
            .bind("klab_preissued_42")
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn claim_free_trial_returns_none_for_an_unknown_non_marker_key() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());
    let config = FreeTrialConfig::new("FREE_TRIAL", "free");

    let out = r
        .claim_free_trial(&auth_key("klab_nope"), &device("dev-nope"), &config)
        .await
        .expect("claim must not error");

    assert!(out.is_none());
}
