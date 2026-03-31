use crate::models::{AuthenticationKey, CreateKeyRequest, UpdateKeyRequest};
use crate::server::subscription_service::list_subscription_types;
use leptos::prelude::*;

#[component]
pub fn KeyForm(
    #[prop(optional)] editing: Option<AuthenticationKey>,
    #[prop(into)] on_submit: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let is_editing = editing.is_some();
    let key = editing.clone();

    let subs_resource = Resource::new(|| (), |_| list_subscription_types());

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
    let subscription_type_id = RwSignal::new(
        key.as_ref()
            .and_then(|k| k.subscription_type_id)
            .map(|id| id.to_string())
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
        let subscription_type_id_val = subscription_type_id.get();
        let rate_limit = rate_limit.get();
        let expired_at = expired_at.get();
        let on_submit = on_submit.clone();

        let st_id: Option<i32> = subscription_type_id_val.parse().ok();

        leptos::task::spawn_local(async move {
            let result = if is_editing {
                let req = UpdateKeyRequest {
                    device_id: Some(device_id),
                    username: if username.is_empty() { None } else { Some(username) },
                    email: if email.is_empty() { None } else { Some(email) },
                    subscription: None, // derived from subscription_type_id on server
                    subscription_type_id: st_id,
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
                    subscription: None, // derived from subscription_type_id on server
                    subscription_type_id: st_id,
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
                <Suspense fallback=|| view! { <span class="text-gray-400">"Loading..."</span> }>
                    {move || subs_resource.get().map(|result| {
                        match result {
                            Ok(subs) => {
                                // When subscription changes, auto-fill rate limit
                                let subs_for_change = subs.clone();
                                let on_sub_change = move |ev: leptos::ev::Event| {
                                    let val = event_target_value(&ev);
                                    subscription_type_id.set(val.clone());
                                    if let Ok(id) = val.parse::<i32>() {
                                        if let Some(sub) = subs_for_change.iter().find(|s| s.id == id) {
                                            rate_limit.set(sub.rate_limit_amount.to_string());
                                        }
                                    }
                                };
                                view! {
                                    <select
                                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                                        prop:value=move || subscription_type_id.get()
                                        on:change=on_sub_change
                                    >
                                        <option value="">"Select..."</option>
                                        {subs.into_iter().map(|sub| {
                                            let val = sub.id.to_string();
                                            view! {
                                                <option value=val>{sub.display_name}</option>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </select>
                                }.into_any()
                            },
                            Err(_) => view! {
                                <span class="text-red-500">"Failed to load subscriptions"</span>
                            }.into_any(),
                        }
                    })}
                </Suspense>
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Rate Limit"</label>
                <input
                    type="number"
                    min="0"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg bg-gray-50 text-gray-500 cursor-not-allowed"
                    prop:value=move || rate_limit.get()
                    readonly=move || !is_editing
                    on:input=move |ev| { if is_editing { rate_limit.set(event_target_value(&ev)) } }
                />
                <p class="text-xs text-gray-400 mt-1">
                    {if is_editing { "Override rate limit if needed." } else { "Determined by subscription." }}
                </p>
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
