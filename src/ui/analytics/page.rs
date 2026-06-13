use crate::server::analytics_service::{
    get_analytics_snapshot, get_rate_limit_usage, AnalyticsRange, AnalyticsSnapshot,
    RateLimitUsageSnapshot,
};
use crate::ui::analytics::panels::{
    render_analytics_charts, render_usage_chart, DenialReasonsPanel, LatencyPanel, TrafficPanel,
    UsagePanel,
};
use crate::ui::analytics::summary_cards::SummaryCards;
use crate::ui::analytics::tables::{truncate_key, BusiestKeysTable, QuotaPressureTable};
use crate::ui::design::select::Select;
use crate::ui::shell::layout::Layout;
use leptos::prelude::*;

/// How often the page silently re-fetches the snapshot.
const REFRESH_INTERVAL_SECS: u64 = 30;

/// Parse a non-empty string to `Some(i64)`, empty string to `None`. The
/// dropdown stores the key id as a string (HTML value attributes are always
/// strings); this converts back for the server fn param.
fn parse_key_id(s: String) -> Option<i64> {
    if s.is_empty() {
        None
    } else {
        s.parse::<i64>().ok()
    }
}

#[component]
pub fn AnalyticsPage() -> impl IntoView {
    let range = RwSignal::new(AnalyticsRange::Last24h);
    let selected_key = RwSignal::new(String::new());

    let snapshot = Resource::new(move || range.get(), get_analytics_snapshot);
    let usage_resource = Resource::new(
        move || (range.get(), selected_key.get()),
        |(r, k)| get_rate_limit_usage(r, parse_key_id(k)),
    );

    // Last-good snapshot, paired with the range it was fetched under. Panels
    // render from this rather than directly from the Resource so a 30s
    // auto-refetch (which re-arms Suspense's fallback) doesn't blank the page
    // on every tick — Suspense only gates the very first load, while this
    // signal keeps the previous data on screen until the new one arrives.
    let last_good = RwSignal::new(None::<(AnalyticsRange, AnalyticsSnapshot)>);
    let last_good_usage = RwSignal::new(None::<RateLimitUsageSnapshot>);

    // When the snapshot resolves Ok, stash it and hand it to the Chart.js
    // bridge. Mirrors the dashboard's `Effect::new` + `render_charts` pattern;
    // re-runs on range switches and refetches (the bridge destroys the prior
    // canvas before re-create).
    Effect::new(move || {
        if let Some(Ok(snap)) = snapshot.get() {
            let r = range.get_untracked();
            render_analytics_charts(&snap, r == AnalyticsRange::Last24h);
            last_good.set(Some((r, snap)));
        }
    });

    // Bridge usage snapshot → JS chart renderer.
    Effect::new(move || {
        if let Some(Ok(snap)) = usage_resource.get() {
            let r = range.get_untracked();
            render_usage_chart(&snap, r == AnalyticsRange::Last24h);
            last_good_usage.set(Some(snap));
        }
    });

    // Auto-refresh: tick every 30s and refetch, skipping ticks while the tab
    // is hidden so background tabs don't hammer ClickHouse. The handle is
    // cleared on disposal or it leaks across client-side navigations.
    Effect::new(move |_| {
        let handle = set_interval_with_handle(
            move || {
                if !document().hidden() {
                    snapshot.refetch();
                    usage_resource.refetch();
                }
            },
            std::time::Duration::from_secs(REFRESH_INTERVAL_SECS),
        );
        if let Ok(handle) = handle {
            on_cleanup(move || handle.clear());
        }
    });

    let button_class = move |r: AnalyticsRange| {
        if range.get() == r {
            "px-3 py-1.5 text-sm font-medium rounded-lg bg-blue-600 text-white"
        } else {
            "px-3 py-1.5 text-sm font-medium rounded-lg bg-white text-gray-700 hover:bg-gray-100 border"
        }
    };

    let class_24h = move || button_class(AnalyticsRange::Last24h);
    let class_7d = move || button_class(AnalyticsRange::Last7d);
    let class_30d = move || button_class(AnalyticsRange::Last30d);

    view! {
        <Layout active_tab="analytics">
            <div class="space-y-6">
                <div class="flex items-center justify-between">
                    <h1 class="text-2xl font-bold text-gray-900">"Analytics"</h1>
                    <div class="flex items-center space-x-2 flex-wrap gap-y-2">
                        <button class=class_24h on:click=move |_| range.set(AnalyticsRange::Last24h)>"24 Hours"</button>
                        <button class=class_7d on:click=move |_| range.set(AnalyticsRange::Last7d)>"7 Days"</button>
                        <button class=class_30d on:click=move |_| range.set(AnalyticsRange::Last30d)>"30 Days"</button>
                    </div>
                </div>

                // `Transition`, not `Suspense`: the page auto-refetches every
                // 30s, and `Suspense` re-arms its fallback on every pending
                // read — flashing the spinner and tearing down the Chart.js
                // canvases on each tick. `Transition` shows the fallback only
                // on the first load and keeps the current panels on screen
                // while a refetch is in flight.
                <Transition fallback=|| view! {
                    <div class="flex justify-center py-12">
                        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                    </div>
                }>
                    // Read the Resource so the Transition knows when the first
                    // load resolves, but render panels from `last_good`. An
                    // initial error surfaces here; refetch errors leave the
                    // last-good data on screen.
                    {move || snapshot.get().and_then(|r| match r {
                        Ok(_) => last_good.get().map(|(_, snap)| {
                            let allowed: u64 = snap.traffic.iter().map(|b| b.allowed).sum();
                            let denied: u64 = snap.traffic.iter().map(|b| b.denied).sum();
                            let has_denials = !snap.denial_reasons.is_empty();
                            let busiest_keys = snap.busiest_keys.clone();
                            let quota_pressure = snap.quota_pressure.clone();
                            let summary = snap.summary.clone();
                            view! {
                                <SummaryCards summary=summary/>
                                <TrafficPanel/>
                                <div class="flex space-x-6 text-sm">
                                    <span class="text-gray-700">
                                        <span class="inline-block w-3 h-3 rounded-sm bg-green-500 mr-2"></span>
                                        {format!("Allowed: {allowed}")}
                                    </span>
                                    <span class="text-gray-700">
                                        <span class="inline-block w-3 h-3 rounded-sm bg-red-600 mr-2"></span>
                                        {format!("Denied: {denied}")}
                                    </span>
                                </div>
                                <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                                    <LatencyPanel/>
                                    <DenialReasonsPanel has_denials=has_denials/>
                                </div>
                                // Usage chart with per-key filter dropdown.
                                {move || {
                                    let usage_snap = last_good_usage.get();
                                    let has_data = usage_snap
                                        .as_ref()
                                        .map(|s| s.buckets.iter().any(|b| b.usage > 0.0))
                                        .unwrap_or(false);
                                    let available_keys = usage_snap
                                        .map(|s| s.available_keys)
                                        .unwrap_or_default();
                                    view! {
                                        <div class="space-y-3">
                                            <div class="flex items-center space-x-3">
                                                <label class="text-sm font-medium text-gray-700 whitespace-nowrap">
                                                    "Filter by key"
                                                </label>
                                                <div class="w-64">
                                                    <Select value=selected_key>
                                                        <option value="">"All keys"</option>
                                                        {available_keys.into_iter().map(|k| {
                                                            let label = truncate_key(&k.auth_key);
                                                            let val = k.auth_key_id.to_string();
                                                            view! {
                                                                <option value=val>{label}</option>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </Select>
                                                </div>
                                            </div>
                                            <UsagePanel has_data=has_data/>
                                        </div>
                                    }
                                }}
                                <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                                    <BusiestKeysTable rows=busiest_keys/>
                                    <QuotaPressureTable rows=quota_pressure/>
                                </div>
                            }.into_any()
                        }),
                        Err(e) => Some(view! {
                            <p class="text-sm text-red-600">{format!("Failed to load: {e}")}</p>
                        }.into_any()),
                    })}
                </Transition>
            </div>
        </Layout>
    }
}
