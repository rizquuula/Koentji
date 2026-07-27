use leptos::prelude::*;

// Every state is a whole literal class string. Tailwind's JIT scanner reads
// `./src/**/*.rs` for concrete classes — `format!` or string concatenation
// would hide them and the utilities would never be emitted.
const ROOT_ENABLED: &str = "mt-2 inline-flex items-center gap-2 py-2 rounded-control cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2";
const ROOT_DISABLED: &str = "mt-2 inline-flex items-center gap-2 py-2 rounded-control opacity-50 cursor-not-allowed focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 focus-visible:ring-offset-2";

const TRACK_ON: &str = "relative inline-flex h-6 w-11 shrink-0 items-center rounded-full bg-brand-600 transition-colors duration-quick motion-reduce:transition-none";
const TRACK_OFF: &str = "relative inline-flex h-6 w-11 shrink-0 items-center rounded-full bg-surface-strong transition-colors duration-quick motion-reduce:transition-none";

const KNOB_ON: &str = "inline-block h-5 w-5 translate-x-5 rounded-full bg-surface-base shadow-raised transition-transform duration-quick motion-reduce:transition-none";
const KNOB_OFF: &str = "inline-block h-5 w-5 translate-x-0.5 rounded-full bg-surface-base shadow-raised transition-transform duration-quick motion-reduce:transition-none";

const LABEL: &str = "text-sm text-ink-body select-none";

/// A boolean on/off switch — the first `role="switch"` control in the
/// dashboard, and the counterpart to `Input` for non-text values.
///
/// The whole control is one `<button role="switch">` that wraps both the
/// track and the label text. A sibling `<label for=…>` is deliberately not
/// used: clicking a `<label>` does not activate a `<button>`, so putting the
/// text inside the control is what makes the label clickable *and* supplies
/// the switch's accessible name in one move.
///
/// `type="button"` is mandatory — this primitive lives inside `<form
/// on:submit>`, where the default `type="submit"` would turn a flip into a
/// form submission.
#[component]
pub fn Toggle(
    /// Two-way bound state, mirroring `Input`'s `RwSignal<String>` shape.
    checked: RwSignal<bool>,
    /// Visible text — also the switch's accessible name.
    #[prop(into)]
    label: &'static str,
    /// Optional DOM id, for callers that need to address the switch.
    #[prop(optional, into)]
    id: Option<&'static str>,
    #[prop(optional, into)] disabled: Signal<bool>,
    /// Fired with the new value after the flip, for callers that must
    /// mirror the state into another signal.
    #[prop(optional, into)]
    on_change: Option<Callback<bool>>,
) -> impl IntoView {
    let id = id.unwrap_or("");
    view! {
        <button
            id=id
            type="button"
            role="switch"
            class=move || if disabled.get() { ROOT_DISABLED } else { ROOT_ENABLED }
            aria-checked=move || if checked.get() { "true" } else { "false" }
            disabled=move || disabled.get()
            on:click=move |_| {
                if disabled.get() {
                    return;
                }
                let next = !checked.get_untracked();
                checked.set(next);
                if let Some(cb) = on_change {
                    cb.run(next);
                }
            }
        >
            <span aria-hidden="true" class=move || if checked.get() { TRACK_ON } else { TRACK_OFF }>
                <span class=move || if checked.get() { KNOB_ON } else { KNOB_OFF } />
            </span>
            <span class=LABEL>{label}</span>
        </button>
    }
}
