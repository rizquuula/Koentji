//! Integration tests for
//! [`koentji::infrastructure::postgres::PostgresIssuedKeyRepository`].
//!
//! These hit a real Postgres (the shared test DB the harness sets up)
//! — mocking the DB would defeat the point. Every test calls
//! `reset(&pool)` first so cross-test pollution is impossible.
//!
//! Coverage:
//! - `find` hydrates username/email/window_seconds/expired_at/deleted_at.
//! - `consume_quota` atomically decrements and refuses to go negative.
//! - `consume_quota` under concurrency never oversells.
//! - `claim_free_trial` inserts on the FREE_TRIAL marker.
//! - `claim_free_trial` rebinds a pre-issued `device_id = '-'` row.
//! - `claim_free_trial` returns `None` for a plain unknown key.
//! - Admin verbs: `issue_key`, `revoke_key`, `reassign_device`,
//!   `reset_rate_limit`, `extend_expiration` — each on the happy path,
//!   the unknown-id branch, and (where relevant) the idempotent branch.

#![cfg(feature = "ssr")]

mod common;

use chrono::{DateTime, Duration, Utc};
use koentji::domain::authentication::{
    AuthKey, ConsumeOutcome, DeviceId, FreeTrialConfig, IssueKeyCommand, IssuedKeyId,
    IssuedKeyRepository, RateLimitAmount, RateLimitUsage, SubscriptionName,
};
use koentji::infrastructure::postgres::PostgresIssuedKeyRepository;
use std::sync::Arc;

use common::{a_key, fresh_pool};

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn repo(pool: sqlx::PgPool) -> PostgresIssuedKeyRepository {
    PostgresIssuedKeyRepository::new(pool)
}

fn auth_key(s: &str) -> AuthKey {
    AuthKey::parse(s.to_string()).expect("valid test key")
}

fn device(s: &str) -> DeviceId {
    DeviceId::parse(s.to_string()).expect("valid test device")
}

fn usage(n: i32) -> RateLimitUsage {
    RateLimitUsage::new(n).expect("non-negative usage")
}

/// The default `a_key()` insert leaves `rate_limit_updated_at` NULL,
/// which the consume SQL treats as "window has elapsed" → reset. For
/// tests that want the within-window branch, stamp a recent timestamp.
async fn stamp_updated_now(pool: &sqlx::PgPool, id: i32) {
    sqlx::query("UPDATE authentication_keys SET rate_limit_updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .expect("stamp updated_at");
}

#[tokio::test]
async fn find_returns_none_for_an_unknown_key() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());

    let found = r
        .find(&auth_key("no_such_key"), &device("no_such_device"))
        .await
        .expect("find must not error");
    assert!(found.is_none());
}

#[tokio::test]
async fn find_hydrates_identity_and_ledger_fields() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_find_id")
        .with_device("dev-find")
        .with_rate_limit(100)
        .with_remaining(37)
        .with_username("alice")
        .with_email("alice@example.com")
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let snap = r
        .find(&auth_key(&inserted.key), &device(&inserted.device_id))
        .await
        .expect("find must not error")
        .expect("key exists");

    assert_eq!(snap.key.as_str(), "klab_find_id");
    assert_eq!(snap.device_id.as_str(), "dev-find");
    assert_eq!(snap.rate_limit.daily.value(), 100);
    assert_eq!(snap.rate_limit.remaining.value(), 37);
    assert_eq!(snap.username.as_deref(), Some("alice"));
    assert_eq!(snap.email.as_deref(), Some("alice@example.com"));
    assert!(snap.revoked_at.is_none());
}

#[tokio::test]
async fn find_treats_a_revoked_key_as_revoked_not_missing() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_revoked")
        .with_device("dev-revoked")
        .revoked()
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let snap = r
        .find(&auth_key(&inserted.key), &device(&inserted.device_id))
        .await
        .expect("find must not error")
        .expect("revoked row still hydrates");
    assert!(snap.revoked_at.is_some());
}

