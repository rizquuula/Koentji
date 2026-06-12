//! Integration coverage for the dashboard "Expiring Soon" early-warning
//! widget. The pool-level `expiring_keys` query is exercised directly with a
//! fixed `now`, so the 30-day window and ordering are pinned without leaning
//! on the wall clock.
//!
//! Active keys (`deleted_at IS NULL`) whose `expired_at` falls in
//! `(now, now + 30 days]` must surface, soonest first, capped at ten. Deleted
//! keys, already-expired keys, and keys lapsing beyond the window stay out.

#![cfg(feature = "ssr")]

mod common;

use chrono::{Duration, TimeZone, Utc};
use common::fresh_pool;
use koentji::server::insights_service::expiring_keys;

#[tokio::test]
async fn expiring_keys_returns_only_the_next_30_days_soonest_first() {
    let pool = fresh_pool().await;

    // A fixed reference instant so the window boundaries are deterministic.
    let now = Utc.with_ymd_and_hms(2026, 6, 12, 12, 0, 0).unwrap();

    let in_3_days = now + Duration::days(3);
    let in_20_days = now + Duration::days(20);
    let in_60_days = now + Duration::days(60);

    // Surfaces: inside the window, active.
    common::a_key()
        .with_key("klab_exp_20d")
        .with_device("dev-20d")
        .with_username("alice")
        .expires_at(in_20_days)
        .insert(&pool)
        .await;

    common::a_key()
        .with_key("klab_exp_3d")
        .with_device("dev-3d")
        .with_email("soon@example.com")
        .expires_at(in_3_days)
        .insert(&pool)
        .await;

    // Excluded: beyond the 30-day window.
    common::a_key()
        .with_key("klab_exp_60d")
        .with_device("dev-60d")
        .expires_at(in_60_days)
        .insert(&pool)
        .await;

    // Excluded: deleted, even though it would otherwise be in-window.
    common::a_key()
        .with_key("klab_exp_deleted")
        .with_device("dev-deleted")
        .expires_at(in_3_days)
        .revoked()
        .insert(&pool)
        .await;

    // Excluded: already expired (expired_at <= now).
    common::a_key()
        .with_key("klab_exp_past")
        .with_device("dev-past")
        .expires_at(now - Duration::days(1))
        .insert(&pool)
        .await;

    let rows = expiring_keys(&pool, now)
        .await
        .expect("query expiring keys");

    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(
        keys,
        vec!["klab_exp_3d", "klab_exp_20d"],
        "only in-window active keys, soonest first"
    );

    // Days-left is the ceiling of the remaining duration: 3 and 20 here.
    assert_eq!(rows[0].days_left, 3, "3-day key reports 3 days left");
    assert_eq!(rows[1].days_left, 20, "20-day key reports 20 days left");

    // Owner fields ride through for the UI's Owner column.
    assert_eq!(rows[0].email.as_deref(), Some("soon@example.com"));
    assert_eq!(rows[1].username.as_deref(), Some("alice"));

    // Clean up the rows this test seeded.
    sqlx::query("DELETE FROM authentication_keys WHERE key LIKE 'klab_exp_%'")
        .execute(&pool)
        .await
        .expect("clean up seeded keys");
}
