use crate::models::DashboardStats;
use crate::server::insights_service::get_dashboard_insights;
use crate::server::stats_service::get_dashboard_stats;
use crate::ui::dashboard::activity_feed::ActivityFeed;
use crate::ui::dashboard::charts::Charts;
use crate::ui::dashboard::date_range_picker::DateRangePicker;
use crate::ui::dashboard::expiring_keys::ExpiringKeys;
use crate::ui::dashboard::key_hygiene::KeyHygiene;
use crate::ui::dashboard::stats_cards::StatsCards;
use crate::ui::dashboard::tier_health::TierHealthPanel;
use crate::ui::shell::layout::Layout;
use crate::ui::tz::use_tz_offset;
use leptos::prelude::*;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let range = RwSignal::new("all".to_string());
    let start_date = RwSignal::new(String::new());
    let end_date = RwSignal::new(String::new());
    // Viewer's local offset (minutes east of UTC). Seeded 0 on the server, so
    // the SSR load buckets the daily trend by UTC day; once the browser offset
    // lands it re-fetches and the trend re-buckets to local days.
    let tz = use_tz_offset();

    let stats_resource = Resource::new(
        move || (range.get(), start_date.get(), end_date.get(), tz.get()),
        move |(r, s, e, tz)| get_dashboard_stats(r, s, e, tz),
    );

    // Last-good stats so a date-range refetch keeps the current numbers on
    // screen: StatsCards reads `stats.get().map(...).unwrap_or(0)`, so a pending
    // `None` would flash every card to 0 even with the children kept mounted.
    let last_good_stats = RwSignal::new(None::<DashboardStats>);
    Effect::new(move || {
        if let Some(Ok(s)) = stats_resource.get() {
            last_good_stats.set(Some(s));
        }
    });
    let stats_signal = Signal::derive(move || last_good_stats.get());

    // Current-state insights ignore the date-range picker, so this resource
    // takes no reactive deps — it loads once and shows the live picture.
    let insights_resource = Resource::new(|| (), move |_| get_dashboard_insights());
    let insights_signal = Signal::derive(move || insights_resource.get().and_then(|r| r.ok()));

    view! {
        <Layout active_tab="dashboard">
            <div class="space-y-6">
                <div class="flex items-center justify-between">
                    <h1 class="text-2xl font-bold text-gray-900">"Dashboard"</h1>
                    <DateRangePicker range=range start_date=start_date end_date=end_date/>
                </div>

                <Transition fallback=|| view! {
                    <div class="flex justify-center py-12">
                        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                    </div>
                }>
                    {move || stats_resource.get().map(|r| match r {
                        Ok(_) => view! {
                            <StatsCards stats=stats_signal/>
                            <Charts stats=stats_signal/>
                        }.into_any(),
                        Err(e) => view! {
                            <p class="text-sm text-red-600">{format!("Failed to load: {e}")}</p>
                        }.into_any(),
                    })}
                </Transition>

                <Suspense fallback=|| view! {
                    <div class="flex justify-center py-12">
                        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                    </div>
                }>
                    <div class="space-y-6">
                        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                            <ExpiringKeys insights=insights_signal/>
                            <ActivityFeed insights=insights_signal/>
                        </div>
                        <TierHealthPanel insights=insights_signal/>
                        <KeyHygiene insights=insights_signal/>
                    </div>
                </Suspense>
            </div>
        </Layout>

    }
}
