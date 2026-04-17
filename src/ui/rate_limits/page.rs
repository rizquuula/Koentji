use leptos::prelude::*;

use crate::models::*;
use crate::server::rate_limit_service::*;
use crate::ui::design::modal::{ConfirmModal, Modal};
use crate::ui::design::toast::use_toast;
use crate::ui::design::{
    Badge, BadgeTone, Button, ButtonType, ButtonVariant, DataTable, Input, PageHeader, Stack,
};
use crate::ui::shell::layout::Layout;

#[component]
pub fn LimitsIntervalPage() -> impl IntoView {
    let toast = use_toast();
    let refresh_trigger = RwSignal::new(0u32);

    let intervals_resource = Resource::new(
        move || refresh_trigger.get(),
        |_| list_all_rate_limit_intervals(),
    );

    let confirm_delete_id = RwSignal::new(None::<i32>);

    let show_form = RwSignal::new(false);
    let editing = RwSignal::new(None::<RateLimitInterval>);

    let open_create = move |_| {
        editing.set(None);
        show_form.set(true);
    };

    let open_edit = move |interval: RateLimitInterval| {
        editing.set(Some(interval));
        show_form.set(true);
    };

    let on_form_submit = Callback::new(move |_: ()| {
        show_form.set(false);
        editing.set(None);
        refresh_trigger.update(|v| *v += 1);
        toast.success("Rate limit interval saved successfully.");
    });

    let on_form_cancel = Callback::new(move |_: ()| {
        show_form.set(false);
        editing.set(None);
    });

    let handle_toggle_active = move |id: i32, current_active: bool| {
        leptos::task::spawn_local(async move {
            let req = UpdateRateLimitIntervalRequest {
                name: None,
                display_name: None,
                duration_seconds: None,
                is_active: Some(!current_active),
            };
            match update_rate_limit_interval(id, req).await {
                Ok(_) => {
                    refresh_trigger.update(|v| *v += 1);
                    toast.success(if current_active {
                        "Interval deactivated."
                    } else {
                        "Interval activated."
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
                match delete_rate_limit_interval(id).await {
                    Ok(_) => {
                        refresh_trigger.update(|v| *v += 1);
                        toast.success("Interval deleted.");
                    }
                    Err(e) => toast.error(&format!("Failed: {}", e)),
                }
            });
        }
    };

    view! {
        <Layout active_tab="limits_interval">
            <div class="space-y-6">
                <div class="flex justify-between items-center">
                    <PageHeader
                        title="Rate Limit Intervals"
                        subtitle="Manage available rate limit interval periods"
                    />
                    <Button
                        variant=ButtonVariant::Primary
                        on_click=Callback::new(open_create)
                    >
                        <span class="inline-flex items-center space-x-2">
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                            </svg>
                            <span>"Add Interval"</span>
                        </span>
                    </Button>
                </div>

                <Suspense fallback=|| view! {
                    <div class="bg-surface-base rounded-card shadow-raised border border-surface-border p-8 text-center text-ink-disabled">
                        "Loading..."
                    </div>
                }>
                    {move || intervals_resource.get().map(|result| {
                        match result {
                            Ok(intervals) => view! {
                                <DataTable headers=vec!["Name", "Display Name", "Duration", "Status", "Actions"]>
                                    {intervals.into_iter().map(|interval| {
                                        let interval_edit = interval.clone();
                                        let id = interval.id;
                                        let is_active = interval.is_active;
                                        view! {
                                            <tr class="hover:bg-surface-subtle">
                                                <td class="px-6 py-4 text-sm font-mono text-ink-heading">{interval.name.clone()}</td>
                                                <td class="px-6 py-4 text-sm text-ink-heading">{interval.display_name.clone()}</td>
                                                <td class="px-6 py-4 text-sm text-ink-muted">{format_duration(interval.duration_seconds)}</td>
                                                <td class="px-6 py-4">
                                                    <Badge tone=if interval.is_active { BadgeTone::Success } else { BadgeTone::Neutral }>
                                                        {if interval.is_active { "Active" } else { "Inactive" }}
                                                    </Badge>
                                                </td>
                                                <td class="px-6 py-4 text-right space-x-2">
                                                    <button
                                                        class="text-sm text-brand-600 hover:text-brand-800"
                                                        on:click=move |_| open_edit(interval_edit.clone())
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
                        title="Rate Limit Interval"
                    >
                        <RateLimitIntervalForm
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
                title="Delete Interval"
                message="Are you sure you want to delete this rate limit interval? This action cannot be undone."
                confirm_label="Delete"
                danger=true
            />
        </Layout>
    }
}

fn format_duration(seconds: i64) -> String {
    if seconds < 60 {
        format!("{} second{}", seconds, if seconds == 1 { "" } else { "s" })
    } else if seconds < 3600 {
        let mins = seconds / 60;
        format!("{} minute{}", mins, if mins == 1 { "" } else { "s" })
    } else if seconds < 86400 {
        let hours = seconds / 3600;
        format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
    } else if seconds < 604800 {
        let days = seconds / 86400;
        format!("{} day{}", days, if days == 1 { "" } else { "s" })
    } else if seconds < 2592000 {
        let weeks = seconds / 604800;
        format!("{} week{}", weeks, if weeks == 1 { "" } else { "s" })
    } else if seconds < 31536000 {
        let months = seconds / 2592000;
        format!("{} month{}", months, if months == 1 { "" } else { "s" })
    } else {
        let years = seconds / 31536000;
        format!("{} year{}", years, if years == 1 { "" } else { "s" })
    }
}

#[component]
fn RateLimitIntervalForm(
    #[prop(into)] editing: Signal<Option<RateLimitInterval>>,
    #[prop(into)] on_submit: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    let is_editing = move || editing.get().is_some();

    let name = RwSignal::new(
        editing
            .get_untracked()
            .as_ref()
            .map(|i| i.name.clone())
            .unwrap_or_default(),
    );
    let display_name = RwSignal::new(
        editing
            .get_untracked()
            .as_ref()
            .map(|i| i.display_name.clone())
            .unwrap_or_default(),
    );
    let duration_seconds = RwSignal::new(
        editing
            .get_untracked()
            .as_ref()
            .map(|i| i.duration_seconds.to_string())
            .unwrap_or_default(),
    );
    let submitting = RwSignal::new(false);

    let editing_id = editing.get_untracked().as_ref().map(|i| i.id);

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        submitting.set(true);

        let name = name.get();
        let display_name = display_name.get();
        let duration_seconds = duration_seconds.get();

        leptos::task::spawn_local(async move {
            let result = if let Some(id) = editing_id {
                let req = UpdateRateLimitIntervalRequest {
                    name: Some(name),
                    display_name: Some(display_name),
                    duration_seconds: duration_seconds.parse().ok(),
                    is_active: None,
                };
                update_rate_limit_interval(id, req).await.map(|_| ())
            } else {
                let req = CreateRateLimitIntervalRequest {
                    name,
                    display_name,
                    duration_seconds: duration_seconds.parse().unwrap_or(0),
                };
                create_rate_limit_interval(req).await.map(|_| ())
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
                    <label for="interval-name" class="block text-sm font-medium text-ink-body mb-1">"Name *"</label>
                    <Input id="interval-name" value=name required=true placeholder="e.g. 3_hourly" />
                </div>
                <div>
                    <label for="interval-display-name" class="block text-sm font-medium text-ink-body mb-1">"Display Name *"</label>
                    <Input id="interval-display-name" value=display_name required=true placeholder="e.g. 3 Hourly" />
                </div>
                <div>
                    <label for="interval-duration" class="block text-sm font-medium text-ink-body mb-1">"Duration (seconds) *"</label>
                    <Input
                        id="interval-duration"
                        value=duration_seconds
                        required=true
                        input_type="number"
                        min="1"
                        placeholder="e.g. 10800"
                    />
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
