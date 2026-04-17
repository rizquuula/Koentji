use crate::models::DashboardStats;
use leptos::prelude::*;

#[component]
pub fn StatsCards(#[prop(into)] stats: Signal<Option<DashboardStats>>) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            <StatCard
                title="Total Keys"
                value=Signal::derive(move || stats.get().map(|s| s.total).unwrap_or(0))
                icon_color="text-blue-600"
                bg_color="bg-blue-50"
            />
            <StatCard
                title="Active Keys"
                value=Signal::derive(move || stats.get().map(|s| s.active).unwrap_or(0))
                icon_color="text-green-600"
                bg_color="bg-green-50"
            />
            <StatCard
                title="Expired Keys"
                value=Signal::derive(move || stats.get().map(|s| s.expired).unwrap_or(0))
                icon_color="text-yellow-600"
                bg_color="bg-yellow-50"
            />
            <StatCard
                title="Deleted Keys"
                value=Signal::derive(move || stats.get().map(|s| s.deleted).unwrap_or(0))
                icon_color="text-red-600"
                bg_color="bg-red-50"
            />
        </div>
    }
}

#[component]
fn StatCard(
    #[prop(into)] title: String,
    #[prop(into)] value: Signal<i64>,
    #[prop(into)] icon_color: String,
    #[prop(into)] bg_color: String,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow p-6">
            <div class="flex items-center">
                <div class=format!("p-3 rounded-full {}", bg_color)>
                    <svg class=format!("w-6 h-6 {}", icon_color) fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/>
                    </svg>
                </div>
                <div class="ml-4">
                    <p class="text-sm font-medium text-gray-500">{title}</p>
                    <p class="text-2xl font-bold text-gray-900">{move || value.get()}</p>
                </div>
            </div>
        </div>
    }
}