#[tokio::test]
async fn find_uses_daily_window_when_no_interval_linked() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_default_window")
        .with_device("dev-window")
        .with_interval("daily")
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let snap = r
        .find(&auth_key(&inserted.key), &device(&inserted.device_id))
        .await
        .expect("find must not error")
        .expect("key exists");
    assert_eq!(snap.rate_limit.window.as_seconds(), 86_400);
}

#[tokio::test]
async fn consume_quota_allows_and_decrements_within_quota() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_consume_1")
        .with_device("dev-consume-1")
        .with_rate_limit(100)
        .with_remaining(10)
        .insert(&pool)
        .await;
    stamp_updated_now(&pool, inserted.id).await;

    let r = repo(pool.clone());
    let out = r
        .consume_quota(
            &auth_key(&inserted.key),
            &device(&inserted.device_id),
            usage(1),
            now(),
        )
        .await
        .expect("consume must not error");

    match out {
        ConsumeOutcome::Allowed { remaining, .. } => assert_eq!(remaining.value(), 9),
        ConsumeOutcome::RateLimitExceeded => panic!("expected Allowed"),
    }
}

#[tokio::test]
async fn consume_quota_allows_exactly_at_the_boundary() {
    // remaining == usage is Allowed — predicate is `>=`. The last slot
    // drains to exactly 0, and the next consume is refused.
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_consume_boundary")
        .with_device("dev-boundary")
        .with_rate_limit(10)
        .with_remaining(1)
        .insert(&pool)
        .await;
    stamp_updated_now(&pool, inserted.id).await;

    let r = repo(pool.clone());
    let out = r
        .consume_quota(
            &auth_key(&inserted.key),
            &device(&inserted.device_id),
            usage(1),
            now(),
        )
        .await
        .expect("consume must not error");

    match out {
        ConsumeOutcome::Allowed { remaining, .. } => assert_eq!(remaining.value(), 0),
        ConsumeOutcome::RateLimitExceeded => panic!("expected Allowed at boundary"),
    }

    let next = r
        .consume_quota(
            &auth_key(&inserted.key),
            &device(&inserted.device_id),
            usage(1),
            now(),
        )
        .await
        .expect("consume must not error");
    assert!(matches!(next, ConsumeOutcome::RateLimitExceeded));
}

#[tokio::test]
async fn consume_quota_resets_when_the_window_has_elapsed() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_consume_reset")
        .with_device("dev-reset")
        .with_rate_limit(10)
        .with_remaining(0)
        .insert(&pool)
        .await;

    sqlx::query("UPDATE authentication_keys SET rate_limit_updated_at = $1 WHERE id = $2")
        .bind(Utc::now() - Duration::days(2))
        .bind(inserted.id)
        .execute(&pool)
        .await
        .expect("nudge timestamp");

    let r = repo(pool.clone());
    let out = r
        .consume_quota(
            &auth_key(&inserted.key),
            &device(&inserted.device_id),
            usage(1),
            now(),
        )
        .await
        .expect("consume must not error");

    match out {
        ConsumeOutcome::Allowed { remaining, .. } => assert_eq!(remaining.value(), 9),
        ConsumeOutcome::RateLimitExceeded => panic!("expected reset + Allowed"),
    }
}

