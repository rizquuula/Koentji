//! Integration tests for [`PostgresAuditEventRepository`].
//!
//! Proves two things: (1) every `DomainEvent` variant survives the
//! round-trip through the JSONB payload and comes back out with the
//! right shape, and (2) the adapter's fire-and-forget contract holds —
//! a write failure does not panic or return an error to the caller.
//!
//! These tests share the process-wide test DB and serialize via
//! `--test-threads=1` (see Makefile).

#![cfg(feature = "ssr")]

mod common;

use chrono::{DateTime, TimeZone, Utc};
use koentji::domain::authentication::{AuditEventPort, DomainEvent};
use koentji::infrastructure::postgres::PostgresAuditEventRepository;
use serde_json::Value;
use sqlx::Row;

use common::db::fresh_pool;

fn at(offset: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000 + offset, 0).unwrap()
}

#[tokio::test]
async fn key_issued_event_is_persisted_with_payload_fields() {
    let pool = fresh_pool().await;
    let adapter = PostgresAuditEventRepository::new(pool.clone());

    adapter
        .publish(DomainEvent::KeyIssued {
            aggregate_id: 42,
            device: "dev-1".to_string(),
            subscription: Some("premium".to_string()),
            actor: "admin".to_string(),
            occurred_at: at(0),
        })
        .await;

    let row = sqlx::query(
        "SELECT event_type, aggregate_id, actor, payload, occurred_at FROM audit_log WHERE aggregate_id = 42",
    )
    .fetch_one(&pool)
    .await
    .expect("row present");

    let event_type: String = row.get("event_type");
    let aggregate_id: i32 = row.get("aggregate_id");
    let actor: String = row.get("actor");
    let payload: Value = row.get("payload");

    assert_eq!(event_type, "KeyIssued");
    assert_eq!(aggregate_id, 42);
    assert_eq!(actor, "admin");
    assert_eq!(payload["device"], "dev-1");
    assert_eq!(payload["subscription"], "premium");
}

#[tokio::test]
async fn key_revoked_event_payload_carries_the_device() {
    let pool = fresh_pool().await;
    let adapter = PostgresAuditEventRepository::new(pool.clone());

    adapter
        .publish(DomainEvent::KeyRevoked {
            aggregate_id: 7,
            device: "dev-rev".to_string(),
            actor: "op".to_string(),
            occurred_at: at(100),
        })
        .await;

    let row = sqlx::query("SELECT event_type, payload FROM audit_log WHERE aggregate_id = 7")
        .fetch_one(&pool)
        .await
        .expect("row present");

    let event_type: String = row.get("event_type");
    let payload: Value = row.get("payload");
    assert_eq!(event_type, "KeyRevoked");
    assert_eq!(payload["device"], "dev-rev");
}

#[tokio::test]
async fn device_reassigned_event_carries_both_devices() {
    let pool = fresh_pool().await;
    let adapter = PostgresAuditEventRepository::new(pool.clone());

    adapter
        .publish(DomainEvent::DeviceReassigned {
            aggregate_id: 3,
            previous_device: "dev-old".to_string(),
            current_device: "dev-new".to_string(),
            actor: "admin".to_string(),
            occurred_at: at(200),
        })
        .await;

    let payload: Value = sqlx::query_scalar("SELECT payload FROM audit_log WHERE aggregate_id = 3")
        .fetch_one(&pool)
        .await
        .expect("row present");

    assert_eq!(payload["previous_device"], "dev-old");
    assert_eq!(payload["current_device"], "dev-new");
}

#[tokio::test]
async fn rate_limit_reset_event_persists() {
    let pool = fresh_pool().await;
    let adapter = PostgresAuditEventRepository::new(pool.clone());

    adapter
        .publish(DomainEvent::RateLimitReset {
            aggregate_id: 11,
            device: "dev-reset".to_string(),
            actor: "admin".to_string(),
            occurred_at: at(300),
        })
        .await;

    let (event_type, payload): (String, Value) =
        sqlx::query_as("SELECT event_type, payload FROM audit_log WHERE aggregate_id = 11")
            .fetch_one(&pool)
            .await
            .expect("row present");

    assert_eq!(event_type, "RateLimitReset");
    assert_eq!(payload["device"], "dev-reset");
}

#[tokio::test]
async fn key_expiration_extended_serialises_both_set_and_clear() {
    let pool = fresh_pool().await;
    let adapter = PostgresAuditEventRepository::new(pool.clone());

    let new_expiry = at(9_999);
    adapter
        .publish(DomainEvent::KeyExpirationExtended {
            aggregate_id: 5,
            device: "dev-set".to_string(),
            new_expiry: Some(new_expiry),
            actor: "admin".to_string(),
            occurred_at: at(400),
        })
        .await;
    adapter
        .publish(DomainEvent::KeyExpirationExtended {
            aggregate_id: 6,
            device: "dev-clear".to_string(),
            new_expiry: None,
            actor: "admin".to_string(),
            occurred_at: at(500),
        })
        .await;

    let set_payload: Value =
        sqlx::query_scalar("SELECT payload FROM audit_log WHERE aggregate_id = 5")
            .fetch_one(&pool)
            .await
            .expect("set row present");
    assert_eq!(set_payload["device"], "dev-set");
    assert!(
        set_payload["new_expiry"].is_string(),
        "new_expiry present as ISO-8601 string"
    );

    let clear_payload: Value =
        sqlx::query_scalar("SELECT payload FROM audit_log WHERE aggregate_id = 6")
            .fetch_one(&pool)
            .await
            .expect("clear row present");
    assert_eq!(clear_payload["device"], "dev-clear");
    assert!(
        clear_payload["new_expiry"].is_null(),
        "cleared expiry serialises as JSON null"
    );
}

#[tokio::test]
async fn publish_is_fire_and_forget_on_backend_failure() {
    // A closed pool forces every subsequent query to error. The
    // adapter must absorb the failure without panicking — the audit
    // trail is best-effort; losing a row is preferable to failing the
    // operation it witnesses.
    let pool = fresh_pool().await;
    pool.close().await;
    let adapter = PostgresAuditEventRepository::new(pool);

    adapter
        .publish(DomainEvent::KeyRevoked {
            aggregate_id: 1,
            device: "dev".to_string(),
            actor: "admin".to_string(),
            occurred_at: at(0),
        })
        .await;
    // Reaching here at all is the assertion.
}
