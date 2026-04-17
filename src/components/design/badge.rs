use leptos::prelude::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum BadgeTone {
    #[default]
    Neutral,
    Brand,
    Success,
    Warning,
    Danger,
}

fn badge_class(tone: BadgeTone) -> &'static str {
    // Pair of bg + ink per tone — literal strings so Tailwind keeps them.
    match tone {
        BadgeTone::Neutral => {
            "px-2 py-1 rounded-full text-xs font-medium bg-surface-muted text-ink-subdued"
        }
        BadgeTone::Brand => "px-2 py-1 rounded-full text-xs font-medium bg-brand-50 text-brand-800",
        BadgeTone::Success => {
            "px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-feedback-success-ink"
        }
        BadgeTone::Warning => {
            "px-2 py-1 rounded-full text-xs font-medium bg-yellow-100 text-feedback-warning-ink"
        }
        BadgeTone::Danger => {
            "px-2 py-1 rounded-full text-xs font-medium bg-red-100 text-feedback-danger-ink"
        }
    }
}

/// Small status pill — subscription tier, auth decision, row state.
#[component]
pub fn Badge(#[prop(optional)] tone: BadgeTone, children: Children) -> impl IntoView {
    let cls = badge_class(tone);
    view! { <span class=cls>{children()}</span> }
}
