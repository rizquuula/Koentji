//! Regression coverage for the stats_service SQL-injection fix (B8 / 0.2).
//!
//! The old `get_dashboard_stats` built its filter with `format!` and the
//! `"custom"` branch accepted arbitrary strings as date fields. These
//! tests pin the new contract: only well-formed YYYY-MM-DD dates produce
//! a bound window — anything else degrades to `(None, None)` so nothing
//! user-controlled reaches the SQL layer (queries now bind `Option<_>`).

#![cfg(feature = "ssr")]

use koentji::server::stats_service::resolve_date_range;

#[test]
fn custom_range_rejects_sql_payload_on_start_date() {
    let (start, end) = resolve_date_range("custom", "'; DROP TABLE users; --", "2026-01-31");
    assert!(start.is_none(), "payload must not parse to a DateTime");
    assert!(end.is_none(), "one bad side forces both to None");
}

#[test]
fn custom_range_rejects_sql_payload_on_end_date() {
    let (start, end) = resolve_date_range("custom", "2026-01-01", "2026-01-31' OR '1'='1");
    assert!(start.is_none());
    assert!(end.is_none());
}

#[test]
fn custom_range_accepts_well_formed_dates() {
    let (start, end) = resolve_date_range("custom", "2026-01-01", "2026-01-31");
    let start = start.expect("valid start");
    let end = end.expect("valid end");
    assert!(start < end, "start must precede end");
}

#[test]
fn non_custom_ranges_return_a_window() {
    for r in ["7d", "30d", "90d"] {
        let (start, end) = resolve_date_range(r, "", "");
        assert!(start.is_some(), "{r} should yield a start bound");
        assert!(end.is_some(), "{r} should yield an end bound");
    }
}

#[test]
fn unknown_range_returns_no_window() {
    let (start, end) = resolve_date_range("forever", "", "");
    assert!(start.is_none());
    assert!(end.is_none());
}

#[test]
fn custom_range_with_empty_dates_returns_no_window() {
    let (start, end) = resolve_date_range("custom", "", "");
    assert!(start.is_none());
    assert!(end.is_none());
}
