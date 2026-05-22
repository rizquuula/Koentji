//! Rate-limit value objects.
//!
//! Three distinct types for what was previously the same `i32`:
//!
//! - [`RateLimitAmount`] — the daily (well, per-window) quota granted
//!   to a key. Non-negative, finite.
//! - [`RateLimitUsage`] — how many units a single `/v1/auth` call is
//!   consuming. Non-negative, finite.
//! - [`RateLimitWindow`] — the length of the reset interval, in seconds.
//!   Positive.
//!
//! Keeping them apart makes the arithmetic in the aggregate type-
//! checked: you cannot accidentally subtract an amount from a window
//! or compare usage to a window.
//!
//! Storage is `f64` so subscriptions can express fractional quotas
//! (e.g. ClickHouse cost units). `Eq`/`Hash`/`Ord` are intentionally
//! not derived — IEEE-754 forbids total ordering / hashing of `f64`.

use crate::domain::errors::{DomainError, InvalidReason};
use chrono::Duration;

/// The quota a subscription grants per window.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct RateLimitAmount(f64);

impl RateLimitAmount {
    pub fn new(amount: f64) -> Result<Self, DomainError> {
        if amount.is_nan() || amount.is_infinite() {
            return Err(DomainError::InvalidRateLimitAmount(
                InvalidReason::NotFinite,
            ));
        }
        if amount < 0.0 {
            return Err(DomainError::InvalidRateLimitAmount(InvalidReason::Negative));
        }
        Ok(Self(amount))
    }

    /// Convenience for the common literal-in-tests case. Panics on a
    /// negative / non-finite input — only call from test code where the
    /// value is a compile-time literal.
    pub fn literal(amount: f64) -> Self {
        Self::new(amount).expect("literal rate-limit amount must be finite and non-negative")
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

/// How many units one `/v1/auth` call consumes. Currently always `1` in
/// practice, but the API permits higher values, and the domain is the
/// place to carry that intent.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct RateLimitUsage(f64);

impl RateLimitUsage {
    pub fn new(usage: f64) -> Result<Self, DomainError> {
        if usage.is_nan() || usage.is_infinite() {
            return Err(DomainError::InvalidRateLimitUsage(InvalidReason::NotFinite));
        }
        if usage < 0.0 {
            return Err(DomainError::InvalidRateLimitUsage(InvalidReason::Negative));
        }
        Ok(Self(usage))
    }

    /// Convenience for tests.
    pub fn literal(usage: f64) -> Self {
        Self::new(usage).expect("literal rate-limit usage must be finite and non-negative")
    }

    /// The default when the client omits `rate_limit_usage` from the
    /// request body.
    pub fn default_one() -> Self {
        Self(1.0)
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

/// The length of the reset interval attached to a subscription's rate
/// limit policy. Stored as whole seconds — matches
/// `rate_limit_intervals.duration_seconds`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RateLimitWindow {
    seconds: i64,
}

impl RateLimitWindow {
    pub fn from_seconds(seconds: i64) -> Result<Self, DomainError> {
        if seconds <= 0 {
            return Err(DomainError::InvalidRateLimitWindow(InvalidReason::Zero));
        }
        Ok(Self { seconds })
    }

    /// The legacy default when a row has no linked interval.
    pub fn daily() -> Self {
        Self { seconds: 86_400 }
    }

    pub fn as_seconds(&self) -> i64 {
        self.seconds
    }

    pub fn as_duration(&self) -> Duration {
        Duration::seconds(self.seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amount_accepts_zero() {
        assert_eq!(RateLimitAmount::new(0.0).unwrap().value(), 0.0);
    }

    #[test]
    fn amount_accepts_positive() {
        assert_eq!(RateLimitAmount::new(6_000.0).unwrap().value(), 6_000.0);
    }

    #[test]
    fn amount_accepts_fractional() {
        assert_eq!(RateLimitAmount::new(0.5).unwrap().value(), 0.5);
    }

    #[test]
    fn amount_rejects_negative() {
        assert_eq!(
            RateLimitAmount::new(-1.0).unwrap_err(),
            DomainError::InvalidRateLimitAmount(InvalidReason::Negative)
        );
    }

    #[test]
    fn amount_rejects_nan() {
        assert_eq!(
            RateLimitAmount::new(f64::NAN).unwrap_err(),
            DomainError::InvalidRateLimitAmount(InvalidReason::NotFinite)
        );
    }

    #[test]
    fn amount_rejects_infinity() {
        assert_eq!(
            RateLimitAmount::new(f64::INFINITY).unwrap_err(),
            DomainError::InvalidRateLimitAmount(InvalidReason::NotFinite)
        );
        assert_eq!(
            RateLimitAmount::new(f64::NEG_INFINITY).unwrap_err(),
            DomainError::InvalidRateLimitAmount(InvalidReason::NotFinite)
        );
    }

    #[test]
    fn usage_rejects_negative() {
        assert_eq!(
            RateLimitUsage::new(-5.0).unwrap_err(),
            DomainError::InvalidRateLimitUsage(InvalidReason::Negative)
        );
    }

    #[test]
    fn usage_rejects_nan_and_inf() {
        assert_eq!(
            RateLimitUsage::new(f64::NAN).unwrap_err(),
            DomainError::InvalidRateLimitUsage(InvalidReason::NotFinite)
        );
        assert_eq!(
            RateLimitUsage::new(f64::INFINITY).unwrap_err(),
            DomainError::InvalidRateLimitUsage(InvalidReason::NotFinite)
        );
    }

    #[test]
    fn usage_default_is_one() {
        assert_eq!(RateLimitUsage::default_one().value(), 1.0);
    }

    #[test]
    fn window_rejects_zero_and_negative() {
        assert_eq!(
            RateLimitWindow::from_seconds(0).unwrap_err(),
            DomainError::InvalidRateLimitWindow(InvalidReason::Zero)
        );
        assert_eq!(
            RateLimitWindow::from_seconds(-1).unwrap_err(),
            DomainError::InvalidRateLimitWindow(InvalidReason::Zero)
        );
    }

    #[test]
    fn window_daily_is_86400_seconds() {
        assert_eq!(RateLimitWindow::daily().as_seconds(), 86_400);
        assert_eq!(RateLimitWindow::daily().as_duration(), Duration::days(1));
    }

    #[test]
    fn distinct_types_do_not_conflate() {
        // This compiles only because each type is its own nominal type
        // — you can't pass a RateLimitUsage where RateLimitAmount is
        // expected, which is the whole point of the split.
        fn takes_amount(_: RateLimitAmount) {}
        fn takes_usage(_: RateLimitUsage) {}
        takes_amount(RateLimitAmount::literal(5.0));
        takes_usage(RateLimitUsage::literal(1.0));
    }

    #[test]
    fn fractional_usage_leaves_a_fractional_remainder() {
        // Pure arithmetic check — usage 0.5 on a daily of 1.0 leaves 0.5.
        let daily = RateLimitAmount::literal(1.0);
        let usage = RateLimitUsage::literal(0.5);
        assert_eq!(daily.value() - usage.value(), 0.5);
    }
}
