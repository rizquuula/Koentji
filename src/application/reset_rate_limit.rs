//! `ResetRateLimit` — admin "give the quota back" verb.
//!
//! Restores `remaining` to `daily` and stamps `rate_limit_updated_at`
//! so the window starts fresh. Any cached snapshot is evicted — the
//! next `/v1/auth` call re-reads the row and repopulates — and the
//! `RateLimitReset` event is appended to the audit trail (Phase 3.4).

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::domain::authentication::{
    AuditEventPort, AuthCachePort, DomainEvent, IssuedKeyId, IssuedKeyRepository, RepositoryError,
};

pub struct ResetRateLimit {
    repo: Arc<dyn IssuedKeyRepository>,
    cache: Arc<dyn AuthCachePort>,
    audit: Arc<dyn AuditEventPort>,
}

impl ResetRateLimit {
    pub fn new(
        repo: Arc<dyn IssuedKeyRepository>,
        cache: Arc<dyn AuthCachePort>,
        audit: Arc<dyn AuditEventPort>,
    ) -> Self {
        Self { repo, cache, audit }
    }

    /// Returns `Ok(true)` when a row matched and was reset, `Ok(false)`
    /// when no row with `id` exists.
    pub async fn execute(
        &self,
        id: IssuedKeyId,
        now: DateTime<Utc>,
        updated_by: &str,
    ) -> Result<bool, RepositoryError> {
        match self.repo.reset_rate_limit(id, now, updated_by).await? {
            Some((key, device)) => {
                self.cache.invalidate(&key, &device).await;
                self.audit
                    .publish(DomainEvent::RateLimitReset {
                        aggregate_id: id.value(),
                        device: device.as_str().to_string(),
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
