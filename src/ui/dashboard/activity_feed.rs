use crate::models::{AuditEntry, DashboardInsights};
use crate::ui::design::{Badge, BadgeTone, Surface};
use crate::ui::tz::{to_local, use_tz_offset};
use leptos::prelude::*;

/// Badge tone per event type: a green Success for issuance, red Danger for
/// revocation, Brand for a device reassignment, amber Warning for a rate-limit
/// reset, Neutral for an expiration extension — and Neutral for anything the
/// feed doesn't recognise.
fn event_tone(event_type: &str) -> BadgeTone {
    match event_type {
        "KeyIssued" => BadgeTone::Success,
        "KeyRevoked" => BadgeTone::Danger,
        "KeyUnrevoked" => BadgeTone::Success,
        "DeviceReassigned" => BadgeTone::Brand,
        "RateLimitReset" => BadgeTone::Warning,
        "KeyExpirationExtended" => BadgeTone::Neutral,
        _ => BadgeTone::Neutral,
    }
}

/// A coarse, human relative timestamp: "just now", then minutes, hours, days.
/// Anything in the future (clock skew between server and viewer) reads as
/// "just now" rather than a negative age. Pure, so the thresholds are pinned
/// by unit tests.
fn relative_time(seconds_ago: i64) -> String {
    if seconds_ago < 60 {
        return "just now".to_string();
    }
    let minutes = seconds_ago / 60;
    if minutes < 60 {
        return format!("{minutes}m ago");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    format!("{days}d ago")
}

/// "Recent Admin Activity" feed: the latest admin events, newest first, as a
/// semantic timeline. Independent of the date-range picker — it always shows
/// the live picture. The heading is a real `<h2>` so e2e can target the panel
/// by role.
#[component]
pub fn ActivityFeed(#[prop(into)] insights: Signal<Option<DashboardInsights>>) -> impl IntoView {
    let entries = Signal::derive(move || {
        insights
            .get()
            .map(|i| i.recent_activity)
            .unwrap_or_default()
    });
    // Local-timezone offset for the exact-time tooltip, read reactively.
    let tz = use_tz_offset();

    view! {
        <Surface padded=true>
            <h2 class="text-sm font-medium text-ink-muted mb-4">"Recent Admin Activity"</h2>
            <Show
                when=move || !entries.get().is_empty()
                fallback=|| view! {
                    <p class="text-sm text-ink-muted py-4">
                        "No admin activity yet"
                    </p>
                }
            >
                <ul class="space-y-3">
                    <For
                        each=move || entries.get().into_iter().enumerate()
                        key=|(index, entry)| (*index, entry.event_type.clone(), entry.occurred_at)
                        let:item
                    >
                        {
                            let (_, entry): (usize, AuditEntry) = item;
                            let tone = event_tone(&entry.event_type);
                            let label = entry.event_type.clone();
                            let summary = entry.summary.clone();
                            let seconds_ago = (chrono::Utc::now() - entry.occurred_at).num_seconds();
                            let when = relative_time(seconds_ago);
                            let occurred_at = entry.occurred_at;
                            let title = move || {
                                to_local(occurred_at, tz.get())
                                    .format("%d %b %Y %H:%M")
                                    .to_string()
                            };
                            view! {
                                <li class="flex items-start gap-3">
                                    <Badge tone=tone>{label}</Badge>
                                    // `min-w-0` lets this flex child shrink below
                                    // its content's intrinsic width, and
                                    // `break-words` breaks an over-long unbroken
                                    // token (e.g. a 64-char device hash) onto the
                                    // next line instead of spilling past the
                                    // column into the timestamp.
                                    <span class="flex-1 min-w-0 break-words text-sm text-ink-subdued">
                                        {summary}
                                    </span>
                                    <span class="text-xs text-ink-muted whitespace-nowrap" title=title>
                                        {when}
                                    </span>
                                </li>
                            }
                        }
                    </For>
                </ul>
            </Show>
        </Surface>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_time_under_a_minute_is_just_now() {
        assert_eq!(relative_time(0), "just now");
        assert_eq!(relative_time(59), "just now");
    }

    #[test]
    fn relative_time_future_reads_just_now() {
        assert_eq!(relative_time(-10), "just now");
    }

    #[test]
    fn relative_time_minutes() {
        assert_eq!(relative_time(60), "1m ago");
        assert_eq!(relative_time(3 * 60 + 30), "3m ago");
        assert_eq!(relative_time(59 * 60), "59m ago");
    }

    #[test]
    fn relative_time_hours() {
        assert_eq!(relative_time(60 * 60), "1h ago");
        assert_eq!(relative_time(2 * 60 * 60), "2h ago");
        assert_eq!(relative_time(23 * 60 * 60), "23h ago");
    }

    #[test]
    fn relative_time_days() {
        assert_eq!(relative_time(24 * 60 * 60), "1d ago");
        assert_eq!(relative_time(5 * 24 * 60 * 60), "5d ago");
    }

    #[test]
    fn event_tone_per_type() {
        assert_eq!(event_tone("KeyIssued"), BadgeTone::Success);
        assert_eq!(event_tone("KeyRevoked"), BadgeTone::Danger);
        assert_eq!(event_tone("DeviceReassigned"), BadgeTone::Brand);
        assert_eq!(event_tone("RateLimitReset"), BadgeTone::Warning);
        assert_eq!(event_tone("KeyExpirationExtended"), BadgeTone::Neutral);
    }

    #[test]
    fn event_tone_unknown_is_neutral() {
        assert_eq!(event_tone("SomethingNew"), BadgeTone::Neutral);
    }
}
