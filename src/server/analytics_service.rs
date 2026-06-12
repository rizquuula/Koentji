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

/// One time bucket of traffic: allowed + denied counts. The stack top
/// (allowed + denied) is the total request volume for the bucket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrafficBucket {
    pub ts_unix_ms: i64,
    pub allowed: u64,
    pub denied: u64,
}

/// One time bucket of latency percentiles in milliseconds, aligned to the
/// same bucket boundaries as `TrafficBucket`. Each percentile is `Option`
/// because a bucket with no events is a *gap*, not zero latency — a
/// zero-filled latency line would lie. The chart spans these gaps as breaks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LatencyBucket {
    pub ts_unix_ms: i64,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
}

/// One slice of the denial breakdown: a `denial_reason` and how many denials
/// carried it in the window. Allowed events carry an empty `denial_reason`,
/// so this only ever holds the genuine denial reasons.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DenialReasonCount {
    pub reason: String,
    pub count: u64,
}

/// One row of the busiest-keys table: a key's request volume, denials, and
/// the unix-seconds timestamp of its most recent event in the window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyTrafficRow {
    pub auth_key: String,
    pub requests: u64,
    pub denied: u64,
    pub last_seen_unix: i64,
}

/// Microseconds → milliseconds for display. Pure (TDD'd) so the conversion
/// lives in one tested place rather than scattered across the wire layer.
pub fn micros_to_millis(micros: f64) -> f64 {
    micros / 1000.0
}

/// Data-only analytics payload. The UI renders it client-side via Chart.js;
/// no server-rendered SVG. Grows in later milestones (more panels).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSnapshot {
    pub traffic: Vec<TrafficBucket>,
    pub latency: Vec<LatencyBucket>,
    pub denial_reasons: Vec<DenialReasonCount>,
    pub busiest_keys: Vec<KeyTrafficRow>,
}

/// Fill `bucket_seconds`-aligned gaps so sparse traffic doesn't render a
/// misleading chart. `queried` holds only the buckets that had events;
/// `now_unix_ms` is the window end. Every aligned bucket from window start
/// (`now - range_seconds`, snapped down to a bucket boundary) through the
/// `now` bucket is emitted, with missing ones as `{ allowed: 0, denied: 0 }`.
///
/// Pure: no ClickHouse, no async — unit-tested under plain `cargo test`.
pub fn fill_missing_buckets(
    queried: &[TrafficBucket],
    now_unix_ms: i64,
    range: AnalyticsRange,
) -> Vec<TrafficBucket> {
    let bucket_ms = range.bucket_seconds() as i64 * 1000;
    let range_ms = range.range_seconds() as i64 * 1000;

    // Snap both ends down to a bucket boundary, mirroring ClickHouse's
    // `toStartOfInterval` so queried timestamps land on these slots exactly.
    let last_bucket = (now_unix_ms / bucket_ms) * bucket_ms;
    let first_bucket = ((now_unix_ms - range_ms) / bucket_ms) * bucket_ms;

    let mut lookup = std::collections::HashMap::new();
    for b in queried {
        lookup.insert(b.ts_unix_ms, (b.allowed, b.denied));
    }

    let mut out = Vec::new();
    let mut ts = first_bucket;
    while ts <= last_bucket {
        let (allowed, denied) = lookup.get(&ts).copied().unwrap_or((0, 0));
        out.push(TrafficBucket {
            ts_unix_ms: ts,
            allowed,
            denied,
        });
        ts += bucket_ms;
    }
    out
}

