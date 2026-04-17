use crate::models::AuthenticationKey;
use leptos::prelude::*;

#[component]
pub fn KeyRow(
    key: AuthenticationKey,
    #[prop(into)] on_edit: Callback<AuthenticationKey>,
    #[prop(into)] on_delete: Callback<i32>,
    #[prop(into)] on_reset: Callback<i32>,
) -> impl IntoView {
    let key_id = key.id;
    let masked = key.masked_key();
    let status = key.status().to_string();
    let rate_pct = key.rate_limit_percentage();
    let key_clone = key.clone();
    let revealed_key = RwSignal::new(None::<String>);

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

    let handle_reveal = move |_| {
        let key_id = key_id;
        leptos::task::spawn_local(async move {
            if let Ok(full_key) = crate::server::key_service::reveal_key(key_id).await {
                revealed_key.set(Some(full_key));
            }
        });
    };

    let handle_copy = move |_| {
        // navigator.clipboard.writeText(k) is fire-and-forget — the
        // returned Promise is discarded. The previous implementation
        // used `js_sys::eval` with string concatenation, which broke
        // on any key containing a backtick, newline, or apostrophe and
        // was a log-injection shaped XSS risk. Typed API means the
        // key is passed as a JS string value, not source code, so no
        // escaping is needed.
        if let Some(k) = revealed_key.get() {
            if let Some(win) = web_sys::window() {
                let _ = win.navigator().clipboard().write_text(&k);
            }
        }
    };

    let edit_key = key_clone.clone();

    view! {
        <tr class="hover:bg-gray-50 border-b">
            <td class="px-4 py-3 text-sm font-mono">
                <div class="flex items-center space-x-2">
                    <span class="text-gray-600">
                        {move || {
                            if let Some(ref full) = revealed_key.get() {
                                full.clone()
                            } else {
                                masked.clone()
                            }
                        }}
                    </span>
                    <button
                        class="text-blue-500 hover:text-blue-700 text-xs"
                        on:click=handle_reveal
                        title="Reveal"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/>
                        </svg>
                    </button>
                    <button
                        class="text-gray-400 hover:text-gray-600 text-xs"
                        on:click=handle_copy
                        title="Copy"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/>
                        </svg>
                    </button>
                </div>
            </td>
            <td class="px-4 py-3 text-sm">{key.device_id.clone()}</td>
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
                <div class="flex items-center space-x-2">
                    <div class="w-24 bg-gray-200 rounded-full h-2">
                        <div
                            class=format!("h-2 rounded-full {}", rate_bar_class)
                            style=format!("width: {}%", rate_pct.min(100.0))
                        />
                    </div>
                    <span class="text-xs text-gray-500">
                        {format!("{}/{}", key.rate_limit_remaining, key.rate_limit_daily)}
                    </span>
                </div>
            </td>
            <td class="px-4 py-3">
                <span class=format!("px-2 py-1 rounded-full text-xs font-medium {}", status_badge_class)>
                    {status.clone()}
                </span>
            </td>
            <td class="px-4 py-3 text-sm text-gray-500">
                {key.created_at.format("%Y-%m-%d %H:%M").to_string()}
            </td>
            <td class="px-4 py-3">
                <div class="flex items-center space-x-2">
                    <button
                        class="text-yellow-600 hover:text-yellow-800 text-xs"
                        title="Reset Rate Limit"
                        on:click=move |_| on_reset.run(key_id)
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                        </svg>
                    </button>
                    <button
                        class="text-blue-600 hover:text-blue-800 text-xs"
                        title="Edit"
                        on:click={
                            let ek = edit_key.clone();
                            move |_| on_edit.run(ek.clone())
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/>
                        </svg>
                    </button>
                    <button
                        class="text-red-600 hover:text-red-800 text-xs"
                        title="Revoke"
                        on:click=move |_| on_delete.run(key_id)
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                        </svg>
                    </button>
                </div>
            </td>
        </tr>
    }
}
