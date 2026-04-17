//! Orchestration tests for the Phase 2 admin verbs.
//!
//! These isolate the use-case layer (`IssueKey`, `RevokeKey`,
//! `ReassignDevice`, `ResetRateLimit`, `ExtendExpiration`) from the DB
//! and the Moka adapter via in-memory fakes. The goal is to pin the
//! composition contract between the repository port and the cache port
//! — every verb that mutates state must invalidate the cache, device
//! reassignment must evict *both* entries (B9), and unknown ids must
//! short-circuit without touching the cache at all.
//!
//! The integration tests in `postgres_issued_key_repository.rs` cover
//! the SQL; these tests cover the coordination.

#![cfg(feature = "ssr")]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use koentji::application::{ExtendExpiration, IssueKey, ReassignDevice, ResetRateLimit, RevokeKey};
use koentji::domain::authentication::{
    AuthCachePort, AuthKey, ConsumeOutcome, DeviceId, DeviceReassignment, FreeTrialConfig,
    IssueKeyCommand, IssuedKey, IssuedKeyId, IssuedKeyRepository, RateLimitAmount, RateLimitLedger,
    RateLimitUsage, RateLimitWindow, RepositoryError, SubscriptionName,
};
use std::sync::Arc;
use tokio::sync::Mutex;

// ---- Fakes ----------------------------------------------------------------

#[derive(Default)]
struct FakeRepo {
    calls: Mutex<Vec<RepoCall>>,
    // Configured outcomes for the admin verbs — tests set these up front.
    revoke: Mutex<Option<Option<(AuthKey, DeviceId)>>>,
    reassign: Mutex<Option<Option<DeviceReassignment>>>,
    reset: Mutex<Option<Option<(AuthKey, DeviceId)>>>,
    extend: Mutex<Option<Option<(AuthKey, DeviceId)>>>,
    issue: Mutex<Option<Result<IssuedKey, RepositoryError>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RepoCall {
    Issue {
        issued_by: String,
        device: String,
    },
    Revoke {
        id: i32,
        by: String,
    },
    Reassign {
        id: i32,
        new_device: String,
        by: String,
    },
    Reset {
        id: i32,
        by: String,
    },
    Extend {
        id: i32,
        expiry: Option<DateTime<Utc>>,
        by: String,
    },
}

#[async_trait]
impl IssuedKeyRepository for FakeRepo {
    async fn find(
        &self,
        _key: &AuthKey,
        _device: &DeviceId,
    ) -> Result<Option<IssuedKey>, RepositoryError> {
        Ok(None)
    }

    async fn consume_quota(
        &self,
        _key: &AuthKey,
        _device: &DeviceId,
        _usage: RateLimitUsage,
        _now: DateTime<Utc>,
    ) -> Result<ConsumeOutcome, RepositoryError> {
        Ok(ConsumeOutcome::RateLimitExceeded)
    }

    async fn claim_free_trial(
        &self,
        _key: &AuthKey,
        _device: &DeviceId,
        _config: &FreeTrialConfig,
    ) -> Result<Option<IssuedKey>, RepositoryError> {
        Ok(None)
    }

    async fn issue_key(&self, command: IssueKeyCommand) -> Result<IssuedKey, RepositoryError> {
        self.calls.lock().await.push(RepoCall::Issue {
            issued_by: command.issued_by.clone(),
            device: command.device.as_str().to_string(),
        });
        self.issue
            .lock()
            .await
            .take()
            .unwrap_or_else(|| Ok(sample_issued_key("klab_fake_issue", "fake-dev")))
    }

    async fn revoke_key(
        &self,
        id: IssuedKeyId,
        revoked_by: &str,
    ) -> Result<Option<(AuthKey, DeviceId)>, RepositoryError> {
        self.calls.lock().await.push(RepoCall::Revoke {
            id: id.value(),
            by: revoked_by.to_string(),
        });
        Ok(self.revoke.lock().await.take().unwrap_or(None))
    }

    async fn reassign_device(
        &self,
        id: IssuedKeyId,
        new_device: &DeviceId,
        updated_by: &str,
    ) -> Result<Option<DeviceReassignment>, RepositoryError> {
        self.calls.lock().await.push(RepoCall::Reassign {
            id: id.value(),
            new_device: new_device.as_str().to_string(),
            by: updated_by.to_string(),
        });
        Ok(self.reassign.lock().await.take().unwrap_or(None))
    }

    async fn reset_rate_limit(
        &self,
        id: IssuedKeyId,
        _now: DateTime<Utc>,
        updated_by: &str,
    ) -> Result<Option<(AuthKey, DeviceId)>, RepositoryError> {
        self.calls.lock().await.push(RepoCall::Reset {
            id: id.value(),
            by: updated_by.to_string(),
        });
        Ok(self.reset.lock().await.take().unwrap_or(None))
    }

