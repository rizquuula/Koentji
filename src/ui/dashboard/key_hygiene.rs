use crate::models::{DashboardInsights, HygieneKey};
use crate::ui::design::{Badge, BadgeTone, DataTable, Surface};
use leptos::prelude::*;

/// Truncate a long API key for display: first 8 chars + "…" + last 4. Short
/// keys (≤ 13 chars) pass through unchanged; the full key rides in the cell's
/// `title`. Mirrors `expiring_keys.rs` so keys read identically across the
/// dashboard.
fn truncate_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 13 {
        return key.to_string();
    }
    let head: String = chars.iter().take(8).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{head}…{tail}")
}

/// Truncate a long device id the same way as a key. Short ids pass through.
fn truncate_device(device_id: &str) -> String {
    truncate_key(device_id)
}

/// The Owner column: prefer a human name, fall back to email, then to an em
/// dash. Unlike the expiring-keys panel there's no device fallback — an
/// unclaimed key has no real device, and a dormant key shows its device in its
/// own column.
fn owner_label(row: &HygieneKey) -> String {
    if let Some(username) = row.username.as_ref().filter(|s| !s.is_empty()) {
        return username.clone();
    }
    if let Some(email) = row.email.as_ref().filter(|s| !s.is_empty()) {
        return email.clone();
    }
    "—".to_string()
}

/// Compact age label: "12d". Pure, so the formatting is pinned by unit tests.
fn age_label(age_days: i64) -> String {
    format!("{age_days}d")
}

/// "Showing N of M" caption, only meaningful when the population exceeds the
/// shown rows. Returns `None` when nothing is truncated so the caller can skip
/// the line entirely.
fn capped_caption(shown: usize, total: i64) -> Option<String> {
    if total > shown as i64 {
        return Some(format!("Showing {shown} of {total}"));
    }
    None
}

/// "Key Hygiene" panel: two issued-but-unused populations an admin can clean
/// up — Unclaimed (pre-issued, still on the `-` sentinel) and Dormant (claimed
/// but full quota, never used). The heading is a real `<h2>` and each
/// sub-section a real `<h3>` so e2e can target them by role.
#[component]
pub fn KeyHygiene(#[prop(into)] insights: Signal<Option<DashboardInsights>>) -> impl IntoView {
    let hygiene = Signal::derive(move || insights.get().map(|i| i.key_hygiene).unwrap_or_default());

    let unclaimed = Signal::derive(move || hygiene.get().unclaimed);
    let unclaimed_total = Signal::derive(move || hygiene.get().unclaimed_total);
    let dormant = Signal::derive(move || hygiene.get().dormant);
    let dormant_total = Signal::derive(move || hygiene.get().dormant_total);

    view! {
        <Surface padded=true>
            <h2 class="text-sm font-medium text-ink-muted mb-4">"Key Hygiene"</h2>
            <div class="space-y-6">
                <HygieneSection
                    title="Unclaimed"
                    rows=unclaimed
                    total=unclaimed_total
                    empty_label="No unclaimed keys"
                    show_device=false
                />
                <HygieneSection
                    title="Dormant"
                    rows=dormant
                    total=dormant_total
                    empty_label="No dormant keys"
                    show_device=true
                />
            </div>
        </Surface>
    }
}

/// One labelled sub-section of the panel: an `<h3>` heading row (carrying a
/// Warning-tone count badge when the population is non-empty), a `DataTable`,
/// and a muted "Showing N of M" caption when the list is capped. The Dormant
/// table adds a Device column; the Unclaimed table omits it.
#[component]
fn HygieneSection(
    title: &'static str,
    #[prop(into)] rows: Signal<Vec<HygieneKey>>,
    #[prop(into)] total: Signal<i64>,
    empty_label: &'static str,
    show_device: bool,
) -> impl IntoView {
    let headers = if show_device {
        vec!["Key", "Owner", "Device", "Age"]
    } else {
        vec!["Key", "Owner", "Age"]
    };

    let has_population = Signal::derive(move || total.get() > 0);
    let count_label = Signal::derive(move || total.get().to_string());
    let has_rows = Signal::derive(move || !rows.get().is_empty());
    let caption = Signal::derive(move || capped_caption(rows.get().len(), total.get()));
    let is_capped = Signal::derive(move || caption.get().is_some());

    view! {
        <div>
            <div class="flex items-center gap-2 mb-3">
                <h3 class="text-sm font-medium text-ink-heading">{title}</h3>
                <Show when=move || has_population.get()>
                    <Badge tone=BadgeTone::Warning>
                        {move || count_label.get()}
                    </Badge>
                </Show>
            </div>
            <Show
                when=move || has_rows.get()
                fallback=move || view! {
                    <p class="text-sm text-ink-muted py-4">{empty_label}</p>
                }
            >
                <DataTable headers=headers.clone()>
                    <For
                        each=move || rows.get()
                        key=|r| r.key.clone()
                        let:row
                    >
                        {
                            let full_key = row.key.clone();
                            let truncated = truncate_key(&row.key);
                            let owner = owner_label(&row);
                            let age = age_label(row.age_days);
                            let device_cell = show_device.then(|| {
                                let device = row
                                    .device_id
                                    .as_ref()
                                    .map(|d| truncate_device(d))
                                    .unwrap_or_else(|| "—".to_string());
                                let device_title = row.device_id.clone().unwrap_or_default();
                                view! {
                                    <td class="px-6 py-3 text-sm font-mono text-ink-subdued" title=device_title>
                                        {device}
                                    </td>
                                }
                            });
                            view! {
                                <tr>
                                    <td class="px-6 py-3 text-sm font-mono text-ink-heading" title=full_key>
                                        {truncated}
                                    </td>
                                    <td class="px-6 py-3 text-sm text-ink-subdued">
                                        {owner}
                                    </td>
                                    {device_cell}
                                    <td class="px-6 py-3 text-sm text-ink-subdued">
                                        {age}
                                    </td>
                                </tr>
                            }
                        }
                    </For>
                </DataTable>
                <Show when=move || is_capped.get()>
                    <p class="text-xs text-ink-muted mt-2">
                        {move || caption.get().unwrap_or_default()}
                    </p>
                </Show>
            </Show>
        </div>
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

    fn a_row() -> HygieneKey {
        HygieneKey {
            key: "klab_owner".to_string(),
            username: None,
            email: None,
            device_id: None,
            created_at: chrono::Utc::now(),
            age_days: 12,
        }
    }

    #[test]
    fn owner_prefers_username_then_email_then_dash() {
        let mut row = a_row();
        assert_eq!(owner_label(&row), "—");

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
        assert_eq!(owner_label(&row), "—");
    }

    #[test]
    fn age_label_formats_days() {
        assert_eq!(age_label(0), "0d");
        assert_eq!(age_label(1), "1d");
        assert_eq!(age_label(12), "12d");
    }

    #[test]
    fn capped_caption_only_when_truncated() {
        assert_eq!(capped_caption(10, 25), Some("Showing 10 of 25".to_string()));
        assert_eq!(capped_caption(3, 3), None);
        assert_eq!(capped_caption(10, 10), None);
        // Total below shown (shouldn't happen, but never lies about it).
        assert_eq!(capped_caption(10, 5), None);
    }
}
