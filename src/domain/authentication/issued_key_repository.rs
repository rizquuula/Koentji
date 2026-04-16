//! The persistence port for the `IssuedKey` aggregate.
//!
//! The trait lives in the domain because the domain owns the contract
//! ("this is what the application expects a store to provide"). The
//! Postgres implementation lives under `infrastructure::postgres` and
//! is wired at startup.
//!
//! Keeping the port async-first lets us hand the adapter to spawned
//! workers without runtime gymnastics. `async-trait` is behind the
//! `ssr` feature flag so the WASM build does not pull it in.
//!
//! What the port exposes, deliberately narrow:
//!
//! - [`IssuedKeyRepository::find`] — snapshot read of
//!   `(AuthKey, DeviceId)`; returns `None` when nothing exists so the
//!   use case can route into the free-trial branch or return
//!   `DenialReason::UnknownKey`.
//! - [`IssuedKeyRepository::consume_quota`] — atomic decrement + window
//!   reset in a single SQL round-trip. Callers must have already
//!   short-circuited revoked / expired / unknown in the domain layer
//!   using `IssuedKey::authorize`; this call is only for the allowed
//!   branch where the race with other writers needs to be serialised
//!   by Postgres.
//!
//! Writes that admin commands need (issue, revoke, reassign, reset
//! rate-limit, extend expiry) arrive in Phase 2 as additional methods.

use chrono::{DateTime, Utc};

use super::auth_key::AuthKey;
use super::device_id::DeviceId;
use super::issued_key::IssuedKey;
use super::rate_limit::{RateLimitAmount, RateLimitUsage};

/// Outcome of an atomic quota consume. Mirrors the shape of
/// [`super::auth_decision::AuthDecision`] but only covers the
/// rate-limit axis — the use case composes this with the pure
/// revoked/expired checks already performed upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumeOutcome {
    /// The row was updated under a row lock. `remaining` is the
    /// post-decrement value the caller may echo back to clients /
    /// cache.
    Allowed {
        remaining: RateLimitAmount,
        updated_at: DateTime<Utc>,
    },
    /// The predicate did not match: the window is still open and
    /// `remaining <= usage`, or `daily <= usage`. No row was written.
    RateLimitExceeded,
}

/// Configuration for the free-trial claim. The marker is the public
/// magic string clients send as the auth key (default `FREE_TRIAL`),
/// and `subscription_name` is the subscription-type row we look up to
/// copy quota + interval from (default `free`).
#[derive(Debug, Clone)]
pub struct FreeTrialConfig {
    pub marker: String,
    pub subscription_name: String,
}

impl FreeTrialConfig {
    pub fn new(marker: impl Into<String>, subscription_name: impl Into<String>) -> Self {
        Self {
            marker: marker.into(),
            subscription_name: subscription_name.into(),
        }
    }
}

/// Repository errors the domain cares about. We do not leak SQLx
/// directly; the Postgres adapter collapses its errors into this
/// small enum so the application layer can pattern-match without
/// depending on `sqlx`.
#[derive(Debug)]
pub enum RepositoryError {
    /// Any transport / driver / constraint failure. The display string
    /// is the diagnostic; it never ends up in a client response.
    Backend(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryError::Backend(msg) => write!(f, "repository backend error: {}", msg),
        }
    }
}

impl std::error::Error for RepositoryError {}

#[cfg(feature = "ssr")]
#[async_trait::async_trait]
pub trait IssuedKeyRepository: Send + Sync {
    async fn find(
        &self,
        key: &AuthKey,
        device: &DeviceId,
    ) -> Result<Option<IssuedKey>, RepositoryError>;

    async fn consume_quota(
        &self,
        key: &AuthKey,
        device: &DeviceId,
        usage: RateLimitUsage,
        now: DateTime<Utc>,
    ) -> Result<ConsumeOutcome, RepositoryError>;

    /// Two-way door the legacy handler exposed at the miss-in-DB
    /// boundary:
    ///
    /// - if `key` equals `config.marker`, issue a fresh free-trial
    ///   row for `device` (subscription + quota copied from
    ///   `config.subscription_name`, expiring on the 1st of next
    ///   month in UTC);
    /// - otherwise if a row exists with the same key and the
    ///   unclaimed-device sentinel (`"-"`), rebind its `device_id`
    ///   to the requested one.
    ///
    /// Returns `Ok(None)` when neither branch fires — the use case
    /// treats that as `DenialReason::UnknownKey`.
    async fn claim_free_trial(
        &self,
        key: &AuthKey,
        device: &DeviceId,
        config: &FreeTrialConfig,
    ) -> Result<Option<IssuedKey>, RepositoryError>;
}
