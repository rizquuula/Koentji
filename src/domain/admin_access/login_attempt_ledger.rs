//! In-process per-IP sliding-window lockout for admin login.
//!
//! Every failed admin login appends one timestamp to a per-IP deque.
//! If the IP has accumulated the configured number of failures inside
//! the configured window, further attempts are locked out until the
//! oldest failure in the window ages out. A successful login clears
//! the IP's deque immediately.
//!
//! Single-replica by design. The audit in the remediation plan calls
//! out distributed rate-limiting (Redis) as an explicit out-of-scope
//! item — this ledger is enough for the single-admin, single-process
//! dashboard and restarts reset the state (acceptable: an attacker
//! who can crash and restart our own process has bigger problems).
//!
//! All methods take `now: DateTime<Utc>` rather than reading the
//! wall clock so tests can pin the window's edges with a fake clock.
//! The login path passes `Utc::now()`.

#![cfg(feature = "ssr")]

use chrono::{DateTime, Duration, Utc};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

/// Configuration for the sliding-window policy. Chosen to match the
/// remediation plan's "5 attempts / 5 min" default but exposed as
/// fields so tests can collapse the window to milliseconds.
#[derive(Debug, Clone, Copy)]
pub struct LockoutPolicy {
    pub max_failures: u32,
    pub window: Duration,
}

impl LockoutPolicy {
    pub const fn new(max_failures: u32, window: Duration) -> Self {
        Self {
            max_failures,
            window,
        }
    }

    pub fn default_admin() -> Self {
        Self::new(5, Duration::minutes(5))
    }
}

/// Outcome of a `check_and_record_attempt`: either the attempt is
/// permitted (caller proceeds to verify credentials) or the IP is
/// locked and the caller must refuse without evaluating the password.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptDecision {
    Allowed,
    LockedOut { retry_after: Duration },
}

pub struct LoginAttemptLedger {
    policy: LockoutPolicy,
    state: Mutex<HashMap<String, VecDeque<DateTime<Utc>>>>,
}

impl LoginAttemptLedger {
    pub fn new(policy: LockoutPolicy) -> Self {
        Self {
            policy,
            state: Mutex::new(HashMap::new()),
        }
    }

    /// Is the given IP currently locked out? Returns `Allowed` when
    /// fewer than `max_failures` failures are inside the rolling
    /// window, otherwise `LockedOut { retry_after }`. Does NOT record
    /// a new failure — callers call this to gate the password verify.
    pub fn check(&self, ip: &str, now: DateTime<Utc>) -> AttemptDecision {
        let mut state = self.state.lock().expect("ledger mutex poisoned");
        let entry = state.entry(ip.to_string()).or_default();
        prune(entry, now, self.policy.window);
        self.decide(entry, now)
    }

    /// Record a failure for this IP. Returns the decision *after* the
    /// new failure has been counted — so if this was the 5th failure
    /// inside the window, the caller sees `LockedOut` and can report
    /// the lockout back to the client in the same response.
    pub fn record_failure(&self, ip: &str, now: DateTime<Utc>) -> AttemptDecision {
        let mut state = self.state.lock().expect("ledger mutex poisoned");
        let entry = state.entry(ip.to_string()).or_default();
        prune(entry, now, self.policy.window);
        entry.push_back(now);
        self.decide(entry, now)
    }

    /// Clear the IP's failure history. Called on a successful login so
    /// an admin who eventually types the right password doesn't stay
    /// locked out for the rest of the window.
    pub fn clear(&self, ip: &str) {
        let mut state = self.state.lock().expect("ledger mutex poisoned");
        state.remove(ip);
    }

    fn decide(&self, entry: &VecDeque<DateTime<Utc>>, now: DateTime<Utc>) -> AttemptDecision {
        if entry.len() < self.policy.max_failures as usize {
            return AttemptDecision::Allowed;
        }
        // Oldest failure in the window decides when the lockout ends.
        // `prune` has already dropped anything older than the window
        // so `front()` is the anchor.
        let oldest = entry.front().copied().unwrap_or(now);
        let release = oldest + self.policy.window;
        let retry_after = (release - now).max(Duration::zero());
        AttemptDecision::LockedOut { retry_after }
    }
}

