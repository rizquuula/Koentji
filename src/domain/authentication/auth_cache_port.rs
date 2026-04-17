//! The domain port for the auth lookup cache.
//!
//! The hot path on `/v1/auth` is a read — most requests hit the same
//! `(AuthKey, DeviceId)` within the TTL window. The cache keeps the
//! aggregate and its joined interval so we skip the DB lookup. The
//! port exists so the domain (and the use case once 1.6 lands) can
//! talk about "cache the decision", "invalidate on reassignment",
//! etc. without knowing Moka exists.
//!
//! Semantics:
//!
//! - [`AuthCachePort::get`] returns the cached aggregate snapshot if
//!   there is one, otherwise `None`. TTL is the adapter's problem.
//! - [`AuthCachePort::put`] overwrites the entry — callers hand in an
//!   already-reconciled `IssuedKey` (post-consume `remaining`, fresh
//!   `last_updated_at`).
//! - [`AuthCachePort::invalidate`] evicts a single `(key, device)`
//!   tuple. Device reassignment (2.4) will call this twice: once for
//!   the new pair, once for the old.
//!
//! The port is async because every production adapter worth its salt
//! will be. `async-trait` is gated on `ssr` so WASM builds do not
//! depend on it.

#[cfg(feature = "ssr")]
use super::auth_key::AuthKey;
#[cfg(feature = "ssr")]
use super::device_id::DeviceId;
#[cfg(feature = "ssr")]
use super::issued_key::IssuedKey;

#[cfg(feature = "ssr")]
#[async_trait::async_trait]
pub trait AuthCachePort: Send + Sync {
    async fn get(&self, key: &AuthKey, device: &DeviceId) -> Option<IssuedKey>;
    async fn put(&self, snapshot: IssuedKey);
    async fn invalidate(&self, key: &AuthKey, device: &DeviceId);
}
