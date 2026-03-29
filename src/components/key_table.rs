use crate::components::key_row::KeyRow;
use crate::models::{AuthenticationKey, KeyListResponse};
use leptos::prelude::*;

#[component]
pub fn KeyTable(
    #[prop(into)] data: Signal<Option<KeyListResponse>>,
    #[prop(into)] on_edit: Callback<AuthenticationKey>,
    #[prop(into)] on_delete: Callback<i32>,
    #[prop(into)] on_reset: Callback<i32>,
    #[prop(into)] page: RwSignal<i32>,
) -> impl IntoView {
    let total_pages = move || {
        data.get()
            .map(|d| ((d.total as f64) / (d.per_page as f64)).ceil() as i32)
            .unwrap_or(1)
            .max(1)
    };

    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <div class="overflow-x-auto">
                <table class="w-full">
                    <thead class="bg-gray-50 border-b">
                        <tr>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"API Key"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Device ID"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"User"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Subscription"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Rate Limit"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Status"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Created"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Actions"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            data.get().map(|d| {
                                if d.keys.is_empty() {
                                    view! {
                                        <tr>
                                            <td colspan="8" class="px-4 py-8 text-center text-gray-500">
                                                "No API keys found"
                                            </td>
                                        </tr>
                                    }.into_any()
                                } else {
                                    d.keys.into_iter().map(|key| {
                                        let on_edit = on_edit.clone();
                                        let on_delete = on_delete.clone();
                                        let on_reset = on_reset.clone();
                                        view! {
                                            <KeyRow
                                                key=key
                                                on_edit=on_edit
                                                on_delete=on_delete
                                                on_reset=on_reset
                                            />
                                        }
                                    }).collect_view().into_any()
                                }
                            })
                        }}
                    </tbody>
                </table>
            </div>

            // Pagination
            <div class="flex items-center justify-between px-4 py-3 border-t bg-gray-50">
                <div class="text-sm text-gray-500">
                    {move || {
                        data.get().map(|d| format!("Showing page {} of {} ({} total keys)", d.page, total_pages(), d.total))
                    }}
                </div>
                <div class="flex space-x-2">
                    <button
                        class="px-3 py-1 text-sm border rounded hover:bg-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || page.get() <= 1
                        on:click=move |_| page.set(page.get() - 1)
                    >
                        "Previous"
                    </button>
                    <button
                        class="px-3 py-1 text-sm border rounded hover:bg-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || page.get() >= total_pages()
                        on:click=move |_| page.set(page.get() + 1)
                    >
                        "Next"
                    </button>
                </div>
            </div>
        </div>
    }
}
