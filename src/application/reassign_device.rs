//! `ReassignDevice` — admin verb to move a key from one device to
//! another.
//!
//! On a successful reassignment, both the *previous* and the *current*
//! `(key, device)` cache entries are evicted:
//!
//! - previous — because the cache snapshot under the old device still
//!   claims this key belongs to it; that entry is now stale and must
//!   go or the hot path keeps handing out stale auth.
//! - current — defensive: something may have populated it between the
//!   UPDATE and here (unlikely but cheap to cover).
//!
//! The repository's atomic `reassign_device` returns
//! [`DeviceReassignment`] with both devices, so no second DB lookup is
//! needed. After eviction, a `DeviceReassigned` domain event is
//! published through [`AuditEventPort`] (Phase 3.4).

use std::sync::Arc;

use chrono::Utc;

use crate::domain::authentication::{
    AuditEventPort, AuthCachePort, DeviceId, DeviceReassignment, DomainEvent, IssuedKeyId,
    IssuedKeyRepository, RepositoryError,
};

pub struct ReassignDevice {
    repo: Arc<dyn IssuedKeyRepository>,
    cache: Arc<dyn AuthCachePort>,
    audit: Arc<dyn AuditEventPort>,
}

impl ReassignDevice {
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
        new_device: DeviceId,
        updated_by: &str,
    ) -> Result<Option<DeviceReassignment>, RepositoryError> {
        match self
            .repo
            .reassign_device(id, &new_device, updated_by)
            .await?
        {
            Some(reassignment) => {
                self.cache
                    .invalidate(&reassignment.key, &reassignment.previous_device)
                    .await;
                self.cache
                    .invalidate(&reassignment.key, &reassignment.current_device)
                    .await;
                self.audit
                    .publish(DomainEvent::DeviceReassigned {
                        aggregate_id: id.value(),
                        previous_device: reassignment.previous_device.as_str().to_string(),
                        current_device: reassignment.current_device.as_str().to_string(),
                        actor: updated_by.to_string(),
                        occurred_at: Utc::now(),
                    })
                    .await;
                Ok(Some(reassignment))
            }
            None => Ok(None),
        }
    }
}
