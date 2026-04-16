//! The `IssuedKey` aggregate — one `(AuthKey, DeviceId)` row, in
//! domain-speak. Lifecycle verbs replace the CRUD-shaped `update_*`
//! calls on the old `AuthenticationKey` struct:
//!
//! - [`IssuedKey::authorize`] — pure decision: given a usage and a
//!   clock reading, may this request proceed? No mutation yet — the
//!   caller decides whether to persist the decrement.
//! - [`IssuedKey::revoke`] / [`IssuedKey::reassign_to`] /
//!   [`IssuedKey::reset_rate_limit`] / [`IssuedKey::extend_until`] —
//!   in-memory state transitions. Persisting them is the repository's
//!   job, not the aggregate's.
//!
//! The aggregate is deliberately decoupled from SQLx. 1.4 will bolt
//! the Postgres adapter on through a repository port.

use chrono::{DateTime, Utc};

use super::auth_decision::{AuthDecision, DenialReason};
use super::auth_key::AuthKey;
use super::device_id::DeviceId;
use super::rate_limit::{RateLimitAmount, RateLimitUsage, RateLimitWindow};
use super::subscription_name::SubscriptionName;

/// The free-trial marker used in the legacy schema to distinguish auto-
/// provisioned rows from admin-issued ones. Kept as a constant so the
/// string literal doesn't leak across the codebase.
pub const FREE_TRIAL_MARKER_DEFAULT: &str = "FREE_TRIAL";

/// Identity — once persisted, every aggregate has a stable numeric id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IssuedKeyId(i32);

impl IssuedKeyId {
    pub fn new(id: i32) -> Self {
        Self(id)
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}

/// The quota ledger attached to an `IssuedKey`.
///
/// Tracks the daily allotment, the remaining quota inside the current
/// window, and when that window last advanced. The reset decision is a
/// pure function of `(window, last_updated, now)` — no IO needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitLedger {
    pub daily: RateLimitAmount,
    pub remaining: RateLimitAmount,
    pub window: RateLimitWindow,
    pub last_updated_at: Option<DateTime<Utc>>,
}

impl RateLimitLedger {
    /// Has the current window elapsed as of `now`? If yes, the next
    /// consume resets `remaining` to `daily - usage` instead of
    /// decrementing.
    fn window_has_elapsed(&self, now: DateTime<Utc>) -> bool {
        match self.last_updated_at {
            None => true,
            Some(last) => (now - last) >= self.window.as_duration(),
        }
    }
}

/// The full aggregate. `authorize` is pure; the lifecycle verbs return
/// `&mut self` updates without touching infrastructure.
///
/// `username` and `email` ride along because the success envelope on
/// `/v1/auth` echoes them back to clients. They are not part of the
/// authorization decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedKey {
    pub id: IssuedKeyId,
    pub key: AuthKey,
    pub device_id: DeviceId,
    pub subscription: Option<SubscriptionName>,
    pub rate_limit: RateLimitLedger,
    pub expired_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub is_free_trial: bool,
    pub username: Option<String>,
    pub email: Option<String>,
}

impl IssuedKey {
    /// Pure decision: would a consume of `usage` succeed at `now`?
    ///
    /// Invariant order matches the legacy handler so the HTTP envelope
    /// does not change in Phase 1.6:
    ///
    /// 1. revoked → `Denied(Revoked)`
    /// 2. expired (free-trial vs admin split) → `Denied(FreeTrialEnded | Expired)`
    /// 3. rate limit — respect the window-reset branch, then the legacy
    ///    off-by-one predicate (see `src/rate_limit.rs`).
    pub fn authorize(&self, usage: RateLimitUsage, now: DateTime<Utc>) -> AuthDecision {
        if let Some(at) = self.revoked_at {
            return AuthDecision::Denied {
                reason: DenialReason::Revoked { at },
            };
        }

        if let Some(at) = self.expired_at {
            if at <= now {
                let reason = if self.is_free_trial {
                    DenialReason::FreeTrialEnded { at }
                } else {
                    DenialReason::Expired { at }
                };
                return AuthDecision::Denied { reason };
            }
        }

        // Legacy off-by-one: daily must be strictly greater than usage,
        // and either the window has elapsed (reset branch) or the
        // current remaining must also be strictly greater.
        if self.rate_limit.daily.value() <= usage.value() {
            return AuthDecision::Denied {
                reason: DenialReason::RateLimitExceeded,
            };
        }

        let window_elapsed = self.rate_limit.window_has_elapsed(now);
        if !window_elapsed && self.rate_limit.remaining.value() <= usage.value() {
            return AuthDecision::Denied {
                reason: DenialReason::RateLimitExceeded,
            };
        }

        let new_remaining_value = if window_elapsed {
            self.rate_limit.daily.value() - usage.value()
        } else {
            self.rate_limit.remaining.value() - usage.value()
        };

        AuthDecision::Allowed {
            remaining: RateLimitAmount::literal(new_remaining_value),
            updated_at: now,
        }
    }

    /// Mark this key as revoked by an admin. Idempotent — re-revoking
    /// does not bump the timestamp.
    pub fn revoke(&mut self, at: DateTime<Utc>) {
        if self.revoked_at.is_none() {
            self.revoked_at = Some(at);
        }
    }

