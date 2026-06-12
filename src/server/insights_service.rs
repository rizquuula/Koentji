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

    Ok(DashboardInsights {
        expiring_keys: expiring,
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
