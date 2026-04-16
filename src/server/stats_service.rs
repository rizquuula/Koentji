use crate::models::DashboardStats;
use leptos::prelude::*;

#[server]
pub async fn get_dashboard_stats(
    range: String,
    start_date: String,
    end_date: String,
) -> Result<DashboardStats, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let (start, end) = resolve_date_range(&range, &start_date, &end_date);

    // Every query threads the two bounds as NULL-able parameters, so the
    // same SQL works whether a window was requested or not. This closes the
    // SQL-injection hole that the previous `format!`-based filter had on
    // the "custom" range branch (B8).
    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM authentication_keys
           WHERE ($1::timestamptz IS NULL OR created_at >= $1)
             AND ($2::timestamptz IS NULL OR created_at <= $2)"#,
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let active: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM authentication_keys
           WHERE deleted_at IS NULL
             AND (expired_at IS NULL OR expired_at > NOW())
             AND ($1::timestamptz IS NULL OR created_at >= $1)
             AND ($2::timestamptz IS NULL OR created_at <= $2)"#,
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let expired: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM authentication_keys
           WHERE expired_at IS NOT NULL AND expired_at <= NOW() AND deleted_at IS NULL
             AND ($1::timestamptz IS NULL OR created_at >= $1)
             AND ($2::timestamptz IS NULL OR created_at <= $2)"#,
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let deleted: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM authentication_keys
           WHERE deleted_at IS NOT NULL
             AND ($1::timestamptz IS NULL OR created_at >= $1)
             AND ($2::timestamptz IS NULL OR created_at <= $2)"#,
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let sub_rows: Vec<(Option<String>, i64)> = sqlx::query_as(
        r#"SELECT subscription, COUNT(*) as count FROM authentication_keys
           WHERE ($1::timestamptz IS NULL OR created_at >= $1)
             AND ($2::timestamptz IS NULL OR created_at <= $2)
           GROUP BY subscription
           ORDER BY count DESC"#,
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let subscription_distribution: Vec<(String, i64)> = sub_rows
        .into_iter()
        .map(|(s, c)| (s.unwrap_or_else(|| "None".to_string()), c))
        .collect();

    let rate_limit_buckets: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT
            CASE
                WHEN (rate_limit_daily - rate_limit_remaining)::float / NULLIF(rate_limit_daily, 0) <= 0.25 THEN '0-25%'
                WHEN (rate_limit_daily - rate_limit_remaining)::float / NULLIF(rate_limit_daily, 0) <= 0.50 THEN '25-50%'
                WHEN (rate_limit_daily - rate_limit_remaining)::float / NULLIF(rate_limit_daily, 0) <= 0.75 THEN '50-75%'
                ELSE '75-100%'
            END as bucket,
            COUNT(*) as count
        FROM authentication_keys
        WHERE deleted_at IS NULL
          AND ($1::timestamptz IS NULL OR created_at >= $1)
          AND ($2::timestamptz IS NULL OR created_at <= $2)
        GROUP BY bucket
        ORDER BY bucket"#,
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let daily_trend: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT DATE(created_at)::text as day, COUNT(*) as count FROM authentication_keys
           WHERE ($1::timestamptz IS NULL OR created_at >= $1)
             AND ($2::timestamptz IS NULL OR created_at <= $2)
           GROUP BY DATE(created_at)
           ORDER BY day"#,
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(DashboardStats {
        total,
        active,
        expired,
        deleted,
        subscription_distribution,
        rate_limit_buckets,
        daily_trend,
    })
}

/// Parse a range + (optional) custom start/end into a bounded window.
/// Exposed for regression coverage of the SQL-injection fix (B8):
/// malformed custom dates must degrade to `(None, None)` so nothing
/// user-controlled reaches a query — queries bind the bounds as
/// `Option<DateTime<Utc>>`, never as interpolated strings.
#[cfg(feature = "ssr")]
pub fn resolve_date_range(
    range: &str,
    start_date: &str,
    end_date: &str,
) -> (
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
) {
    use chrono::{Duration, NaiveDate, NaiveTime, TimeZone, Utc};

    let end_of_day = |d: NaiveDate| {
        Utc.from_utc_datetime(&d.and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap()))
    };
    let start_of_day = |d: NaiveDate| Utc.from_utc_datetime(&d.and_time(NaiveTime::MIN));

    match range {
        "7d" | "30d" | "90d" => {
            let days = match range {
                "7d" => 7,
                "30d" => 30,
                _ => 90,
            };
            let end = Utc::now();
            let start = end - Duration::days(days);
            (Some(start), Some(end))
        }
        "custom" => {
            // Reject anything that isn't a YYYY-MM-DD date — no free-form
            // strings reach the query layer, so there's nothing to inject.
            let parse = |s: &str| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok();
            match (parse(start_date), parse(end_date)) {
                (Some(s), Some(e)) => (Some(start_of_day(s)), Some(end_of_day(e))),
                _ => (None, None),
            }
        }
        _ => (None, None),
    }
}
