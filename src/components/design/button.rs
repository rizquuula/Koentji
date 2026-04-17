use leptos::prelude::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Danger,
    Ghost,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonType {
    #[default]
    Button,
    Submit,
}

impl ButtonType {
    fn as_attr(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Submit => "submit",
        }
    }
}

fn variant_class(variant: ButtonVariant, full_width: bool) -> &'static str {
    // The cartesian (variant × full_width) is enumerated so Tailwind's JIT
    // scanner finds every concrete class string — `format!` would hide them.
    match (variant, full_width) {
        (ButtonVariant::Primary, false) => "px-4 py-2 bg-brand-600 text-ink-inverse rounded-control hover:bg-brand-700 transition-colors duration-quick disabled:opacity-50 disabled:cursor-not-allowed font-medium",
        (ButtonVariant::Primary, true)  => "w-full px-4 py-2 bg-brand-600 text-ink-inverse rounded-control hover:bg-brand-700 focus:ring-2 focus:ring-offset-2 focus:ring-brand-500 transition-colors duration-quick disabled:opacity-50 disabled:cursor-not-allowed font-medium",
        (ButtonVariant::Secondary, false) => "px-4 py-2 text-ink-body bg-surface-muted hover:bg-surface-border rounded-control transition-colors duration-quick disabled:opacity-50",
        (ButtonVariant::Secondary, true)  => "w-full px-4 py-2 text-ink-body bg-surface-muted hover:bg-surface-border rounded-control transition-colors duration-quick disabled:opacity-50",
        (ButtonVariant::Danger, false) => "px-4 py-2 bg-feedback-danger text-ink-inverse hover:bg-red-700 rounded-control transition-colors duration-quick disabled:opacity-50 font-medium",
        (ButtonVariant::Danger, true)  => "w-full px-4 py-2 bg-feedback-danger text-ink-inverse hover:bg-red-700 rounded-control transition-colors duration-quick disabled:opacity-50 font-medium",
        (ButtonVariant::Ghost, false) => "px-4 py-2 text-brand-600 hover:text-brand-800 hover:bg-surface-muted rounded-control transition-colors duration-quick disabled:opacity-50",
        (ButtonVariant::Ghost, true)  => "w-full px-4 py-2 text-brand-600 hover:text-brand-800 hover:bg-surface-muted rounded-control transition-colors duration-quick disabled:opacity-50",
    }
}

/// A primary action control.
///
/// Pass `on_click` as a `Callback<MouseEvent>` for non-submit buttons. Submit
/// buttons leave the click handler off the prop and let the wrapping `<form
/// on:submit>` own the work — Leptos is wired that way in every form in this
/// codebase.
#[component]
pub fn Button(
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] button_type: ButtonType,
    /// Expand to fill the container's width — the login button's shape.
    #[prop(optional)]
    full_width: bool,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] on_click: Option<Callback<leptos::ev::MouseEvent>>,
    children: Children,
) -> impl IntoView {
    let cls = variant_class(variant, full_width);
    let ty = button_type.as_attr();
    view! {
        <button
            type=ty
            class=cls
            disabled=move || disabled.get()
            on:click=move |ev| {
                if let Some(cb) = on_click {
                    cb.run(ev);
                }
            }
        >
            {children()}
        </button>
    }
}
