use crate::server::analytics_service::{micros_to_millis, WindowSummary};
use crate::ui::analytics::tables::deny_rate_pct;
use leptos::prelude::*;

/// Five window-summary cards above the charts. Mirrors the dashboard's
/// `StatCard` chrome (white card, colored icon circle, gray title, bold
/// value) but the values are pre-formatted Strings — deny rate as "%", p95
/// as "ms" (or "—" when the window is empty).
#[component]
pub fn SummaryCards(summary: WindowSummary) -> impl IntoView {
    let total = summary.total.to_string();
    let deny_rate = format!("{:.1}%", deny_rate_pct(summary.total, summary.denied));
    let p95 = summary
        .p95_us
        .map(|us| format!("{:.1} ms", micros_to_millis(us)))
        .unwrap_or_else(|| "—".to_string());
    let unique_keys = summary.unique_keys.to_string();
    let unique_devices = summary.unique_devices.to_string();

    view! {
        <div class="grid grid-cols-2 lg:grid-cols-5 gap-6">
            <SummaryCard title="Total Requests" value=total icon_color="text-blue-600" bg_color="bg-blue-50"/>
            <SummaryCard title="Deny Rate" value=deny_rate icon_color="text-red-600" bg_color="bg-red-50"/>
            <SummaryCard title="p95 Latency" value=p95 icon_color="text-amber-600" bg_color="bg-amber-50"/>
            <SummaryCard title="Unique Keys" value=unique_keys icon_color="text-green-600" bg_color="bg-green-50"/>
            <SummaryCard title="Unique Devices" value=unique_devices icon_color="text-purple-600" bg_color="bg-purple-50"/>
        </div>
    }
}

#[component]
fn SummaryCard(
    #[prop(into)] title: String,
    #[prop(into)] value: String,
    #[prop(into)] icon_color: String,
    #[prop(into)] bg_color: String,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow p-6">
            <div class="flex items-center">
                <div class=format!("p-3 rounded-full {}", bg_color)>
                    <svg class=format!("w-6 h-6 {}", icon_color) fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/>
                    </svg>
                </div>
                <div class="ml-4">
                    <p class="text-sm font-medium text-gray-500">{title}</p>
                    <p class="text-2xl font-bold text-gray-900">{value}</p>
                </div>
            </div>
        </div>
    }
}
