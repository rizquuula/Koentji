//! `RevokeKey` — admin-side soft-delete of an issued key.
//!
//! One of the Phase 2 admin verbs. The old `delete_key` server fn did
//! three things inline: run the `UPDATE authentication_keys SET
//! deleted_at = NOW()` statement, fetch the `(key, device_id)` pair by
//! id, and poke the global `OnceLock` auth cache to invalidate that
//! entry. The three are now split by responsibility:
//!
//! - the repository owns the SQL (`IssuedKeyRepository::revoke_key`);
//! - the auth cache port owns the eviction
//!   (`AuthCachePort::invalidate`);
//! - this use case composes the two, so the server fn is a pure adapter
//!   (parse `id`, delegate, render).
//!
//! Idempotent: calling `execute` on an already-revoked id re-evicts the
//! cache and returns `Ok(())`. Unknown ids come back as `Ok(false)` so
//! the server fn can decide whether to 404 or shrug (the admin UI
//! currently shrugs).
//!
//! Emits a past-tense `KeyRevoked` log line — the outbox/audit adapter
//! lands in 3.4.

use std::sync::Arc;

use crate::domain::authentication::{
    AuthCachePort, IssuedKeyId, IssuedKeyRepository, RepositoryError,
};

pub struct RevokeKey {
    repo: Arc<dyn IssuedKeyRepository>,
    cache: Arc<dyn AuthCachePort>,
}

impl RevokeKey {
    pub fn new(repo: Arc<dyn IssuedKeyRepository>, cache: Arc<dyn AuthCachePort>) -> Self {
        Self { repo, cache }
    }

    /// Returns `Ok(true)` when a row with `id` existed (and is now
    /// revoked + cache-evicted), `Ok(false)` when no row matched.
    pub async fn execute(
        &self,
        id: IssuedKeyId,
        revoked_by: &str,
    ) -> Result<bool, RepositoryError> {
        match self.repo.revoke_key(id, revoked_by).await? {
            Some((key, device)) => {
                self.cache.invalidate(&key, &device).await;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
