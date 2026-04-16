//! Short identifier of the subscription plan attached to a key
//! (e.g. `"free"`, `"pro"`). The full plan record lives in the
//! `billing_plans` context; here we only need the name to present back
//! on the `/v1/auth` success envelope.

use crate::domain::errors::{DomainError, InvalidReason};

pub const MAX_LEN: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriptionName(String);

impl SubscriptionName {
    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(DomainError::InvalidSubscriptionName(InvalidReason::Empty));
        }
        if raw.len() > MAX_LEN {
            return Err(DomainError::InvalidSubscriptionName(InvalidReason::TooLong));
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

impl std::fmt::Display for SubscriptionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_free_plan() {
        assert_eq!(SubscriptionName::parse("free").unwrap().as_str(), "free");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            SubscriptionName::parse("").unwrap_err(),
            DomainError::InvalidSubscriptionName(InvalidReason::Empty)
        );
    }

    #[test]
    fn rejects_over_long() {
        let long = "x".repeat(MAX_LEN + 1);
        assert_eq!(
            SubscriptionName::parse(long).unwrap_err(),
            DomainError::InvalidSubscriptionName(InvalidReason::TooLong)
        );
    }
}
