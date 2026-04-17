//! Past-tense domain events for the authentication bounded context.
//!
//! Each admin verb emits one of these on success; the auth endpoint's
//! `AuthenticationSucceeded` / `AuthenticationDenied` counterparts can
//! join later (they are on the hot path, so adding an I/O write per
//! request wants more thought around back-pressure).
//!
//! The events are deliberately flat — no references to in-memory
//! aggregates, no lifetimes. An [`AuditEventPort`] adapter serialises
//! them for durable storage (the Postgres `audit_log` table in 3.3)
//! and the application layer publishes after the repository write
//! succeeds.
//!
//! [`AuditEventPort`]: crate::domain::authentication::AuditEventPort

use chrono::{DateTime, Utc};

/// A past-tense fact about the authentication bounded context.
///
/// Variants carry only primitives + `DateTime<Utc>` so they are cheap
/// to clone, cheap to serialise, and they do not keep the aggregate
/// alive any longer than necessary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    /// An admin minted a fresh `(key, device)` pair.
    KeyIssued {
        aggregate_id: i32,
        device: String,
        subscription: Option<String>,
        actor: String,
        occurred_at: DateTime<Utc>,
    },
    /// An admin soft-deleted the key at `aggregate_id`.
    KeyRevoked {
        aggregate_id: i32,
        device: String,
        actor: String,
        occurred_at: DateTime<Utc>,
    },
    /// The `(key, previous_device)` row has been rebound to
    /// `current_device`.
    DeviceReassigned {
        aggregate_id: i32,
        previous_device: String,
        current_device: String,
        actor: String,
        occurred_at: DateTime<Utc>,
    },
    /// An admin refilled the remaining quota for `aggregate_id`.
    RateLimitReset {
        aggregate_id: i32,
        device: String,
        actor: String,
        occurred_at: DateTime<Utc>,
    },
    /// An admin set (or cleared, when `new_expiry == None`) the
    /// expiration for `aggregate_id`.
    KeyExpirationExtended {
        aggregate_id: i32,
        device: String,
        new_expiry: Option<DateTime<Utc>>,
        actor: String,
        occurred_at: DateTime<Utc>,
    },
}

impl DomainEvent {
    /// Stable identifier used as the `audit_log.event_type` column —
    /// never changes shape across payload evolutions.
    pub fn event_type(&self) -> &'static str {
        match self {
            DomainEvent::KeyIssued { .. } => "KeyIssued",
            DomainEvent::KeyRevoked { .. } => "KeyRevoked",
            DomainEvent::DeviceReassigned { .. } => "DeviceReassigned",
            DomainEvent::RateLimitReset { .. } => "RateLimitReset",
            DomainEvent::KeyExpirationExtended { .. } => "KeyExpirationExtended",
        }
    }

    pub fn aggregate_id(&self) -> i32 {
        match self {
            DomainEvent::KeyIssued { aggregate_id, .. }
            | DomainEvent::KeyRevoked { aggregate_id, .. }
            | DomainEvent::DeviceReassigned { aggregate_id, .. }
            | DomainEvent::RateLimitReset { aggregate_id, .. }
            | DomainEvent::KeyExpirationExtended { aggregate_id, .. } => *aggregate_id,
        }
    }

    pub fn actor(&self) -> &str {
        match self {
            DomainEvent::KeyIssued { actor, .. }
            | DomainEvent::KeyRevoked { actor, .. }
            | DomainEvent::DeviceReassigned { actor, .. }
            | DomainEvent::RateLimitReset { actor, .. }
            | DomainEvent::KeyExpirationExtended { actor, .. } => actor,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            DomainEvent::KeyIssued { occurred_at, .. }
            | DomainEvent::KeyRevoked { occurred_at, .. }
            | DomainEvent::DeviceReassigned { occurred_at, .. }
            | DomainEvent::RateLimitReset { occurred_at, .. }
            | DomainEvent::KeyExpirationExtended { occurred_at, .. } => *occurred_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(offset: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000 + offset, 0).unwrap()
    }

    #[test]
    fn event_type_is_past_tense_and_stable() {
        let e = DomainEvent::KeyIssued {
            aggregate_id: 1,
            device: "d".into(),
            subscription: None,
            actor: "admin".into(),
            occurred_at: at(0),
        };
        assert_eq!(e.event_type(), "KeyIssued");
    }

    #[test]
    fn accessors_match_variant_fields() {
        let e = DomainEvent::DeviceReassigned {
            aggregate_id: 42,
            previous_device: "dev-old".into(),
            current_device: "dev-new".into(),
            actor: "admin".into(),
            occurred_at: at(100),
        };
        assert_eq!(e.event_type(), "DeviceReassigned");
        assert_eq!(e.aggregate_id(), 42);
        assert_eq!(e.actor(), "admin");
        assert_eq!(e.occurred_at(), at(100));
    }
}
