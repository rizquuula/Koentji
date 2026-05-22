use crate::server::analytics_service::{
    get_allow_deny_counts, get_requests_per_second, AnalyticsRange,
};
use crate::ui::shell::layout::Layout;
use leptos::prelude::*;

#[component]
pub fn AnalyticsPage() -> impl IntoView {
    let range = RwSignal::new(AnalyticsRange::Last24h);

    let rps_resource = Resource::new(move || range.get(), get_requests_per_second);
    let ratio_resource = Resource::new(move || range.get(), get_allow_deny_counts);

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
                    <div class="bg-white rounded-lg shadow p-6">
                        <h3 class="text-sm font-medium text-gray-500 mb-4">"Requests per Second"</h3>
                        {move || rps_resource.get().map(|r| match r {
                            Ok(res) => view! { <div inner_html=res.svg></div> }.into_any(),
                            Err(e) => view! {
                                <p class="text-sm text-red-600">{format!("Failed to load: {e}")}</p>
                            }.into_any(),
                        })}
                    </div>

                    <div class="bg-white rounded-lg shadow p-6">
                        <h3 class="text-sm font-medium text-gray-500 mb-4">"Allow vs Deny"</h3>
                        {move || ratio_resource.get().map(|r| match r {
                            Ok(res) => view! {
                                <div>
                                    <div inner_html=res.svg.clone()></div>
                                    <div class="mt-4 flex space-x-6 text-sm">
                                        <span class="text-gray-700">
                                            <span class="inline-block w-3 h-3 rounded-sm bg-green-500 mr-2"></span>
                                            {format!("Allowed: {}", res.counts.allowed)}
                                        </span>
                                        <span class="text-gray-700">
                                            <span class="inline-block w-3 h-3 rounded-sm bg-red-600 mr-2"></span>
                                            {format!("Denied: {}", res.counts.denied)}
                                        </span>
                                    </div>
                                </div>
                            }.into_any(),
                            Err(e) => view! {
                                <p class="text-sm text-red-600">{format!("Failed to load: {e}")}</p>
                            }.into_any(),
                        })}
                    </div>
                </Suspense>
            </div>
        </Layout>
    }
}
