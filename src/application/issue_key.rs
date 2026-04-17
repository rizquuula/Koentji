//! `IssueKey` — the admin-side "an operator mints a fresh API key"
//! use case.
//!
//! Phase 2.1 peels the write path off `server::key_service::create_key`
//! and routes it through a domain verb. The server function still owns
//! request parsing (string → value objects), subscription-type lookup,
//! and the return-shape the frontend expects; this use case is the
//! single place where a new `authentication_keys` row is actually
//! materialised. Keeping it thin is intentional — 2.2 (`RevokeKey`),
//! 2.3 (reassign/reset/extend), 2.4 (cache eviction) will grow the
//! siblings alongside it.
//!
//! Emits a past-tense log line (`KeyIssued`) on success so the event
//! story shows up in logs before the outbox/audit adapter lands in 3.4.

use std::sync::Arc;

use crate::domain::authentication::{
    IssueKeyCommand, IssuedKey, IssuedKeyRepository, RepositoryError,
};

pub struct IssueKey {
    repo: Arc<dyn IssuedKeyRepository>,
}

impl IssueKey {
    pub fn new(repo: Arc<dyn IssuedKeyRepository>) -> Self {
        Self { repo }
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
        Ok(issued)
    }
}
