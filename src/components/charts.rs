use crate::models::DashboardStats;
use leptos::prelude::*;

#[component]
pub fn Charts(#[prop(into)] stats: Signal<Option<DashboardStats>>) -> impl IntoView {
    // Render charts when stats update
    Effect::new(move || {
        if let Some(stats) = stats.get() {
            render_charts(&stats);
        }
    });

    view! {
        <div class="grid grid-cols-1 lg:grid-cols-3 gap-6 mt-6">
            <div class="bg-white rounded-lg shadow p-6">
                <h3 class="text-sm font-medium text-gray-500 mb-4">"Subscription Distribution"</h3>
                <div class="relative" style="height: 250px">
                    <canvas id="subscription-chart"></canvas>
                </div>
            </div>
            <div class="bg-white rounded-lg shadow p-6">
                <h3 class="text-sm font-medium text-gray-500 mb-4">"Rate Limit Usage"</h3>
                <div class="relative" style="height: 250px">
                    <canvas id="rate-limit-chart"></canvas>
                </div>
            </div>
            <div class="bg-white rounded-lg shadow p-6">
                <h3 class="text-sm font-medium text-gray-500 mb-4">"Daily Creation Trend"</h3>
                <div class="relative" style="height: 250px">
                    <canvas id="trend-chart"></canvas>
                </div>
            </div>
        </div>
    }
}

fn render_charts(stats: &DashboardStats) {
    use wasm_bindgen::JsValue;

    let sub_labels: Vec<String> = stats.subscription_distribution.iter().map(|(l, _)| l.clone()).collect();
    let sub_values: Vec<i64> = stats.subscription_distribution.iter().map(|(_, v)| *v).collect();
    let rate_labels: Vec<String> = stats.rate_limit_buckets.iter().map(|(l, _)| l.clone()).collect();
    let rate_values: Vec<i64> = stats.rate_limit_buckets.iter().map(|(_, v)| *v).collect();
    let trend_labels: Vec<String> = stats.daily_trend.iter().map(|(l, _)| l.clone()).collect();
    let trend_values: Vec<i64> = stats.daily_trend.iter().map(|(_, v)| *v).collect();

    let data = serde_json::json!({
        "subscriptionLabels": sub_labels,
        "subscriptionValues": sub_values,
        "rateLimitLabels": rate_labels,
        "rateLimitValues": rate_values,
        "trendLabels": trend_labels,
        "trendValues": trend_values,
    });

    if let Ok(json_str) = serde_json::to_string(&data) {
        let window = web_sys::window().unwrap();
        let _ = js_sys::Reflect::set(
            &window,
            &JsValue::from_str("__chartData"),
            &JsValue::from_str(&json_str),
        );

        // Call JS render function
        let _ = js_sys::eval("if(typeof renderCharts === 'function') renderCharts()");
    }
}
