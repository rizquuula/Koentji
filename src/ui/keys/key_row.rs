use crate::models::AuthenticationKey;
use leptos::prelude::*;

// Heroicons "copy" and "check" outline path data. The copy buttons swap to the
// check for 3s after a successful copy, then revert to the copy icon.
const COPY_ICON_PATH: &str = "M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z";
const CHECK_ICON_PATH: &str = "M5 13l4 4L19 7";

/// Tidy a rate-limit figure to at most 2 decimals, dropping trailing zeros so
/// whole numbers read cleanly (`6000.0` → "6000") and float noise is hidden
/// (`5377.899999999991` → "5377.9"). The ledger keeps the exact `f64`; only the
/// on-screen readout is rounded.
fn fmt_amount(value: f64) -> String {
    let s = format!("{value:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[component]
pub fn KeyRow(
    key: AuthenticationKey,
    #[prop(into)] on_edit: Callback<AuthenticationKey>,
    #[prop(into)] on_delete: Callback<i32>,
    #[prop(into)] on_reset: Callback<i32>,
) -> impl IntoView {
    let key_id = key.id;
    // Render the "created" timestamp in the viewer's local zone, not UTC.
    let offset = crate::ui::tz::use_tz_offset();
    let created_at = key.created_at;
    let masked = key.masked_key();
    let status = key.status().to_string();
    let rate_pct = key.rate_limit_percentage();
    let key_clone = key.clone();
    // The full key is already shipped to the client (`list_keys` serialises it
    // unredacted; `masked_key()` computes the mask here), so reveal and copy are
    // purely local — no server round-trip buys any secrecy.
    let full_key = key.key.clone();
    let key_revealed = RwSignal::new(false);

    // The device id is likewise non-secret and already on the client, so its
    // reveal is a local visual toggle too.
    let device_id = key.device_id.clone();
    let masked_device = key.masked_device_id();
    let device_revealed = RwSignal::new(false);

    // Copy buttons flash a checkmark for 3s after a successful copy.
    let key_copied = RwSignal::new(false);
    let device_copied = RwSignal::new(false);
    let key_copy_icon = move || {
        if key_copied.get() {
            CHECK_ICON_PATH
        } else {
            COPY_ICON_PATH
        }
    };
    let device_copy_icon = move || {
        if device_copied.get() {
            CHECK_ICON_PATH
        } else {
            COPY_ICON_PATH
        }
    };

    let status_badge_class = match status.as_str() {
        "active" => "bg-green-100 text-green-800",
        "expired" => "bg-yellow-100 text-yellow-800",
        "deleted" => "bg-red-100 text-red-800",
        _ => "bg-gray-100 text-gray-800",
    };

    let rate_bar_class = if rate_pct <= 25.0 {
        "bg-green-500"
    } else if rate_pct <= 50.0 {
        "bg-yellow-500"
    } else if rate_pct <= 75.0 {
        "bg-orange-500"
    } else {
        "bg-red-500"
    };

    // Reveal just flips the on-screen masking; the full key is already local.
    let handle_reveal = move |_| key_revealed.update(|shown| *shown = !*shown);

    // Flash the checkmark for 3s after a successful copy.
    let flash_copied = move || {
        key_copied.set(true);
        set_timeout(
            move || key_copied.set(false),
            std::time::Duration::from_secs(3),
        );
    };

    let handle_copy = {
        let full_key = full_key.clone();
        move |_| {
            // Fully synchronous so the browser's transient user-activation from
            // the click is still live when the clipboard write fires (an
            // intervening `await` would expire it and silently drop the write).
            // Copy never unmasks the on-screen key.
            copy_to_clipboard(&full_key);
            flash_copied();
        }
    };

    let toggle_device = move |_| device_revealed.update(|shown| *shown = !*shown);
    let copy_device = {
        let device_id = device_id.clone();
        move |_| {
            copy_to_clipboard(&device_id);
            device_copied.set(true);
            set_timeout(
                move || device_copied.set(false),
                std::time::Duration::from_secs(3),
            );
        }
    };
    let device_full = device_id.clone();

    let edit_key = key_clone.clone();

    view! {
        <tr class="hover:bg-gray-50 border-b">
            <td class="px-4 py-3 text-sm font-mono">
                <div class="flex items-center space-x-2">
                    <span class="text-gray-600">
                        {move || {
                            if key_revealed.get() {
                                full_key.clone()
                            } else {
                                masked.clone()
                            }
                        }}
                    </span>
                    <button
                        type="button"
                        class="text-blue-500 hover:text-blue-700 text-xs"
                        on:click=handle_reveal
                        title="Reveal"
                        aria-label="Reveal full key"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/>
                        </svg>
                    </button>
                    <button
                        type="button"
                        class="text-gray-400 hover:text-gray-600 text-xs"
                        on:click=handle_copy
                        title="Copy"
                        aria-label="Copy key to clipboard"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d=key_copy_icon/>
                        </svg>
                    </button>
                </div>
            </td>
            <td class="px-4 py-3 text-sm">
                <div class="flex items-center space-x-2">
                    <span class="font-mono text-gray-600" aria-hidden="true">
                        {move || {
                            if device_revealed.get() {
                                device_full.clone()
                            } else {
                                masked_device.clone()
                            }
                        }}
                    </span>
                    // Full id kept for assistive tech (the device id is not a
                    // secret), so screen readers and role/text locators still
                    // resolve the row by its real device id while the eye only
                    // toggles the on-screen masking.
                    <span class="sr-only">{device_id.clone()}</span>
                    <button
                        type="button"
                        class="text-blue-500 hover:text-blue-700 text-xs"
                        on:click=toggle_device
                        title="Reveal"
                        aria-label="Reveal full device ID"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/>
                        </svg>
                    </button>
                    <button
                        type="button"
                        class="text-gray-400 hover:text-gray-600 text-xs"
                        on:click=copy_device
                        title="Copy"
                        aria-label="Copy device ID to clipboard"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d=device_copy_icon/>
                        </svg>
                    </button>
                </div>
            </td>
            <td class="px-4 py-3 text-sm">
                <div>
                    <span class="text-gray-900">{key.username.clone().unwrap_or_else(|| "-".to_string())}</span>
                    {key.email.as_ref().map(|e| view! {
                        <span class="block text-xs text-gray-500">{e.clone()}</span>
                    })}
                </div>
            </td>
            <td class="px-4 py-3 text-sm">
                <span class="px-2 py-1 bg-gray-100 text-gray-700 rounded text-xs font-medium">
                    {key.subscription.clone().unwrap_or_else(|| "None".to_string())}
                </span>
            </td>
            <td class="px-4 py-3 text-sm">
                <div class="flex flex-col space-y-1">
                    <div class="w-24 bg-gray-200 rounded-full h-2">
                        <div
                            class=format!("h-2 rounded-full {}", rate_bar_class)
                            style=format!("width: {}%", rate_pct.min(100.0))
                        />
                    </div>
                    <span class="text-[10px] text-gray-500">
                        {format!("{}/{}", fmt_amount(key.rate_limit_remaining), fmt_amount(key.rate_limit_daily))}
                    </span>
                </div>
            </td>
            <td class="px-4 py-3">
                <span class=format!("px-2 py-1 rounded-full text-xs font-medium {}", status_badge_class)>
                    {status.clone()}
                </span>
            </td>
            <td class="px-4 py-3 text-sm text-gray-500">
                {move || crate::ui::tz::to_local(created_at, offset.get()).format("%Y-%m-%d %H:%M").to_string()}
            </td>
            <td class="px-4 py-3">
                <div class="flex items-center space-x-2">
                    <button
                        type="button"
                        class="text-yellow-600 hover:text-yellow-800 text-xs"
                        title="Reset Rate Limit"
                        aria-label="Reset rate limit"
                        on:click=move |_| on_reset.run(key_id)
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                        </svg>
                    </button>
                    <button
                        type="button"
                        class="text-blue-600 hover:text-blue-800 text-xs"
                        title="Edit"
                        aria-label="Edit key"
                        on:click={
                            let ek = edit_key.clone();
                            move |_| on_edit.run(ek.clone())
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/>
                        </svg>
                    </button>
                    <button
                        type="button"
                        class="text-red-600 hover:text-red-800 text-xs"
                        title="Revoke"
                        aria-label="Revoke key"
                        on:click=move |_| on_delete.run(key_id)
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                        </svg>
                    </button>
                </div>
            </td>
        </tr>
    }
}

/// Copy `text` to the clipboard in both secure and insecure contexts.
///
/// `navigator.clipboard` only exists in a *secure context* (HTTPS or
/// localhost). Served over plain HTTP from an IP it is `undefined`, and the
/// non-nullable web-sys `.clipboard()` binding would call
/// `undefined.writeText(...)` — the "can't access property writeText"
/// TypeError. We feature-detect it first and, when it's missing, fall back to
/// the legacy `execCommand('copy')` over a throwaway off-screen `<textarea>`,
/// which is still the only copy path that works on plain HTTP. The text is
/// passed as a JS string value (never as source), so no escaping is needed.
fn copy_to_clipboard(text: &str) {
    let Some(win) = web_sys::window() else {
        return;
    };
    let navigator = win.navigator();

    let nav_val: wasm_bindgen::JsValue = navigator.clone().into();
    let has_clipboard =
        js_sys::Reflect::get(&nav_val, &wasm_bindgen::JsValue::from_str("clipboard"))
            .is_ok_and(|v| !v.is_undefined() && !v.is_null());

    if has_clipboard {
        // Secure context: fire-and-forget the returned Promise.
        let _ = navigator.clipboard().write_text(text);
    } else {
        let _ = copy_via_textarea(&win, text);
    }
}

/// Insecure-context clipboard fallback: select a hidden `<textarea>` and run
/// the deprecated `execCommand('copy')`. The element is appended and removed
/// within one synchronous handler, so it never paints or steals focus/scroll.
fn copy_via_textarea(win: &web_sys::Window, text: &str) -> Option<()> {
    use wasm_bindgen::JsCast;

    let document = win.document()?;
    // `execCommand` lives on HTMLDocument; the page is HTML so this casts.
    let html_document = document.clone().dyn_into::<web_sys::HtmlDocument>().ok()?;

    let textarea = document
        .create_element("textarea")
        .ok()?
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .ok()?;
    textarea.set_value(text);
    textarea
        .set_attribute("style", "position:fixed;top:0;left:-9999px;opacity:0;")
        .ok()?;
    let _ = textarea.set_attribute("readonly", "");

    let body = document.body()?;
    body.append_child(&textarea).ok()?;
    textarea.select();
    let _ = html_document.exec_command("copy");
    let _ = body.remove_child(&textarea);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::fmt_amount;

    #[test]
    fn fmt_amount_hides_float_noise() {
        // The reported case: a fractional ledger leaving float drift.
        assert_eq!(fmt_amount(5377.899999999991), "5377.9");
    }

    #[test]
    fn fmt_amount_drops_trailing_zeros() {
        assert_eq!(fmt_amount(6000.0), "6000");
        assert_eq!(fmt_amount(12.5), "12.5");
        assert_eq!(fmt_amount(0.0), "0");
    }

    #[test]
    fn fmt_amount_rounds_to_two_decimals() {
        assert_eq!(fmt_amount(1234.567), "1234.57");
    }
}
