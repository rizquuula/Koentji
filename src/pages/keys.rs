use crate::components::key_form::KeyForm;
use crate::components::key_table::KeyTable;
use crate::components::layout::Layout;
use crate::components::modal::{ConfirmModal, Modal};
use crate::models::AuthenticationKey;
use crate::server::key_service::{delete_key, list_keys, reset_rate_limit};
use leptos::prelude::*;
use leptos_router::hooks::query_signal;

#[component]
pub fn KeysPage() -> impl IntoView {
    // URL query params are the source of truth for filters — copy-pasting a
    // link or hitting the back button restores the exact search. `None`
    // encodes the default (page 1, no filter) so empty URLs stay clean.
    let (page_q, set_page) = query_signal::<i32>("page");
    let (search_q, set_search) = query_signal::<String>("search");
    let (subscription_q, set_subscription) = query_signal::<String>("subscription");
    let (status_q, set_status) = query_signal::<String>("status");
    let refresh_counter = RwSignal::new(0u32);

    let page = Signal::derive(move || page_q.get().unwrap_or(1));
    let search = Signal::derive(move || search_q.get().unwrap_or_default());
    let subscription_filter = Signal::derive(move || subscription_q.get().unwrap_or_default());
    let status_filter = Signal::derive(move || status_q.get().unwrap_or_default());

    let show_create_modal = RwSignal::new(false);
    let editing_key = RwSignal::new(None::<AuthenticationKey>);

    let keys_resource = Resource::new(
        move || {
            (
                page.get(),
                search.get(),
                subscription_filter.get(),
                status_filter.get(),
                refresh_counter.get(),
            )
        },
        move |(p, s, sub, st, _)| list_keys(p, s, sub, st),
    );

    let keys_signal = Signal::derive(move || keys_resource.get().and_then(|r| r.ok()));

    let on_edit = Callback::new(move |key: AuthenticationKey| {
        editing_key.set(Some(key));
    });

    let confirm_delete_id = RwSignal::new(None::<i32>);
    let confirm_reset_id = RwSignal::new(None::<i32>);

    let on_delete = Callback::new(move |id: i32| {
        confirm_delete_id.set(Some(id));
    });

    let on_reset = Callback::new(move |id: i32| {
        confirm_reset_id.set(Some(id));
    });

    let do_delete = move || {
        if let Some(id) = confirm_delete_id.get_untracked() {
            confirm_delete_id.set(None);
            leptos::task::spawn_local(async move {
                if let Ok(()) = delete_key(id).await {
                    refresh_counter.update(|c| *c += 1);
                }
            });
        }
    };

    let do_reset = move || {
        if let Some(id) = confirm_reset_id.get_untracked() {
            confirm_reset_id.set(None);
            leptos::task::spawn_local(async move {
                if let Ok(()) = reset_rate_limit(id).await {
                    refresh_counter.update(|c| *c += 1);
                }
            });
        }
    };

    // Debounced search — one cancellable timer reused across keystrokes.
    // The previous incarnation scheduled a fresh `set_timeout` on every
    // input event without cancelling the prior one, so five rapid keys
    // fired five `search.set(...)` calls 300ms apart, each triggering a
    // resource refetch. Keeping the pending handle in a StoredValue and
    // clearing it before re-scheduling collapses N timers back to one.
    //
    // `search_input` is the literal input value (updates on every
    // keystroke); the debounced write lands in the URL query param.
    let search_input = RwSignal::new(search.get_untracked());
    let pending_debounce = StoredValue::new(None::<leptos::prelude::TimeoutHandle>);
    Effect::new(move |_| {
        let val = search_input.get();
        pending_debounce.update_value(|slot| {
            if let Some(handle) = slot.take() {
                handle.clear();
            }
        });
        let handle = set_timeout_with_handle(
            move || {
                set_search.set(if val.is_empty() { None } else { Some(val) });
                set_page.set(None);
            },
            std::time::Duration::from_millis(300),
        );
        if let Ok(handle) = handle {
            pending_debounce.set_value(Some(handle));
        }
    });

    view! {
        <Layout active_tab="keys">
            <div class="space-y-6">
                <div class="flex items-center justify-between">
                    <h1 class="text-2xl font-bold text-gray-900">"API Keys"</h1>
                    <button
                        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors flex items-center space-x-2"
                        on:click=move |_| show_create_modal.set(true)
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        <span>"Create Key"</span>
                    </button>
                </div>

                // Filters
                <div class="bg-white rounded-lg shadow p-4">
                    <div class="flex flex-wrap items-center gap-4">
                        <div class="flex-1 min-w-[200px]">
                            <input
                                type="text"
                                placeholder="Search by device ID, username, or email..."
                                class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                                prop:value=move || search_input.get()
                                on:input=move |ev| search_input.set(event_target_value(&ev))
                            />
                        </div>
                        <select
                            class="px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                            prop:value=move || subscription_filter.get()
                            on:change=move |ev| {
                                let v = event_target_value(&ev);
                                set_subscription.set(if v.is_empty() { None } else { Some(v) });
                                set_page.set(None);
                            }
                        >
                            <option value="">"All Subscriptions"</option>
                            <option value="free">"Free"</option>
                            <option value="basic">"Basic"</option>
                            <option value="pro">"Pro"</option>
                            <option value="enterprise">"Enterprise"</option>
                        </select>
                        <select
                            class="px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                            prop:value=move || status_filter.get()
                            on:change=move |ev| {
                                let v = event_target_value(&ev);
                                set_status.set(if v.is_empty() { None } else { Some(v) });
                                set_page.set(None);
                            }
                        >
                            <option value="">"All Statuses"</option>
                            <option value="active">"Active"</option>
                            <option value="expired">"Expired"</option>
                            <option value="deleted">"Deleted"</option>
                        </select>
                    </div>
                </div>

                // Table
                <Suspense fallback=|| view! {
                    <div class="flex justify-center py-12">
                        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                    </div>
                }>
                    <KeyTable
                        data=keys_signal
                        on_edit=on_edit
                        on_delete=on_delete
                        on_reset=on_reset
                        page=page
                        on_page_change=Callback::new(move |p: i32| {
                            // Page 1 is the default — encode it as "no param" so URLs stay clean.
                            set_page.set(if p <= 1 { None } else { Some(p) });
                        })
                    />
                </Suspense>
            </div>

            // Create Modal
            <Modal
                show=Signal::derive(move || show_create_modal.get())
                on_close=Callback::new(move |_| show_create_modal.set(false))
                title="Create New API Key"
            >
                <KeyForm
                    on_submit=Callback::new(move |_| {
                        show_create_modal.set(false);
                        refresh_counter.update(|c| *c += 1);
                    })
                    on_cancel=Callback::new(move |_| show_create_modal.set(false))
                />
            </Modal>

            // Edit Modal
            <Modal
                show=Signal::derive(move || editing_key.get().is_some())
                on_close=Callback::new(move |_| editing_key.set(None))
                title="Edit API Key"
            >
                {move || editing_key.get().map(|key| {
                    view! {
                        <KeyForm
                            editing=key
                            on_submit=Callback::new(move |_| {
                                editing_key.set(None);
                                refresh_counter.update(|c| *c += 1);
                            })
                            on_cancel=Callback::new(move |_| editing_key.set(None))
                        />
                    }
                })}
            </Modal>

            // Delete Confirmation Modal
            <ConfirmModal
                show=Signal::derive(move || confirm_delete_id.get().is_some())
                on_confirm=Callback::new(move |_| do_delete())
                on_cancel=Callback::new(move |_| confirm_delete_id.set(None))
                title="Revoke API Key"
                message="Are you sure you want to revoke this API key? This action cannot be undone."
                confirm_label="Revoke"
                danger=true
            />

            // Reset Rate Limit Confirmation Modal
            <ConfirmModal
                show=Signal::derive(move || confirm_reset_id.get().is_some())
                on_confirm=Callback::new(move |_| do_reset())
                on_cancel=Callback::new(move |_| confirm_reset_id.set(None))
                title="Reset Rate Limit"
                message="Are you sure you want to reset the rate limit for this API key? The remaining count will be restored to the daily limit."
                confirm_label="Reset"
            />
        </Layout>
    }
}
