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
use koentji::server::insights_service::{expiring_keys, recent_admin_activity, tier_health};
use koentji::server::stats_service::subscription_distribution;
use serde_json::json;

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

/// `recent_admin_activity` is the first read path for `audit_log`. Seed twelve
/// rows with distinct `occurred_at` values across all real event types, then
/// assert the helper returns the newest ten, newest first, with a rendered
/// summary per event type.
#[tokio::test]
async fn recent_admin_activity_returns_newest_ten_with_summaries() {
    let pool = fresh_pool().await;

    let base = Utc.with_ymd_and_hms(2026, 6, 12, 12, 0, 0).unwrap();
    const TEST_ACTOR: &str = "klab_test_admin";

    // Twelve events, oldest (offset 0) to newest (offset 11). The payload
    // shapes mirror `audit_event_repository::encode`. Cycling the event types
    // exercises every summary branch within the most-recent ten.
    let events: Vec<(&str, i32, serde_json::Value)> = vec![
        (
            "KeyIssued",
            1,
            json!({ "device": "dev-1", "subscription": "free" }),
        ),
        ("KeyRevoked", 2, json!({ "device": "dev-2" })),
        (
            "DeviceReassigned",
            3,
            json!({ "previous_device": "old-3", "current_device": "new-3" }),
        ),
        ("RateLimitReset", 4, json!({ "device": "dev-4" })),
        (
            "KeyExpirationExtended",
            5,
            json!({ "device": "dev-5", "new_expiry": "2026-07-01" }),
        ),
        (
            "KeyIssued",
            6,
            json!({ "device": "dev-6", "subscription": "pro" }),
        ),
        ("KeyRevoked", 7, json!({ "device": "dev-7" })),
        (
            "DeviceReassigned",
            8,
            json!({ "previous_device": "old-8", "current_device": "new-8" }),
        ),
        ("RateLimitReset", 9, json!({ "device": "dev-9" })),
        (
            "KeyExpirationExtended",
            10,
            json!({ "device": "dev-10", "new_expiry": "2026-08-01" }),
        ),
        (
            "KeyIssued",
            11,
            json!({ "device": "dev-11", "subscription": "enterprise" }),
        ),
        ("KeyRevoked", 12, json!({ "device": "dev-12" })),
    ];

    for (offset, (event_type, aggregate_id, payload)) in events.iter().enumerate() {
        let occurred_at = base + Duration::minutes(offset as i64);
        sqlx::query(
            r#"INSERT INTO audit_log (event_type, aggregate_id, actor, payload, occurred_at)
               VALUES ($1, $2, $3, $4, $5)"#,
        )
        .bind(event_type)
        .bind(aggregate_id)
        .bind(TEST_ACTOR)
        .bind(payload)
        .bind(occurred_at)
        .execute(&pool)
        .await
        .expect("seed audit_log row");
    }

    let activity = recent_admin_activity(&pool, 10)
        .await
        .expect("query recent admin activity");

    // Capped at ten, newest first — the two oldest (offsets 0 and 1) drop off.
    assert_eq!(activity.len(), 10, "limit caps the feed at ten");
    let aggregate_ids: Vec<Option<i32>> = activity.iter().map(|e| e.aggregate_id).collect();
    assert_eq!(
        aggregate_ids,
        vec![
            Some(12),
            Some(11),
            Some(10),
            Some(9),
            Some(8),
            Some(7),
            Some(6),
            Some(5),
            Some(4),
            Some(3),
        ],
        "newest first, oldest two dropped"
    );

    // Each event type renders its summary sentence.
    assert_eq!(activity[0].event_type, "KeyRevoked");
    assert_eq!(activity[0].summary, "Key #12 revoked on device dev-12");
    assert_eq!(activity[1].event_type, "KeyIssued");
    assert_eq!(
        activity[1].summary,
        "Key #11 issued on device dev-11 (enterprise)"
    );
    assert_eq!(activity[2].event_type, "KeyExpirationExtended");
    assert_eq!(
        activity[2].summary,
        "Key #10 expiration extended to 2026-08-01"
    );
    assert_eq!(activity[3].event_type, "RateLimitReset");
    assert_eq!(
        activity[3].summary,
        "Rate limit for Key #9 reset on device dev-9"
    );
    assert_eq!(activity[4].event_type, "DeviceReassigned");
    assert_eq!(
        activity[4].summary,
        "Device for Key #8 reassigned from old-8 to new-8"
    );

    // The actor rides through for any later attribution column.
    assert!(activity.iter().all(|e| e.actor == TEST_ACTOR));

    // Clean up the rows this test seeded.
    sqlx::query("DELETE FROM audit_log WHERE actor = $1")
        .bind(TEST_ACTOR)
        .execute(&pool)
        .await
        .expect("clean up seeded audit rows");
}

