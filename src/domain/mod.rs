//! The Koentji domain layer.
//!
//! Nothing in here knows about Actix, Leptos, SQLx, Moka, or HTTP. Every
//! type is either:
//!
//! - a **value object** (immutable, validated at construction, compared
//!   by attributes),
//! - an **entity** (has identity — a database row corresponds to one),
//! - an **event** (past-tense fact, emitted by aggregates), or
//! - a **port** (trait an outer layer implements to talk to the domain).
//!
//! Bounded contexts live under sub-modules: `authentication`,
//! `key_management`, `billing_plans`, `rate_limit_policy`,
//! `admin_access`. They start empty and fill up as the phases land.
//!
//! Phase 1.1 seeds the `authentication` context with the value objects
//! that the `/v1/auth` path needs: `AuthKey`, `DeviceId`,
//! `RateLimitAmount`, `RateLimitUsage`, `RateLimitWindow`,
//! `SubscriptionName`. Nothing is wired into the HTTP path yet — that's
//! 1.6. This commit just makes the vocabulary exist.

pub mod admin_access;
pub mod authentication;
pub mod errors;

pub use errors::DomainError;
