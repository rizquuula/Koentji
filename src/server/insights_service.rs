use crate::models::DashboardInsights;
use leptos::prelude::*;

/// Current-state dashboard insights — the "Expiring Soon" early-warning list.
/// Unlike `get_dashboard_stats`, this ignores the date-range picker: it always
/// reports the live picture so an admin can act before keys lapse.
#[server]
pub async fn get_dashboard_insights() -> Result<DashboardInsights, ServerFnError> {
    super::require_admin().await?;

    use chrono::Utc;
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let expiring = expiring_keys(pool.get_ref(), Utc::now())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let recent_activity = recent_admin_activity(pool.get_ref(), 10)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let tier_health = tier_health(pool.get_ref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let unclaimed = unclaimed_keys(pool.get_ref(), 10)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let dormant = dormant_keys(pool.get_ref(), 10)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let key_hygiene = crate::models::KeyHygiene {
        unclaimed: unclaimed.rows,
        unclaimed_total: unclaimed.total,
        dormant: dormant.rows,
        dormant_total: dormant.total,
    };

    Ok(DashboardInsights {
        expiring_keys: expiring,
        recent_activity,
        tier_health,
        key_hygiene,
    })
}

/// One row per subscription tier — *every* tier, including inactive and
/// zero-key ones — with its quota, interval, active flag, and a count of the
/// *live* keys it carries (not deleted, not yet expired). The `LEFT JOIN`
/// keeps tiers with no keys; the `FILTER` excludes deleted/expired keys from
/// the count without dropping the tier row. Ordered most-populated first,
/// display_name as the stable tiebreak. Exposed at the pool level (mirroring
/// `expiring_keys`) so the integration suite can call it without a Leptos
/// request context.
#[cfg(feature = "ssr")]
pub async fn tier_health(
    pool: &sqlx::PgPool,
) -> Result<Vec<crate::models::TierHealth>, sqlx::Error> {
    let rows: Vec<TierHealthRow> = sqlx::query_as(
        r#"SELECT st.display_name,
                  st.rate_limit_amount::bigint AS rate_limit_amount,
                  rli.display_name AS interval,
                  st.is_active,
                  COUNT(ak.id) FILTER (
                      WHERE ak.deleted_at IS NULL
                        AND (ak.expired_at IS NULL OR ak.expired_at > NOW())
                  ) AS active_keys
               FROM subscription_types st
               JOIN rate_limit_intervals rli ON rli.id = st.rate_limit_interval_id
               LEFT JOIN authentication_keys ak ON ak.subscription_type_id = st.id
               GROUP BY st.id, st.display_name, st.rate_limit_amount, rli.display_name, st.is_active
               ORDER BY active_keys DESC, st.display_name"#,
    )
    .fetch_all(pool)
    .await?;

    let tiers = rows
        .into_iter()
        .map(|row| crate::models::TierHealth {
            display_name: row.display_name,
            rate_limit_amount: row.rate_limit_amount,
            interval: row.interval,
            is_active: row.is_active,
            active_keys: row.active_keys,
        })
        .collect();

    Ok(tiers)
}

/// Raw projection of the tier-health query. Kept private to this module — the
/// public type is `crate::models::TierHealth`.
#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow)]
struct TierHealthRow {
    display_name: String,
    rate_limit_amount: i64,
    interval: String,
    is_active: bool,
    active_keys: i64,
}

/// The ≤10 soonest-expiring active keys lapsing within the next 90 days,
/// soonest first. `now` is a parameter so tests drive the window with a fixed
/// clock. Exposed at the pool level (mirroring `resolve_date_range`) so the
/// integration suite can call it without a Leptos request context.
///
/// Active means `deleted_at IS NULL`; the window is the half-open interval
/// `(now, now + 90 days]` so an already-expired key never shows.
#[cfg(feature = "ssr")]
pub async fn expiring_keys(
    pool: &sqlx::PgPool,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<crate::models::ExpiringKey>, sqlx::Error> {
    let rows: Vec<ExpiringKeyRow> = sqlx::query_as(
        r#"SELECT key, username, email, device_id, expired_at
               FROM authentication_keys
               WHERE deleted_at IS NULL
                 AND expired_at > $1
                 AND expired_at <= $1 + INTERVAL '90 days'
               ORDER BY expired_at ASC
               LIMIT 10"#,
    )
    .bind(now)
    .fetch_all(pool)
    .await?;

    let expiring = rows
        .into_iter()
        .map(|row| crate::models::ExpiringKey {
            days_left: days_left(now, row.expired_at),
            key: row.key,
            username: row.username,
            email: row.email,
            device_id: row.device_id,
            expired_at: row.expired_at,
        })
        .collect();

    Ok(expiring)
}

/// Raw projection of the expiring-keys query before `days_left` is folded in.
/// Kept private to this module — the public type is `crate::models::ExpiringKey`.
#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow)]
struct ExpiringKeyRow {
    key: String,
    username: Option<String>,
    email: Option<String>,
    device_id: String,
    expired_at: chrono::DateTime<chrono::Utc>,
}

/// Whole days until `expired_at`, rounded up so a partial day still reads as a
/// full day of runway (2.5 days left → "3 days", never "2"). Pure, so the
/// arithmetic is pinned by the integration test's fixed clock.
#[cfg(feature = "ssr")]
fn days_left(now: chrono::DateTime<chrono::Utc>, expired_at: chrono::DateTime<chrono::Utc>) -> i64 {
    let seconds = (expired_at - now).num_seconds();
    // Ceiling division; `i64::div_ceil` is still unstable on this toolchain.
    (seconds + SECONDS_PER_DAY - 1) / SECONDS_PER_DAY
}

#[cfg(feature = "ssr")]
const SECONDS_PER_DAY: i64 = 86_400;

/// The unclaimed-device sentinel: a pre-issued key not yet bound to a real
/// device carries `device_id = '-'`. Mirrors `domain::authentication::DeviceId`.
#[cfg(feature = "ssr")]
const UNCLAIMED_SENTINEL: &str = "-";

/// Pre-issued keys still on the `-` sentinel — issued, never adopted by a real
/// device — oldest first (they've waited longest), capped at `limit`. Returns
/// the capped rows plus the full population `total` (via `COUNT(*) OVER ()`) so
/// the panel can render "Showing N of M". Active only: `deleted_at IS NULL`.
/// Exposed at the pool level (mirroring `expiring_keys`) so the integration
/// suite can call it without a Leptos request context.
#[cfg(feature = "ssr")]
pub async fn unclaimed_keys(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<crate::models::HygieneSet, sqlx::Error> {
    let rows: Vec<HygieneRow> = sqlx::query_as(
        r#"SELECT key, username, email, device_id, created_at,
                  COUNT(*) OVER () AS total
               FROM authentication_keys
               WHERE device_id = $1
                 AND deleted_at IS NULL
               ORDER BY created_at ASC
               LIMIT $2"#,
    )
    .bind(UNCLAIMED_SENTINEL)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    // The sentinel device is not a real binding — drop it from the row so the
    // view shows "—" rather than the bare `-`.
    Ok(into_hygiene_set(rows, chrono::Utc::now(), false))
}

/// Dormant keys: *claimed* (a real device, not the `-` sentinel) and live, with
/// quota never touched (`rate_limit_remaining >= rate_limit_daily` — `>=` is
/// float-safe and tolerates a manual quota bump). Excludes the sentinel so a
/// pre-issued row never double-counts as both unclaimed and dormant. Oldest
/// first, capped at `limit`, with the full population `total` alongside.
/// Exposed at the pool level so the integration suite can call it directly.
#[cfg(feature = "ssr")]
pub async fn dormant_keys(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<crate::models::HygieneSet, sqlx::Error> {
    let rows: Vec<HygieneRow> = sqlx::query_as(
        r#"SELECT key, username, email, device_id, created_at,
                  COUNT(*) OVER () AS total
               FROM authentication_keys
               WHERE device_id <> $1
                 AND deleted_at IS NULL
                 AND (expired_at IS NULL OR expired_at > NOW())
                 AND rate_limit_remaining >= rate_limit_daily
               ORDER BY created_at ASC
               LIMIT $2"#,
    )
    .bind(UNCLAIMED_SENTINEL)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(into_hygiene_set(rows, chrono::Utc::now(), true))
}

/// Raw projection of the hygiene queries. `total` rides on every row via
/// `COUNT(*) OVER ()` (the same value on each), so an empty result set yields a
/// `total` of 0. Kept private — the public types are `crate::models::HygieneSet`
/// and `crate::models::HygieneKey`.
#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow)]
struct HygieneRow {
    key: String,
    username: Option<String>,
    email: Option<String>,
    device_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    total: i64,
}

/// Fold raw hygiene rows into a `HygieneSet`, computing `age_days` against
/// `now` and keeping the device only when `keep_device` (dormant rows carry a
/// real device; unclaimed rows drop the `-` sentinel). The `total` is read off
/// the first row — every row carries the same window count — and is 0 for an
/// empty set.
#[cfg(feature = "ssr")]
fn into_hygiene_set(
    rows: Vec<HygieneRow>,
    now: chrono::DateTime<chrono::Utc>,
    keep_device: bool,
) -> crate::models::HygieneSet {
    let total = rows.first().map(|r| r.total).unwrap_or(0);
    let keys = rows
        .into_iter()
        .map(|row| crate::models::HygieneKey {
            age_days: age_days(now, row.created_at),
            device_id: keep_device.then_some(row.device_id),
            key: row.key,
            username: row.username,
            email: row.email,
            created_at: row.created_at,
        })
        .collect();

    crate::models::HygieneSet { rows: keys, total }
}

/// Whole days a key has existed: floor of the elapsed duration since
/// `created_at`, so a key reads as "12d" once it's at least twelve full days
/// old. Clock skew (a `created_at` in the future) clamps to 0. Pure, so the
/// arithmetic is pinned by unit tests.
#[cfg(feature = "ssr")]
fn age_days(now: chrono::DateTime<chrono::Utc>, created_at: chrono::DateTime<chrono::Utc>) -> i64 {
    let seconds = (now - created_at).num_seconds();
    if seconds <= 0 {
        return 0;
    }
    seconds / SECONDS_PER_DAY
}

/// The last `limit` admin events, newest first. This is the first read path
/// for `audit_log` (until now write-only); `(occurred_at DESC, id DESC)` rides
/// the `idx_audit_log_occurred_at` index and breaks ties on the monotonic id
/// so same-instant rows still order deterministically. Exposed at the pool
/// level (mirroring `expiring_keys`) so the integration suite can call it
/// without a Leptos request context.
#[cfg(feature = "ssr")]
pub async fn recent_admin_activity(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<crate::models::AuditEntry>, sqlx::Error> {
    let rows: Vec<AuditLogRow> = sqlx::query_as(
        r#"SELECT event_type, aggregate_id, actor, payload, occurred_at
               FROM audit_log
               ORDER BY occurred_at DESC, id DESC
               LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let activity = rows
        .into_iter()
        .map(|row| crate::models::AuditEntry {
            summary: activity_summary(&row.event_type, row.aggregate_id, &row.payload),
            event_type: row.event_type,
            aggregate_id: row.aggregate_id,
            actor: row.actor,
            occurred_at: row.occurred_at,
        })
        .collect();

    Ok(activity)
}

/// Raw projection of the `audit_log` query before the summary is folded in.
/// Kept private to this module — the public type is `crate::models::AuditEntry`.
#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow)]
struct AuditLogRow {
    event_type: String,
    aggregate_id: Option<i32>,
    actor: String,
    payload: serde_json::Value,
    occurred_at: chrono::DateTime<chrono::Utc>,
}

/// A human sentence for one audit event, built from its type and JSONB
/// payload. Reads payload fields defensively: a missing or wrong-typed field
/// never panics — it falls back to the bare verb. Unknown event types degrade
/// to the raw `event_type`. Pure, so the wording is pinned by unit tests.
#[cfg(feature = "ssr")]
fn activity_summary(
    event_type: &str,
    aggregate_id: Option<i32>,
    payload: &serde_json::Value,
) -> String {
    let key_ref = match aggregate_id {
        Some(id) => format!("Key #{id}"),
        None => "Key".to_string(),
    };
    let field = |name: &str| {
        payload
            .get(name)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };

    match event_type {
        "KeyIssued" => {
            let Some(device) = field("device") else {
                return format!("{key_ref} issued");
            };
            match field("subscription") {
                Some(subscription) => {
                    format!("{key_ref} issued on device {device} ({subscription})")
                }
                None => format!("{key_ref} issued on device {device}"),
            }
        }
        "KeyRevoked" => {
            let Some(device) = field("device") else {
                return format!("{key_ref} revoked");
            };
            format!("{key_ref} revoked on device {device}")
        }
        "KeyUnrevoked" => {
            let Some(device) = field("device") else {
                return format!("{key_ref} unrevoked");
            };
            format!("{key_ref} unrevoked on device {device}")
        }
        "DeviceReassigned" => {
            let (Some(previous), Some(current)) =
                (field("previous_device"), field("current_device"))
            else {
                return format!("Device for {key_ref} reassigned");
            };
            format!("Device for {key_ref} reassigned from {previous} to {current}")
        }
        "RateLimitReset" => {
            let Some(device) = field("device") else {
                return format!("Rate limit for {key_ref} reset");
            };
            format!("Rate limit for {key_ref} reset on device {device}")
        }
        "KeyExpirationExtended" => {
            let Some(new_expiry) = field("new_expiry") else {
                return format!("{key_ref} expiration extended");
            };
            format!("{key_ref} expiration extended to {new_expiry}")
        }
        other => other.to_string(),
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{activity_summary, age_days};
    use chrono::{Duration, TimeZone, Utc};
    use serde_json::json;

    #[test]
    fn age_days_floors_whole_days_elapsed() {
        let created = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        // Exactly 12 days later reads as 12.
        assert_eq!(age_days(created + Duration::days(12), created), 12);
        // A partial 13th day still reads as 12 (floor).
        assert_eq!(
            age_days(created + Duration::days(12) + Duration::hours(23), created),
            12
        );
    }

    #[test]
    fn age_days_brand_new_key_is_zero() {
        let created = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        assert_eq!(age_days(created, created), 0);
        assert_eq!(age_days(created + Duration::hours(5), created), 0);
    }

    #[test]
    fn age_days_future_created_at_clamps_to_zero() {
        let created = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        assert_eq!(age_days(created - Duration::days(3), created), 0);
    }

    #[test]
    fn key_issued_reads_device_and_subscription() {
        let summary = activity_summary(
            "KeyIssued",
            Some(42),
            &json!({ "device": "abc", "subscription": "pro" }),
        );
        assert_eq!(summary, "Key #42 issued on device abc (pro)");
    }

    #[test]
    fn key_revoked_reads_device() {
        let summary = activity_summary("KeyRevoked", Some(42), &json!({ "device": "abc" }));
        assert_eq!(summary, "Key #42 revoked on device abc");
    }

    #[test]
    fn key_unrevoked_reads_device() {
        let summary = activity_summary("KeyUnrevoked", Some(42), &json!({ "device": "abc" }));
        assert_eq!(summary, "Key #42 unrevoked on device abc");
    }

    #[test]
    fn device_reassigned_reads_both_devices() {
        let summary = activity_summary(
            "DeviceReassigned",
            Some(42),
            &json!({ "previous_device": "old", "current_device": "new" }),
        );
        assert_eq!(summary, "Device for Key #42 reassigned from old to new");
    }

    #[test]
    fn rate_limit_reset_reads_device() {
        let summary = activity_summary("RateLimitReset", Some(7), &json!({ "device": "dev-7" }));
        assert_eq!(summary, "Rate limit for Key #7 reset on device dev-7");
    }

    #[test]
    fn expiration_extended_reads_new_expiry() {
        let summary = activity_summary(
            "KeyExpirationExtended",
            Some(42),
            &json!({ "device": "abc", "new_expiry": "2026-07-01" }),
        );
        assert_eq!(summary, "Key #42 expiration extended to 2026-07-01");
    }

    #[test]
    fn missing_fields_fall_back_to_bare_verb() {
        assert_eq!(
            activity_summary("KeyRevoked", Some(42), &json!({})),
            "Key #42 revoked"
        );
        assert_eq!(
            activity_summary(
                "DeviceReassigned",
                Some(42),
                &json!({ "previous_device": "old" })
            ),
            "Device for Key #42 reassigned"
        );
        assert_eq!(
            activity_summary("KeyExpirationExtended", Some(42), &json!({})),
            "Key #42 expiration extended"
        );
    }

    #[test]
    fn null_aggregate_id_drops_the_number() {
        assert_eq!(
            activity_summary("KeyRevoked", None, &json!({ "device": "abc" })),
            "Key revoked on device abc"
        );
    }

    #[test]
    fn unknown_event_type_degrades_to_raw_type() {
        assert_eq!(
            activity_summary("SomethingNew", Some(42), &json!({})),
            "SomethingNew"
        );
    }

    #[test]
    fn wrong_typed_field_does_not_panic() {
        // A numeric `device` where a string is expected falls back gracefully.
        assert_eq!(
            activity_summary("KeyRevoked", Some(42), &json!({ "device": 99 })),
            "Key #42 revoked"
        );
    }
}
