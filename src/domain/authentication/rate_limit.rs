//! Rate-limit value objects.
//!
//! Three distinct types for what was previously the same `i32`:
//!
//! - [`RateLimitAmount`] — the daily (well, per-window) quota granted
//!   to a key. Non-negative.
//! - [`RateLimitUsage`] — how many units a single `/v1/auth` call is
//!   consuming. Non-negative.
//! - [`RateLimitWindow`] — the length of the reset interval, in seconds.
//!   Positive.
//!
//! Keeping them apart makes the arithmetic in the aggregate type-
//! checked: you cannot accidentally subtract an amount from a window
//! or compare usage to a window.

use crate::domain::errors::{DomainError, InvalidReason};
use chrono::Duration;

/// The quota a subscription grants per window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RateLimitAmount(i32);

impl RateLimitAmount {
    pub fn new(amount: i32) -> Result<Self, DomainError> {
        if amount < 0 {
            return Err(DomainError::InvalidRateLimitAmount(InvalidReason::Negative));
        }
        Ok(Self(amount))
    }

    /// Convenience for the common literal-in-tests case. Panics on a
    /// negative input — only call from test code where the value is a
    /// compile-time literal.
    pub fn literal(amount: i32) -> Self {
        Self::new(amount).expect("literal rate-limit amount must be non-negative")
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}

/// How many units one `/v1/auth` call consumes. Currently always `1` in
/// practice, but the API permits higher values, and the domain is the
/// place to carry that intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RateLimitUsage(i32);

impl RateLimitUsage {
    pub fn new(usage: i32) -> Result<Self, DomainError> {
        if usage < 0 {
            return Err(DomainError::InvalidRateLimitUsage(InvalidReason::Negative));
        }
        Ok(Self(usage))
    }

    /// Convenience for tests.
    pub fn literal(usage: i32) -> Self {
        Self::new(usage).expect("literal rate-limit usage must be non-negative")
    }

    /// The default when the client omits `rate_limit_usage` from the
    /// request body.
    pub fn default_one() -> Self {
        Self(1)
    }

    pub fn value(&self) -> i32 {
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
        assert_eq!(RateLimitAmount::new(0).unwrap().value(), 0);
    }

    #[test]
    fn amount_accepts_positive() {
        assert_eq!(RateLimitAmount::new(6_000).unwrap().value(), 6_000);
    }

    #[test]
    fn amount_rejects_negative() {
        assert_eq!(
            RateLimitAmount::new(-1).unwrap_err(),
            DomainError::InvalidRateLimitAmount(InvalidReason::Negative)
        );
    }

    #[test]
    fn usage_rejects_negative() {
        assert_eq!(
            RateLimitUsage::new(-5).unwrap_err(),
            DomainError::InvalidRateLimitUsage(InvalidReason::Negative)
        );
    }

    #[test]
    fn usage_default_is_one() {
        assert_eq!(RateLimitUsage::default_one().value(), 1);
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
        takes_amount(RateLimitAmount::literal(5));
        takes_usage(RateLimitUsage::literal(1));
    }
}
