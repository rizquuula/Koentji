use leptos::prelude::*;

const SELECT_CLASS: &str = "w-full px-3 py-2 border border-surface-strong rounded-control focus:ring-2 focus:ring-brand-500 focus:border-brand-500";

/// Dropdown control. Children are the `<option>` tags the caller composes.
///
/// Keeping options at the call site preserves whatever `Suspense`/iteration
/// the caller needs (subscription list, interval list, etc). Wiring the
/// two-way binding here is the real duplication saver.
#[component]
pub fn Select(
    value: RwSignal<String>,
    #[prop(optional)] required: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <select
            required=required
            class=SELECT_CLASS
            prop:value=move || value.get()
            on:change=move |ev| value.set(event_target_value(&ev))
        >
            {children()}
        </select>
    }
}