#[tokio::test]
async fn consume_quota_never_oversells_under_concurrency() {
    // A classic read-modify-write leak would let 21 spawned consumers
    // race past remaining==20 — the atomic UPDATE with `>=` ensures
    // exactly 20 Allowed and the surplus refused.
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_consume_race")
        .with_device("dev-race")
        .with_rate_limit(20)
        .with_remaining(20)
        .insert(&pool)
        .await;
    stamp_updated_now(&pool, inserted.id).await;
    let r = Arc::new(repo(pool.clone()));

    let mut handles = Vec::new();
    for _ in 0..21 {
        let r = r.clone();
        let k = inserted.key.clone();
        let d = inserted.device_id.clone();
        handles.push(tokio::spawn(async move {
            r.consume_quota(&auth_key(&k), &device(&d), usage(1), now())
                .await
                .expect("consume must not error")
        }));
    }

    let mut allowed = 0;
    for h in handles {
        if matches!(h.await.unwrap(), ConsumeOutcome::Allowed { .. }) {
            allowed += 1;
        }
    }
    // Predicate `>=`: all 20 starting slots are consumable, so
    // exactly 20 attempts win and the 21st is refused.
    assert_eq!(allowed, 20);

    let (final_remaining,): (i32,) =
        sqlx::query_as("SELECT rate_limit_remaining FROM authentication_keys WHERE id = $1")
            .bind(inserted.id)
            .fetch_one(&pool)
            .await
            .expect("read back");
    assert_eq!(final_remaining, 0);
}

#[tokio::test]
async fn consume_quota_denies_when_usage_exceeds_daily() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_consume_usage_gt_daily")
        .with_device("dev-usage-gt-daily")
        .with_rate_limit(5)
        .insert(&pool)
        .await;
    stamp_updated_now(&pool, inserted.id).await;

    let r = repo(pool.clone());
    let out = r
        .consume_quota(
            &auth_key(&inserted.key),
            &device(&inserted.device_id),
            usage(6),
            now(),
        )
        .await
        .expect("consume must not error");

    assert!(matches!(out, ConsumeOutcome::RateLimitExceeded));
}

#[tokio::test]
async fn claim_free_trial_inserts_on_marker_match() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());
    let config = FreeTrialConfig::new("FREE_TRIAL", "free");

    let out = r
        .claim_free_trial(&auth_key("FREE_TRIAL"), &device("dev-new-trial"), &config)
        .await
        .expect("claim must not error")
        .expect("row was created");

    assert_eq!(out.key.as_str(), "FREE_TRIAL");
    assert_eq!(out.device_id.as_str(), "dev-new-trial");
    assert!(out.expired_at.is_some());
    // Free-trial rows carry the "free" subscription by default.
    assert_eq!(out.subscription.as_ref().map(|s| s.as_str()), Some("free"));
    assert!(out.is_free_trial);
}

#[tokio::test]
async fn claim_free_trial_rebinds_a_pre_issued_key() {
    // Admin-issued "pre-bound" row sits with device_id='-'; the first
    // device to call in claims it by rebinding.
    let pool = fresh_pool().await;
    let _ = a_key()
        .with_key("klab_preissued_42")
        .with_device("-")
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let config = FreeTrialConfig::new("FREE_TRIAL", "free");
    let out = r
        .claim_free_trial(
            &auth_key("klab_preissued_42"),
            &device("dev-new-owner"),
            &config,
        )
        .await
        .expect("claim must not error")
        .expect("rebind returns a fresh snapshot");

    assert_eq!(out.device_id.as_str(), "dev-new-owner");

    // Only one row for that key — device_id='-' is gone.
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM authentication_keys WHERE key = $1")
            .bind("klab_preissued_42")
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn claim_free_trial_returns_none_for_an_unknown_non_marker_key() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());
    let config = FreeTrialConfig::new("FREE_TRIAL", "free");

    let out = r
        .claim_free_trial(&auth_key("klab_nope"), &device("dev-nope"), &config)
        .await
        .expect("claim must not error");

    assert!(out.is_none());
}

// ---- Admin verbs (Phase 2.1–2.3) ------------------------------------------

fn issue_command(key: &str, device_s: &str, daily: i32) -> IssueKeyCommand {
    IssueKeyCommand {
        key: auth_key(key),
        device: device(device_s),
        subscription: Some(SubscriptionName::parse("free".to_string()).unwrap()),
        subscription_type_id: None,
        rate_limit_daily: RateLimitAmount::new(daily).expect("positive daily"),
        rate_limit_interval_id: None,
        username: Some("ada".to_string()),
        email: Some("ada@example.com".to_string()),
        expired_at: None,
        issued_by: "test-admin".to_string(),
    }
}

