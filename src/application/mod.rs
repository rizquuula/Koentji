//! Application layer — orchestrates the domain's use cases. Every
//! external entry point (HTTP handler, Leptos server function,
//! future scheduler) speaks to the outside world, parses / validates
//! the request, calls a single use case here, then renders the
//! outcome back into its transport envelope.
//!
//! Phase 1.6 seeds the `authenticate_api_key` use case. Phases 2 and
//! 4 will add admin-side commands alongside it.

#![cfg(feature = "ssr")]

pub mod authenticate_api_key;
pub mod issue_key;
pub mod revoke_key;

pub use authenticate_api_key::{AuthOutcome, AuthenticateApiKey};
pub use issue_key::IssueKey;
pub use revoke_key::RevokeKey;
