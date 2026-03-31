use leptos::prelude::*;

#[component]
pub fn ConfirmModal(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] on_confirm: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
    #[prop(into)] title: String,
    #[prop(into)] message: String,
    #[prop(into, default = "Confirm".into())] confirm_label: String,
    #[prop(default = false)] danger: bool,
) -> impl IntoView {
    let btn_class = if danger {
        "px-4 py-2 text-sm text-white bg-red-600 hover:bg-red-700 rounded-lg transition-colors"
    } else {
        "px-4 py-2 text-sm text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
    };

    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 z-40 bg-black/50"></div>
        </Show>
        <div
            class="fixed inset-0 z-50 flex items-center justify-center"
            style:display=move || if show.get() { "flex" } else { "none" }
        >
            <div
                class="fixed inset-0"
                on:click=move |_| on_cancel.run(())
            />
            <div class="relative bg-white rounded-lg shadow-xl w-full max-w-md mx-4 z-10">
                <div class="px-6 pt-6 pb-4">
                    <div class="flex items-start space-x-3">
                        {if danger {
                            view! {
                                <div class="flex-shrink-0 w-10 h-10 rounded-full bg-red-100 flex items-center justify-center">
                                    <svg class="w-5 h-5 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                                    </svg>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="flex-shrink-0 w-10 h-10 rounded-full bg-yellow-100 flex items-center justify-center">
                                    <svg class="w-5 h-5 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                    </svg>
                                </div>
                            }.into_any()
                        }}
                        <div>
                            <h3 class="text-base font-semibold text-gray-900">{title}</h3>
                            <p class="mt-1 text-sm text-gray-600">{message}</p>
                        </div>
                    </div>
                </div>
                <div class="flex justify-end space-x-3 px-6 py-4 border-t bg-gray-50 rounded-b-lg">
                    <button
                        class="px-4 py-2 text-sm text-gray-700 bg-white border border-gray-300 hover:bg-gray-50 rounded-lg transition-colors"
                        on:click=move |_| on_cancel.run(())
                    >
                        "Cancel"
                    </button>
                    <button
                        class=btn_class
                        on:click=move |_| on_confirm.run(())
                    >
                        {confirm_label}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn Modal(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    let content = children();

    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 z-40 bg-black/50"></div>
        </Show>
        <div
            class="fixed inset-0 z-50 flex items-center justify-center"
            style:display=move || if show.get() { "flex" } else { "none" }
        >
            <div
                class="fixed inset-0"
                on:click=move |_| on_close.run(())
            />
            <div class="relative bg-white rounded-lg shadow-xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto z-10">
                <div class="flex items-center justify-between px-6 py-4 border-b">
                    <h3 class="text-lg font-semibold text-gray-900">{title}</h3>
                    <button
                        class="text-gray-400 hover:text-gray-600"
                        on:click=move |_| on_close.run(())
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>
                <div class="px-6 py-4">
                    {content}
                </div>
            </div>
        </div>
    }
}