fn prune(entry: &mut VecDeque<DateTime<Utc>>, now: DateTime<Utc>, window: Duration) {
    let cutoff = now - window;
    while entry.front().map(|t| *t <= cutoff).unwrap_or(false) {
        entry.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms_policy(max: u32, window_ms: i64) -> LockoutPolicy {
        LockoutPolicy::new(max, Duration::milliseconds(window_ms))
    }

    fn t(ms: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp_millis(1_000_000 + ms).unwrap()
    }

    #[test]
    fn first_failure_does_not_lock_out() {
        let ledger = LoginAttemptLedger::new(ms_policy(5, 5_000));
        assert_eq!(
            ledger.record_failure("1.2.3.4", t(0)),
            AttemptDecision::Allowed
        );
    }

    #[test]
    fn hitting_max_failures_inside_window_locks_out() {
        let ledger = LoginAttemptLedger::new(ms_policy(3, 10_000));
        assert_eq!(
            ledger.record_failure("1.2.3.4", t(0)),
            AttemptDecision::Allowed
        );
        assert_eq!(
            ledger.record_failure("1.2.3.4", t(1)),
            AttemptDecision::Allowed
        );
        match ledger.record_failure("1.2.3.4", t(2)) {
            AttemptDecision::LockedOut { retry_after } => {
                // Oldest failure at t(0); window = 10s; now = t(2ms);
                // release = t(10_000ms); retry_after ≈ 9998ms.
                assert!(retry_after.num_milliseconds() > 9_000);
                assert!(retry_after.num_milliseconds() <= 10_000);
            }
            other => panic!("expected lockout, got {other:?}"),
        }
    }

    #[test]
    fn check_without_recording_reports_status_without_tick() {
        let ledger = LoginAttemptLedger::new(ms_policy(2, 10_000));
        ledger.record_failure("1.2.3.4", t(0));
        ledger.record_failure("1.2.3.4", t(1));
        // `check` must NOT add a new timestamp — if it did, a pure
        // observation would move the lockout window.
        assert!(matches!(
            ledger.check("1.2.3.4", t(2)),
            AttemptDecision::LockedOut { .. }
        ));
        assert!(matches!(
            ledger.check("1.2.3.4", t(3)),
            AttemptDecision::LockedOut { .. }
        ));
    }

    #[test]
    fn lockout_releases_after_window_ages_out() {
        let ledger = LoginAttemptLedger::new(ms_policy(2, 1_000));
        ledger.record_failure("1.2.3.4", t(0));
        ledger.record_failure("1.2.3.4", t(500));
        // At t(500): 2 failures inside window → locked.
        assert!(matches!(
            ledger.check("1.2.3.4", t(500)),
            AttemptDecision::LockedOut { .. }
        ));
        // At t(1_001): the t(0) failure has aged out (> 1s ago), so
        // only t(500) remains — 1 < max, allowed.
        assert_eq!(ledger.check("1.2.3.4", t(1_001)), AttemptDecision::Allowed);
    }

    #[test]
    fn successful_login_clears_the_ip() {
        let ledger = LoginAttemptLedger::new(ms_policy(3, 10_000));
        ledger.record_failure("1.2.3.4", t(0));
        ledger.record_failure("1.2.3.4", t(1));
        ledger.clear("1.2.3.4");
        // After clear, a single failure is still Allowed — the window
        // is fresh.
        assert_eq!(
            ledger.record_failure("1.2.3.4", t(2)),
            AttemptDecision::Allowed
        );
    }

    #[test]
    fn per_ip_lockout_is_independent() {
        let ledger = LoginAttemptLedger::new(ms_policy(2, 10_000));
        ledger.record_failure("1.2.3.4", t(0));
        ledger.record_failure("1.2.3.4", t(1));
        // 5.6.7.8 has no history — should still be allowed even though
        // 1.2.3.4 is locked.
        assert_eq!(ledger.check("5.6.7.8", t(2)), AttemptDecision::Allowed);
        assert!(matches!(
            ledger.check("1.2.3.4", t(2)),
            AttemptDecision::LockedOut { .. }
        ));
    }

    #[test]
    fn retry_after_shrinks_as_time_passes() {
        let ledger = LoginAttemptLedger::new(ms_policy(2, 10_000));
        ledger.record_failure("1.2.3.4", t(0));
        ledger.record_failure("1.2.3.4", t(1));
        let AttemptDecision::LockedOut { retry_after: early } = ledger.check("1.2.3.4", t(100))
        else {
            panic!("expected lockout");
        };
        let AttemptDecision::LockedOut { retry_after: late } = ledger.check("1.2.3.4", t(5_000))
        else {
            panic!("expected lockout");
        };
        assert!(late < early);
    }

    #[test]
    fn failures_exactly_at_window_edge_age_out() {
        // An attempt whose age is EQUAL to the window duration is
        // considered aged-out. The policy is "within the last N", and
        // "within" is open at the far edge — otherwise an attacker who
        // waits exactly `window` always sees a fresh slot.
        let ledger = LoginAttemptLedger::new(ms_policy(2, 1_000));
        ledger.record_failure("1.2.3.4", t(0));
        ledger.record_failure("1.2.3.4", t(500));
        // At t(1_000): t(0) is exactly `window` old → pruned. One
        // failure remains → allowed.
        assert_eq!(ledger.check("1.2.3.4", t(1_000)), AttemptDecision::Allowed);
    }

    #[test]
    fn record_failure_after_lockout_keeps_caller_informed() {
        // A bot that keeps hammering after the lockout triggers still
        // receives `LockedOut` with each attempt — and each new
        // timestamp extends the window. That's a feature, not a bug.
        let ledger = LoginAttemptLedger::new(ms_policy(2, 10_000));
        ledger.record_failure("1.2.3.4", t(0));
        ledger.record_failure("1.2.3.4", t(1));
        assert!(matches!(
            ledger.record_failure("1.2.3.4", t(2)),
            AttemptDecision::LockedOut { .. }
        ));
        assert!(matches!(
            ledger.record_failure("1.2.3.4", t(3)),
            AttemptDecision::LockedOut { .. }
        ));
    }
}
