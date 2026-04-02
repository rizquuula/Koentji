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

    // Calculate date range
    let (start, end) = resolve_date_range(&range, &start_date, &end_date);

    let date_filter = if let (Some(s), Some(e)) = (&start, &end) {
        format!("AND created_at >= '{}' AND created_at <= '{}'", s, e)
    } else {
        String::new()
    };

    // Total counts
    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM authentication_keys WHERE 1=1 {}",
        date_filter
    ))
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let active: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM authentication_keys WHERE deleted_at IS NULL AND (expired_at IS NULL OR expired_at > NOW()) {}", date_filter
    ))
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let expired: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM authentication_keys WHERE expired_at IS NOT NULL AND expired_at <= NOW() AND deleted_at IS NULL {}", date_filter
    ))
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let deleted: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM authentication_keys WHERE deleted_at IS NOT NULL {}",
        date_filter
    ))
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Subscription distribution
    let sub_rows: Vec<(Option<String>, i64)> = sqlx::query_as(&format!(
        "SELECT subscription, COUNT(*) as count FROM authentication_keys WHERE 1=1 {} GROUP BY subscription ORDER BY count DESC",
        date_filter
    ))
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let subscription_distribution: Vec<(String, i64)> = sub_rows
        .into_iter()
        .map(|(s, c)| (s.unwrap_or_else(|| "None".to_string()), c))
        .collect();

    // Rate limit usage buckets
    let bucket_sql = format!(
        r#"SELECT
            CASE
                WHEN (rate_limit_daily - rate_limit_remaining)::float / NULLIF(rate_limit_daily, 0) <= 0.25 THEN '0-25%'
                WHEN (rate_limit_daily - rate_limit_remaining)::float / NULLIF(rate_limit_daily, 0) <= 0.50 THEN '25-50%'
                WHEN (rate_limit_daily - rate_limit_remaining)::float / NULLIF(rate_limit_daily, 0) <= 0.75 THEN '50-75%'
                ELSE '75-100%'
            END as bucket,
            COUNT(*) as count
        FROM authentication_keys
        WHERE deleted_at IS NULL {}
        GROUP BY bucket
        ORDER BY bucket"#,
        date_filter
    );
    let bucket_rows: Vec<(String, i64)> = sqlx::query_as(&bucket_sql)
        .fetch_all(pool.get_ref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Daily creation trend
    let trend_sql = format!(
        "SELECT DATE(created_at)::text as day, COUNT(*) as count FROM authentication_keys WHERE 1=1 {} GROUP BY DATE(created_at) ORDER BY day",
        date_filter
    );
    let daily_trend: Vec<(String, i64)> = sqlx::query_as(&trend_sql)
        .fetch_all(pool.get_ref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(DashboardStats {
        total,
        active,
        expired,
        deleted,
        subscription_distribution,
        rate_limit_buckets: bucket_rows,
        daily_trend,
    })
}

#[cfg(feature = "ssr")]
fn resolve_date_range(
    range: &str,
    start_date: &str,
    end_date: &str,
) -> (Option<String>, Option<String>) {
    use chrono::{Duration, Utc};

    match range {
        "7d" => {
            let end = Utc::now();
            let start = end - Duration::days(7);
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d 23:59:59").to_string()),
            )
        }
        "30d" => {
            let end = Utc::now();
            let start = end - Duration::days(30);
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d 23:59:59").to_string()),
            )
        }
        "90d" => {
            let end = Utc::now();
            let start = end - Duration::days(90);
            (
                Some(start.format("%Y-%m-%d").to_string()),
                Some(end.format("%Y-%m-%d 23:59:59").to_string()),
            )
        }
        "custom" => {
            if !start_date.is_empty() && !end_date.is_empty() {
                (
                    Some(start_date.to_string()),
                    Some(format!("{} 23:59:59", end_date)),
                )
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    }
}
