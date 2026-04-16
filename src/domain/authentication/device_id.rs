//! Identifier for the specific device/installation an auth key is bound to.
//!
//! A single `AuthKey` can be registered for more than one device id
//! (free-trial auto-provisioning relies on this), so `(AuthKey,
//! DeviceId)` is the actual tenancy unit — not the key alone.

use crate::domain::errors::{DomainError, InvalidReason};

/// Matches the `VARCHAR(255)` column.
pub const MAX_LEN: usize = 255;

/// The sentinel an admin-issued key uses before a real device claims it.
/// Kept as an explicit constant so `"-"` never appears as a magic
/// string in the code.
pub const UNCLAIMED_SENTINEL: &str = "-";

/// A validated device identifier string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(DomainError::InvalidDeviceId(InvalidReason::Empty));
        }
        if raw.len() > MAX_LEN {
            return Err(DomainError::InvalidDeviceId(InvalidReason::TooLong));
        }
        Ok(Self(raw))
    }

    /// The `"-"` placeholder used by admin-issued keys that haven't
    /// claimed a device yet. A key in this state is not yet usable on
    /// the public endpoint.
    pub fn unclaimed() -> Self {
        Self(UNCLAIMED_SENTINEL.to_string())
    }

    pub fn is_unclaimed(&self) -> bool {
        self.0 == UNCLAIMED_SENTINEL
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_device_id() {
        let d = DeviceId::parse("device-1").unwrap();
        assert_eq!(d.as_str(), "device-1");
        assert!(!d.is_unclaimed());
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            DeviceId::parse("").unwrap_err(),
            DomainError::InvalidDeviceId(InvalidReason::Empty)
        );
    }

    #[test]
    fn rejects_over_long() {
        let long = "x".repeat(MAX_LEN + 1);
        assert_eq!(
            DeviceId::parse(long).unwrap_err(),
            DomainError::InvalidDeviceId(InvalidReason::TooLong)
        );
    }

    #[test]
    fn unclaimed_sentinel_is_known() {
        let s = DeviceId::unclaimed();
        assert!(s.is_unclaimed());
        assert_eq!(s.as_str(), "-");
    }

    #[test]
    fn parsing_the_sentinel_is_also_recognised_as_unclaimed() {
        let s = DeviceId::parse("-").unwrap();
        assert!(s.is_unclaimed());
    }
}
