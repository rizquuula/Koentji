use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

/// Collect focusable descendants of `root` in document order.
fn focusable_within(root: &web_sys::Element) -> Vec<HtmlElement> {
    let selector = "a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex=\"-1\"])";
    let Ok(list) = root.query_selector_all(selector) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(list.length() as usize);
    for i in 0..list.length() {
        if let Some(node) = list.item(i) {
            if let Ok(el) = node.dyn_into::<HtmlElement>() {
                out.push(el);
            }
        }
    }
    out
}

/// Keep Tab / Shift+Tab inside `root`. Callers filter on `key == "Tab"`
/// before invoking so the non-focus keys return immediately.
fn trap_tab(event: &ev::KeyboardEvent, root: &web_sys::Element) {
    let focusables = focusable_within(root);
    if focusables.is_empty() {
        event.prevent_default();
        return;
    }
    let first = focusables.first().unwrap();
    let last = focusables.last().unwrap();

    let active_el = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element());

    if event.shift_key() {
        let at_start = active_el
            .as_ref()
            .map(|a| a == first.as_ref() || !root.contains(Some(a)))
            .unwrap_or(true);
        if at_start {
            event.prevent_default();
            let _ = last.focus();
        }
    } else {
        let at_end = active_el
            .as_ref()
            .map(|a| a == last.as_ref() || !root.contains(Some(a)))
            .unwrap_or(true);
        if at_end {
            event.prevent_default();
            let _ = first.focus();
        }
    }
}

/// Snapshot the currently-focused element so we can restore focus to it
/// when the dialog closes. Works around the fact that a dialog that
/// takes focus and never returns it leaves keyboard users lost.
fn snapshot_focus() -> Option<HtmlElement> {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
}

/// Focus the first focusable descendant of `dialog`, or the dialog itself
/// if it has none. Runs once per open; no-op if the dialog vanished first.
fn focus_first(dialog: &web_sys::Element) {
    let focusables = focusable_within(dialog);
    if let Some(first) = focusables.first() {
        let _ = first.focus();
    } else if let Ok(el) = dialog.clone().dyn_into::<HtmlElement>() {
        let _ = el.focus();
    }
}

#[component]
pub fn ConfirmModal(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] on_confirm: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
    #[prop(into)] title: String,
    #[prop(into)] message: String,
    #[prop(into, default = "Confirm".into())] confirm_label: String,
    #[prop(default = false)] danger: bool,
) -> impl IntoView {
    let btn_class = if danger {
        "px-4 py-2 text-sm text-white bg-feedback-danger hover:bg-feedback-danger-ink rounded-control transition-colors duration-quick"
    } else {
        "px-4 py-2 text-sm text-white bg-brand-600 hover:bg-brand-700 rounded-control transition-colors duration-quick"
    };

    let dialog_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let previously_focused = StoredValue::new(None::<HtmlElement>);
    let was_open = StoredValue::new(false);

    Effect::new(move |_| {
        let is_open = show.get();
        let prev = was_open.get_value();
        if is_open && !prev {
            previously_focused.set_value(snapshot_focus());
            if let Some(el) = dialog_ref.get() {
                focus_first(&el);
            }
        } else if !is_open && prev {
            previously_focused.update_value(|saved| {
                if let Some(el) = saved.take() {
                    let _ = el.focus();
                }
            });
        }
        was_open.set_value(is_open);
    });

    let handle_keydown = move |event: ev::KeyboardEvent| match event.key().as_str() {
        "Escape" => {
            event.prevent_default();
            on_cancel.run(());
        }
        "Tab" => {
            if let Some(el) = dialog_ref.get() {
                trap_tab(&event, &el);
            }
        }
        _ => {}
    };

    let title_for_aria = title.clone();
    let message_for_aria = message.clone();

    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 z-40 bg-black/50" aria-hidden="true"></div>
        </Show>
        <div
            class="fixed inset-0 z-50 flex items-center justify-center"
            style:display=move || if show.get() { "flex" } else { "none" }
        >
            <div
                class="fixed inset-0"
                on:click=move |_| on_cancel.run(())
                aria-hidden="true"
            />
            <div
                node_ref=dialog_ref
                role="alertdialog"
                aria-modal="true"
                aria-label=title_for_aria
                aria-description=message_for_aria
                tabindex="-1"
                on:keydown=handle_keydown
                class="relative bg-surface-base rounded-card shadow-overlay w-full max-w-md mx-4 z-10"
            >
                <div class="px-6 pt-6 pb-4">
                    <div class="flex items-start space-x-3">
                        {if danger {
                            view! {
                                <div class="flex-shrink-0 w-10 h-10 rounded-full bg-red-100 flex items-center justify-center">
                                    <svg class="w-5 h-5 text-feedback-danger" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                                    </svg>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="flex-shrink-0 w-10 h-10 rounded-full bg-yellow-100 flex items-center justify-center">
                                    <svg class="w-5 h-5 text-feedback-warning" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                    </svg>
                                </div>
                            }.into_any()
                        }}
                        <div>
                            <h3 class="text-base font-semibold text-ink-heading">{title}</h3>
                            <p class="mt-1 text-sm text-ink-body">{message}</p>
                        </div>
                    </div>
                </div>
                <div class="flex justify-end space-x-3 px-6 py-4 border-t border-surface-border bg-surface-subtle rounded-b-card">
                    <button
                        type="button"
                        class="px-4 py-2 text-sm text-ink-body bg-surface-base border border-surface-strong hover:bg-surface-subtle rounded-control transition-colors duration-quick"
                        on:click=move |_| on_cancel.run(())
                    >
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        class=btn_class
                        on:click=move |_| on_confirm.run(())
                    >
                        {confirm_label}
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn Modal(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    let content = children();

    let dialog_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let previously_focused = StoredValue::new(None::<HtmlElement>);
    let was_open = StoredValue::new(false);

    Effect::new(move |_| {
        let is_open = show.get();
        let prev = was_open.get_value();
        if is_open && !prev {
            previously_focused.set_value(snapshot_focus());
            if let Some(el) = dialog_ref.get() {
                focus_first(&el);
            }
        } else if !is_open && prev {
            previously_focused.update_value(|saved| {
                if let Some(el) = saved.take() {
                    let _ = el.focus();
                }
            });
        }
        was_open.set_value(is_open);
    });

    let handle_keydown = move |event: ev::KeyboardEvent| match event.key().as_str() {
        "Escape" => {
            event.prevent_default();
            on_close.run(());
        }
        "Tab" => {
            if let Some(el) = dialog_ref.get() {
                trap_tab(&event, &el);
            }
        }
        _ => {}
    };

    let title_for_aria = title.clone();

    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 z-40 bg-black/50" aria-hidden="true"></div>
        </Show>
        <div
            class="fixed inset-0 z-50 flex items-center justify-center"
            style:display=move || if show.get() { "flex" } else { "none" }
        >
            <div
                class="fixed inset-0"
                on:click=move |_| on_close.run(())
                aria-hidden="true"
            />
            <div
                node_ref=dialog_ref
                role="dialog"
                aria-modal="true"
                aria-label=title_for_aria
                tabindex="-1"
                on:keydown=handle_keydown
                class="relative bg-surface-base rounded-card shadow-overlay w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto z-10"
            >
                <div class="flex items-center justify-between px-6 py-4 border-b border-surface-border">
                    <h3 class="text-lg font-semibold text-ink-heading">{title}</h3>
                    <button
                        type="button"
                        class="text-ink-disabled hover:text-ink-body"
                        on:click=move |_| on_close.run(())
                        aria-label="Close dialog"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
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
