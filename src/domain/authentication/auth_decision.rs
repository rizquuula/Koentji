//! The outcome of asking "may this `(key, device)` consume quota right
//! now?". Pure data — no HTTP, no i18n, no DB.
//!
//! 1.3 will replace the inline `DenialReason` below with a richer,
//! dedicated enum plus an en/id translation mapper living at the HTTP
//! edge. For now the enum is just rich enough to let tests pin the
//! decision table.

use chrono::{DateTime, Utc};

use super::rate_limit::RateLimitAmount;

/// Why a key was refused. Dates carry forward so the caller can render
/// messages like "revoked since …".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenialReason {
    /// No matching `(key, device)` and the free-trial shortcut did not
    /// apply.
    UnknownKey,
    /// Soft-deleted by an admin.
    Revoked { at: DateTime<Utc> },
    /// Non-trial key whose `expired_at` is in the past.
    Expired { at: DateTime<Utc> },
    /// Free-trial key whose `expired_at` is in the past.
    FreeTrialEnded { at: DateTime<Utc> },
    /// Window is open and the remaining quota cannot cover the
    /// requested usage (or the usage itself exceeds the daily cap).
    RateLimitExceeded,
}

/// The outcome of [`super::issued_key::IssuedKey::authorize`].
///
/// When `Allowed`, the returned `remaining` reflects the
/// post-decrement value the caller should persist; the decrement is
/// pure and has not yet touched infrastructure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthDecision {
    Allowed {
        remaining: RateLimitAmount,
        updated_at: DateTime<Utc>,
    },
    Denied {
        reason: DenialReason,
    },
}

impl AuthDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed { .. })
    }

    pub fn denial_reason(&self) -> Option<&DenialReason> {
        match self {
            Self::Denied { reason } => Some(reason),
            Self::Allowed { .. } => None,
        }
    }
}
