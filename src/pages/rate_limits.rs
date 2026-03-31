use leptos::prelude::*;

use crate::components::layout::Layout;
use crate::components::modal::Modal;
use crate::components::toast::use_toast;
use crate::models::*;
use crate::server::rate_limit_service::*;

#[component]
pub fn RateLimitsPage() -> impl IntoView {
    let toast = use_toast();
    let refresh_trigger = RwSignal::new(0u32);

    let intervals_resource = Resource::new(
        move || refresh_trigger.get(),
        |_| list_all_rate_limit_intervals(),
    );

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
        let toast = toast.clone();
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

    let handle_delete = move |id: i32| {
        let toast = toast.clone();
        leptos::task::spawn_local(async move {
            match delete_rate_limit_interval(id).await {
                Ok(_) => {
                    refresh_trigger.update(|v| *v += 1);
                    toast.success("Interval deleted.");
                }
                Err(e) => toast.error(&format!("Failed: {}", e)),
            }
        });
    };

    view! {
        <Layout active_tab="rate_limits">
            <div class="space-y-6">
                <div class="flex justify-between items-center">
                    <div>
                        <h1 class="text-2xl font-bold text-gray-900">"Rate Limit Intervals"</h1>
                        <p class="text-sm text-gray-500 mt-1">"Manage available rate limit interval periods"</p>
                    </div>
                    <button
                        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors flex items-center space-x-2"
                        on:click=open_create
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        <span>"Add Interval"</span>
                    </button>
                </div>

                <div class="bg-white rounded-xl shadow-sm border overflow-hidden">
                    <Suspense fallback=|| view! { <div class="p-8 text-center text-gray-400">"Loading..."</div> }>
                        {move || intervals_resource.get().map(|result| {
                            match result {
                                Ok(intervals) => view! {
                                    <table class="w-full">
                                        <thead class="bg-gray-50">
                                            <tr>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Name"</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Display Name"</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Duration"</th>
                                                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Status"</th>
                                                <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase">"Actions"</th>
                                            </tr>
                                        </thead>
                                        <tbody class="divide-y divide-gray-200">
                                            {intervals.into_iter().map(|interval| {
                                                let interval_edit = interval.clone();
                                                let id = interval.id;
                                                let is_active = interval.is_active;
                                                view! {
                                                    <tr class="hover:bg-gray-50">
                                                        <td class="px-6 py-4 text-sm font-mono text-gray-900">{interval.name.clone()}</td>
                                                        <td class="px-6 py-4 text-sm text-gray-900">{interval.display_name.clone()}</td>
                                                        <td class="px-6 py-4 text-sm text-gray-500">{format_duration(interval.duration_seconds)}</td>
                                                        <td class="px-6 py-4">
                                                            <span class={if interval.is_active {
                                                                "px-2 py-1 text-xs font-medium rounded-full bg-green-100 text-green-800"
                                                            } else {
                                                                "px-2 py-1 text-xs font-medium rounded-full bg-gray-100 text-gray-600"
                                                            }}>
                                                                {if interval.is_active { "Active" } else { "Inactive" }}
                                                            </span>
                                                        </td>
                                                        <td class="px-6 py-4 text-right space-x-2">
                                                            <button
                                                                class="text-sm text-blue-600 hover:text-blue-800"
                                                                on:click=move |_| open_edit(interval_edit.clone())
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
        editing.get_untracked().as_ref().map(|i| i.name.clone()).unwrap_or_default(),
    );
    let display_name = RwSignal::new(
        editing.get_untracked().as_ref().map(|i| i.display_name.clone()).unwrap_or_default(),
    );
    let duration_seconds = RwSignal::new(
        editing.get_untracked().as_ref().map(|i| i.duration_seconds.to_string()).unwrap_or_default(),
    );
    let submitting = RwSignal::new(false);

    let editing_id = editing.get_untracked().as_ref().map(|i| i.id);

    let handle_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        submitting.set(true);

        let name = name.get();
        let display_name = display_name.get();
        let duration_seconds = duration_seconds.get();
        let on_submit = on_submit.clone();

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
        <form on:submit=handle_submit class="space-y-4">
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Name *"</label>
                <input
                    type="text"
                    required
                    placeholder="e.g. 3_hourly"
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
                    placeholder="e.g. 3 Hourly"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || display_name.get()
                    on:input=move |ev| display_name.set(event_target_value(&ev))
                />
            </div>
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Duration (seconds) *"</label>
                <input
                    type="number"
                    required
                    min="1"
                    placeholder="e.g. 10800"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    prop:value=move || duration_seconds.get()
                    on:input=move |ev| duration_seconds.set(event_target_value(&ev))
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
                    {move || if is_editing() { "Update" } else { "Create" }}
                </button>
            </div>
        </form>
    }
}
