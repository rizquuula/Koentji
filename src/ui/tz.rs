//! Viewer timezone offset, provided once and read wherever a UTC timestamp is
//! rendered for a human.
//!
//! The offset is **minutes east of UTC** (UTC+7 → `420`), the sign convention
//! the formatting helpers add straight onto a UTC instant. It lives in a
//! context `RwSignal<i32>` seeded at `0` (UTC). The server-side render and the
//! very first hydration frame both read `0`, so the hydrated markup matches the
//! SSR markup byte-for-byte — no hydration mismatch. A client-only `Effect`
//! then reads the browser's real offset and updates the signal, which
//! reactively re-renders every timestamp into the viewer's local time.

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use leptos::prelude::*;

/// Minutes east of UTC for the viewer's browser. Seeded `0` (UTC) so SSR and
/// the first hydration frame agree; updated to the real offset on the client.
#[derive(Clone, Copy)]
pub struct TzOffset(pub RwSignal<i32>);

/// Provide the offset context and, on the client, populate it from the browser.
/// Call once near the app root. On the server the `Effect` never runs, so the
/// signal stays `0` and timestamps render in UTC exactly as before.
pub fn provide_tz_offset() {
    let offset = RwSignal::new(0);
    provide_context(TzOffset(offset));

    // Effects run only in the browser. `Date::getTimezoneOffset()` returns
    // minutes *behind* UTC (UTC+7 → `-420`), so negate to get minutes east.
    Effect::new(move |_| {
        let browser = js_sys::Date::new_0().get_timezone_offset();
        offset.set(-(browser as i32));
    });
}

/// Read the offset signal. Falls back to a fixed `0` (UTC) signal when no
/// provider is in scope, so a component rendered outside the app shell (e.g.
/// an isolated test mount) still renders rather than panicking.
pub fn use_tz_offset() -> Signal<i32> {
    let TzOffset(sig) = use_context::<TzOffset>().unwrap_or_else(|| TzOffset(RwSignal::new(0)));
    sig.into()
}

/// Shift a UTC instant by `offset_minutes` to the viewer's local wall-clock
/// time, returned as a *naive* datetime ready to `.format()`. Pure, so the
/// arithmetic is pinned by a unit test in one place.
pub fn to_local(dt: DateTime<Utc>, offset_minutes: i32) -> NaiveDateTime {
    (dt + Duration::minutes(offset_minutes as i64)).naive_utc()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn to_local_shifts_east_by_offset() {
        // 2026-06-13 22:30 UTC, viewer at UTC+7 → 2026-06-14 05:30 local.
        let utc = Utc.with_ymd_and_hms(2026, 6, 13, 22, 30, 0).unwrap();
        let local = to_local(utc, 420);
        assert_eq!(
            local.format("%Y-%m-%d %H:%M").to_string(),
            "2026-06-14 05:30"
        );
    }

    #[test]
    fn to_local_shifts_west_for_negative_offset() {
        // 2026-06-13 02:00 UTC, viewer at UTC-5 → 2026-06-12 21:00 local.
        let utc = Utc.with_ymd_and_hms(2026, 6, 13, 2, 0, 0).unwrap();
        let local = to_local(utc, -300);
        assert_eq!(
            local.format("%Y-%m-%d %H:%M").to_string(),
            "2026-06-12 21:00"
        );
    }

    #[test]
    fn to_local_zero_offset_is_utc() {
        let utc = Utc.with_ymd_and_hms(2026, 6, 13, 12, 0, 0).unwrap();
        assert_eq!(to_local(utc, 0), utc.naive_utc());
    }
}
