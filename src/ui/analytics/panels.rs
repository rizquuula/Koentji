use crate::server::analytics_service::AnalyticsSnapshot;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

/// Typed bridge to the global `renderAnalyticsCharts(dataJson)` declared
/// in `public/js/analytics_charts.js` — same CSP-safe pattern as the
/// dashboard's `renderCharts`: no `eval`, no `window` smuggling, data
/// crosses the wasm → JS boundary as a single JSON *string*.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = renderAnalyticsCharts)]
    fn render_analytics_charts_js(data_json: &str);
}

/// Serialize the snapshot into the wire payload the JS bridge expects and
/// hand it across. Labels are formatted here (Rust stays the brains, JS
/// stays dumb) — "HH:MM" UTC for the 24h window, "DD MMM HH:MM" for wider
/// windows where a bare time would be ambiguous across days.
pub fn render_analytics_charts(snapshot: &AnalyticsSnapshot, range_is_24h: bool) {
    let labels: Vec<String> = snapshot
        .traffic
        .iter()
        .map(|b| format_label(b.ts_unix_ms, range_is_24h))
        .collect();
    let allowed: Vec<u64> = snapshot.traffic.iter().map(|b| b.allowed).collect();
    let denied: Vec<u64> = snapshot.traffic.iter().map(|b| b.denied).collect();

    // Latency lines reuse the traffic x-axis labels (same bucket grid). Gaps
    // travel as JSON `null`; the JS renderer sets `spanGaps: false` so the
    // line breaks rather than interpolating across windows with no traffic.
    let latency_p50: Vec<Option<f64>> = snapshot.latency.iter().map(|b| b.p50_ms).collect();
    let latency_p95: Vec<Option<f64>> = snapshot.latency.iter().map(|b| b.p95_ms).collect();
    let latency_p99: Vec<Option<f64>> = snapshot.latency.iter().map(|b| b.p99_ms).collect();

    let data = serde_json::json!({
        "trafficLabels": labels,
        "trafficAllowed": allowed,
        "trafficDenied": denied,
        "latencyP50": latency_p50,
        "latencyP95": latency_p95,
        "latencyP99": latency_p99,
    });

    if let Ok(json_str) = serde_json::to_string(&data) {
        render_analytics_charts_js(&json_str);
    }
}

fn format_label(ts_unix_ms: i64, range_is_24h: bool) -> String {
    use chrono::{DateTime, Utc};
    let dt: DateTime<Utc> = DateTime::from_timestamp_millis(ts_unix_ms).unwrap_or_default();
    if range_is_24h {
        dt.format("%H:%M").to_string()
    } else {
        dt.format("%d %b %H:%M").to_string()
    }
}

#[component]
pub fn TrafficPanel() -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow p-6">
            <h3 class="text-sm font-medium text-gray-500 mb-4">"Traffic"</h3>
            <div class="relative" style="height: 300px">
                <canvas id="traffic-chart"></canvas>
            </div>
        </div>
    }
}

#[component]
pub fn LatencyPanel() -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow p-6">
            <h3 class="text-sm font-medium text-gray-500 mb-4">"Latency (ms)"</h3>
            <div class="relative" style="height: 300px">
                <canvas id="latency-chart"></canvas>
            </div>
        </div>
    }
}
