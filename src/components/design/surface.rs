use leptos::prelude::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum SurfaceElevation {
    #[default]
    Raised,
    Overlay,
}

fn surface_class(elevation: SurfaceElevation, padded: bool) -> &'static str {
    // Full matrix enumerated for Tailwind's JIT scanner — format! would hide
    // the combinations from tree-shaking.
    match (elevation, padded) {
        (SurfaceElevation::Raised, false) => "bg-surface-base rounded-card shadow-raised",
        (SurfaceElevation::Raised, true) => "bg-surface-base rounded-card shadow-raised p-6",
        (SurfaceElevation::Overlay, false) => "bg-surface-base rounded-card shadow-overlay",
        (SurfaceElevation::Overlay, true) => "bg-surface-base rounded-card shadow-overlay p-8",
    }
}

/// A card / panel. Wraps content in the design system's elevated surface,
/// optionally with the default `p-6` or `p-8` inset padding.
#[component]
pub fn Surface(
    #[prop(optional)] elevation: SurfaceElevation,
    /// Include the default inner padding (`p-6` / `p-8`) — most surfaces
    /// want it; a few (e.g. stats cards with custom internal layout) don't.
    #[prop(optional)]
    padded: bool,
    children: Children,
) -> impl IntoView {
    let cls = surface_class(elevation, padded);
    view! { <div class=cls>{children()}</div> }
}
