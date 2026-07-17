use crate::server::analytics_service::{
    effective_granularity, get_analytics_snapshot, get_rate_limit_usage, AnalyticsRange,
    AnalyticsSnapshot, RateLimitUsageSnapshot, TimeGranularity,
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

/// Default auto-refresh cadence, in seconds — the dropdown's initial pick.
/// `0` is the "Off" sentinel: the page loads frozen on its first snapshot and
/// polls nothing until the admin picks a live cadence. Each tick would re-run
/// the full analytics query batch against a memory-capped ClickHouse, so
/// defaulting to Off keeps idle dashboards from generating sustained load.
const DEFAULT_REFRESH_SECS: u64 = 0;

/// Auto-refresh dropdown options as `(label, seconds)`. `0` is the "Off"
/// sentinel — a `0`-second interval means "don't poll", so the page freezes on
/// the current snapshot until the admin picks a live cadence again.
const REFRESH_OPTIONS: [(&str, u64); 5] =
    [("Off", 0), ("5s", 5), ("15s", 15), ("30s", 30), ("60s", 60)];

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
    let range = RwSignal::new(AnalyticsRange::Last30d);
    // Daily (30 buckets over 30d) rather than a finer bucket: a lighter first
    // paint against the memory-capped ClickHouse. The bucket selector still
    // offers Hourly/Minutely for a finer view on demand.
    let granularity = RwSignal::new(TimeGranularity::Daily);
    let selected_key = RwSignal::new(String::new());
    // `0` is the "Off" sentinel — see `REFRESH_OPTIONS`.
    let refresh_secs = RwSignal::new(DEFAULT_REFRESH_SECS);

    let snapshot = Resource::new(
        move || (range.get(), granularity.get()),
        |(r, g)| get_analytics_snapshot(r, g),
    );
    let usage_resource = Resource::new(
        move || (range.get(), granularity.get(), selected_key.get()),
        |(r, g, k)| get_rate_limit_usage(r, parse_key_id(k), g),
    );

    // Last-good snapshot, paired with the range it was fetched under. Panels
    // render from this rather than directly from the Resource so the periodic
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

    // Auto-refresh: tick at the admin-chosen cadence and refetch, skipping
    // ticks while the tab is hidden so background tabs don't hammer ClickHouse.
    // The interval is rebuilt whenever `refresh_secs` changes — the prior
    // handle is cleared first (held in a `StoredValue` so re-runs and final
    // disposal both reach it) or timers leak across cadence switches and
    // client-side navigations. A `0` cadence ("Off") tears the timer down and
    // installs none, freezing the page on the current snapshot.
    let timer = StoredValue::new(None::<leptos::prelude::IntervalHandle>);
    let clear_timer = move || {
        timer.update_value(|slot| {
            if let Some(handle) = slot.take() {
                handle.clear();
            }
        });
    };
    Effect::new(move |_| {
        let secs = refresh_secs.get();
        clear_timer();
        if secs == 0 {
            return;
        }
        let handle = set_interval_with_handle(
            move || {
                if !document().hidden() {
                    snapshot.refetch();
                    usage_resource.refetch();
                }
            },
            std::time::Duration::from_secs(secs),
        );
        match handle {
            Ok(handle) => timer.set_value(Some(handle)),
            // Don't fail silently — a dead timer would leave the dropdown
            // showing a live cadence while nothing refetches.
            Err(e) => leptos::logging::warn!("analytics auto-refresh timer failed to start: {e:?}"),
        }
    });
    on_cleanup(clear_timer);

    let button_class = move |r: AnalyticsRange| {
        if range.get() == r {
            "px-3 py-1.5 text-sm font-medium rounded-lg bg-blue-600 text-white transition-colors duration-quick"
        } else {
            "px-3 py-1.5 text-sm font-medium rounded-lg bg-white text-gray-700 hover:bg-gray-100 border transition-colors duration-quick"
        }
    };

    let class_24h = move || button_class(AnalyticsRange::Last24h);
    let class_7d = move || button_class(AnalyticsRange::Last7d);
    let class_30d = move || button_class(AnalyticsRange::Last30d);

    // Shared look for the two dropdowns so they sit flush with the range pills.
    const CONTROL_CLASS: &str = "px-3 py-1.5 text-sm font-medium rounded-lg bg-white text-gray-700 border focus:ring-2 focus:ring-blue-500 focus:border-blue-500";

    // When the picked granularity is too fine for the window it's coerced up
    // (see `effective_bucket_seconds`); tell the admin what they're actually
    // looking at rather than silently swapping the resolution under them.
    let coercion_note = move || {
        let r = range.get();
        let g = granularity.get();
        let effective = effective_granularity(r, g);
        (effective != g).then(|| {
            format!(
                "{} is too fine for this window — showing {} buckets.",
                g.label(),
                effective.label().to_lowercase()
            )
        })
    };

    view! {
        <Layout active_tab="analytics">
            <div class="space-y-6">
                <div class="flex items-start justify-between gap-4 flex-wrap">
                    <h1 class="text-2xl font-bold text-gray-900">"Analytics"</h1>
                    <div class="flex flex-col items-end gap-2">
                        <div class="flex items-center space-x-2 flex-wrap gap-y-2 justify-end">
                            <button class=class_24h on:click=move |_| range.set(AnalyticsRange::Last24h)>"24 Hours"</button>
                            <button class=class_7d on:click=move |_| range.set(AnalyticsRange::Last7d)>"7 Days"</button>
                            <button class=class_30d on:click=move |_| range.set(AnalyticsRange::Last30d)>"30 Days"</button>

                            // Bucket granularity — independent of the window above.
                            <label class="text-sm font-medium text-gray-500 ml-2">"Bucket"</label>
                            <select
                                class=CONTROL_CLASS
                                aria-label="Bucket granularity"
                                prop:value=move || granularity.get().as_value().to_string()
                                on:change=move |ev| granularity.set(
                                    TimeGranularity::from_value(&event_target_value(&ev))
                                )
                            >
                                {TimeGranularity::ALL.into_iter().map(|g| view! {
                                    <option value=g.as_value()>{g.label()}</option>
                                }).collect::<Vec<_>>()}
                            </select>

                            // Auto-refresh cadence — "Off" stops polling entirely.
                            <label class="text-sm font-medium text-gray-500 ml-2">"Refresh"</label>
                            <select
                                class=CONTROL_CLASS
                                aria-label="Auto-refresh interval"
                                prop:value=move || refresh_secs.get().to_string()
                                on:change=move |ev| refresh_secs.set(
                                    event_target_value(&ev).parse::<u64>().unwrap_or(DEFAULT_REFRESH_SECS)
                                )
                            >
                                {REFRESH_OPTIONS.into_iter().map(|(label, secs)| view! {
                                    <option value=secs.to_string()>{label}</option>
                                }).collect::<Vec<_>>()}
                            </select>
                        </div>
                        {move || coercion_note().map(|note| view! {
                            <p class="text-xs text-amber-600">{note}</p>
                        })}
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
