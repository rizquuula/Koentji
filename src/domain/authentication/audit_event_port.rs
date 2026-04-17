//! The domain port for durable domain-event publishing.
//!
//! The application layer composes this port with the repository and
//! cache ports: after an admin command succeeds, it calls
//! `publish(event)` once. Adapters (3.4's Postgres `audit_log` writer,
//! a future pub/sub fan-out, an in-memory test capture) live in
//! infrastructure.
//!
//! Publish is intentionally fire-and-forget — a failed audit write
//! must never fail the operation it is witnessing, or admins will
//! hesitate to click through legitimate actions. The adapter logs
//! its own errors.

use super::events::DomainEvent;

#[cfg(feature = "ssr")]
#[async_trait::async_trait]
pub trait AuditEventPort: Send + Sync {
    async fn publish(&self, event: DomainEvent);
}