    /// Move this key to a different device. Does not reset the ledger;
    /// quota follows the key, not the device. Cache invalidation of the
    /// old `(key, old_device)` entry is the caller's job (see 2.4).
    pub fn reassign_to(&mut self, device: DeviceId) {
        self.device_id = device;
    }

    /// Admin "give me my quota back" verb — resets the window to full
    /// daily and stamps `now`.
    pub fn reset_rate_limit(&mut self, now: DateTime<Utc>) {
        self.rate_limit.remaining = self.rate_limit.daily;
        self.rate_limit.last_updated_at = Some(now);
    }

    /// Push the expiry forward (or clear it with `None`).
    pub fn extend_until(&mut self, at: Option<DateTime<Utc>>) {
        self.expired_at = at;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn clock() -> DateTime<Utc> {
        // Fixed "now" so tests stay deterministic and we don't depend
        // on a real clock inside the aggregate.
        DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn active_key_with_quota(daily: i32, remaining: i32) -> IssuedKey {
        IssuedKey {
            id: IssuedKeyId::new(1),
            key: AuthKey::parse("klab_test").unwrap(),
            device_id: DeviceId::parse("dev-1").unwrap(),
            subscription: Some(SubscriptionName::parse("free").unwrap()),
            rate_limit: RateLimitLedger {
                daily: RateLimitAmount::literal(daily),
                remaining: RateLimitAmount::literal(remaining),
                window: RateLimitWindow::daily(),
                last_updated_at: Some(clock() - Duration::minutes(1)),
            },
            expired_at: None,
            revoked_at: None,
            is_free_trial: false,
            username: None,
            email: None,
        }
    }

    #[test]
    fn authorizes_an_active_under_quota_key() {
        let k = active_key_with_quota(100, 50);
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        match d {
            AuthDecision::Allowed { remaining, .. } => assert_eq!(remaining.value(), 49),
            other => panic!("expected Allowed, got {:?}", other),
        }
    }

    #[test]
    fn denies_a_revoked_key_even_with_quota() {
        let mut k = active_key_with_quota(100, 50);
        k.revoke(clock() - Duration::days(1));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::Revoked { .. }));
    }

    #[test]
    fn revoke_is_idempotent() {
        let mut k = active_key_with_quota(100, 50);
        let first = clock() - Duration::days(1);
        let second = clock();
        k.revoke(first);
        k.revoke(second);
        assert_eq!(k.revoked_at, Some(first));
    }

    #[test]
    fn denies_an_expired_admin_key_with_expired_reason() {
        let mut k = active_key_with_quota(100, 50);
        k.expired_at = Some(clock() - Duration::minutes(1));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::Expired { .. }));
    }

    #[test]
    fn denies_an_expired_free_trial_with_its_own_reason() {
        let mut k = active_key_with_quota(100, 50);
        k.is_free_trial = true;
        k.expired_at = Some(clock() - Duration::minutes(1));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::FreeTrialEnded { .. }));
    }

    #[test]
    fn respects_the_legacy_off_by_one_when_remaining_equals_usage() {
        let k = active_key_with_quota(10, 1);
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::RateLimitExceeded));
    }

    #[test]
    fn rejects_usage_greater_than_or_equal_to_daily() {
        let k = active_key_with_quota(10, 10);
        let d = k.authorize(RateLimitUsage::literal(10), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::RateLimitExceeded));
    }

    #[test]
    fn resets_quota_after_window_elapses() {
        let mut k = active_key_with_quota(10, 0);
        // Last update two days ago — daily window has long elapsed.
        k.rate_limit.last_updated_at = Some(clock() - Duration::days(2));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        match d {
            AuthDecision::Allowed { remaining, .. } => assert_eq!(remaining.value(), 9),
            other => panic!("expected reset + Allowed, got {:?}", other),
        }
    }

    #[test]
    fn treats_a_null_last_updated_as_an_elapsed_window() {
        let mut k = active_key_with_quota(10, 0);
        k.rate_limit.last_updated_at = None;
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        assert!(d.is_allowed());
    }

    #[test]
    fn reassign_to_updates_device_without_touching_ledger() {
        let mut k = active_key_with_quota(10, 5);
        let before = k.rate_limit.clone();
        k.reassign_to(DeviceId::parse("dev-2").unwrap());
        assert_eq!(k.device_id.as_str(), "dev-2");
        assert_eq!(k.rate_limit, before);
    }

    #[test]
    fn reset_rate_limit_restores_full_daily_and_stamps_now() {
        let mut k = active_key_with_quota(10, 3);
        k.reset_rate_limit(clock());
        assert_eq!(k.rate_limit.remaining.value(), 10);
        assert_eq!(k.rate_limit.last_updated_at, Some(clock()));
    }

    #[test]
    fn extend_until_sets_and_clears_expiry() {
        let mut k = active_key_with_quota(10, 5);
        let later = clock() + Duration::days(30);
        k.extend_until(Some(later));
        assert_eq!(k.expired_at, Some(later));
        k.extend_until(None);
        assert_eq!(k.expired_at, None);
    }

    fn matches_denial(d: &AuthDecision, ok: impl Fn(&DenialReason) -> bool) {
        match d {
            AuthDecision::Denied { reason } => {
                assert!(ok(reason), "got unexpected reason: {:?}", reason)
            }
            AuthDecision::Allowed { .. } => panic!("expected Denied, got Allowed"),
        }
    }
}
