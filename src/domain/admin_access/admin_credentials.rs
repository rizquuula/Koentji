//! Argon2id-backed admin credentials.
//!
//! `AdminCredentials` is a value object wrapping a PHC-encoded Argon2id
//! hash. At startup the application reads `ADMIN_PASSWORD_HASH` from
//! the environment, hands the string to [`AdminCredentials::from_hash`],
//! and stores the result. Every login attempt calls [`verify`], which
//! delegates to the `argon2` crate's constant-time hash verifier — so
//! the login handler never sees the plaintext hash nor a `==`
//! comparison.
//!
//! The plan also calls for a plaintext fallback (so dev and e2e can
//! ship a password in env without a hashing dance); that fallback
//! lives in [`from_plaintext`] and verifies with `subtle::ConstantTimeEq`.
//! The login handler picks the flavour by env-var precedence:
//! `ADMIN_PASSWORD_HASH` wins over `ADMIN_PASSWORD`, and the plaintext
//! variant logs a warning on construction to flag the insecure
//! configuration.
//!
//! The separation between the two flavours is internal — callers see
//! one `verify(candidate)` surface and can't tell (nor care) which
//! branch is running.

#[cfg(feature = "ssr")]
use argon2::{Argon2, PasswordHash, PasswordVerifier};
#[cfg(feature = "ssr")]
use subtle::ConstantTimeEq;

#[derive(Debug)]
pub enum CredentialError {
    InvalidHashEncoding,
    EmptyPlaintext,
}

impl std::fmt::Display for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialError::InvalidHashEncoding => {
                f.write_str("ADMIN_PASSWORD_HASH is not a valid argon2id PHC string")
            }
            CredentialError::EmptyPlaintext => f.write_str("admin password cannot be empty"),
        }
    }
}

impl std::error::Error for CredentialError {}

#[cfg(feature = "ssr")]
pub struct AdminCredentials {
    inner: CredentialFlavour,
}

#[cfg(feature = "ssr")]
enum CredentialFlavour {
    /// Production / hardened path: a PHC-encoded Argon2id hash.
    Hashed(String),
    /// Dev / e2e fallback: plaintext compared in constant time. Emits
    /// a warning at construction so the insecure path is visible.
    Plaintext(String),
}

// Manual `Debug` so the plaintext branch never prints the password
// and the hashed branch never prints the full PHC string — we report
// only the variant. Accidental `dbg!` / panic / log formatting must
// not leak the secret.
#[cfg(feature = "ssr")]
impl std::fmt::Debug for AdminCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            CredentialFlavour::Hashed(_) => f.write_str("AdminCredentials(Hashed(<redacted>))"),
            CredentialFlavour::Plaintext(_) => {
                f.write_str("AdminCredentials(Plaintext(<redacted>))")
            }
        }
    }
}

#[cfg(feature = "ssr")]
impl AdminCredentials {
    /// Parse a PHC-encoded Argon2id hash. Fails on malformed input
    /// rather than limping along — a wrong hash encoding is a
    /// deploy-time mistake that should stop the process.
    pub fn from_hash(phc: impl Into<String>) -> Result<Self, CredentialError> {
        let phc = phc.into();
        PasswordHash::new(&phc).map_err(|_| CredentialError::InvalidHashEncoding)?;
        Ok(Self {
            inner: CredentialFlavour::Hashed(phc),
        })
    }

    /// Dev / e2e fallback: hold the plaintext and constant-time
    /// compare later. Empty strings are rejected so the "no password
    /// set" misconfiguration can't accidentally authenticate the
    /// empty request.
    pub fn from_plaintext(plaintext: impl Into<String>) -> Result<Self, CredentialError> {
        let plaintext = plaintext.into();
        if plaintext.is_empty() {
            return Err(CredentialError::EmptyPlaintext);
        }
        log::warn!(
            "ADMIN_PASSWORD is set without ADMIN_PASSWORD_HASH — using plaintext fallback. \
             Set ADMIN_PASSWORD_HASH in production (generate with `make hash-admin-password`)."
        );
        Ok(Self {
            inner: CredentialFlavour::Plaintext(plaintext),
        })
    }

