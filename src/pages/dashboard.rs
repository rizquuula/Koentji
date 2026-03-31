use crate::components::charts::Charts;
use crate::components::date_range_picker::DateRangePicker;
use crate::components::layout::Layout;
use crate::components::stats_cards::StatsCards;
use crate::server::stats_service::get_dashboard_stats;
use leptos::prelude::*;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let range = RwSignal::new("all".to_string());
    let start_date = RwSignal::new(String::new());
    let end_date = RwSignal::new(String::new());

    let stats_resource = Resource::new(
        move || (range.get(), start_date.get(), end_date.get()),
        move |(r, s, e)| get_dashboard_stats(r, s, e),
    );

    let stats_signal = Signal::derive(move || {
        stats_resource.get().and_then(|r| r.ok())
    });

    view! {
        <Layout active_tab="dashboard">
            <div class="space-y-6">
                <div class="flex items-center justify-between">
                    <h1 class="text-2xl font-bold text-gray-900">"Dashboard"</h1>
                    <DateRangePicker range=range start_date=start_date end_date=end_date/>
                </div>

                <Suspense fallback=|| view! {
                    <div class="flex justify-center py-12">
                        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                    </div>
                }>
                    <StatsCards stats=stats_signal/>
                    <Charts stats=stats_signal/>
                </Suspense>
            </div>
        </Layout>

    }
}
