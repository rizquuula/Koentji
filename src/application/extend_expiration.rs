//! `ExtendExpiration` — admin verb to push a key's expiry forward (or
//! clear it with `None`).
//!
//! A cache eviction is mandatory: an in-cache snapshot still carries
//! the old `expired_at`, and the domain's `IssuedKey::authorize`
//! consults it to decide `Denied { Expired }`. Without eviction, a
//! just-extended key can still be denied by the hot path until TTL.
//!
//! After eviction, a `KeyExpirationExtended` event — including the new
//! expiry (or `None` if cleared) — is appended to the audit trail
//! (Phase 3.4).

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::domain::authentication::{
    AuditEventPort, AuthCachePort, DomainEvent, IssuedKeyId, IssuedKeyRepository, RepositoryError,
};

pub struct ExtendExpiration {
    repo: Arc<dyn IssuedKeyRepository>,
    cache: Arc<dyn AuthCachePort>,
    audit: Arc<dyn AuditEventPort>,
}

impl ExtendExpiration {
    pub fn new(
        repo: Arc<dyn IssuedKeyRepository>,
        cache: Arc<dyn AuthCachePort>,
        audit: Arc<dyn AuditEventPort>,
    ) -> Self {
        Self { repo, cache, audit }
    }

    pub async fn execute(
        &self,
        id: IssuedKeyId,
        new_expiry: Option<DateTime<Utc>>,
        updated_by: &str,
    ) -> Result<bool, RepositoryError> {
        match self
            .repo
            .extend_expiration(id, new_expiry, updated_by)
            .await?
        {
            Some((key, device)) => {
                self.cache.invalidate(&key, &device).await;
                self.audit
                    .publish(DomainEvent::KeyExpirationExtended {
                        aggregate_id: id.value(),
                        device: device.as_str().to_string(),
                        new_expiry,
                        actor: updated_by.to_string(),
                        occurred_at: Utc::now(),
                    })
                    .await;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
