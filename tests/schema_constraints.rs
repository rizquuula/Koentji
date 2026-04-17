//! Tests that pin invariants enforced by the schema itself — not the
//! repository code. Migration 004 adds a composite unique index on
//! `(key, device_id)`; these tests prove Postgres rejects duplicates
//! even when a caller bypasses the repository layer.
//!
//! Running these next to the adapter tests keeps the whole schema
//! story (rows, indexes, constraints) in one test crate.

#![cfg(feature = "ssr")]

mod common;

use common::{a_key, fresh_pool};

#[tokio::test]
async fn insert_rejects_a_duplicate_key_device_pair() {
    // Migration 004 enforces one row per (key, device_id). Two admin
    // clicks racing on issue should not produce two rows.
    let pool = fresh_pool().await;
    let inserted = a_key()
        .with_key("klab_uniq_probe")
        .with_device("dev-uniq-probe")
        .insert(&pool)
        .await;

    let err = sqlx::query(
        "INSERT INTO authentication_keys
            (key, device_id, rate_limit_daily, rate_limit_remaining, created_by)
         VALUES ($1, $2, 6000, 6000, 'test')",
    )
    .bind(&inserted.key)
    .bind(&inserted.device_id)
    .execute(&pool)
    .await
    .expect_err("duplicate (key, device_id) must be rejected");

    // sqlx wraps the unique-violation; the message must mention our
    // index so a future migration renaming it fails this test loudly.
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("idx_auth_keys_key_device_unique"),
        "expected unique-index violation, got: {msg}",
    );
}

#[tokio::test]
async fn insert_permits_the_same_key_with_different_devices() {
    // FREE_TRIAL (and pre-issued keys) legitimately share one `key`
    // across many `device_id` values. Migration 003 dropped the old
    // UNIQUE(key); migration 004 must not re-introduce it.
    let pool = fresh_pool().await;
    a_key()
        .with_key("klab_shared_key")
        .with_device("dev-a")
        .insert(&pool)
        .await;

    // The second insert for the same key with a different device must
    // succeed — KeyBuilder::insert panics on failure, which is what we
    // want here.
    a_key()
        .with_key("klab_shared_key")
        .with_device("dev-b")
        .insert(&pool)
        .await;

    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM authentication_keys WHERE key = $1")
            .bind("klab_shared_key")
            .fetch_one(&pool)
            .await
            .expect("count rows");
    assert_eq!(count, 2);
}