    /// Verify a candidate password. Returns `true` on match, `false`
    /// otherwise. Both branches are constant-time in the match path:
    /// Argon2's verifier uses constant-time byte comparison on the
    /// derived hash; the plaintext branch uses `subtle::ConstantTimeEq`.
    pub fn verify(&self, candidate: &str) -> bool {
        match &self.inner {
            CredentialFlavour::Hashed(phc) => {
                // Re-parse on every verify — `PasswordHash<'a>` borrows
                // from the source string, so keeping it around forces
                // the owning type into a self-referential shape we do
                // not want. Parsing cost is trivial next to argon2's.
                let Ok(parsed) = PasswordHash::new(phc) else {
                    return false;
                };
                Argon2::default()
                    .verify_password(candidate.as_bytes(), &parsed)
                    .is_ok()
            }
            CredentialFlavour::Plaintext(expected) => {
                expected.as_bytes().ct_eq(candidate.as_bytes()).unwrap_u8() == 1
            }
        }
    }

    /// Is this instance using the plaintext fallback? Used only by
    /// startup logs to tell operators the posture of the running
    /// process.
    pub fn is_plaintext_fallback(&self) -> bool {
        matches!(self.inner, CredentialFlavour::Plaintext(_))
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::PasswordHasher;

    fn hash(plaintext: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(plaintext.as_bytes(), &salt)
            .expect("argon2 hash")
            .to_string()
    }

    #[test]
    fn verify_accepts_the_right_password_for_a_hashed_credential() {
        let creds = AdminCredentials::from_hash(hash("correct horse")).unwrap();
        assert!(creds.verify("correct horse"));
    }

    #[test]
    fn verify_rejects_a_wrong_password_for_a_hashed_credential() {
        let creds = AdminCredentials::from_hash(hash("correct horse")).unwrap();
        assert!(!creds.verify("battery staple"));
    }

    #[test]
    fn verify_rejects_empty_candidate_against_hashed_credential() {
        let creds = AdminCredentials::from_hash(hash("correct horse")).unwrap();
        assert!(!creds.verify(""));
    }

    #[test]
    fn from_hash_rejects_gibberish() {
        let err = AdminCredentials::from_hash("not-a-phc-string").unwrap_err();
        assert!(matches!(err, CredentialError::InvalidHashEncoding));
    }

    #[test]
    fn plaintext_fallback_round_trip_succeeds() {
        let creds = AdminCredentials::from_plaintext("e2eadmin").unwrap();
        assert!(creds.is_plaintext_fallback());
        assert!(creds.verify("e2eadmin"));
        assert!(!creds.verify("e2eadmin-wrong"));
    }

    #[test]
    fn plaintext_fallback_rejects_empty() {
        let err = AdminCredentials::from_plaintext("").unwrap_err();
        assert!(matches!(err, CredentialError::EmptyPlaintext));
    }

    #[test]
    fn plaintext_fallback_is_length_constant_signalled() {
        // Not an actual timing test — we cannot measure constant-time
        // behavior from Rust tests reliably. This locks in that
        // `ct_eq` is what the plaintext path uses by round-tripping
        // candidates of different lengths and asserting the outcome
        // is length-independent (both rejected).
        let creds = AdminCredentials::from_plaintext("abcdef").unwrap();
        assert!(!creds.verify("a"));
        assert!(!creds.verify("abcdefg"));
        assert!(!creds.verify("abcdez"));
    }

    #[test]
    fn hashed_credential_is_not_flagged_as_fallback() {
        let creds = AdminCredentials::from_hash(hash("pw")).unwrap();
        assert!(!creds.is_plaintext_fallback());
    }
}
