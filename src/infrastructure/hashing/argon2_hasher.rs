//! Generate an argon2id PHC-encoded hash for a plaintext password.
//!
//! One-shot helper used by the `hash-admin-password` binary — generate
//! a fresh random salt with `OsRng`, run the plaintext through
//! `Argon2::default()` (argon2id, m=19_456, t=2, p=1 per the crate's
//! defaults), and return the PHC string. The caller pastes that string
//! into `ADMIN_PASSWORD_HASH`; verification at login-time lives in
//! [`crate::domain::admin_access::AdminCredentials::verify`].

use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};

#[derive(Debug)]
pub enum HashError {
    Hashing(String),
}

impl std::fmt::Display for HashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashError::Hashing(msg) => write!(f, "argon2 hashing failed: {msg}"),
        }
    }
}

impl std::error::Error for HashError {}

pub fn hash_password(plaintext: &str) -> Result<String, HashError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plaintext.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| HashError::Hashing(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::admin_access::AdminCredentials;

    #[test]
    fn hash_password_round_trips_through_admin_credentials() {
        let phc = hash_password("correct horse battery staple").unwrap();
        let creds = AdminCredentials::from_hash(phc).expect("valid PHC hash");
        assert!(creds.verify("correct horse battery staple"));
        assert!(!creds.verify("wrong"));
    }

    #[test]
    fn hash_password_produces_different_hashes_for_same_plaintext() {
        // Salts are random — two calls must not collide, otherwise the
        // salt isn't actually randomising anything.
        let a = hash_password("same-pw").unwrap();
        let b = hash_password("same-pw").unwrap();
        assert_ne!(a, b);
    }
}
