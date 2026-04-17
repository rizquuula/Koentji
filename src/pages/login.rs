use crate::auth::{get_current_user, login};
use crate::components::design::surface::SurfaceElevation;
use crate::components::design::{Button, ButtonType, Input, Stack, Surface};
use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    let user_resource = Resource::new(|| (), |_| get_current_user());

    Effect::new(move |_| {
        if let Some(Ok(Some(_))) = user_resource.get() {
            leptos::prelude::window()
                .location()
                .set_href("/dashboard")
                .ok();
        }
    });

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
        <div class="min-h-screen flex items-center justify-center bg-surface-subtle">
            <div class="max-w-md w-full mx-4">
                <div class="text-center mb-8">
                    <svg class="w-16 h-16 text-brand-600 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/>
                    </svg>
                    <h1 class="text-3xl font-bold text-ink-heading mt-4">"Koentji"</h1>
                    <p class="text-ink-muted mt-2">"API Key Management Dashboard"</p>
                </div>

                <Surface elevation=SurfaceElevation::Overlay padded=true>
                    <form on:submit=handle_submit>
                        <Stack gap=crate::components::design::StackGap::Loose>
                            <Show when=move || error.get().is_some()>
                                <div
                                    role="alert"
                                    class="bg-red-50 border border-red-200 text-feedback-danger-ink px-4 py-3 rounded-control text-sm"
                                >
                                    {move || error.get().unwrap_or_default()}
                                </div>
                            </Show>

                            <div>
                                <label for="login-username" class="block text-sm font-medium text-ink-body mb-1">"Username"</label>
                                <Input id="login-username" value=username required=true placeholder="Enter your username" />
                            </div>

                            <div>
                                <label for="login-password" class="block text-sm font-medium text-ink-body mb-1">"Password"</label>
                                <Input
                                    id="login-password"
                                    value=password
                                    input_type="password"
                                    required=true
                                    placeholder="Enter your password"
                                />
                            </div>

                            <Button
                                button_type=ButtonType::Submit
                                full_width=true
                                disabled=Signal::derive(move || loading.get())
                            >
                                {move || if loading.get() { "Signing in..." } else { "Sign In" }}
                            </Button>
                        </Stack>
                    </form>
                </Surface>
            </div>
        </div>
    }
}
