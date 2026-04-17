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
//! needed.

use std::sync::Arc;

use crate::domain::authentication::{
    AuthCachePort, DeviceId, DeviceReassignment, IssuedKeyId, IssuedKeyRepository, RepositoryError,
};

pub struct ReassignDevice {
    repo: Arc<dyn IssuedKeyRepository>,
    cache: Arc<dyn AuthCachePort>,
}

impl ReassignDevice {
    pub fn new(repo: Arc<dyn IssuedKeyRepository>, cache: Arc<dyn AuthCachePort>) -> Self {
        Self { repo, cache }
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
                Ok(Some(reassignment))
            }
            None => Ok(None),
        }
    }
}
