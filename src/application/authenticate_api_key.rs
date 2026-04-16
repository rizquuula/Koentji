//! `AuthenticateApiKey` — the `/v1/auth` use case.
//!
//! Orchestrates the flow the inline handler in `src/main.rs` used to
//! do procedurally:
//!
//! 1. Cache lookup → if hit, skip the DB read.
//! 2. Miss → repository find. If still missing, try the free-trial /
//!    device-binding branch.
//! 3. `IssuedKey::authorize` — pure decision against the snapshot.
//! 4. On `Allowed`, call `consume_quota` (atomic SQL) to handle
//!    concurrent consumers.
//! 5. On success, refresh the cache with the post-decrement snapshot
//!    so the next call is a cache hit.
//!
//! The use case takes trait-object ports (`Arc<dyn …>`) so tests can
//! swap in a stub and the HTTP layer can hand out a single concrete
//! handler without threading generics through every type signature.
//! The outward-facing [`AuthOutcome`] is consumed by
//! `src/interface/http/auth_endpoint.rs` to render the same envelope
//! the legacy handler produced.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::domain::authentication::{
    AuthCachePort, AuthDecision, AuthKey, ConsumeOutcome, DenialReason, DeviceId, FreeTrialConfig,
    IssuedKey, IssuedKeyRepository, RateLimitAmount, RateLimitUsage,
};

/// The terminal outcome the HTTP adapter translates to a status +
/// envelope.
#[derive(Debug, Clone)]
pub enum AuthOutcome {
    /// Quota consumed. The returned `IssuedKey` reflects the
    /// post-decrement state so the envelope can echo the identity
    /// fields (`username`, `email`, `subscription`, `valid_until`).
    Success {
        key: IssuedKey,
        remaining: RateLimitAmount,
    },
    /// A typed denial. The status code and bilingual text are chosen
    /// at the HTTP edge via `interface::http::i18n`.
    Denied { reason: DenialReason },
    /// A backend failure (SQLx error, pool timeout). The adapter
    /// returns a generic 500 — we do not leak the diagnostic to the
    /// client.
    BackendError,
}

pub struct AuthenticateApiKey {
    repo: Arc<dyn IssuedKeyRepository>,
    cache: Arc<dyn AuthCachePort>,
    free_trial: FreeTrialConfig,
}

impl AuthenticateApiKey {
    pub fn new(
        repo: Arc<dyn IssuedKeyRepository>,
        cache: Arc<dyn AuthCachePort>,
        free_trial: FreeTrialConfig,
    ) -> Self {
        Self {
            repo,
            cache,
            free_trial,
        }
    }

    pub async fn execute(
        &self,
        key: AuthKey,
        device: DeviceId,
        usage: RateLimitUsage,
        now: DateTime<Utc>,
    ) -> AuthOutcome {
        let snapshot = match self.resolve_snapshot(&key, &device).await {
            ResolveOutcome::Snapshot(s) => s,
            ResolveOutcome::Unknown => {
                return AuthOutcome::Denied {
                    reason: DenialReason::UnknownKey,
                }
            }
            ResolveOutcome::Backend => return AuthOutcome::BackendError,
        };

        let decision = snapshot.authorize(usage, now);
        match decision {
            AuthDecision::Denied { reason } => AuthOutcome::Denied { reason },
            AuthDecision::Allowed { .. } => self.consume(snapshot, &key, &device, usage, now).await,
        }
    }

    async fn resolve_snapshot(&self, key: &AuthKey, device: &DeviceId) -> ResolveOutcome {
        if let Some(cached) = self.cache.get(key, device).await {
            return ResolveOutcome::Snapshot(cached);
        }

        match self.repo.find(key, device).await {
            Err(e) => {
                log::error!(
                    "issued_key.find failed for device={}: {}",
                    device.as_str(),
                    e
                );
                ResolveOutcome::Backend
            }
            Ok(Some(snapshot)) => {
                self.cache.put(snapshot.clone()).await;
                ResolveOutcome::Snapshot(snapshot)
            }
            Ok(None) => match self
                .repo
                .claim_free_trial(key, device, &self.free_trial)
                .await
            {
                Err(e) => {
                    log::error!(
                        "claim_free_trial failed for device={}: {}",
                        device.as_str(),
                        e
                    );
                    ResolveOutcome::Backend
                }
                Ok(Some(snapshot)) => {
                    self.cache.put(snapshot.clone()).await;
                    ResolveOutcome::Snapshot(snapshot)
                }
                Ok(None) => ResolveOutcome::Unknown,
            },
        }
    }

    async fn consume(
        &self,
        snapshot: IssuedKey,
        key: &AuthKey,
        device: &DeviceId,
        usage: RateLimitUsage,
        now: DateTime<Utc>,
    ) -> AuthOutcome {
        match self.repo.consume_quota(key, device, usage, now).await {
            Err(e) => {
                log::error!("consume_quota failed for device={}: {}", device.as_str(), e);
                AuthOutcome::BackendError
            }
            Ok(ConsumeOutcome::RateLimitExceeded) => AuthOutcome::Denied {
                reason: DenialReason::RateLimitExceeded,
            },
            Ok(ConsumeOutcome::Allowed {
                remaining,
                updated_at,
            }) => {
                let mut updated = snapshot;
                updated.rate_limit.remaining = remaining;
                updated.rate_limit.last_updated_at = Some(updated_at);
                self.cache.put(updated.clone()).await;
                AuthOutcome::Success {
                    key: updated,
                    remaining,
                }
            }
        }
    }
}

enum ResolveOutcome {
    Snapshot(IssuedKey),
    Unknown,
    Backend,
}
