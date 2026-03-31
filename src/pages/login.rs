use crate::auth::login;
use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        loading.set(true);
        error.set(None);

        let user = username.get();
        let pass = password.get();

        leptos::task::spawn_local(async move {
            match login(user, pass).await {
                Ok(true) => {
                    leptos::prelude::window()
                        .location()
                        .set_href("/dashboard")
                        .ok();
                }
                Ok(false) => {
                    error.set(Some("Invalid username or password".to_string()));
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Login failed: {}", e)));
                    loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50">
            <div class="max-w-md w-full mx-4">
                <div class="text-center mb-8">
                    <svg class="w-16 h-16 text-blue-600 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/>
                    </svg>
                    <h1 class="text-3xl font-bold text-gray-900 mt-4">"Koentji"</h1>
                    <p class="text-gray-500 mt-2">"API Key Management Dashboard"</p>
                </div>

                <div class="bg-white rounded-lg shadow-lg p-8">
                    <form on:submit=handle_submit class="space-y-6">
                        <Show when=move || error.get().is_some()>
                            <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm">
                                {move || error.get().unwrap_or_default()}
                            </div>
                        </Show>

                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Username"</label>
                            <input
                                type="text"
                                required
                                class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                                placeholder="Enter your username"
                                prop:value=move || username.get()
                                on:input=move |ev| username.set(event_target_value(&ev))
                            />
                        </div>

                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Password"</label>
                            <input
                                type="password"
                                required
                                class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                                placeholder="Enter your password"
                                prop:value=move || password.get()
                                on:input=move |ev| password.set(event_target_value(&ev))
                            />
                        </div>

                        <button
                            type="submit"
                            class="w-full py-2 px-4 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 transition-colors disabled:opacity-50"
                            disabled=move || loading.get()
                        >
                            {move || if loading.get() { "Signing in..." } else { "Sign In" }}
                        </button>
                    </form>
                </div>
            </div>
        </div>
    }
}
