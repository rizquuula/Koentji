//! AdminAccess bounded context — the login page.
//!
//! Mirrors the domain module `crate::domain::admin_access`, which owns
//! `AdminCredentials` and the `LoginAttemptLedger`.

pub mod page;

pub use page::LoginPage;
