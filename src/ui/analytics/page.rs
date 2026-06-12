use crate::server::analytics_service::{get_analytics_snapshot, AnalyticsRange};
use crate::ui::analytics::panels::{
    render_analytics_charts, DenialReasonsPanel, LatencyPanel, TrafficPanel,
};
use crate::ui::shell::layout::Layout;
use leptos::prelude::*;

#[component]
pub fn AnalyticsPage() -> impl IntoView {
    let range = RwSignal::new(AnalyticsRange::Last24h);

    let snapshot = Resource::new(move || range.get(), get_analytics_snapshot);

    // When the snapshot resolves Ok, hand it to the Chart.js bridge. Mirrors
    // the dashboard's `Effect::new` + `render_charts` pattern; re-runs on
    // range switches (the bridge destroys the prior canvas before re-create).
    Effect::new(move || {
        if let Some(Ok(snap)) = snapshot.get() {
            let is_24h = range.get_untracked() == AnalyticsRange::Last24h;
            render_analytics_charts(&snap, is_24h);
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
                    {move || snapshot.get().map(|r| match r {
                        Ok(snap) => {
                            let allowed: u64 = snap.traffic.iter().map(|b| b.allowed).sum();
                            let denied: u64 = snap.traffic.iter().map(|b| b.denied).sum();
                            let has_denials = !snap.denial_reasons.is_empty();
                            view! {
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
                            }.into_any()
                        }
                        Err(e) => view! {
                            <p class="text-sm text-red-600">{format!("Failed to load: {e}")}</p>
                        }.into_any(),
                    })}
                </Suspense>
            </div>
        </Layout>
    }
}
