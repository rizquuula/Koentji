use leptos::prelude::*;

const INPUT_BASE: &str = "w-full px-3 py-2 border border-surface-strong rounded-control focus:ring-2 focus:ring-brand-500 focus:border-brand-500";
const INPUT_READONLY: &str = "w-full px-3 py-2 border border-surface-strong rounded-control bg-surface-subtle text-ink-muted cursor-not-allowed";

/// Text-like form input. Binds two-way to the caller's `RwSignal<String>`.
///
/// `input_type` is any HTML input type that accepts free-form text (text,
/// email, password, number, datetime-local). Radios / checkboxes are not
/// covered — they need a different binding shape.
#[component]
pub fn Input(
    value: RwSignal<String>,
    #[prop(optional, into)] input_type: Option<&'static str>,
    #[prop(optional)] required: bool,
    #[prop(optional, into)] placeholder: Option<&'static str>,
    /// Lowest acceptable numeric value — emitted only for `type="number"`.
    #[prop(optional, into)]
    min: Option<&'static str>,
    /// Reactive read-only flag. A readonly input gets the muted-surface
    /// treatment so it visibly signals its non-interactivity.
    #[prop(optional, into)]
    readonly: Signal<bool>,
) -> impl IntoView {
    let ty = input_type.unwrap_or("text");
    let placeholder = placeholder.unwrap_or("");
    let min = min.unwrap_or("");
    view! {
        <input
            type=ty
            required=required
            placeholder=placeholder
            min=min
            class=move || if readonly.get() { INPUT_READONLY } else { INPUT_BASE }
            prop:value=move || value.get()
            prop:readOnly=move || readonly.get()
            on:input=move |ev| {
                if !readonly.get() {
                    value.set(event_target_value(&ev));
                }
            }
        />
    }
}
