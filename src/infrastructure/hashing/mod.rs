//! Hashing adapters — the concrete argon2id hashing the
//! [`crate::domain::admin_access::AdminCredentials`] value object
//! consumes as a PHC-encoded string.
//!
//! The domain side only knows `from_hash(&str)` / `verify(&str)`. This
//! module owns the one-shot hasher used by the admin-side binary
//! (`hash-admin-password`) that prints a freshly-salted PHC hash to
//! stdout for operators to paste into `ADMIN_PASSWORD_HASH`. Keeping
//! the dependency on `argon2` + `password-hash` here (not in the
//! domain) preserves the "domain never imports infrastructure" rule.

#![cfg(feature = "ssr")]

pub mod argon2_hasher;

pub use argon2_hasher::hash_password;
