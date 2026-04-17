use leptos::prelude::*;

/// Dashboard page header: title + optional subtitle.
///
/// Kept deliberately simple — primary actions live beside it via a flex
/// wrapper at the call site, so the shape is `<header><PageHeader …
/// /><Button … /></header>` rather than burying the action in a slot.
/// That keeps `PageHeader` reusable on pages without a single primary CTA.
#[component]
pub fn PageHeader(
    title: &'static str,
    #[prop(optional)] subtitle: Option<&'static str>,
) -> impl IntoView {
    view! {
        <div>
            <h1 class="text-2xl font-bold text-ink-heading">{title}</h1>
            {subtitle.map(|s| view! { <p class="text-sm text-ink-muted mt-1">{s}</p> })}
        </div>
    }
}
