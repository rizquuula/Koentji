use leptos::prelude::*;

use crate::models::*;
use crate::server::rate_limit_service::list_rate_limit_intervals;
use crate::server::subscription_service::*;
use crate::ui::design::modal::{ConfirmModal, Modal};
use crate::ui::design::toast::use_toast;
use crate::ui::design::{
    Badge, BadgeTone, Button, ButtonType, ButtonVariant, DataTable, Input, PageHeader, Select,
    Stack,
};
use crate::ui::shell::layout::Layout;

#[component]
pub fn SubscriptionsPage() -> impl IntoView {
    let toast = use_toast();
    let refresh_trigger = RwSignal::new(0u32);

    let subs_resource = Resource::new(
        move || refresh_trigger.get(),
        |_| list_all_subscription_types(),
    );

    let intervals_resource = Resource::new(|| (), |_| list_rate_limit_intervals());

    let confirm_delete_id = RwSignal::new(None::<i32>);

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

    let do_delete = move || {
        if let Some(id) = confirm_delete_id.get_untracked() {
            confirm_delete_id.set(None);
            leptos::task::spawn_local(async move {
                match delete_subscription_type(id).await {
                    Ok(_) => {
                        refresh_trigger.update(|v| *v += 1);
                        toast.success("Subscription type deleted.");
                    }
                    Err(e) => toast.error(&format!("Failed: {}", e)),
                }
            });
        }
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
                    <PageHeader
                        title="Subscription Types"
                        subtitle="Manage subscription tiers and their rate limits"
                    />
                    <Button
                        variant=ButtonVariant::Primary
                        on_click=Callback::new(open_create)
                    >
                        <span class="inline-flex items-center space-x-2">
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                            </svg>
                            <span>"Add Subscription"</span>
                        </span>
                    </Button>
                </div>

                <Suspense fallback=|| view! {
                    <div class="bg-surface-base rounded-card shadow-raised border border-surface-border p-8 text-center text-ink-disabled">
                        "Loading..."
                    </div>
                }>
                    {move || subs_resource.get().map(|result| {
                        let intervals = interval_map();
                        match result {
                            Ok(subs) => view! {
                                <DataTable headers=vec!["Name", "Display Name", "Rate Limit", "Interval", "Status", "Actions"]>
                                    {subs.into_iter().map(|sub| {
                                        let sub_edit = sub.clone();
                                        let id = sub.id;
                                        let is_active = sub.is_active;
                                        let interval_name = intervals.iter()
                                            .find(|i| i.id == sub.rate_limit_interval_id)
                                            .map(|i| i.display_name.clone())
                                            .unwrap_or_else(|| "Unknown".to_string());
                                        view! {
                                            <tr class="hover:bg-surface-subtle">
                                                <td class="px-6 py-4 text-sm font-mono text-ink-heading">{sub.name.clone()}</td>
                                                <td class="px-6 py-4 text-sm text-ink-heading">{sub.display_name.clone()}</td>
                                                <td class="px-6 py-4 text-sm text-ink-heading">{sub.rate_limit_amount.to_string()}</td>
                                                <td class="px-6 py-4 text-sm text-ink-muted">{interval_name}</td>
                                                <td class="px-6 py-4">
                                                    <Badge tone=if sub.is_active { BadgeTone::Success } else { BadgeTone::Neutral }>
                                                        {if sub.is_active { "Active" } else { "Inactive" }}
                                                    </Badge>
                                                </td>
                                                <td class="px-6 py-4 text-right space-x-2">
                                                    <button
                                                        class="text-sm text-brand-600 hover:text-brand-800"
                                                        on:click=move |_| open_edit(sub_edit.clone())
                                                    >
                                                        "Edit"
                                                    </button>
                                                    <button
                                                        class="text-sm text-feedback-warning hover:text-feedback-warning-ink"
                                                        on:click=move |_| handle_toggle_active(id, is_active)
                                                    >
                                                        {if is_active { "Deactivate" } else { "Activate" }}
                                                    </button>
                                                    <button
                                                        class="text-sm text-feedback-danger hover:text-feedback-danger-ink"
                                                        on:click=move |_| confirm_delete_id.set(Some(id))
                                                    >
                                                        "Delete"
                                                    </button>
                                                </td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </DataTable>
                            }.into_any(),
                            Err(e) => view! {
                                <div class="bg-surface-base rounded-card shadow-raised border border-surface-border p-8 text-center text-feedback-danger">
                                    {format!("Error: {}", e)}
                                </div>
                            }.into_any(),
                        }
                    })}
                </Suspense>

                <Show when=move || show_form.get()>
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
                </Show>
            </div>

            <ConfirmModal
                show=Signal::derive(move || confirm_delete_id.get().is_some())
                on_confirm=Callback::new(move |_| do_delete())
                on_cancel=Callback::new(move |_| confirm_delete_id.set(None))
                title="Delete Subscription Type"
                message="Are you sure you want to delete this subscription type? This action cannot be undone."
                confirm_label="Delete"
                danger=true
            />
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
        editing
            .get_untracked()
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_default(),
    );
    let display_name = RwSignal::new(
        editing
            .get_untracked()
            .as_ref()
            .map(|s| s.display_name.clone())
            .unwrap_or_default(),
    );
    let rate_limit_amount = RwSignal::new(
        editing
            .get_untracked()
            .as_ref()
            .map(|s| s.rate_limit_amount.to_string())
            .unwrap_or_else(|| "6000".to_string()),
    );
    let rate_limit_interval_id = RwSignal::new(
        editing
            .get_untracked()
            .as_ref()
            .map(|s| s.rate_limit_interval_id.to_string())
            .unwrap_or_default(),
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
        <form on:submit=handle_submit>
            <Stack>
                <div>
                    <label for="sub-name" class="block text-sm font-medium text-ink-body mb-1">"Name *"</label>
                    <Input id="sub-name" value=name required=true placeholder="e.g. basic" />
                </div>
                <div>
                    <label for="sub-display-name" class="block text-sm font-medium text-ink-body mb-1">"Display Name *"</label>
                    <Input id="sub-display-name" value=display_name required=true placeholder="e.g. Basic" />
                </div>
                <div>
                    <label for="sub-rate-amount" class="block text-sm font-medium text-ink-body mb-1">"Rate Limit Amount *"</label>
                    <Input id="sub-rate-amount" value=rate_limit_amount required=true input_type="number" min="1" />
                </div>
                <div>
                    <label for="sub-rate-interval" class="block text-sm font-medium text-ink-body mb-1">"Rate Limit Interval *"</label>
                    <Suspense fallback=|| view! { <span class="text-ink-disabled">"Loading intervals..."</span> }>
                        {move || intervals_resource.get().map(|result| {
                            match result {
                                Ok(intervals) => view! {
                                    <Select id="sub-rate-interval" value=rate_limit_interval_id required=true>
                                        <option value="">"Select interval..."</option>
                                        {intervals.into_iter().map(|interval| {
                                            let val = interval.id.to_string();
                                            view! {
                                                <option value=val>{interval.display_name}</option>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </Select>
                                }.into_any(),
                                Err(_) => view! {
                                    <span class="text-feedback-danger">"Failed to load intervals"</span>
                                }.into_any(),
                            }
                        })}
                    </Suspense>
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
                        {move || if is_editing() { "Update" } else { "Create" }}
                    </Button>
                </div>
            </Stack>
        </form>
    }
}
