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

    // --- Priority-of-denial-reasons coverage --------------------------------
    //
    // The legacy handler checked `revoked > expired > rate_limit`. A key
    // that is revoked and expired must still report `Revoked`, because
    // the admin action is the overriding fact. These tests pin that
    // ordering so a refactor cannot silently flip it.

    #[test]
    fn revoked_beats_expired_and_rate_limit() {
        let mut k = active_key_with_quota(10, 0);
        k.revoke(clock() - Duration::days(1));
        k.expired_at = Some(clock() - Duration::minutes(1));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::Revoked { .. }));
    }

    #[test]
    fn expired_beats_rate_limit_when_not_revoked() {
        let mut k = active_key_with_quota(10, 0);
        k.expired_at = Some(clock() - Duration::minutes(1));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::Expired { .. }));
    }

    #[test]
    fn free_trial_ended_takes_priority_over_rate_limit() {
        let mut k = active_key_with_quota(10, 0);
        k.is_free_trial = true;
        k.expired_at = Some(clock() - Duration::minutes(1));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::FreeTrialEnded { .. }));
    }

    // --- Clock-boundary coverage -------------------------------------------

    #[test]
    fn denies_at_the_exact_expiry_instant() {
        // Legacy predicate is `expired_at <= now`, so `==` denies.
        let mut k = active_key_with_quota(10, 5);
        k.expired_at = Some(clock());
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::Expired { .. }));
    }

    #[test]
    fn allows_one_nanosecond_before_expiry() {
        let mut k = active_key_with_quota(10, 5);
        k.expired_at = Some(clock() + Duration::nanoseconds(1));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        assert!(d.is_allowed());
    }

    #[test]
    fn window_reset_fires_exactly_at_the_window_boundary() {
        // `(now - last) >= window` — at `==` we reset.
        let mut k = active_key_with_quota(10, 0);
        k.rate_limit.last_updated_at = Some(clock() - Duration::seconds(86_400));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        assert!(d.is_allowed());
    }

    #[test]
    fn window_reset_does_not_fire_one_nanosecond_early() {
        let mut k = active_key_with_quota(10, 0);
        k.rate_limit.last_updated_at =
            Some(clock() - Duration::seconds(86_400) + Duration::nanoseconds(1));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::RateLimitExceeded));
    }

    // --- Ledger arithmetic edge cases --------------------------------------

    #[test]
    fn allows_when_remaining_is_exactly_one_more_than_usage() {
        let k = active_key_with_quota(10, 2);
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        match d {
            AuthDecision::Allowed { remaining, .. } => assert_eq!(remaining.value(), 1),
            other => panic!("expected Allowed, got {:?}", other),
        }
    }

    #[test]
    fn denies_when_remaining_is_one_less_than_usage() {
        let k = active_key_with_quota(10, 3);
        let d = k.authorize(RateLimitUsage::literal(4), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::RateLimitExceeded));
    }

    #[test]
    fn large_usage_equal_to_daily_is_denied() {
        let k = active_key_with_quota(6_000, 6_000);
        let d = k.authorize(RateLimitUsage::literal(6_000), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::RateLimitExceeded));
    }

    #[test]
    fn large_usage_just_under_daily_is_allowed_after_window_reset() {
        let mut k = active_key_with_quota(6_000, 0);
        k.rate_limit.last_updated_at = Some(clock() - Duration::days(2));
        let d = k.authorize(RateLimitUsage::literal(5_999), clock());
        match d {
            AuthDecision::Allowed { remaining, .. } => assert_eq!(remaining.value(), 1),
            other => panic!("expected Allowed, got {:?}", other),
        }
    }

    #[test]
    fn zero_usage_is_allowed_and_does_not_decrement() {
        // Usage==0 is an unusual-but-valid client request (refresh the
        // ledger without consuming). The legacy SQL predicate
        // `daily > 0` still holds, and `remaining > 0` or a window reset
        // should allow it.
        let k = active_key_with_quota(10, 5);
        let d = k.authorize(RateLimitUsage::literal(0), clock());
        match d {
            AuthDecision::Allowed { remaining, .. } => assert_eq!(remaining.value(), 5),
            other => panic!("expected Allowed, got {:?}", other),
        }
    }

    // --- Non-default window coverage ---------------------------------------

    #[test]
    fn respects_a_custom_sixty_second_window() {
        let mut k = active_key_with_quota(10, 0);
        k.rate_limit.window = RateLimitWindow::from_seconds(60).unwrap();
        k.rate_limit.last_updated_at = Some(clock() - Duration::seconds(59));
        // Still inside the window — remaining==0 denies.
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::RateLimitExceeded));

        // Advance past the window — reset fires.
        k.rate_limit.last_updated_at = Some(clock() - Duration::seconds(61));
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        assert!(d.is_allowed());
    }

    // --- Purity --------------------------------------------------------------

    #[test]
    fn authorize_does_not_mutate_the_aggregate() {
        let k = active_key_with_quota(10, 3);
        let before = k.clone();
        let _ = k.authorize(RateLimitUsage::literal(1), clock());
        let _ = k.authorize(RateLimitUsage::literal(1), clock());
        assert_eq!(k, before);
    }

    // --- Updated-at stamp ---------------------------------------------------

    #[test]
    fn allowed_decisions_stamp_updated_at_with_now() {
        let k = active_key_with_quota(10, 5);
        let at = clock();
        match k.authorize(RateLimitUsage::literal(1), at) {
            AuthDecision::Allowed { updated_at, .. } => assert_eq!(updated_at, at),
            other => panic!("expected Allowed, got {:?}", other),
        }
    }

    // --- DenialReason carries the original timestamps -----------------------

    #[test]
    fn revoked_denial_carries_the_revocation_timestamp() {
        let mut k = active_key_with_quota(10, 5);
        let revoked_at = clock() - Duration::hours(3);
        k.revoke(revoked_at);
        match k.authorize(RateLimitUsage::literal(1), clock()) {
            AuthDecision::Denied {
                reason: DenialReason::Revoked { at },
            } => assert_eq!(at, revoked_at),
            other => panic!("expected Revoked, got {:?}", other),
        }
    }

    #[test]
    fn expired_denial_carries_the_expiry_timestamp() {
        let mut k = active_key_with_quota(10, 5);
        let expires_at = clock() - Duration::minutes(5);
        k.expired_at = Some(expires_at);
        match k.authorize(RateLimitUsage::literal(1), clock()) {
            AuthDecision::Denied {
                reason: DenialReason::Expired { at },
            } => assert_eq!(at, expires_at),
            other => panic!("expected Expired, got {:?}", other),
        }
    }

    #[test]
    fn free_trial_denial_carries_the_expiry_timestamp() {
        let mut k = active_key_with_quota(10, 5);
        k.is_free_trial = true;
        let expires_at = clock() - Duration::minutes(5);
        k.expired_at = Some(expires_at);
        match k.authorize(RateLimitUsage::literal(1), clock()) {
            AuthDecision::Denied {
                reason: DenialReason::FreeTrialEnded { at },
            } => assert_eq!(at, expires_at),
            other => panic!("expected FreeTrialEnded, got {:?}", other),
        }
    }

    // --- Lifecycle verbs: extra coverage ------------------------------------

    #[test]
    fn reassign_to_does_not_resurrect_revocation() {
        let mut k = active_key_with_quota(10, 5);
        k.revoke(clock() - Duration::days(1));
        k.reassign_to(DeviceId::parse("dev-2").unwrap());
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::Revoked { .. }));
    }

    #[test]
    fn reset_rate_limit_does_not_revive_a_revoked_key() {
        let mut k = active_key_with_quota(10, 0);
        k.revoke(clock() - Duration::days(1));
        k.reset_rate_limit(clock());
        let d = k.authorize(RateLimitUsage::literal(1), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::Revoked { .. }));
    }

    #[test]
    fn extend_until_can_push_expiry_into_the_future_to_re_allow() {
        let mut k = active_key_with_quota(10, 5);
        k.expired_at = Some(clock() - Duration::minutes(1));
        // Confirm it's currently denied.
        assert!(!k
            .authorize(RateLimitUsage::literal(1), clock())
            .is_allowed());
        k.extend_until(Some(clock() + Duration::days(30)));
        assert!(k
            .authorize(RateLimitUsage::literal(1), clock())
            .is_allowed());
    }

    #[test]
    fn extend_until_none_leaves_an_endless_key() {
        let mut k = active_key_with_quota(10, 5);
        k.expired_at = Some(clock() + Duration::days(30));
        k.extend_until(None);
        assert!(k
            .authorize(RateLimitUsage::literal(1), clock())
            .is_allowed());
    }

    // --- Active-key happy path: full-quota consume after reset --------------

    #[test]
    fn full_daily_consume_on_a_fresh_window_leaves_one_remaining() {
        // Legacy off-by-one: `daily - 1` is the max consumable value per
        // window.
        let mut k = active_key_with_quota(10, 0);
        k.rate_limit.last_updated_at = Some(clock() - Duration::days(2));
        let d = k.authorize(RateLimitUsage::literal(9), clock());
        match d {
            AuthDecision::Allowed { remaining, .. } => assert_eq!(remaining.value(), 1),
            other => panic!("expected Allowed, got {:?}", other),
        }
    }

    #[test]
    fn daily_consume_on_a_fresh_window_is_still_denied_by_legacy_off_by_one() {
        let mut k = active_key_with_quota(10, 0);
        k.rate_limit.last_updated_at = Some(clock() - Duration::days(2));
        let d = k.authorize(RateLimitUsage::literal(10), clock());
        matches_denial(&d, |r| matches!(r, DenialReason::RateLimitExceeded));
    }
}
