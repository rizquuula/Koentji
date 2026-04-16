//! Moka-backed [`AuthCachePort`] implementation.
//!
//! Stores an `IssuedKey` aggregate keyed by `(AuthKey, DeviceId)`.
//! TTL is set at construction (matches the `AUTH_CACHE_TTL_SECONDS`
//! env var in `main.rs`). The legacy `AuthCache` in `src/cache.rs`
//! still exists for the current `/v1/auth` handler; 1.6 swaps the
//! handler over to this adapter and deletes the legacy type.

use std::time::Duration;

use moka::future::Cache;

use crate::domain::authentication::{AuthCachePort, AuthKey, DeviceId, IssuedKey};

type CacheKey = (String, String);

pub struct MokaAuthCache {
    cache: Cache<CacheKey, IssuedKey>,
}

impl MokaAuthCache {
    pub fn new(ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();
        Self { cache }
    }

    fn cache_key(key: &AuthKey, device: &DeviceId) -> CacheKey {
        (key.as_str().to_string(), device.as_str().to_string())
    }
}

#[async_trait::async_trait]
impl AuthCachePort for MokaAuthCache {
    async fn get(&self, key: &AuthKey, device: &DeviceId) -> Option<IssuedKey> {
        self.cache.get(&Self::cache_key(key, device)).await
    }

    async fn put(&self, snapshot: IssuedKey) {
        let k = Self::cache_key(&snapshot.key, &snapshot.device_id);
        self.cache.insert(k, snapshot).await;
    }

    async fn invalidate(&self, key: &AuthKey, device: &DeviceId) {
        self.cache.invalidate(&Self::cache_key(key, device)).await;
    }
}
