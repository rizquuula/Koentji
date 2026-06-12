use crate::server::analytics_service::{get_analytics_snapshot, AnalyticsRange, AnalyticsSnapshot};
use crate::ui::analytics::panels::{
    render_analytics_charts, DenialReasonsPanel, LatencyPanel, TrafficPanel,
};
use crate::ui::analytics::summary_cards::SummaryCards;
use crate::ui::analytics::tables::{BusiestKeysTable, QuotaPressureTable};
use crate::ui::shell::layout::Layout;
use leptos::prelude::*;

/// How often the page silently re-fetches the snapshot.
const REFRESH_INTERVAL_SECS: u64 = 30;

#[component]
pub fn AnalyticsPage() -> impl IntoView {
    let range = RwSignal::new(AnalyticsRange::Last24h);

    let snapshot = Resource::new(move || range.get(), get_analytics_snapshot);

    // Last-good snapshot, paired with the range it was fetched under. Panels
    // render from this rather than directly from the Resource so a 30s
    // auto-refetch (which re-arms Suspense's fallback) doesn't blank the page
    // on every tick — Suspense only gates the very first load, while this
    // signal keeps the previous data on screen until the new one arrives.
    let last_good = RwSignal::new(None::<(AnalyticsRange, AnalyticsSnapshot)>);

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

    // Auto-refresh: tick every 30s and refetch, skipping ticks while the tab
    // is hidden so background tabs don't hammer ClickHouse. The handle is
    // cleared on disposal or it leaks across client-side navigations.
    Effect::new(move |_| {
        let handle = set_interval_with_handle(
            move || {
                if !document().hidden() {
                    snapshot.refetch();
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

                <Suspense fallback=|| view! {
                    <div class="flex justify-center py-12">
                        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                    </div>
                }>
                    // Read the Resource so Suspense knows when the first load
                    // resolves, but render panels from `last_good` so refetches
                    // don't blank the page. An initial error surfaces here;
                    // refetch errors leave the last-good data on screen.
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
                </Suspense>
            </div>
        </Layout>
    }
}
