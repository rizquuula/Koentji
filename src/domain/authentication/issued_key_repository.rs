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
use super::subscription_name::SubscriptionName;

/// Command describing an admin-initiated `IssueKey`. The use case
/// constructs this from the HTTP request, the repository materialises
/// it as a persisted aggregate.
#[derive(Debug, Clone)]
pub struct IssueKeyCommand {
    pub key: AuthKey,
    pub device: DeviceId,
    pub subscription: Option<SubscriptionName>,
    pub subscription_type_id: Option<i32>,
    pub rate_limit_daily: RateLimitAmount,
    pub rate_limit_interval_id: Option<i32>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub expired_at: Option<DateTime<Utc>>,
    pub issued_by: String,
}

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

    /// Admin command — persist a brand-new issued key and return the
    /// resulting aggregate. `(key, device)` must be unique; a unique-
    /// constraint violation surfaces as [`RepositoryError::Backend`].
    async fn issue_key(&self, command: IssueKeyCommand) -> Result<IssuedKey, RepositoryError>;

    /// Admin command — mark the row at `id` as revoked (soft-delete).
    ///
    /// Idempotent: a second call with the same id is a no-op and still
    /// returns `Ok(Some(_))` so the use case can repeat the cache
    /// invalidation. Returns `Ok(None)` when no row with `id` exists.
    /// The `(AuthKey, DeviceId)` in the return tuple lets the use case
    /// evict the cache entry without a second DB round-trip.
    async fn revoke_key(
        &self,
        id: super::issued_key::IssuedKeyId,
        revoked_by: &str,
    ) -> Result<Option<(AuthKey, DeviceId)>, RepositoryError>;

    /// Admin command — move `id`'s device from its current value to
    /// `new_device`. Returns both the previous and the current device
    /// so the use case can evict the *old* `(key, previous_device)`
    /// cache entry (the new one isn't populated until the next auth).
    async fn reassign_device(
        &self,
        id: super::issued_key::IssuedKeyId,
        new_device: &DeviceId,
        updated_by: &str,
    ) -> Result<Option<DeviceReassignment>, RepositoryError>;

    /// Admin command — restore the full daily quota and stamp the
    /// window's `rate_limit_updated_at`.
    async fn reset_rate_limit(
        &self,
        id: super::issued_key::IssuedKeyId,
        now: DateTime<Utc>,
        updated_by: &str,
    ) -> Result<Option<(AuthKey, DeviceId)>, RepositoryError>;

    /// Admin command — set `expired_at` to `new_expiry` (or clear it
    /// with `None`). `updated_by` is stamped for the audit trail.
    async fn extend_expiration(
        &self,
        id: super::issued_key::IssuedKeyId,
        new_expiry: Option<DateTime<Utc>>,
        updated_by: &str,
    ) -> Result<Option<(AuthKey, DeviceId)>, RepositoryError>;
}

/// Outcome of a successful `reassign_device`. The previous device is
/// what the cache is keyed on *before* the move; the current device is
/// what the aggregate now holds. The use case invalidates both, because
/// concurrent reads might still be racing with either key.
#[derive(Debug, Clone)]
pub struct DeviceReassignment {
    pub key: AuthKey,
    pub previous_device: DeviceId,
    pub current_device: DeviceId,
}