/// `tier_health` reports one row per subscription tier — including inactive
/// and zero-key tiers — carrying its quota, interval, active state, and a live
/// key count. The headline anomaly the widget exists to surface is an
/// *inactive* tier that still has live keys, so this seeds exactly that case.
///
/// Seeds a temp interval + two temp tiers (one active, one inactive) and three
/// keys: an active key on the active tier, an active key on the INACTIVE tier,
/// and a deleted key on the active tier that must not count. All seeded rows
/// carry the `klab_tier_` prefix and are cleaned up FK-safe (keys first, then
/// tiers, then the interval).
#[tokio::test]
async fn tier_health_counts_live_keys_per_tier_and_flags_inactive_with_keys() {
    let pool = fresh_pool().await;

    // A recognizable temp interval the two tiers point at.
    let interval_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO rate_limit_intervals (name, display_name, duration_seconds)
           VALUES ('klab_tier_interval', 'Klab Tier Window', 3600)
           RETURNING id"#,
    )
    .fetch_one(&pool)
    .await
    .expect("seed temp interval");

    // An ACTIVE tier and an INACTIVE tier, both on the temp interval, with
    // distinct quotas so the ride-through is observable.
    let active_tier_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO subscription_types
               (name, display_name, rate_limit_amount, rate_limit_interval_id, is_active)
           VALUES ('klab_tier_active', 'Klab Tier Active', 12000, $1, true)
           RETURNING id"#,
    )
    .bind(interval_id)
    .fetch_one(&pool)
    .await
    .expect("seed active tier");

    let inactive_tier_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO subscription_types
               (name, display_name, rate_limit_amount, rate_limit_interval_id, is_active)
           VALUES ('klab_tier_inactive', 'Klab Tier Inactive', 34000, $1, false)
           RETURNING id"#,
    )
    .bind(interval_id)
    .fetch_one(&pool)
    .await
    .expect("seed inactive tier");

    // Active key on the active tier — counts.
    sqlx::query(
        r#"INSERT INTO authentication_keys
               (key, device_id, subscription, rate_limit_daily, rate_limit_remaining,
                subscription_type_id, rate_limit_interval_id, created_by)
           VALUES ('klab_tier_k_active', 'klab_tier_d1', 'klab_tier_active', 12000, 12000,
                   $1, $2, 'test')"#,
    )
    .bind(active_tier_id)
    .bind(interval_id)
    .execute(&pool)
    .await
    .expect("seed active-tier key");

    // Deleted key on the active tier — must NOT count.
    sqlx::query(
        r#"INSERT INTO authentication_keys
               (key, device_id, subscription, rate_limit_daily, rate_limit_remaining,
                subscription_type_id, rate_limit_interval_id, created_by, deleted_at)
           VALUES ('klab_tier_k_deleted', 'klab_tier_d2', 'klab_tier_active', 12000, 12000,
                   $1, $2, 'test', NOW())"#,
    )
    .bind(active_tier_id)
    .bind(interval_id)
    .execute(&pool)
    .await
    .expect("seed deleted active-tier key");

    // Active key on the INACTIVE tier — the anomaly: an inactive tier with a
    // live key.
    sqlx::query(
        r#"INSERT INTO authentication_keys
               (key, device_id, subscription, rate_limit_daily, rate_limit_remaining,
                subscription_type_id, rate_limit_interval_id, created_by)
           VALUES ('klab_tier_k_inactive', 'klab_tier_d3', 'klab_tier_inactive', 34000, 34000,
                   $1, $2, 'test')"#,
    )
    .bind(inactive_tier_id)
    .bind(interval_id)
    .execute(&pool)
    .await
    .expect("seed inactive-tier key");

    let rows = tier_health(&pool).await.expect("query tier health");

    let active = rows
        .iter()
        .find(|r| r.display_name == "Klab Tier Active")
        .expect("active tier present");
    let inactive = rows
        .iter()
        .find(|r| r.display_name == "Klab Tier Inactive")
        .expect("inactive tier present");

    // The deleted key is excluded; only the one live key counts.
    assert_eq!(
        active.active_keys, 1,
        "active tier: only the live key counts"
    );
    assert!(active.is_active, "active tier flagged active");
    assert_eq!(
        active.rate_limit_amount, 12000,
        "active tier quota rides through"
    );
    assert_eq!(
        active.interval, "Klab Tier Window",
        "active tier interval rides through"
    );

    // The inactive tier still has a live key — the anomaly.
    assert_eq!(
        inactive.active_keys, 1,
        "inactive tier still has a live key"
    );
    assert!(!inactive.is_active, "inactive tier flagged inactive");
    assert_eq!(
        inactive.rate_limit_amount, 34000,
        "inactive tier quota rides through"
    );

    // Clean up FK-safe: keys, then tiers, then the interval. (Seeded catalogue
    // rows in the shared DB are otherwise long-lived.)
    sqlx::query("DELETE FROM authentication_keys WHERE key LIKE 'klab_tier_%'")
        .execute(&pool)
        .await
        .expect("clean up seeded keys");
    sqlx::query("DELETE FROM subscription_types WHERE name LIKE 'klab_tier_%'")
        .execute(&pool)
        .await
        .expect("clean up seeded tiers");
    sqlx::query("DELETE FROM rate_limit_intervals WHERE name LIKE 'klab_tier_%'")
        .execute(&pool)
        .await
        .expect("clean up seeded interval");
}

