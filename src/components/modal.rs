use leptos::prelude::*;

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
