use crate::models::{DashboardInsights, TierHealth};
use crate::ui::design::{Badge, BadgeTone, DataTable, Surface};
use leptos::prelude::*;

/// Format a quota with thousands separators: 12000 → "12,000". Negative values
/// keep their sign ahead of the grouped digits. Pure, so the grouping is
/// pinned by unit tests.
fn format_quota(amount: i64) -> String {
    let negative = amount < 0;
    let digits = amount.unsigned_abs().to_string();
    let bytes = digits.as_bytes();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(*b as char);
    }
    if negative {
        format!("-{grouped}")
    } else {
        grouped
    }
}

/// Status badge per tier: an active tier is a green Success "Active". An
/// inactive tier with no live keys is a Neutral "Retired" — expected. An
/// inactive tier that *still* carries live keys is the anomaly this widget
/// exists to surface: a red Danger "Retired · keys live". Pure, so the mapping
/// is pinned by unit tests.
fn status_badge(is_active: bool, active_keys: i64) -> (BadgeTone, &'static str) {
    if is_active {
        return (BadgeTone::Success, "Active");
    }
    if active_keys > 0 {
        return (BadgeTone::Danger, "Retired · keys live");
    }
    (BadgeTone::Neutral, "Retired")
}

/// "Tier Health" panel: one row per subscription tier with its live key
/// population, quota, interval, and status. Independent of the date-range
/// picker — it always shows the live catalogue. The heading is a real `<h2>`
/// so e2e can target the panel by role.
#[component]
pub fn TierHealthPanel(#[prop(into)] insights: Signal<Option<DashboardInsights>>) -> impl IntoView {
    let rows = Signal::derive(move || insights.get().map(|i| i.tier_health).unwrap_or_default());

    view! {
        <Surface padded=true>
            <h2 class="text-sm font-medium text-ink-muted mb-4">"Tier Health"</h2>
            <Show
                when=move || !rows.get().is_empty()
                fallback=|| view! {
                    <p class="text-sm text-ink-muted py-4">
                        "No subscription tiers configured"
                    </p>
                }
            >
                <DataTable headers=vec!["Tier", "Active Keys", "Quota", "Interval", "Status"]>
                    <For
                        each=move || rows.get()
                        key=|r: &TierHealth| r.display_name.clone()
                        let:row
                    >
                        {
                            let tier = row.display_name.clone();
                            let active_keys = row.active_keys.to_string();
                            let quota = format_quota(row.rate_limit_amount);
                            let interval = row.interval.clone();
                            let (tone, label) = status_badge(row.is_active, row.active_keys);
                            view! {
                                <tr>
                                    <td class="px-6 py-3 text-sm text-ink-heading">
                                        {tier}
                                    </td>
                                    <td class="px-6 py-3 text-sm text-ink-subdued">
                                        {active_keys}
                                    </td>
                                    <td class="px-6 py-3 text-sm text-ink-subdued">
                                        {quota}
                                    </td>
                                    <td class="px-6 py-3 text-sm text-ink-subdued">
                                        {interval}
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
    fn format_quota_groups_thousands() {
        assert_eq!(format_quota(0), "0");
        assert_eq!(format_quota(6000), "6,000");
        assert_eq!(format_quota(12000), "12,000");
        assert_eq!(format_quota(200000), "200,000");
        assert_eq!(format_quota(1234567), "1,234,567");
    }

    #[test]
    fn format_quota_small_numbers_pass_through() {
        assert_eq!(format_quota(1), "1");
        assert_eq!(format_quota(42), "42");
        assert_eq!(format_quota(999), "999");
    }

    #[test]
    fn format_quota_keeps_sign() {
        assert_eq!(format_quota(-1000), "-1,000");
    }

    #[test]
    fn status_active_tier_is_success() {
        assert_eq!(status_badge(true, 0), (BadgeTone::Success, "Active"));
        assert_eq!(status_badge(true, 5), (BadgeTone::Success, "Active"));
    }

    #[test]
    fn status_inactive_with_no_keys_is_neutral_retired() {
        assert_eq!(status_badge(false, 0), (BadgeTone::Neutral, "Retired"));
    }

    #[test]
    fn status_inactive_with_live_keys_is_danger() {
        assert_eq!(
            status_badge(false, 1),
            (BadgeTone::Danger, "Retired · keys live")
        );
        assert_eq!(
            status_badge(false, 99),
            (BadgeTone::Danger, "Retired · keys live")
        );
    }
}
