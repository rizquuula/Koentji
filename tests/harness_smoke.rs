//! Smoke test for the integration harness (commit 0.1).
//!
//! Proves: the shared test DB boots, migrations apply, the key builder
//! round-trips a row, and the test clock advances deterministically.
//!
//! Skipped under any config that doesn't enable `ssr` (e.g. `cargo test`
//! without `--features ssr` simply has no tests here to run).

#![cfg(feature = "ssr")]

mod common;

use chrono::{Duration, Utc};
use common::{a_key, fresh_pool, Clock, SystemClock, TestClock};

#[tokio::test]
async fn harness_round_trips_a_key() {
    let pool = fresh_pool().await;

    let seeded = a_key()
        .with_device("smoke-device")
        .with_rate_limit(100)
        .insert(&pool)
        .await;

    let fetched: (String, String, i32) = sqlx::query_as(
        "SELECT key, device_id, rate_limit_daily FROM authentication_keys WHERE id = $1",
    )
    .bind(seeded.id)
    .fetch_one(&pool)
    .await
    .expect("fetch seeded key");

    assert_eq!(fetched.0, seeded.key);
    assert_eq!(fetched.1, "smoke-device");
    assert_eq!(fetched.2, 100);
}

#[tokio::test]
async fn test_clock_advances() {
    let start = Utc::now();
    let clock = TestClock::at(start);
    assert_eq!(clock.now(), start);

    clock.advance(Duration::minutes(15));
    assert_eq!(clock.now(), start + Duration::minutes(15));
}

#[test]
fn system_clock_is_monotonic_within_a_test() {
    let c = SystemClock;
    let a = c.now();
    let b = c.now();
    assert!(b >= a);
}
