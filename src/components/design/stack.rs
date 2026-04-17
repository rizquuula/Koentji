use leptos::prelude::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum StackGap {
    Tight, // space-y-2
    #[default]
    Normal, // space-y-4
    Loose, // space-y-6
}

fn gap_class(gap: StackGap) -> &'static str {
    match gap {
        StackGap::Tight => "space-y-2",
        StackGap::Normal => "space-y-4",
        StackGap::Loose => "space-y-6",
    }
}

/// Vertical layout primitive. Enforces one of three gap sizes so spacing
/// in the dashboard is quantised rather than ad-hoc.
#[component]
pub fn Stack(#[prop(optional)] gap: StackGap, children: Children) -> impl IntoView {
    let cls = gap_class(gap);
    view! { <div class=cls>{children()}</div> }
}
