use crate::server::analytics_service::{KeyTrafficRow, QuotaPressureRow};
use crate::ui::tz::{to_local, use_tz_offset};
use leptos::prelude::*;

/// Deny rate as a percentage of total requests, rounded to one decimal.
/// Zero requests is 0% (not NaN) — a key with no traffic hasn't denied
/// anything. Pure so the arithmetic is tested once, here.
pub fn deny_rate_pct(requests: u64, denied: u64) -> f64 {
    if requests == 0 {
        return 0.0;
    }
    let raw = (denied as f64 / requests as f64) * 100.0;
    (raw * 10.0).round() / 10.0
}

/// Truncate a long API key for display: first 8 chars + "…" + last 4. Short
/// keys (≤ 13 chars, so the ellipsised form wouldn't actually be shorter)
/// pass through unchanged. The full key still rides in the cell's `title`.
pub fn truncate_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 13 {
        return key.to_string();
    }
    let head: String = chars.iter().take(8).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{head}…{tail}")
}

/// Percent of quota *remaining*, clamped to 0–100. `None` when the limit is
/// missing or non-positive (can't compute a ratio) — the caller renders the
/// raw remaining and skips the bar. A limit that shrank below the recorded
/// remaining must not read as >100%, so the clamp caps it at 100.
pub fn percent_remaining(remaining: f64, limit: Option<f64>) -> Option<f64> {
    match limit {
        Some(l) if l > 0.0 => Some(((remaining / l) * 100.0).clamp(0.0, 100.0)),
        _ => None,
    }
}

/// Tailwind bar color by percent *remaining*: green when there's headroom,
/// red when nearly exhausted. (Inverse of `keys/key_row.rs`, which colors by
/// percent *used* — here low remaining is the alarming end.)
fn quota_bar_class(pct_remaining: f64) -> &'static str {
    if pct_remaining > 50.0 {
        "bg-green-500"
    } else if pct_remaining >= 20.0 {
        "bg-amber-500"
    } else {
        "bg-red-500"
    }
}

/// Trim trailing zeros off a float for compact display (e.g. `12.0` → "12",
/// `12.50` → "12.5"). The ledger stores exact `f64`; the table wants tidy.
fn trim_decimals(value: f64) -> String {
    let s = format!("{value:.2}");
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    trimmed.to_string()
}

/// Render a unix-seconds timestamp as "DD MMM HH:MM" in the viewer's local
/// timezone (`offset_minutes` east of UTC) — same shape as the charts'
/// wider-window labels, so the page reads consistently.
fn format_last_seen(unix_secs: i64, offset_minutes: i32) -> String {
    use chrono::{DateTime, Utc};
    let dt: DateTime<Utc> = DateTime::from_timestamp(unix_secs, 0).unwrap_or_default();
    to_local(dt, offset_minutes)
        .format("%d %b %H:%M")
        .to_string()
}

/// Heroicons "eye-off" outline path data. The reveal button swaps to this
/// while the full key is shown; the default eye is the two-path icon inlined
/// below (same family as the keys page).
const EYE_OFF_ICON_PATH: &str = "M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21";

/// A monospace API-key cell: the truncated key plus an eye button that
/// reveals/hides the full value. The full key already rides in the analytics
/// rows on the client, so reveal is a local visual toggle — no server fetch,
/// mirroring the device-id reveal on the keys page. The full key stays in the
/// wrapper's `title` so hover surfaces it in either state.
#[component]
fn RevealableKey(full: String) -> impl IntoView {
    let truncated = truncate_key(&full);
    let shown = RwSignal::new(false);
    let full_text = full.clone();
    view! {
        <div class="flex items-center space-x-2" title=full.clone()>
            <span>
                {move || if shown.get() { full_text.clone() } else { truncated.clone() }}
            </span>
            <button
                type="button"
                class="text-blue-500 hover:text-blue-700 text-xs shrink-0"
                on:click=move |_| shown.update(|s| *s = !*s)
                title=move || if shown.get() { "Hide key" } else { "Reveal key" }
                aria-label=move || if shown.get() { "Hide full key" } else { "Reveal full key" }
            >
                {move || if shown.get() {
                    view! {
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d=EYE_OFF_ICON_PATH/>
                        </svg>
                    }.into_any()
                } else {
                    view! {
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/>
                        </svg>
                    }.into_any()
                }}
            </button>
        </div>
    }
}

