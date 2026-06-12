use crate::models::{DashboardInsights, ExpiringKey};
use crate::ui::design::{Badge, BadgeTone, DataTable, Surface};
use leptos::prelude::*;

/// Truncate a long API key for display: first 8 chars + "…" + last 4. Short
/// keys (≤ 13 chars) pass through unchanged; the full key rides in the cell's
/// `title`. Mirrors the analytics tables' idiom so keys read identically
/// across the dashboard and the analytics page.
fn truncate_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 13 {
        return key.to_string();
    }
    let head: String = chars.iter().take(8).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{head}…{tail}")
}

/// The Owner column: prefer a human name, fall back to email, then to the
/// bound device. A pre-issued key with none of those still shows its sentinel
/// device rather than an empty cell.
fn owner_label(row: &ExpiringKey) -> String {
    if let Some(username) = row.username.as_ref().filter(|s| !s.is_empty()) {
        return username.clone();
    }
    if let Some(email) = row.email.as_ref().filter(|s| !s.is_empty()) {
        return email.clone();
    }
    row.device_id.clone()
}

/// Days-left badge tone: red inside the one-week danger zone, amber beyond.
fn days_left_tone(days_left: i64) -> BadgeTone {
    if days_left <= 7 {
        return BadgeTone::Danger;
    }
    BadgeTone::Warning
}

fn days_left_label(days_left: i64) -> String {
    if days_left == 1 {
        return "1 day".to_string();
    }
    format!("{days_left} days")
}

/// "Expiring Soon" early-warning panel: the soonest-lapsing active keys in
/// the next 90 days. Independent of the date-range picker — it always shows
/// the live picture. The heading is a real `<h2>` so e2e can target the panel
/// by role.
#[component]
pub fn ExpiringKeys(#[prop(into)] insights: Signal<Option<DashboardInsights>>) -> impl IntoView {
    let rows = Signal::derive(move || insights.get().map(|i| i.expiring_keys).unwrap_or_default());

    view! {
        <Surface padded=true>
            <h2 class="text-sm font-medium text-ink-muted mb-4">"Expiring Soon"</h2>
            <Show
                when=move || !rows.get().is_empty()
                fallback=|| view! {
                    <p class="text-sm text-ink-muted py-4">
                        "No keys expiring in the next 90 days"
                    </p>
                }
            >
                <DataTable headers=vec!["Key", "Owner", "Expires", "Days left"]>
                    <For
                        each=move || rows.get()
                        key=|r| r.key.clone()
                        let:row
                    >
                        {
                            let full_key = row.key.clone();
                            let truncated = truncate_key(&row.key);
                            let owner = owner_label(&row);
                            let expires = row.expired_at.format("%d %b %Y").to_string();
                            let tone = days_left_tone(row.days_left);
                            let label = days_left_label(row.days_left);
                            view! {
                                <tr>
                                    <td class="px-6 py-3 text-sm font-mono text-ink-heading" title=full_key>
                                        {truncated}
                                    </td>
                                    <td class="px-6 py-3 text-sm text-ink-subdued">
                                        {owner}
                                    </td>
                                    <td class="px-6 py-3 text-sm text-ink-subdued">
                                        {expires}
                                    </td>
                                    <td class="px-6 py-3 text-sm">
                                        <Badge tone=tone>{label}</Badge>
                                    </td>
                                </tr>
                            }
                        }
                    </For>
                </DataTable>
            </Show>
        </Surface>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_long_key_keeps_head_and_tail() {
        assert_eq!(truncate_key("klab_ABCDEFGHIJKLMNOP"), "klab_ABC…MNOP");
    }

    #[test]
    fn truncate_short_key_passes_through() {
        assert_eq!(truncate_key("klab_short"), "klab_short");
        assert_eq!(truncate_key("1234567890123"), "1234567890123");
    }

    fn a_row() -> ExpiringKey {
        ExpiringKey {
            key: "klab_owner".to_string(),
            username: None,
            email: None,
            device_id: "device-7".to_string(),
            expired_at: chrono::Utc::now(),
            days_left: 5,
        }
    }

    #[test]
    fn owner_prefers_username_then_email_then_device() {
        let mut row = a_row();
        assert_eq!(owner_label(&row), "device-7");

        row.email = Some("user@example.com".to_string());
        assert_eq!(owner_label(&row), "user@example.com");

        row.username = Some("alice".to_string());
        assert_eq!(owner_label(&row), "alice");
    }

    #[test]
    fn owner_skips_empty_strings() {
        let mut row = a_row();
        row.username = Some(String::new());
        row.email = Some(String::new());
        assert_eq!(owner_label(&row), "device-7");
    }

    #[test]
    fn days_left_danger_within_a_week() {
        assert_eq!(days_left_tone(1), BadgeTone::Danger);
        assert_eq!(days_left_tone(7), BadgeTone::Danger);
        assert_eq!(days_left_tone(8), BadgeTone::Warning);
        assert_eq!(days_left_tone(30), BadgeTone::Warning);
    }

    #[test]
    fn days_left_label_singular_and_plural() {
        assert_eq!(days_left_label(1), "1 day");
        assert_eq!(days_left_label(2), "2 days");
        assert_eq!(days_left_label(30), "30 days");
    }
}
