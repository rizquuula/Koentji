use crate::components::key_form::KeyForm;
use crate::components::key_table::KeyTable;
use crate::components::layout::Layout;
use crate::components::modal::Modal;
use crate::models::AuthenticationKey;
use crate::server::key_service::{delete_key, list_keys, reset_rate_limit};
use leptos::prelude::*;

#[component]
pub fn KeysPage() -> impl IntoView {
    let page = RwSignal::new(1i32);
    let search = RwSignal::new(String::new());
    let subscription_filter = RwSignal::new(String::new());
    let status_filter = RwSignal::new(String::new());
    let refresh_counter = RwSignal::new(0u32);

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

    let keys_signal = Signal::derive(move || {
        keys_resource.get().and_then(|r| r.ok())
    });

    let on_edit = Callback::new(move |key: AuthenticationKey| {
        editing_key.set(Some(key));
    });

    let on_delete = Callback::new(move |id: i32| {
        leptos::task::spawn_local(async move {
            if let Ok(()) = delete_key(id).await {
                refresh_counter.update(|c| *c += 1);
            }
        });
    });

    let on_reset = Callback::new(move |id: i32| {
        leptos::task::spawn_local(async move {
            if let Ok(()) = reset_rate_limit(id).await {
                refresh_counter.update(|c| *c += 1);
            }
        });
    });

    // Debounced search
    let search_input = RwSignal::new(String::new());
    Effect::new(move || {
        let val = search_input.get();
        set_timeout(
            move || {
                search.set(val);
                page.set(1);
            },
            std::time::Duration::from_millis(300),
        );
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
                                subscription_filter.set(event_target_value(&ev));
                                page.set(1);
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
                                status_filter.set(event_target_value(&ev));
                                page.set(1);
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
        </Layout>
    }
}
