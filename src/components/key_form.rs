use crate::components::design::{Button, ButtonType, ButtonVariant, Input, Stack};
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

    let device_id = RwSignal::new(
        key.as_ref()
            .map(|k| k.device_id.clone())
            .unwrap_or_default(),
    );
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

        let st_id: Option<i32> = subscription_type_id_val.parse().ok();

        leptos::task::spawn_local(async move {
            let result = if is_editing {
                let req = UpdateKeyRequest {
                    device_id: Some(device_id),
                    username: if username.is_empty() {
                        None
                    } else {
                        Some(username)
                    },
                    email: if email.is_empty() { None } else { Some(email) },
                    subscription: None, // derived from subscription_type_id on server
                    subscription_type_id: st_id,
                    rate_limit_daily: rate_limit.parse().ok(),
                    expired_at: if expired_at.is_empty() {
                        None
                    } else {
                        Some(expired_at)
                    },
                };
                crate::server::key_service::update_key(editing_id.unwrap(), req)
                    .await
                    .map(|_| ())
            } else {
                let req = CreateKeyRequest {
                    device_id,
                    username: if username.is_empty() {
                        None
                    } else {
                        Some(username)
                    },
                    email: if email.is_empty() { None } else { Some(email) },
                    subscription: None, // derived from subscription_type_id on server
                    subscription_type_id: st_id,
                    rate_limit_daily: rate_limit.parse().ok(),
                    expired_at: if expired_at.is_empty() {
                        None
                    } else {
                        Some(expired_at)
                    },
                };
                crate::server::key_service::create_key(req)
                    .await
                    .map(|_| ())
            };

            submitting.set(false);
            if result.is_ok() {
                on_submit.run(());
            }
        });
    };

    // The subscription <select> cannot use the plain `Select` primitive —
    // selecting a subscription also back-fills the rate-limit field, which
    // is domain coupling that doesn't belong inside a generic Select.
    let readonly_rate = Signal::derive(move || !is_editing);

    view! {
        <form on:submit=handle_submit>
            <Stack>
                <div>
                    <label for="key-device-id" class="block text-sm font-medium text-ink-body mb-1">"Device ID *"</label>
                    <Input id="key-device-id" value=device_id required=true />
                </div>
                <div>
                    <label for="key-username" class="block text-sm font-medium text-ink-body mb-1">"Username"</label>
                    <Input id="key-username" value=username />
                </div>
                <div>
                    <label for="key-email" class="block text-sm font-medium text-ink-body mb-1">"Email"</label>
                    <Input id="key-email" value=email input_type="email" />
                </div>
                <div>
                    <label for="key-subscription" class="block text-sm font-medium text-ink-body mb-1">"Subscription"</label>
                    <Suspense fallback=|| view! { <span class="text-ink-disabled">"Loading..."</span> }>
                        {move || subs_resource.get().map(|result| {
                            match result {
                                Ok(subs) => {
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
                                            id="key-subscription"
                                            class="w-full px-3 py-2 border border-surface-strong rounded-control focus:ring-2 focus:ring-brand-500 focus:border-brand-500"
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
                                    <span class="text-feedback-danger">"Failed to load subscriptions"</span>
                                }.into_any(),
                            }
                        })}
                    </Suspense>
                </div>
                <div>
                    <label for="key-rate-limit" class="block text-sm font-medium text-ink-body mb-1">"Rate Limit"</label>
                    <Input
                        id="key-rate-limit"
                        value=rate_limit
                        input_type="number"
                        min="0"
                        readonly=readonly_rate
                    />
                    <p class="text-xs text-ink-disabled mt-1">
                        {if is_editing { "Override rate limit if needed." } else { "Determined by subscription." }}
                    </p>
                </div>
                <div>
                    <label for="key-expired-at" class="block text-sm font-medium text-ink-body mb-1">"Expiration Date"</label>
                    <Input id="key-expired-at" value=expired_at input_type="datetime-local" />
                </div>
                <div class="flex justify-end space-x-3 pt-4 border-t">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=Callback::new(move |_| on_cancel.run(()))
                    >
                        "Cancel"
                    </Button>
                    <Button
                        variant=ButtonVariant::Primary
                        button_type=ButtonType::Submit
                        disabled=Signal::derive(move || submitting.get())
                    >
                        {if is_editing { "Update Key" } else { "Create Key" }}
                    </Button>
                </div>
            </Stack>
        </form>
    }
}