    async fn extend_expiration(
        &self,
        id: IssuedKeyId,
        new_expiry: Option<DateTime<Utc>>,
        updated_by: &str,
    ) -> Result<Option<(AuthKey, DeviceId)>, RepositoryError> {
        self.calls.lock().await.push(RepoCall::Extend {
            id: id.value(),
            expiry: new_expiry,
            by: updated_by.to_string(),
        });
        Ok(self.extend.lock().await.take().unwrap_or(None))
    }
}

#[derive(Default)]
struct FakeCache {
    invalidations: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl AuthCachePort for FakeCache {
    async fn get(&self, _key: &AuthKey, _device: &DeviceId) -> Option<IssuedKey> {
        None
    }

    async fn put(&self, _snapshot: IssuedKey) {}

    async fn invalidate(&self, key: &AuthKey, device: &DeviceId) {
        self.invalidations
            .lock()
            .await
            .push((key.as_str().to_string(), device.as_str().to_string()));
    }
}

// ---- Helpers --------------------------------------------------------------

fn auth_key(s: &str) -> AuthKey {
    AuthKey::parse(s.to_string()).expect("valid key")
}

fn device(s: &str) -> DeviceId {
    DeviceId::parse(s.to_string()).expect("valid device")
}

fn sample_issued_key(key: &str, device_s: &str) -> IssuedKey {
    IssuedKey {
        id: IssuedKeyId::new(42),
        key: auth_key(key),
        device_id: device(device_s),
        subscription: Some(SubscriptionName::parse("free".to_string()).unwrap()),
        rate_limit: RateLimitLedger {
            daily: RateLimitAmount::new(6000).unwrap(),
            remaining: RateLimitAmount::new(6000).unwrap(),
            window: RateLimitWindow::from_seconds(86_400).unwrap(),
            last_updated_at: None,
        },
        expired_at: None,
        revoked_at: None,
        is_free_trial: false,
        username: Some("ada".to_string()),
        email: Some("ada@example.com".to_string()),
    }
}

fn sample_command(key: &str, device_s: &str) -> IssueKeyCommand {
    IssueKeyCommand {
        key: auth_key(key),
        device: device(device_s),
        subscription: Some(SubscriptionName::parse("free".to_string()).unwrap()),
        subscription_type_id: None,
        rate_limit_daily: RateLimitAmount::new(6000).unwrap(),
        rate_limit_interval_id: None,
        username: None,
        email: None,
        expired_at: None,
        issued_by: "test-admin".to_string(),
    }
}

// ---- IssueKey -------------------------------------------------------------

#[tokio::test]
async fn issue_key_delegates_to_repository_without_cache_interaction() {
    let repo = Arc::new(FakeRepo::default());
    let issue = IssueKey::new(repo.clone());

    let result = issue
        .execute(sample_command("klab_new", "dev-new"))
        .await
        .expect("issue ok");

    assert_eq!(result.key.as_str(), "klab_fake_issue");
    let calls = repo.calls.lock().await;
    assert_eq!(calls.len(), 1);
    assert!(matches!(
        calls.first(),
        Some(RepoCall::Issue { issued_by, device }) if issued_by == "test-admin" && device == "dev-new"
    ));
}

// ---- RevokeKey ------------------------------------------------------------

#[tokio::test]
async fn revoke_key_invalidates_cache_on_success() {
    let repo = Arc::new(FakeRepo::default());
    *repo.revoke.lock().await = Some(Some((auth_key("klab_revoked"), device("dev-revoked"))));
    let cache = Arc::new(FakeCache::default());
    let revoke = RevokeKey::new(repo.clone(), cache.clone());

    let ok = revoke
        .execute(IssuedKeyId::new(7), "test-admin")
        .await
        .expect("revoke ok");

    assert!(ok, "known id reports true");
    let evictions = cache.invalidations.lock().await;
    assert_eq!(evictions.len(), 1);
    assert_eq!(
        evictions.first().unwrap(),
        &("klab_revoked".to_string(), "dev-revoked".to_string())
    );
}

#[tokio::test]
async fn revoke_key_does_not_touch_cache_on_unknown_id() {
    let repo = Arc::new(FakeRepo::default());
    // Default: revoke returns None.
    let cache = Arc::new(FakeCache::default());
    let revoke = RevokeKey::new(repo.clone(), cache.clone());

    let ok = revoke
        .execute(IssuedKeyId::new(9_999), "test-admin")
        .await
        .expect("revoke ok");

    assert!(!ok, "unknown id reports false");
    assert!(
        cache.invalidations.lock().await.is_empty(),
        "no cache eviction when nothing was revoked",
    );
}

// ---- ReassignDevice -------------------------------------------------------

#[tokio::test]
async fn reassign_device_invalidates_previous_and_current_entries() {
    // This is the B9 guard: the old cache snapshot under `(key, old_dev)`
    // must be evicted, and defensively the new pair too.
    let repo = Arc::new(FakeRepo::default());
    *repo.reassign.lock().await = Some(Some(DeviceReassignment {
        key: auth_key("klab_move"),
        previous_device: device("dev-old"),
        current_device: device("dev-new"),
    }));
    let cache = Arc::new(FakeCache::default());
    let reassign = ReassignDevice::new(repo.clone(), cache.clone());

    let result = reassign
        .execute(IssuedKeyId::new(3), device("dev-new"), "test-admin")
        .await
        .expect("reassign ok")
        .expect("some result");

    assert_eq!(result.previous_device.as_str(), "dev-old");
    assert_eq!(result.current_device.as_str(), "dev-new");

    let evictions = cache.invalidations.lock().await;
    assert_eq!(evictions.len(), 2, "both entries must be evicted");
    assert_eq!(
        evictions[0],
        ("klab_move".to_string(), "dev-old".to_string()),
        "previous entry evicted first",
    );
    assert_eq!(
        evictions[1],
        ("klab_move".to_string(), "dev-new".to_string()),
        "current entry evicted second",
    );
}

#[tokio::test]
async fn reassign_device_does_not_touch_cache_on_unknown_id() {
    let repo = Arc::new(FakeRepo::default());
    let cache = Arc::new(FakeCache::default());
    let reassign = ReassignDevice::new(repo.clone(), cache.clone());

    let out = reassign
        .execute(IssuedKeyId::new(9_999), device("dev-ghost"), "test-admin")
        .await
        .expect("reassign ok");

    assert!(out.is_none());
    assert!(cache.invalidations.lock().await.is_empty());
}

// ---- ResetRateLimit -------------------------------------------------------

#[tokio::test]
async fn reset_rate_limit_invalidates_cache_on_success() {
    let repo = Arc::new(FakeRepo::default());
    *repo.reset.lock().await = Some(Some((auth_key("klab_reset"), device("dev-reset"))));
    let cache = Arc::new(FakeCache::default());
    let reset = ResetRateLimit::new(repo.clone(), cache.clone());

    let ok = reset
        .execute(IssuedKeyId::new(11), Utc::now(), "test-admin")
        .await
        .expect("reset ok");

    assert!(ok);
    let evictions = cache.invalidations.lock().await;
    assert_eq!(evictions.len(), 1);
    assert_eq!(
        evictions.first().unwrap(),
        &("klab_reset".to_string(), "dev-reset".to_string())
    );
}

#[tokio::test]
async fn reset_rate_limit_does_not_touch_cache_on_unknown_id() {
    let repo = Arc::new(FakeRepo::default());
    let cache = Arc::new(FakeCache::default());
    let reset = ResetRateLimit::new(repo.clone(), cache.clone());

    let ok = reset
        .execute(IssuedKeyId::new(9_999), Utc::now(), "test-admin")
        .await
        .expect("reset ok");

    assert!(!ok);
    assert!(cache.invalidations.lock().await.is_empty());
}

// ---- ExtendExpiration -----------------------------------------------------

#[tokio::test]
async fn extend_expiration_invalidates_cache_on_set_and_on_clear() {
    // A pushed-out expiry and a cleared expiry both stale the cache
    // snapshot — without eviction, `IssuedKey::authorize` could still
    // deny a just-extended key until TTL.
    let cache = Arc::new(FakeCache::default());

    // Set branch.
    let repo_set = Arc::new(FakeRepo::default());
    *repo_set.extend.lock().await = Some(Some((auth_key("klab_ext"), device("dev-ext"))));
    let extend_set = ExtendExpiration::new(repo_set.clone(), cache.clone());
    let ok = extend_set
        .execute(
            IssuedKeyId::new(5),
            Some(Utc::now() + chrono::Duration::days(30)),
            "test-admin",
        )
        .await
        .expect("extend ok");
    assert!(ok);

    // Clear branch (None).
    let repo_clear = Arc::new(FakeRepo::default());
    *repo_clear.extend.lock().await = Some(Some((auth_key("klab_ext"), device("dev-ext"))));
    let extend_clear = ExtendExpiration::new(repo_clear.clone(), cache.clone());
    let ok = extend_clear
        .execute(IssuedKeyId::new(5), None, "test-admin")
        .await
        .expect("extend ok");
    assert!(ok);

    let evictions = cache.invalidations.lock().await;
    assert_eq!(
        evictions.len(),
        2,
        "both the set-new-expiry and clear-expiry branches evict"
    );
    assert_eq!(
        evictions[0],
        ("klab_ext".to_string(), "dev-ext".to_string())
    );
    assert_eq!(
        evictions[1],
        ("klab_ext".to_string(), "dev-ext".to_string())
    );
}

#[tokio::test]
async fn extend_expiration_does_not_touch_cache_on_unknown_id() {
    let repo = Arc::new(FakeRepo::default());
    let cache = Arc::new(FakeCache::default());
    let extend = ExtendExpiration::new(repo.clone(), cache.clone());

    let ok = extend
        .execute(IssuedKeyId::new(9_999), Some(Utc::now()), "test-admin")
        .await
        .expect("extend ok");

    assert!(!ok);
    assert!(cache.invalidations.lock().await.is_empty());
}