#[component]
pub fn BusiestKeysTable(rows: Vec<KeyTrafficRow>) -> impl IntoView {
    let is_empty = rows.is_empty();
    // Local-timezone offset, read reactively so each row's "Last seen" re-renders
    // from UTC to the viewer's zone once the browser offset lands post-hydration.
    let tz = use_tz_offset();
    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <div class="px-6 py-4 border-b">
                <h3 class="text-sm font-medium text-gray-500">"Busiest keys"</h3>
            </div>
            <div class="overflow-x-auto">
                <table class="w-full">
                    <thead class="bg-gray-50 border-b">
                        <tr>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Key"</th>
                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">"Requests"</th>
                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">"Deny rate"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Last seen"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || rows.clone()
                            key=|r| r.auth_key.clone()
                            let:row
                        >
                            {
                                let full_key = row.auth_key.clone();
                                let deny = deny_rate_pct(row.requests, row.denied);
                                let last_seen_unix = row.last_seen_unix;
                                view! {
                                    <tr class="border-b last:border-0">
                                        <td class="px-4 py-3 text-sm font-mono text-gray-900">
                                            <RevealableKey full=full_key/>
                                        </td>
                                        <td class="px-4 py-3 text-sm text-right text-gray-700">
                                            {row.requests.to_string()}
                                        </td>
                                        <td class="px-4 py-3 text-sm text-right text-gray-700">
                                            {format!("{deny:.1}%")}
                                        </td>
                                        <td class="px-4 py-3 text-sm text-gray-500">
                                            {move || format_last_seen(last_seen_unix, tz.get())}
                                        </td>
                                    </tr>
                                }
                            }
                        </For>
                        <Show when=move || is_empty>
                            <tr>
                                <td colspan="4" class="px-4 py-8 text-center text-gray-500">
                                    "No traffic in this window"
                                </td>
                            </tr>
                        </Show>
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[component]
pub fn QuotaPressureTable(rows: Vec<QuotaPressureRow>) -> impl IntoView {
    let is_empty = rows.is_empty();
    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <div class="px-6 py-4 border-b">
                <h3 class="text-sm font-medium text-gray-500">"Quota pressure"</h3>
            </div>
            <div class="overflow-x-auto">
                <table class="w-full">
                    <thead class="bg-gray-50 border-b">
                        <tr>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"Key"</th>
                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">"Remaining"</th>
                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">"Limit"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">"% remaining"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || rows.clone()
                            key=|r| r.auth_key.clone()
                            let:row
                        >
                            {
                                let full_key = row.auth_key.clone();
                                let remaining = trim_decimals(row.remaining);
                                // Deleted-from-Postgres keys show "—" and no bar.
                                let limit_label = row
                                    .limit
                                    .map(trim_decimals)
                                    .unwrap_or_else(|| "—".to_string());
                                let pct = percent_remaining(row.remaining, row.limit);
                                view! {
                                    <tr class="border-b last:border-0">
                                        <td class="px-4 py-3 text-sm font-mono text-gray-900">
                                            <RevealableKey full=full_key/>
                                        </td>
                                        <td class="px-4 py-3 text-sm text-right text-gray-700">
                                            {remaining}
                                        </td>
                                        <td class="px-4 py-3 text-sm text-right text-gray-700">
                                            {limit_label}
                                        </td>
                                        <td class="px-4 py-3 text-sm">
                                            {match pct {
                                                Some(p) => {
                                                    let bar = quota_bar_class(p);
                                                    view! {
                                                        <div class="flex items-center space-x-2">
                                                            <div class="w-24 bg-gray-200 rounded-full h-2">
                                                                <div
                                                                    class=format!("h-2 rounded-full {bar}")
                                                                    style=format!("width: {p}%")
                                                                />
                                                            </div>
                                                            <span class="text-[10px] text-gray-500">
                                                                {format!("{p:.0}%")}
                                                            </span>
                                                        </div>
                                                    }.into_any()
                                                }
                                                None => view! {
                                                    <span class="text-gray-400">"—"</span>
                                                }.into_any(),
                                            }}
                                        </td>
                                    </tr>
                                }
                            }
                        </For>
                        <Show when=move || is_empty>
                            <tr>
                                <td colspan="4" class="px-4 py-8 text-center text-gray-500">
                                    "No active keys in this window"
                                </td>
                            </tr>
                        </Show>
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_rate_zero_requests_is_zero_not_nan() {
        assert_eq!(deny_rate_pct(0, 0), 0.0);
    }

    #[test]
    fn deny_rate_rounds_to_one_decimal() {
        // 1/3 = 33.333… → 33.3
        assert_eq!(deny_rate_pct(3, 1), 33.3);
        // 2/3 = 66.666… → 66.7
        assert_eq!(deny_rate_pct(3, 2), 66.7);
        assert_eq!(deny_rate_pct(4, 1), 25.0);
        assert_eq!(deny_rate_pct(10, 10), 100.0);
    }

    #[test]
    fn truncate_long_key_keeps_head_and_tail() {
        assert_eq!(truncate_key("klab_ABCDEFGHIJKLMNOP"), "klab_ABC…MNOP");
    }

    #[test]
    fn truncate_short_key_passes_through() {
        assert_eq!(truncate_key("klab_short"), "klab_short");
        // Exactly the threshold length is left intact.
        assert_eq!(truncate_key("1234567890123"), "1234567890123");
    }

    #[test]
    fn percent_remaining_normal_case() {
        assert_eq!(percent_remaining(50.0, Some(100.0)), Some(50.0));
        assert_eq!(percent_remaining(0.0, Some(100.0)), Some(0.0));
    }

    #[test]
    fn percent_remaining_missing_or_zero_limit_is_none() {
        // Deleted-from-Postgres key (no limit) → no ratio.
        assert_eq!(percent_remaining(10.0, None), None);
        // A zero or negative limit can't yield a ratio.
        assert_eq!(percent_remaining(10.0, Some(0.0)), None);
        assert_eq!(percent_remaining(10.0, Some(-5.0)), None);
    }

    #[test]
    fn percent_remaining_clamps_to_100_when_limit_shrank() {
        // Limit was lowered below the recorded remaining — must not exceed 100.
        assert_eq!(percent_remaining(150.0, Some(100.0)), Some(100.0));
    }

    #[test]
    fn trim_decimals_drops_trailing_zeros() {
        assert_eq!(trim_decimals(12.0), "12");
        assert_eq!(trim_decimals(12.5), "12.5");
        assert_eq!(trim_decimals(12.50), "12.5");
        assert_eq!(trim_decimals(0.0), "0");
    }
}
