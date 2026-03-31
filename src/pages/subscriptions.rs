use leptos::prelude::*;

use crate::components::layout::Layout;
use crate::components::modal::Modal;
use crate::components::toast::use_toast;
use crate::models::*;
use crate::server::rate_limit_service::list_rate_limit_intervals;
use crate::server::subscription_service::*;

#[component]
pub fn SubscriptionsPage() -> impl IntoView {
    let toast = use_toast();
    let refresh_trigger = RwSignal::new(0u32);

    let subs_resource = Resource::new(
        move || refresh_trigger.get(),
        |_| list_all_subscription_types(),
    );

    let intervals_resource = Resource::new(|| (), |_| list_rate_limit_intervals());

    let show_form = RwSignal::new(false);
    let editing = RwSignal::new(None::<SubscriptionType>);

    let open_create = move |_| {
        editing.set(None);
        show_form.set(true);
    };

    let open_edit = move |sub: SubscriptionType| {
        editing.set(Some(sub));
        show_form.set(true);
    };

    let on_form_submit = Callback::new(move |_: ()| {
        show_form.set(false);
        editing.set(None);
        refresh_trigger.update(|v| *v += 1);
        toast.success("Subscription type saved successfully.");
    });

    let on_form_cancel = Callback::new(move |_: ()| {
        show_form.set(false);
        editing.set(None);
    });

    let handle_toggle_active = move |id: i32, current_active: bool| {
        let toast = toast.clone();
        leptos::task::spawn_local(async move {
            let req = UpdateSubscriptionTypeRequest {
                name: None,
                display_name: None,
                rate_limit_amount: None,
                rate_limit_interval_id: None,
                is_active: Some(!current_active),
            };
            match update_subscription_type(id, req).await {
                Ok(_) => {
                    refresh_trigger.update(|v| *v += 1);
                    toast.success(if current_active {
                        "Subscription deactivated."
                    } else {
                        "Subscription activated."
                    });
                }
                Err(e) => toast.error(&format!("Failed: {}", e)),
            }
        });
    };

    let handle_delete = move |id: i32| {
        let toast = toast.clone();
        leptos::task::spawn_local(async move {
            match delete_subscription_type(id).await {
                Ok(_) => {
                    refresh_trigger.update(|v| *v += 1);
                    toast.success("Subscription type deleted.");
                }
                Err(e) => toast.error(&format!("Failed: {}", e)),
            }
        });
    };

    // Build a lookup map for interval names
    let interval_map = move || {
        intervals_resource
            .get()
            .and_then(|r| r.ok())
            .unwrap_or_default()
    };

    view! {
        <Layout active_tab="subscriptions">
            <div class="space-y-6">
                <div class="flex justify-between items-center">
                    <div>
                        <h1 class="text-2xl font-bold text-gray-900">"Subscription Types"</h1>
                        <p class="text-sm text-gray-500 mt-1">"Manage subscription tiers and their rate limits"</p>
                    </div>
                    <button
                        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors flex items-center space-x-2"
                        on:click=open_create
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        <span>"Add Subscription"</span>
                    </button>
                </div>

                <div class="bg-white rounded-xl shadow-sm border overflow-hidden">
                    <Suspense fallback=|| view! { <div class="p-8 text-center text-gray-400">"Loading..."</div> }>
                        {move || subs_resource.get().map(|result| {
                            let intervals = interval_map();
                            match result {
                                Ok(subs) => view! {
                                    <table class="w-full">
                                        <thead class="bg-gray-50">
                                            <tr>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Name"</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Display Name"</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Rate Limit"</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Interval"</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Status"</th>
                                                <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase">"Actions"</th>
                                            </tr>
                                        </thead>
                                        <tbody class="divide-y divide-gray-200">
                                            {subs.into_iter().map(|sub| {
                                                let sub_edit = sub.clone();
                                                let id = sub.id;
                                                let is_active = sub.is_active;
                                                let interval_name = intervals.iter()
                                                    .find(|i| i.id == sub.rate_limit_interval_id)
                                                    .map(|i| i.display_name.clone())
                                                    .unwrap_or_else(|| "Unknown".to_string());
                                                view! {
                                                    <tr class="hover:bg-gray-50">
                                                        <td class="px-6 py-4 text-sm font-mono text-gray-900">{sub.name.clone()}</td>
                                                        <td class="px-6 py-4 text-sm text-gray-900">{sub.display_name.clone()}</td>
                                                        <td class="px-6 py-4 text-sm text-gray-900">{sub.rate_limit_amount.to_string()}</td>
                                                        <td class="px-6 py-4 text-sm text-gray-500">{interval_name}</td>
                                                        <td class="px-6 py-4">
                                                            <span class={if sub.is_active {
                                                                "px-2 py-1 text-xs font-medium rounded-full bg-green-100 text-green-800"
                                                            } else {
                                                                "px-2 py-1 text-xs font-medium rounded-full bg-gray-100 text-gray-600"
                                                            }}>
                                                                {if sub.is_active { "Active" } else { "Inactive" }}
                                                            </span>
                                                        </td>
                                                        <td class="px-6 py-4 text-right space-x-2">
                                                            <button
                                                                class="text-sm text-blue-600 hover:text-blue-800"
                                                                on:click=move |_| open_edit(sub_edit.clone())
                                                            >
                                                                "Edit"
                                                            </button>
                                                            <button
                                                                class="text-sm text-yellow-600 hover:text-yellow-800"
                                                                on:click=move |_| handle_toggle_active(id, is_active)
                                                            >
                                                                {if is_active { "Deactivate" } else { "Activate" }}
                                                            </button>
                                                            <button
                                                                class="text-sm text-red-600 hover:text-red-800"
                                                                on:click=move |_| handle_delete(id)
                                                            >
                                                                "Delete"
                                                            </button>
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                }.into_any(),
                                Err(e) => view! {
                                    <div class="p-8 text-center text-red-500">{format!("Error: {}", e)}</div>
                                }.into_any(),
                            }
                        })}
                    </Suspense>
                </div>

                <Modal
                    show=Signal::derive(move || show_form.get())
                    on_close=on_form_cancel
                    title="Subscription Type"
                >
                    <SubscriptionForm
                        editing=Signal::derive(move || editing.get())
                        on_submit=on_form_submit
                        on_cancel=on_form_cancel
                    />
                </Modal>
            </div>
        </Layout>
    }
}

#[component]
fn SubscriptionForm(
    #[prop(into)] editing: Signal<Option<SubscriptionType>>,
    #[prop(into)] on_submit: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let is_editing = move || editing.get().is_some();

    let intervals_resource = Resource::new(|| (), |_| list_rate_limit_intervals());

    let name = RwSignal::new(
        editing.get_untracked().as_ref().map(|s| s.name.clone()).unwrap_or_default(),
    );
    let display_name = RwSignal::new(
        editing.get_untracked().as_ref().map(|s| s.display_name.clone()).unwrap_or_default(),
    );
    let rate_limit_amount = RwSignal::new(
        editing.get_untracked().as_ref().map(|s| s.rate_limit_amount.to_string()).unwrap_or_else(|| "6000".to_string()),
    );
    let rate_limit_interval_id = RwSignal::new(
        editing.get_untracked().as_ref().map(|s| s.rate_limit_interval_id.to_string()).unwrap_or_default(),
    );
    let submitting = RwSignal::new(false);

    let editing_id = editing.get_untracked().as_ref().map(|s| s.id);

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        submitting.set(true);

        let name = name.get();
        let display_name = display_name.get();
        let rate_limit_amount = rate_limit_amount.get();
        let rate_limit_interval_id = rate_limit_interval_id.get();
        let on_submit = on_submit.clone();

        leptos::task::spawn_local(async move {
            let result = if let Some(id) = editing_id {
                let req = UpdateSubscriptionTypeRequest {
                    name: Some(name),
                    display_name: Some(display_name),
                    rate_limit_amount: rate_limit_amount.parse().ok(),
                    rate_limit_interval_id: rate_limit_interval_id.parse().ok(),
                    is_active: None,
                };
                update_subscription_type(id, req).await.map(|_| ())
            } else {
                let req = CreateSubscriptionTypeRequest {
                    name,
                    display_name,
                    rate_limit_amount: rate_limit_amount.parse().unwrap_or(6000),
                    rate_limit_interval_id: rate_limit_interval_id.parse().unwrap_or(0),
                };
                create_subscription_type(req).await.map(|_| ())
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
                <label class="block text-sm font-medium text-gray-700 mb-1">"Name *"</label>
                <input
                    type="text"
                    required
                    placeholder="e.g. basic"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                />
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Display Name *"</label>
                <input
                    type="text"
                    required
                    placeholder="e.g. Basic"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || display_name.get()
                    on:input=move |ev| display_name.set(event_target_value(&ev))
                />
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Rate Limit Amount *"</label>
                <input
                    type="number"
                    required
                    min="1"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || rate_limit_amount.get()
                    on:input=move |ev| rate_limit_amount.set(event_target_value(&ev))
                />
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Rate Limit Interval *"</label>
                <Suspense fallback=|| view! { <span class="text-gray-400">"Loading intervals..."</span> }>
                    {move || intervals_resource.get().map(|result| {
                        match result {
                            Ok(intervals) => view! {
                                <select
                                    required
                                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                                    prop:value=move || rate_limit_interval_id.get()
                                    on:change=move |ev| rate_limit_interval_id.set(event_target_value(&ev))
                                >
                                    <option value="">"Select interval..."</option>
                                    {intervals.into_iter().map(|interval| {
                                        let val = interval.id.to_string();
                                        view! {
                                            <option value=val>{interval.display_name}</option>
                                        }
                                    }).collect::<Vec<_>>()}
                                </select>
                            }.into_any(),
                            Err(_) => view! {
                                <span class="text-red-500">"Failed to load intervals"</span>
                            }.into_any(),
                        }
                    })}
                </Suspense>
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
                    {move || if is_editing() { "Update" } else { "Create" }}
                </button>
            </div>
        </form>
    }
}