/// Densify latency percentiles to the same `bucket_seconds`-aligned grid as
/// traffic, but missing buckets stay `None` rather than zero: a gap in the
/// latency line is honest about "no traffic", whereas a 0ms point would
/// imply impossibly fast requests. `queried` holds only buckets with events.
///
/// Pure: no ClickHouse, no async — unit-tested under plain `cargo test`.
pub fn fill_missing_latency_buckets(
    queried: &[LatencyBucket],
    now_unix_ms: i64,
    range: AnalyticsRange,
) -> Vec<LatencyBucket> {
    let bucket_ms = range.bucket_seconds() as i64 * 1000;
    let range_ms = range.range_seconds() as i64 * 1000;

    let last_bucket = (now_unix_ms / bucket_ms) * bucket_ms;
    let first_bucket = ((now_unix_ms - range_ms) / bucket_ms) * bucket_ms;

    let mut lookup = std::collections::HashMap::new();
    for b in queried {
        lookup.insert(b.ts_unix_ms, (b.p50_ms, b.p95_ms, b.p99_ms));
    }

    let mut out = Vec::new();
    let mut ts = first_bucket;
    while ts <= last_bucket {
        let (p50_ms, p95_ms, p99_ms) = lookup.get(&ts).copied().unwrap_or((None, None, None));
        out.push(LatencyBucket {
            ts_unix_ms: ts,
            p50_ms,
            p95_ms,
            p99_ms,
        });
        ts += bucket_ms;
    }
    out
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
struct TrafficRow {
    ts_unix_ms: i64,
    allowed: u64,
    denied: u64,
}

#[cfg(feature = "ssr")]
#[derive(clickhouse::Row, serde::Deserialize)]
struct LatencyRow {
    ts_unix_ms: i64,
    p50_us: f64,
    p95_us: f64,
    p99_us: f64,
}

#[cfg(feature = "ssr")]
#[derive(clickhouse::Row, serde::Deserialize)]
struct DenialReasonRow {
    denial_reason: String,
    count: u64,
}

#[cfg(feature = "ssr")]
#[derive(clickhouse::Row, serde::Deserialize)]
struct BusiestKeyRow {
    auth_key: String,
    requests: u64,
    denied: u64,
    last_seen_unix: i64,
}

#[server(GetAnalyticsSnapshot, "/api")]
pub async fn get_analytics_snapshot(
    range: AnalyticsRange,
) -> Result<AnalyticsSnapshot, ServerFnError> {
    require_admin().await?;

    use actix_web::web;
    use leptos_actix::extract;

    let client = extract::<web::Data<clickhouse::Client>>().await?;
    let bucket_secs = range.bucket_seconds();
    let range_secs = range.range_seconds();

    // Fan the per-panel ClickHouse queries out concurrently rather than
    // awaiting them one after another — they're independent reads against the
    // same window, so serializing them just adds round-trip latency.
    let traffic_fut = client
        .query(
            // `toStartOfInterval(DateTime64, INTERVAL n second)` returns a
            // plain `DateTime` in ClickHouse 24.x, which `toUnixTimestamp64Milli`
            // rejects (it wants a `DateTime64`). Buckets are whole-second
            // aligned anyway, so take epoch seconds and scale to ms.
            "SELECT toInt64(toUnixTimestamp(toStartOfInterval(ts, INTERVAL ? second))) * 1000 AS ts_unix_ms,
                    countIf(decision = 'allowed') AS allowed,
                    countIf(decision = 'denied') AS denied
             FROM auth_events
             WHERE ts >= now() - INTERVAL ? second
             GROUP BY ts_unix_ms
             ORDER BY ts_unix_ms",
        )
        .bind(bucket_secs)
        .bind(range_secs)
        .fetch_all::<TrafficRow>();

    let latency_fut = client
        .query(
            // Same `toStartOfInterval` epoch-ms quirk as traffic above. Empty
            // buckets simply don't appear here — they become gaps client-side.
            "SELECT toInt64(toUnixTimestamp(toStartOfInterval(ts, INTERVAL ? second))) * 1000 AS ts_unix_ms,
                    quantile(0.5)(latency_us) AS p50_us,
                    quantile(0.95)(latency_us) AS p95_us,
                    quantile(0.99)(latency_us) AS p99_us
             FROM auth_events
             WHERE ts >= now() - INTERVAL ? second
             GROUP BY ts_unix_ms
             ORDER BY ts_unix_ms",
        )
        .bind(bucket_secs)
        .bind(range_secs)
        .fetch_all::<LatencyRow>();

    let denial_fut = client
        .query(
            "SELECT denial_reason, count() AS count
             FROM auth_events
             WHERE decision = 'denied' AND ts >= now() - INTERVAL ? second
             GROUP BY denial_reason
             ORDER BY count DESC",
        )
        .bind(range_secs)
        .fetch_all::<DenialReasonRow>();

    let busiest_fut = client
        .query(
            "SELECT auth_key,
                    count() AS requests,
                    countIf(decision = 'denied') AS denied,
                    max(toInt64(toUnixTimestamp(ts))) AS last_seen_unix
             FROM auth_events
             WHERE ts >= now() - INTERVAL ? second
             GROUP BY auth_key
             ORDER BY requests DESC
             LIMIT 10",
        )
        .bind(range_secs)
        .fetch_all::<BusiestKeyRow>();

    let (traffic_rows, latency_rows, denial_rows, busiest_rows) =
        tokio::try_join!(traffic_fut, latency_fut, denial_fut, busiest_fut)
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    let queried_traffic: Vec<TrafficBucket> = traffic_rows
        .into_iter()
        .map(|r| TrafficBucket {
            ts_unix_ms: r.ts_unix_ms,
            allowed: r.allowed,
            denied: r.denied,
        })
        .collect();

    let queried_latency: Vec<LatencyBucket> = latency_rows
        .into_iter()
        .map(|r| LatencyBucket {
            ts_unix_ms: r.ts_unix_ms,
            p50_ms: Some(micros_to_millis(r.p50_us)),
            p95_ms: Some(micros_to_millis(r.p95_us)),
            p99_ms: Some(micros_to_millis(r.p99_us)),
        })
        .collect();

    let denial_reasons: Vec<DenialReasonCount> = denial_rows
        .into_iter()
        .map(|r| DenialReasonCount {
            reason: r.denial_reason,
            count: r.count,
        })
        .collect();

    let busiest_keys: Vec<KeyTrafficRow> = busiest_rows
        .into_iter()
        .map(|r| KeyTrafficRow {
            auth_key: r.auth_key,
            requests: r.requests,
            denied: r.denied,
            last_seen_unix: r.last_seen_unix,
        })
        .collect();

    let now_unix_ms = chrono::Utc::now().timestamp_millis();
    let traffic = fill_missing_buckets(&queried_traffic, now_unix_ms, range);
    let latency = fill_missing_latency_buckets(&queried_latency, now_unix_ms, range);

    Ok(AnalyticsSnapshot {
        traffic,
        latency,
        denial_reasons,
        busiest_keys,
    })
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

    // `Last24h`: 60s buckets => 60_000 ms. Pin `now` to a bucket boundary
    // (28_333_334 * 60_000) so the expected slots are easy to reason about.
    const BUCKET_MS_24H: i64 = 60_000;
    const ALIGNED_NOW: i64 = 1_700_000_040_000;

    #[test]
    fn fill_empty_input_yields_dense_zeroed_window() {
        let now = ALIGNED_NOW;
        let out = fill_missing_buckets(&[], now, AnalyticsRange::Last24h);

        // 24h / 60s = 1440 buckets, plus the inclusive end bucket = 1441.
        assert_eq!(out.len(), 1441);
        assert!(out.iter().all(|b| b.allowed == 0 && b.denied == 0));
        // Boundary-aligned and strictly increasing by one bucket.
        assert_eq!(out.first().unwrap().ts_unix_ms % BUCKET_MS_24H, 0);
        assert_eq!(out.last().unwrap().ts_unix_ms, now);
        for w in out.windows(2) {
            assert_eq!(w[1].ts_unix_ms - w[0].ts_unix_ms, BUCKET_MS_24H);
        }
    }

    #[test]
    fn fill_gap_in_the_middle_inserts_zeros() {
        let now = ALIGNED_NOW;
        // Two real buckets two slots apart, leaving one empty slot between.
        let a = now - 3 * BUCKET_MS_24H;
        let b = now - BUCKET_MS_24H;
        let queried = vec![
            TrafficBucket {
                ts_unix_ms: a,
                allowed: 10,
                denied: 2,
            },
            TrafficBucket {
                ts_unix_ms: b,
                allowed: 5,
                denied: 1,
            },
        ];
        let out = fill_missing_buckets(&queried, now, AnalyticsRange::Last24h);

        let mid = out
            .iter()
            .find(|x| x.ts_unix_ms == now - 2 * BUCKET_MS_24H)
            .expect("missing middle bucket should be synthesized");
        assert_eq!((mid.allowed, mid.denied), (0, 0));

        let first_real = out.iter().find(|x| x.ts_unix_ms == a).unwrap();
        assert_eq!((first_real.allowed, first_real.denied), (10, 2));
        let second_real = out.iter().find(|x| x.ts_unix_ms == b).unwrap();
        assert_eq!((second_real.allowed, second_real.denied), (5, 1));
    }

    #[test]
    fn fill_leading_gap_zero_pads_the_window_start() {
        let now = ALIGNED_NOW;
        // Only the final bucket has data; everything before must be zeros.
        let queried = vec![TrafficBucket {
            ts_unix_ms: now,
            allowed: 7,
            denied: 3,
        }];
        let out = fill_missing_buckets(&queried, now, AnalyticsRange::Last24h);

        assert_eq!(
            (out.first().unwrap().allowed, out.first().unwrap().denied),
            (0, 0)
        );
        assert_eq!(
            (out.last().unwrap().allowed, out.last().unwrap().denied),
            (7, 3)
        );
    }

    #[test]
    fn fill_trailing_gap_zero_pads_the_window_end() {
        let now = ALIGNED_NOW;
        // Only the first bucket has data; everything after must be zeros.
        let first = ((now - 86_400_000) / BUCKET_MS_24H) * BUCKET_MS_24H;
        let queried = vec![TrafficBucket {
            ts_unix_ms: first,
            allowed: 9,
            denied: 0,
        }];
        let out = fill_missing_buckets(&queried, now, AnalyticsRange::Last24h);

        assert_eq!(
            (out.first().unwrap().allowed, out.first().unwrap().denied),
            (9, 0)
        );
        assert_eq!(
            (out.last().unwrap().allowed, out.last().unwrap().denied),
            (0, 0)
        );
    }

    #[test]
    fn micros_to_millis_scales_by_1000() {
        assert_eq!(micros_to_millis(1000.0), 1.0);
        assert_eq!(micros_to_millis(0.0), 0.0);
        assert_eq!(micros_to_millis(2500.0), 2.5);
        // Sub-millisecond latencies keep their fractional part.
        assert_eq!(micros_to_millis(500.0), 0.5);
    }

    #[test]
    fn fill_latency_empty_input_is_all_gaps_not_zeros() {
        let now = ALIGNED_NOW;
        let out = fill_missing_latency_buckets(&[], now, AnalyticsRange::Last24h);

        assert_eq!(out.len(), 1441);
        // Every slot is a gap (None), never a misleading 0ms point.
        assert!(out
            .iter()
            .all(|b| b.p50_ms.is_none() && b.p95_ms.is_none() && b.p99_ms.is_none()));
        assert_eq!(out.last().unwrap().ts_unix_ms, now);
    }

    #[test]
    fn fill_latency_preserves_values_and_gaps_them_in_between() {
        let now = ALIGNED_NOW;
        let a = now - 2 * BUCKET_MS_24H;
        let queried = vec![LatencyBucket {
            ts_unix_ms: a,
            p50_ms: Some(1.5),
            p95_ms: Some(4.0),
            p99_ms: Some(9.0),
        }];
        let out = fill_missing_latency_buckets(&queried, now, AnalyticsRange::Last24h);

        let real = out.iter().find(|x| x.ts_unix_ms == a).unwrap();
        assert_eq!(real.p50_ms, Some(1.5));
        assert_eq!(real.p95_ms, Some(4.0));
        assert_eq!(real.p99_ms, Some(9.0));

        // The neighbouring slot with no events stays a gap.
        let gap = out
            .iter()
            .find(|x| x.ts_unix_ms == now - BUCKET_MS_24H)
            .unwrap();
        assert!(gap.p50_ms.is_none() && gap.p95_ms.is_none() && gap.p99_ms.is_none());
    }

    #[test]
    fn fill_snaps_unaligned_now_to_bucket_boundaries() {
        // `now` sits mid-bucket; output endpoints must still be aligned.
        let now = 1_700_000_000_000 + 37_123;
        let out = fill_missing_buckets(&[], now, AnalyticsRange::Last24h);

        assert_eq!(out.first().unwrap().ts_unix_ms % BUCKET_MS_24H, 0);
        assert_eq!(out.last().unwrap().ts_unix_ms % BUCKET_MS_24H, 0);
        assert!(out.last().unwrap().ts_unix_ms <= now);
        assert!(out.last().unwrap().ts_unix_ms > now - BUCKET_MS_24H);
    }
}
