//! `ExtendExpiration` — admin verb to push a key's expiry forward (or
//! clear it with `None`).
//!
//! A cache eviction is mandatory: an in-cache snapshot still carries
//! the old `expired_at`, and the domain's `IssuedKey::authorize`
//! consults it to decide `Denied { Expired }`. Without eviction, a
//! just-extended key can still be denied by the hot path until TTL.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::domain::authentication::{
    AuthCachePort, IssuedKeyId, IssuedKeyRepository, RepositoryError,
};

pub struct ExtendExpiration {
    repo: Arc<dyn IssuedKeyRepository>,
    cache: Arc<dyn AuthCachePort>,
}

impl ExtendExpiration {
    pub fn new(repo: Arc<dyn IssuedKeyRepository>, cache: Arc<dyn AuthCachePort>) -> Self {
        Self { repo, cache }
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
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
