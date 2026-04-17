//! Admin-access bounded context.
//!
//! The dashboard operator's side of the system. In the current
//! single-admin model, this context is thin: a single
//! [`AdminCredentials`] value object verifies a candidate password.
//! Phase 4.3 adds [`LoginAttemptLedger`] for per-IP brute-force
//! protection.
//!
//! Context is deliberately separate from `authentication`: the
//! `/v1/auth` endpoint authenticates external applications' end-users,
//! while `admin_access` authenticates the dashboard operator. Mixing
//! them would blur the two trust domains (the external API key is
//! public to its caller; the admin password is not).

pub mod admin_credentials;
#[cfg(feature = "ssr")]
pub mod constant_time;
#[cfg(feature = "ssr")]
pub mod login_attempt_ledger;

#[cfg(feature = "ssr")]
pub use admin_credentials::AdminCredentials;
pub use admin_credentials::CredentialError;
#[cfg(feature = "ssr")]
pub use constant_time::equals_in_constant_time;
#[cfg(feature = "ssr")]
pub use login_attempt_ledger::{AttemptDecision, LockoutPolicy, LoginAttemptLedger};