#[tokio::test]
async fn issue_key_inserts_and_returns_aggregate() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());

    let issued = r
        .issue_key(issue_command("klab_issued_1", "dev-issued-1", 100))
        .await
        .expect("issue must not error");

    assert_eq!(issued.key.as_str(), "klab_issued_1");
    assert_eq!(issued.device_id.as_str(), "dev-issued-1");
    assert_eq!(issued.rate_limit.daily.value(), 100);
    assert_eq!(issued.username.as_deref(), Some("ada"));
    assert_eq!(issued.email.as_deref(), Some("ada@example.com"));
    assert!(issued.revoked_at.is_none());
    assert_eq!(
        issued.subscription.as_ref().map(|s| s.as_str()),
        Some("free")
    );
}

#[tokio::test]
async fn issue_key_defaults_remaining_to_daily() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());

    let issued = r
        .issue_key(issue_command("klab_issued_remaining", "dev-remaining", 250))
        .await
        .expect("issue must not error");

    // Freshly issued rows start the window with full quota.
    assert_eq!(issued.rate_limit.remaining.value(), 250);
    assert_eq!(issued.rate_limit.daily.value(), 250);
}

#[tokio::test]
async fn revoke_key_soft_deletes_and_returns_key_device() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_revoke_me")
        .with_device("dev-revoke-me")
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let out = r
        .revoke_key(IssuedKeyId::new(inserted.id), "test-admin")
        .await
        .expect("revoke must not error")
        .expect("row matched");

    assert_eq!(out.0.as_str(), "klab_revoke_me");
    assert_eq!(out.1.as_str(), "dev-revoke-me");

    let snap = r
        .find(&auth_key("klab_revoke_me"), &device("dev-revoke-me"))
        .await
        .expect("find must not error")
        .expect("row exists (soft-deleted)");
    assert!(snap.revoked_at.is_some());
}

#[tokio::test]
async fn revoke_key_is_idempotent_and_preserves_original_timestamp() {
    // Calling revoke twice must not bump `deleted_at` — a second admin
    // click during a slow network round-trip shouldn't rewrite history.
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_idempotent")
        .with_device("dev-idempotent")
        .insert(&pool)
        .await;

    let r = repo(pool.clone());

    r.revoke_key(IssuedKeyId::new(inserted.id), "admin-1")
        .await
        .expect("first revoke")
        .expect("row matched");
    let first_deleted_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT deleted_at FROM authentication_keys WHERE id = $1")
            .bind(inserted.id)
            .fetch_one(&pool)
            .await
            .expect("snapshot first timestamp");
    let first = first_deleted_at.expect("first revoke stamped deleted_at");

    // Second call still returns Some — the caller can re-evict its cache.
    let out = r
        .revoke_key(IssuedKeyId::new(inserted.id), "admin-2")
        .await
        .expect("second revoke")
        .expect("row still matches");
    assert_eq!(out.0.as_str(), "klab_idempotent");

    let second_deleted_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT deleted_at FROM authentication_keys WHERE id = $1")
            .bind(inserted.id)
            .fetch_one(&pool)
            .await
            .expect("snapshot second timestamp");
    assert_eq!(
        second_deleted_at.expect("still revoked"),
        first,
        "deleted_at must be preserved across idempotent revokes",
    );
}

#[tokio::test]
async fn revoke_key_returns_none_for_an_unknown_id() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());

    let out = r
        .revoke_key(IssuedKeyId::new(9_999_999), "test-admin")
        .await
        .expect("revoke must not error");
    assert!(out.is_none());
}

