use crate::models::DashboardInsights;
use leptos::prelude::*;

/// Current-state dashboard insights — the "Expiring Soon" early-warning list.
/// Unlike `get_dashboard_stats`, this ignores the date-range picker: it always
/// reports the live picture so an admin can act before keys lapse.
#[server]
pub async fn get_dashboard_insights() -> Result<DashboardInsights, ServerFnError> {
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

    Ok(DashboardInsights {
        expiring_keys: expiring,
        recent_activity,
    })
}

/// The ≤10 soonest-expiring active keys lapsing within the next 30 days,
/// soonest first. `now` is a parameter so tests drive the window with a fixed
/// clock. Exposed at the pool level (mirroring `resolve_date_range`) so the
/// integration suite can call it without a Leptos request context.
///
/// Active means `deleted_at IS NULL`; the window is the half-open interval
/// `(now, now + 30 days]` so an already-expired key never shows.
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
                 AND expired_at <= $1 + INTERVAL '30 days'
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
    use super::activity_summary;
    use serde_json::json;

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
