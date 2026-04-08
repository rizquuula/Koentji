use leptos::prelude::*;

use crate::auth::{get_current_user, logout};
use crate::components::toast::ToastContainer;

#[component]
pub fn Layout(
    #[prop(into)] active_tab: String,
    #[prop(default = true)] require_auth: bool,
    children: Children,
) -> impl IntoView {
    let active = active_tab.clone();
    let user_resource = Resource::new(|| (), |_| get_current_user());

    if require_auth {
        Effect::new(move |_| {
            if let Some(Ok(None)) = user_resource.get() {
                leptos::prelude::window().location().set_href("/login").ok();
            }
        });
    }

    let handle_logout = Action::new(|_: &()| async {
        let _ = logout().await;
        leptos::prelude::window().location().set_href("/login").ok();
    });

    let nav_class = move |tab: &str| -> &str {
        if tab == active.as_str() {
            "border-b-2 border-blue-600 text-blue-600 px-4 py-2 font-medium"
        } else {
            "px-4 py-2 text-gray-500 hover:text-gray-700 font-medium"
        }
    };

    let dashboard_class = nav_class("dashboard");
    let keys_class = nav_class("keys");
    let subscriptions_class = nav_class("subscriptions");
    let rate_limits_class = nav_class("limits_interval");
    let quickstart_class = nav_class("quickstart");
    let about_class = nav_class("about");

    let is_logged_in = move || {
        user_resource
            .get()
            .map(|r| matches!(r, Ok(Some(_))))
            .unwrap_or(false)
    };

    view! {
        <div class="min-h-screen bg-gray-50">
            <nav class="bg-white shadow-sm border-b">
                <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="flex justify-between h-16">
                        <div class="flex items-center space-x-8">
                            <a href="/" class="flex items-center space-x-2">
                                <svg class="w-8 h-8 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/>
                                </svg>
                                <span class="text-xl font-bold text-gray-900">"Koentji"</span>
                            </a>
                            <div class="flex space-x-1">
                                <Show when=is_logged_in>
                                    <a href="/dashboard" class=dashboard_class>"Dashboard"</a>
                                    <a href="/keys" class=keys_class>"Keys"</a>
                                    <a href="/subscriptions" class=subscriptions_class>"Subscriptions"</a>
                                    <a href="/limits-interval" class=rate_limits_class>"Limits Interval"</a>
                                </Show>
                                <a href="/quickstart" class=quickstart_class>"Quickstart"</a>
                                <a href="/about" class=about_class>"About"</a>
                                <a href="/docs" target="_blank" class="px-4 py-2 text-gray-500 hover:text-gray-700 font-medium">"API Docs"</a>
                            </div>
                        </div>
                        <div class="flex items-center space-x-4">
                            <Suspense fallback=|| view! { <span class="text-gray-400">"..."</span> }>
                                {move || user_resource.get().map(|result| {
                                    match result {
                                        Ok(Some(username)) => view! {
                                            <span class="text-sm text-gray-600">{username}</span>
                                        }.into_any(),
                                        _ => view! {
                                            <span class="text-sm text-gray-400">"Not logged in"</span>
                                        }.into_any(),
                                    }
                                })}
                            </Suspense>
                            <Show when=is_logged_in>
                                <button
                                    class="text-sm text-gray-500 hover:text-red-600 transition-colors"
                                    on:click=move |_| { let _ = handle_logout.dispatch(()); }
                                >
                                    "Logout"
                                </button>
                            </Show>
                            <Show when=move || !is_logged_in()>
                                <a href="/login" class="text-sm text-blue-600 hover:text-blue-800 font-medium">"Login"</a>
                            </Show>
                        </div>
                    </div>
                </div>
            </nav>

            <main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
                {children()}
            </main>
        </div>

        <ToastContainer/>
    }
}
