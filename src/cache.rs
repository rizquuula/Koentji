use crate::models::AuthenticationKey;
use chrono::{DateTime, Utc};
use moka::future::Cache;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct CachedAuthEntry {
    pub key_data: AuthenticationKey,
    pub interval_seconds: i64,
    pub cached_at: DateTime<Utc>,
}

pub struct AuthCache {
    cache: Cache<(String, String), CachedAuthEntry>,
}

impl AuthCache {
    pub fn new(ttl_seconds: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();
        Self { cache }
    }

    pub async fn get(&self, auth_key: &str, device_id: &str) -> Option<CachedAuthEntry> {
        self.cache
            .get(&(auth_key.to_string(), device_id.to_string()))
            .await
    }

    pub async fn insert(&self, auth_key: &str, device_id: &str, entry: CachedAuthEntry) {
        self.cache
            .insert((auth_key.to_string(), device_id.to_string()), entry)
            .await;
    }

    pub async fn invalidate(&self, auth_key: &str, device_id: &str) {
        self.cache
            .invalidate(&(auth_key.to_string(), device_id.to_string()))
            .await;
    }

}
