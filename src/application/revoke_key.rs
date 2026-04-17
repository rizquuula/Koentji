//! `RevokeKey` — admin-side soft-delete of an issued key.
//!
//! Three responsibilities, split by layer:
//!
//! - the repository owns the SQL (`IssuedKeyRepository::revoke_key`);
//! - the auth cache port owns the eviction
//!   (`AuthCachePort::invalidate`);
//! - the audit port durably records the event
//!   (`AuditEventPort::publish`, Phase 3.4).
//!
//! This use case composes the three, so the server fn is a pure adapter
//! (parse `id`, delegate, render).
//!
//! Idempotent: calling `execute` on an already-revoked id re-evicts the
//! cache and returns `Ok(())`. Unknown ids come back as `Ok(false)` so
//! the server fn can decide whether to 404 or shrug (the admin UI
//! currently shrugs). No audit event is emitted for unknown ids — the
//! domain only records facts that actually changed state.

use std::sync::Arc;

use chrono::Utc;

use crate::domain::authentication::{
    AuditEventPort, AuthCachePort, DomainEvent, IssuedKeyId, IssuedKeyRepository, RepositoryError,
};

pub struct RevokeKey {
    repo: Arc<dyn IssuedKeyRepository>,
    cache: Arc<dyn AuthCachePort>,
    audit: Arc<dyn AuditEventPort>,
}

impl RevokeKey {
    pub fn new(
        repo: Arc<dyn IssuedKeyRepository>,
        cache: Arc<dyn AuthCachePort>,
        audit: Arc<dyn AuditEventPort>,
    ) -> Self {
        Self { repo, cache, audit }
    }

    /// Returns `Ok(true)` when a row with `id` existed (and is now
    /// revoked + cache-evicted + audited), `Ok(false)` when no row
    /// matched.
    pub async fn execute(
        &self,
        id: IssuedKeyId,
        revoked_by: &str,
    ) -> Result<bool, RepositoryError> {
        match self.repo.revoke_key(id, revoked_by).await? {
            Some((key, device)) => {
                self.cache.invalidate(&key, &device).await;
                self.audit
                    .publish(DomainEvent::KeyRevoked {
                        aggregate_id: id.value(),
                        device: device.as_str().to_string(),
                        actor: revoked_by.to_string(),
                        occurred_at: Utc::now(),
                    })
                    .await;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