#[tokio::test]
async fn reassign_device_returns_previous_and_current_devices() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_reassign")
        .with_device("dev-before")
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let out = r
        .reassign_device(
            IssuedKeyId::new(inserted.id),
            &device("dev-after"),
            "test-admin",
        )
        .await
        .expect("reassign must not error")
        .expect("row matched");

    assert_eq!(out.key.as_str(), "klab_reassign");
    assert_eq!(out.previous_device.as_str(), "dev-before");
    assert_eq!(out.current_device.as_str(), "dev-after");

    // The moved row really did move.
    let (device_row,): (String,) =
        sqlx::query_as("SELECT device_id FROM authentication_keys WHERE id = $1")
            .bind(inserted.id)
            .fetch_one(&pool)
            .await
            .expect("row exists");
    assert_eq!(device_row, "dev-after");
}

#[tokio::test]
async fn reassign_device_returns_none_for_an_unknown_id() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());

    let out = r
        .reassign_device(
            IssuedKeyId::new(9_999_999),
            &device("dev-ghost"),
            "test-admin",
        )
        .await
        .expect("reassign must not error");
    assert!(out.is_none());
}

#[tokio::test]
async fn reset_rate_limit_restores_daily_and_stamps_updated_at() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_reset")
        .with_device("dev-reset-admin")
        .with_rate_limit(500)
        .with_remaining(3)
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let reset_at = now();
    let out = r
        .reset_rate_limit(IssuedKeyId::new(inserted.id), reset_at, "test-admin")
        .await
        .expect("reset must not error")
        .expect("row matched");

    assert_eq!(out.0.as_str(), "klab_reset");
    assert_eq!(out.1.as_str(), "dev-reset-admin");

    let (remaining, updated_at): (i32, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT rate_limit_remaining, rate_limit_updated_at FROM authentication_keys WHERE id = $1",
    )
    .bind(inserted.id)
    .fetch_one(&pool)
    .await
    .expect("row exists");
    assert_eq!(remaining, 500);
    let stamped = updated_at.expect("updated_at stamped");
    assert!(
        (stamped - reset_at).num_seconds().abs() < 2,
        "updated_at should be close to the reset moment",
    );
}

#[tokio::test]
async fn reset_rate_limit_returns_none_for_an_unknown_id() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());

    let out = r
        .reset_rate_limit(IssuedKeyId::new(9_999_999), now(), "test-admin")
        .await
        .expect("reset must not error");
    assert!(out.is_none());
}

#[tokio::test]
async fn extend_expiration_sets_and_clears_expiry() {
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_extend")
        .with_device("dev-extend")
        .insert(&pool)
        .await;

    let r = repo(pool.clone());
    let new_expiry = Utc::now() + Duration::days(30);

    let out = r
        .extend_expiration(
            IssuedKeyId::new(inserted.id),
            Some(new_expiry),
            "test-admin",
        )
        .await
        .expect("extend must not error")
        .expect("row matched");
    assert_eq!(out.0.as_str(), "klab_extend");

    let (expired_at,): (Option<DateTime<Utc>>,) =
        sqlx::query_as("SELECT expired_at FROM authentication_keys WHERE id = $1")
            .bind(inserted.id)
            .fetch_one(&pool)
            .await
            .expect("row exists");
    let set = expired_at.expect("expired_at is now set");
    assert!(
        (set - new_expiry).num_seconds().abs() < 2,
        "expired_at should match the requested value",
    );

    // Clearing with None should null it back out.
    r.extend_expiration(IssuedKeyId::new(inserted.id), None, "test-admin")
        .await
        .expect("clear must not error")
        .expect("row matched");
    let (cleared,): (Option<DateTime<Utc>>,) =
        sqlx::query_as("SELECT expired_at FROM authentication_keys WHERE id = $1")
            .bind(inserted.id)
            .fetch_one(&pool)
            .await
            .expect("row exists");
    assert!(cleared.is_none(), "clearing expiry should null the column");
}

#[tokio::test]
async fn extend_expiration_returns_none_for_an_unknown_id() {
    let pool = fresh_pool().await;
    let r = repo(pool.clone());

    let out = r
        .extend_expiration(IssuedKeyId::new(9_999_999), Some(Utc::now()), "test-admin")
        .await
        .expect("extend must not error");
    assert!(out.is_none());
}
