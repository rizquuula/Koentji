//! Postgres implementation of
//! [`crate::domain::authentication::AuditEventPort`].
//!
//! Appends one row per domain event to the `audit_log` table (migration
//! 005). The adapter is deliberately fire-and-forget: a failed insert
//! is logged at `warn!` level but never bubbles back to the caller. The
//! witnessed operation has already committed; blocking that commit on
//! an audit-write failure would make admin verbs feel flaky and invite
//! silent workarounds.
//!
//! Payload shape is JSONB so adding fields later (subscription id,
//! client IP, etc.) is a code change, not a migration. The column set
//! above it — `event_type`, `aggregate_id`, `actor`, `occurred_at` —
//! covers every access pattern the dashboard or an auditor needs.

use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;

use crate::domain::authentication::{AuditEventPort, DomainEvent};

#[derive(Clone)]
pub struct PostgresAuditEventRepository {
    pool: PgPool,
}

impl PostgresAuditEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn encode(event: &DomainEvent) -> (&'static str, i32, &str, serde_json::Value, DateTime<Utc>) {
        let event_type = event.event_type();
        let aggregate_id = event.aggregate_id();
        let actor = event.actor();
        let occurred_at = event.occurred_at();
        let payload = match event {
            DomainEvent::KeyIssued {
                device,
                subscription,
                ..
            } => json!({
                "device": device,
                "subscription": subscription,
            }),
            DomainEvent::KeyRevoked { device, .. } => json!({
                "device": device,
            }),
            DomainEvent::DeviceReassigned {
                previous_device,
                current_device,
                ..
            } => json!({
                "previous_device": previous_device,
                "current_device": current_device,
            }),
            DomainEvent::RateLimitReset { device, .. } => json!({
                "device": device,
            }),
            DomainEvent::KeyExpirationExtended {
                device, new_expiry, ..
            } => json!({
                "device": device,
                "new_expiry": new_expiry,
            }),
        };
        (event_type, aggregate_id, actor, payload, occurred_at)
    }
}

#[async_trait::async_trait]
impl AuditEventPort for PostgresAuditEventRepository {
    async fn publish(&self, event: DomainEvent) {
        let (event_type, aggregate_id, actor, payload, occurred_at) = Self::encode(&event);

        let result = sqlx::query(
            r#"
            INSERT INTO audit_log (event_type, aggregate_id, actor, payload, occurred_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(event_type)
        .bind(aggregate_id)
        .bind(actor)
        .bind(&payload)
        .bind(occurred_at)
        .execute(&self.pool)
        .await;

        if let Err(err) = result {
            log::warn!(
                "audit_log write failed for {}#{}: {}",
                event_type,
                aggregate_id,
                err
            );
        }
    }
}
