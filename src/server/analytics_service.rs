use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnalyticsRange {
    Last24h,
    Last7d,
    Last30d,
}

impl AnalyticsRange {
    /// Bucket size in seconds for the requested window.
    pub fn bucket_seconds(self) -> u32 {
        match self {
            AnalyticsRange::Last24h => 60,
            AnalyticsRange::Last7d => 900,
            AnalyticsRange::Last30d => 3600,
        }
    }

    /// Total window length in seconds.
    pub fn range_seconds(self) -> u32 {
        match self {
            AnalyticsRange::Last24h => 86_400,
            AnalyticsRange::Last7d => 604_800,
            AnalyticsRange::Last30d => 2_592_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBucket {
    pub ts_unix_ms: i64,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowDenyCounts {
    pub allowed: u64,
    pub denied: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpsResult {
    pub points: Vec<TimeBucket>,
    pub svg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowDenyResult {
    pub counts: AllowDenyCounts,
    pub svg: String,
}

#[cfg(feature = "ssr")]
async fn require_admin() -> Result<(), ServerFnError> {
    use actix_session::Session;
    use leptos_actix::extract;

    let session = extract::<Session>().await?;
    let username = session
        .get::<String>("username")
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?;
    if username.is_none() {
        return Err(ServerFnError::ServerError("unauthorized".into()));
    }
    Ok(())
}

#[cfg(feature = "ssr")]
#[derive(clickhouse::Row, serde::Deserialize)]
struct TimeBucketRow {
    ts_unix_ms: i64,
    count: u64,
}

#[cfg(feature = "ssr")]
#[derive(clickhouse::Row, serde::Deserialize)]
struct AllowDenyRow {
    allowed: u64,
    denied: u64,
}

#[server(GetRequestsPerSecond, "/api")]
pub async fn get_requests_per_second(range: AnalyticsRange) -> Result<RpsResult, ServerFnError> {
    require_admin().await?;

    use actix_web::web;
    use leptos_actix::extract;

    let client = extract::<web::Data<clickhouse::Client>>().await?;
    let bucket_secs = range.bucket_seconds();
    let range_secs = range.range_seconds();

    let rows: Vec<TimeBucketRow> = client
        .query(
            "SELECT toUnixTimestamp64Milli(toStartOfInterval(ts, INTERVAL ? second)) AS ts_unix_ms,
                    count() AS count
             FROM auth_events
             WHERE ts >= now() - INTERVAL ? second
             GROUP BY ts_unix_ms
             ORDER BY ts_unix_ms",
        )
        .bind(bucket_secs)
        .bind(range_secs)
        .fetch_all::<TimeBucketRow>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let points: Vec<TimeBucket> = rows
        .into_iter()
        .map(|r| TimeBucket {
            ts_unix_ms: r.ts_unix_ms,
            count: r.count,
        })
        .collect();

    let svg = crate::ui::analytics::charts::render_rps_svg(&points);
    Ok(RpsResult { points, svg })
}

#[server(GetAllowDenyCounts, "/api")]
pub async fn get_allow_deny_counts(
    range: AnalyticsRange,
) -> Result<AllowDenyResult, ServerFnError> {
    require_admin().await?;

    use actix_web::web;
    use leptos_actix::extract;

    let client = extract::<web::Data<clickhouse::Client>>().await?;
    let range_secs = range.range_seconds();

    let row: AllowDenyRow = client
        .query(
            "SELECT
                countIf(decision = 'allowed') AS allowed,
                countIf(decision = 'denied') AS denied
             FROM auth_events
             WHERE ts >= now() - INTERVAL ? second",
        )
        .bind(range_secs)
        .fetch_one::<AllowDenyRow>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let counts = AllowDenyCounts {
        allowed: row.allowed,
        denied: row.denied,
    };
    let svg = crate::ui::analytics::charts::render_allow_deny_svg(&counts);
    Ok(AllowDenyResult { counts, svg })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_seconds_matches_range() {
        assert_eq!(AnalyticsRange::Last24h.bucket_seconds(), 60);
        assert_eq!(AnalyticsRange::Last7d.bucket_seconds(), 900);
        assert_eq!(AnalyticsRange::Last30d.bucket_seconds(), 3600);
    }

    #[test]
    fn range_seconds_matches_range() {
        assert_eq!(AnalyticsRange::Last24h.range_seconds(), 86_400);
        assert_eq!(AnalyticsRange::Last7d.range_seconds(), 604_800);
        assert_eq!(AnalyticsRange::Last30d.range_seconds(), 2_592_000);
    }
}
