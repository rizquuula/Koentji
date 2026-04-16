//! The opaque string an external app presents to `/v1/auth`.
//!
//! `AuthKey` is intentionally opaque: the domain never interprets its
//! content, it only checks that it *exists* and is *within bounds*.
//! Trimming of surrounding whitespace is the caller's job — if the
//! client sends trailing spaces we treat that as a distinct key, since
//! the same rule applies in the DB lookup.

use crate::domain::errors::{DomainError, InvalidReason};

/// Maximum accepted length, matches the `VARCHAR(255)` column.
pub const MAX_LEN: usize = 255;

/// A validated API key string. Construct via [`AuthKey::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuthKey(String);

impl AuthKey {
    /// Reject empty and over-long inputs. Other characters pass through
    /// unmodified — the domain does not impose an alphabet.
    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(DomainError::InvalidAuthKey(InvalidReason::Empty));
        }
        if raw.len() > MAX_LEN {
            return Err(DomainError::InvalidAuthKey(InvalidReason::TooLong));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for AuthKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_plain_key() {
        let k = AuthKey::parse("klab_abc").unwrap();
        assert_eq!(k.as_str(), "klab_abc");
    }

    #[test]
    fn rejects_empty_string() {
        assert_eq!(
            AuthKey::parse("").unwrap_err(),
            DomainError::InvalidAuthKey(InvalidReason::Empty)
        );
    }

    #[test]
    fn rejects_over_long_key() {
        let long = "x".repeat(MAX_LEN + 1);
        assert_eq!(
            AuthKey::parse(long).unwrap_err(),
            DomainError::InvalidAuthKey(InvalidReason::TooLong)
        );
    }

    #[test]
    fn accepts_exactly_max_length() {
        let at_limit = "x".repeat(MAX_LEN);
        assert!(AuthKey::parse(at_limit).is_ok());
    }

    #[test]
    fn preserves_whitespace_verbatim() {
        // Trimming is the caller's call — the DB lookup uses the raw
        // string. If the client sends " leading", that's what we store.
        let k = AuthKey::parse(" leading").unwrap();
        assert_eq!(k.as_str(), " leading");
    }

    #[test]
    fn equality_is_by_value() {
        let a = AuthKey::parse("same").unwrap();
        let b = AuthKey::parse("same").unwrap();
        assert_eq!(a, b);
    }
}
