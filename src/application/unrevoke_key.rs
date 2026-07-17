//! `UnrevokeKey` — admin-side reversal of a soft-delete.
//!
//! The mirror image of [`super::revoke_key::RevokeKey`]. Three
//! responsibilities, split by layer:
//!
//! - the repository owns the SQL (`IssuedKeyRepository::unrevoke_key`);
//! - the auth cache port owns the eviction
//!   (`AuthCachePort::invalidate`) — a cached *revoked* snapshot must be
//!   dropped so the next auth re-reads the now-active row;
//! - the audit port durably records the event
//!   (`AuditEventPort::publish`).
//!
//! Idempotent: calling `execute` on an already-active id re-evicts the
//! cache and returns `Ok(true)`. Unknown ids come back as `Ok(false)`
//! so the server fn can decide whether to 404 or shrug (the admin UI
//! currently shrugs). No audit event is emitted for unknown ids — the
//! domain only records facts that actually changed state.

use std::sync::Arc;

use chrono::Utc;

use crate::domain::authentication::{
    AuditEventPort, AuthCachePort, DomainEvent, IssuedKeyId, IssuedKeyRepository, RepositoryError,
};

pub struct UnrevokeKey {
    repo: Arc<dyn IssuedKeyRepository>,
    cache: Arc<dyn AuthCachePort>,
    audit: Arc<dyn AuditEventPort>,
}

impl UnrevokeKey {
    pub fn new(
        repo: Arc<dyn IssuedKeyRepository>,
        cache: Arc<dyn AuthCachePort>,
        audit: Arc<dyn AuditEventPort>,
    ) -> Self {
        Self { repo, cache, audit }
    }

    /// Returns `Ok(true)` when a row with `id` existed (and is now
    /// active + cache-evicted + audited), `Ok(false)` when no row
    /// matched.
    pub async fn execute(
        &self,
        id: IssuedKeyId,
        unrevoked_by: &str,
    ) -> Result<bool, RepositoryError> {
        match self.repo.unrevoke_key(id, unrevoked_by).await? {
            Some((key, device)) => {
                self.cache.invalidate(&key, &device).await;
                self.audit
                    .publish(DomainEvent::KeyUnrevoked {
                        aggregate_id: id.value(),
                        device: device.as_str().to_string(),
                        actor: unrevoked_by.to_string(),
                        occurred_at: Utc::now(),
                    })
                    .await;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
