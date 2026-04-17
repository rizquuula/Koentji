//! `IssueKey` — the admin-side "an operator mints a fresh API key"
//! use case.
//!
//! Phase 2.1 peels the write path off `server::key_service::create_key`
//! and routes it through a domain verb. The server function still owns
//! request parsing (string → value objects), subscription-type lookup,
//! and the return-shape the frontend expects; this use case is the
//! single place where a new `authentication_keys` row is actually
//! materialised.
//!
//! 3.4 wires the audit outbox — every successful mint publishes a
//! `KeyIssued` domain event through [`AuditEventPort`], which the
//! Postgres adapter appends to `audit_log`. The past-tense log line
//! stays on alongside the event: logs rotate out, the audit table is
//! the durable trail.

use std::sync::Arc;

use chrono::Utc;

use crate::domain::authentication::{
    AuditEventPort, DomainEvent, IssueKeyCommand, IssuedKey, IssuedKeyRepository, RepositoryError,
};

pub struct IssueKey {
    repo: Arc<dyn IssuedKeyRepository>,
    audit: Arc<dyn AuditEventPort>,
}

impl IssueKey {
    pub fn new(repo: Arc<dyn IssuedKeyRepository>, audit: Arc<dyn AuditEventPort>) -> Self {
        Self { repo, audit }
    }

    pub async fn execute(&self, command: IssueKeyCommand) -> Result<IssuedKey, RepositoryError> {
        let issued_by = command.issued_by.clone();
        let issued = self.repo.issue_key(command).await?;
        log::info!(
            "KeyIssued: id={}, device={}, subscription={}, issued_by={}",
            issued.id.value(),
            issued.device_id.as_str(),
            issued
                .subscription
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("<none>"),
            issued_by,
        );
        self.audit
            .publish(DomainEvent::KeyIssued {
                aggregate_id: issued.id.value(),
                device: issued.device_id.as_str().to_string(),
                subscription: issued.subscription.as_ref().map(|s| s.as_str().to_string()),
                actor: issued_by,
                occurred_at: Utc::now(),
            })
            .await;
        Ok(issued)
    }
}
