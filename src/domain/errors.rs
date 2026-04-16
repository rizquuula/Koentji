//! Pure-domain error types.
//!
//! These describe *domain invariant violations* — "you cannot build an
//! `AuthKey` from an empty string" — not infrastructure failures like
//! "the DB is down". Infra errors never leak into the domain; they are
//! mapped at the interface edge.

use std::fmt;

/// A domain-level invariant violation. Value-object constructors return
/// this when their parse rules reject an input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    InvalidAuthKey(InvalidReason),
    InvalidDeviceId(InvalidReason),
    InvalidRateLimitAmount(InvalidReason),
    InvalidRateLimitUsage(InvalidReason),
    InvalidRateLimitWindow(InvalidReason),
    InvalidSubscriptionName(InvalidReason),
}

/// Why a value was rejected. Small stable set so handlers can branch on
/// it (`Empty`, `TooLong`, `Negative`, `Zero`) without string matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidReason {
    Empty,
    TooLong,
    Negative,
    Zero,
}

impl fmt::Display for InvalidReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("must not be empty"),
            Self::TooLong => f.write_str("exceeds the maximum length"),
            Self::Negative => f.write_str("must not be negative"),
            Self::Zero => f.write_str("must be greater than zero"),
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAuthKey(r) => write!(f, "invalid auth key: {}", r),
            Self::InvalidDeviceId(r) => write!(f, "invalid device id: {}", r),
            Self::InvalidRateLimitAmount(r) => write!(f, "invalid rate limit amount: {}", r),
            Self::InvalidRateLimitUsage(r) => write!(f, "invalid rate limit usage: {}", r),
            Self::InvalidRateLimitWindow(r) => write!(f, "invalid rate limit window: {}", r),
            Self::InvalidSubscriptionName(r) => write!(f, "invalid subscription name: {}", r),
        }
    }
}

impl std::error::Error for DomainError {}