/// The subscription-distribution chart groups by the `subscription_types` FK
/// (`display_name`) and only falls back to the legacy `subscription` VARCHAR
/// for keys with no FK mapping. Seeds one mapped key (groups under the tier's
/// display_name) and one unmapped key (NULL FK, legacy string only — groups
/// under that raw string). Window binds are `(None, None)` so all rows count.
#[tokio::test]
async fn subscription_distribution_groups_by_tier_display_name_with_legacy_fallback() {
    let pool = fresh_pool().await;

    // A recognizable temp interval + tier so the mapped key has a display_name
    // distinct from its legacy `subscription` string.
    let interval_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO rate_limit_intervals (name, display_name, duration_seconds)
           VALUES ('klab_dist_interval', 'Klab Dist Window', 3600)
           RETURNING id"#,
    )
    .fetch_one(&pool)
    .await
    .expect("seed temp interval");

    let tier_id: i32 = sqlx::query_scalar(
        r#"INSERT INTO subscription_types
               (name, display_name, rate_limit_amount, rate_limit_interval_id, is_active)
           VALUES ('klab_dist_tier', 'Klab Dist Tier', 9000, $1, true)
           RETURNING id"#,
    )
    .bind(interval_id)
    .fetch_one(&pool)
    .await
    .expect("seed tier");

    // Mapped key: FK set — must group under the tier's display_name, NOT its
    // legacy `subscription` string.
    sqlx::query(
        r#"INSERT INTO authentication_keys
               (key, device_id, subscription, rate_limit_daily, rate_limit_remaining,
                subscription_type_id, rate_limit_interval_id, created_by)
           VALUES ('klab_dist_mapped', 'klab_dist_d1', 'klab_dist_tier', 9000, 9000,
                   $1, $2, 'test')"#,
    )
    .bind(tier_id)
    .bind(interval_id)
    .execute(&pool)
    .await
    .expect("seed mapped key");

    // Unmapped key: NULL FK, only a legacy `subscription` string — must group
    // under that raw string.
    sqlx::query(
        r#"INSERT INTO authentication_keys
               (key, device_id, subscription, rate_limit_daily, rate_limit_remaining,
                subscription_type_id, rate_limit_interval_id, created_by)
           VALUES ('klab_dist_legacy', 'klab_dist_d2', 'klab_dist_legacy_label', 9000, 9000,
                   NULL, $1, 'test')"#,
    )
    .bind(interval_id)
    .execute(&pool)
    .await
    .expect("seed legacy key");

    // No window — all rows count.
    let rows = subscription_distribution(&pool, None, None)
        .await
        .expect("query subscription distribution");

    let mapped = rows
        .iter()
        .find(|(label, _)| label == "Klab Dist Tier")
        .expect("mapped key groups under tier display_name");
    assert_eq!(mapped.1, 1, "one mapped key under the tier display_name");

    let legacy = rows
        .iter()
        .find(|(label, _)| label == "klab_dist_legacy_label")
        .expect("unmapped key groups under its legacy string");
    assert_eq!(
        legacy.1, 1,
        "one legacy key under its raw subscription string"
    );

    // The mapped key must NOT also appear under its legacy `subscription`
    // string — the FK display_name wins.
    assert!(
        !rows.iter().any(|(label, _)| label == "klab_dist_tier"),
        "mapped key does not leak under its raw FK name"
    );

    // Clean up FK-safe: keys, then tier, then interval.
    sqlx::query("DELETE FROM authentication_keys WHERE key LIKE 'klab_dist_%'")
        .execute(&pool)
        .await
        .expect("clean up seeded keys");
    sqlx::query("DELETE FROM subscription_types WHERE name LIKE 'klab_dist_%'")
        .execute(&pool)
        .await
        .expect("clean up seeded tier");
    sqlx::query("DELETE FROM rate_limit_intervals WHERE name LIKE 'klab_dist_%'")
        .execute(&pool)
        .await
        .expect("clean up seeded interval");
}
