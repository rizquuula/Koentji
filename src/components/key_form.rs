use crate::models::{AuthenticationKey, CreateKeyRequest, UpdateKeyRequest};
use leptos::prelude::*;

#[component]
pub fn KeyForm(
    #[prop(optional)] editing: Option<AuthenticationKey>,
    #[prop(into)] on_submit: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let is_editing = editing.is_some();
    let key = editing.clone();

    let device_id = RwSignal::new(key.as_ref().map(|k| k.device_id.clone()).unwrap_or_default());
    let username = RwSignal::new(
        key.as_ref()
            .and_then(|k| k.username.clone())
            .unwrap_or_default(),
    );
    let email = RwSignal::new(
        key.as_ref()
            .and_then(|k| k.email.clone())
            .unwrap_or_default(),
    );
    let subscription = RwSignal::new(
        key.as_ref()
            .and_then(|k| k.subscription.clone())
            .unwrap_or_default(),
    );
    let rate_limit = RwSignal::new(
        key.as_ref()
            .map(|k| k.rate_limit_daily.to_string())
            .unwrap_or_else(|| "6000".to_string()),
    );
    let expired_at = RwSignal::new(
        key.as_ref()
            .and_then(|k| k.expired_at.map(|e| e.format("%Y-%m-%dT%H:%M").to_string()))
            .unwrap_or_default(),
    );
    let submitting = RwSignal::new(false);

    let editing_id = key.as_ref().map(|k| k.id);

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        submitting.set(true);

        let device_id = device_id.get();
        let username = username.get();
        let email = email.get();
        let subscription = subscription.get();
        let rate_limit = rate_limit.get();
        let expired_at = expired_at.get();
        let on_submit = on_submit.clone();

        leptos::task::spawn_local(async move {
            let result = if is_editing {
                let req = UpdateKeyRequest {
                    device_id: Some(device_id),
                    username: if username.is_empty() { None } else { Some(username) },
                    email: if email.is_empty() { None } else { Some(email) },
                    subscription: if subscription.is_empty() { None } else { Some(subscription) },
                    rate_limit_daily: rate_limit.parse().ok(),
                    expired_at: if expired_at.is_empty() { None } else { Some(expired_at) },
                };
                crate::server::key_service::update_key(editing_id.unwrap(), req)
                    .await
                    .map(|_| ())
            } else {
                let req = CreateKeyRequest {
                    device_id,
                    username: if username.is_empty() { None } else { Some(username) },
                    email: if email.is_empty() { None } else { Some(email) },
                    subscription: if subscription.is_empty() { None } else { Some(subscription) },
                    rate_limit_daily: rate_limit.parse().ok(),
                    expired_at: if expired_at.is_empty() { None } else { Some(expired_at) },
                };
                crate::server::key_service::create_key(req).await.map(|_| ())
            };

            submitting.set(false);
            if result.is_ok() {
                on_submit.run(());
            }
        });
    };

    view! {
        <form on:submit=handle_submit class="space-y-4">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Device ID *"</label>
                <input
                    type="text"
                    required
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || device_id.get()
                    on:input=move |ev| device_id.set(event_target_value(&ev))
                />
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Username"</label>
                <input
                    type="text"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || username.get()
                    on:input=move |ev| username.set(event_target_value(&ev))
                />
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Email"</label>
                <input
                    type="email"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || email.get()
                    on:input=move |ev| email.set(event_target_value(&ev))
                />
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Subscription"</label>
                <select
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || subscription.get()
                    on:change=move |ev| subscription.set(event_target_value(&ev))
                >
                    <option value="">"Select..."</option>
                    <option value="free">"Free"</option>
                    <option value="basic">"Basic"</option>
                    <option value="pro">"Pro"</option>
                    <option value="enterprise">"Enterprise"</option>
                </select>
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Daily Rate Limit"</label>
                <input
                    type="number"
                    min="0"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || rate_limit.get()
                    on:input=move |ev| rate_limit.set(event_target_value(&ev))
                />
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Expiration Date"</label>
                <input
                    type="datetime-local"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || expired_at.get()
                    on:input=move |ev| expired_at.set(event_target_value(&ev))
                />
            </div>
            <div class="flex justify-end space-x-3 pt-4 border-t">
                <button
                    type="button"
                    class="px-4 py-2 text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-lg transition-colors"
                    on:click=move |_| on_cancel.run(())
                >
                    "Cancel"
                </button>
                <button
                    type="submit"
                    class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50"
                    disabled=move || submitting.get()
                >
                    {if is_editing { "Update Key" } else { "Create Key" }}
                </button>
            </div>
        </form>
    }
}
