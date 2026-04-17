use leptos::prelude::*;

#[component]
pub fn DateRangePicker(
    #[prop(into)] range: RwSignal<String>,
    #[prop(into)] start_date: RwSignal<String>,
    #[prop(into)] end_date: RwSignal<String>,
) -> impl IntoView {
    let show_custom = move || range.get() == "custom";

    let button_class = move |r: &str| {
        if range.get() == r {
            "px-3 py-1.5 text-sm font-medium rounded-lg bg-blue-600 text-white"
        } else {
            "px-3 py-1.5 text-sm font-medium rounded-lg bg-white text-gray-700 hover:bg-gray-100 border"
        }
    };

    let class_all = move || button_class("all");
    let class_7d = move || button_class("7d");
    let class_30d = move || button_class("30d");
    let class_90d = move || button_class("90d");
    let class_custom = move || button_class("custom");

    view! {
        <div class="flex items-center space-x-2 flex-wrap gap-y-2">
            <button class=class_all on:click=move |_| range.set("all".to_string())>"All Time"</button>
            <button class=class_7d on:click=move |_| range.set("7d".to_string())>"7 Days"</button>
            <button class=class_30d on:click=move |_| range.set("30d".to_string())>"30 Days"</button>
            <button class=class_90d on:click=move |_| range.set("90d".to_string())>"90 Days"</button>
            <button class=class_custom on:click=move |_| range.set("custom".to_string())>"Custom"</button>

            <Show when=show_custom>
                <div class="flex items-center space-x-2 ml-2">
                    <input
                        type="date"
                        class="px-2 py-1 text-sm border rounded-lg"
                        prop:value=move || start_date.get()
                        on:input=move |ev| start_date.set(event_target_value(&ev))
                    />
                    <span class="text-gray-400">"to"</span>
                    <input
                        type="date"
                        class="px-2 py-1 text-sm border rounded-lg"
                        prop:value=move || end_date.get()
                        on:input=move |ev| end_date.set(event_target_value(&ev))
                    />
                </div>
            </Show>
        </div>
    }
}
