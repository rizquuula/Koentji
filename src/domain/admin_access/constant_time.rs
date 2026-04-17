//! Constant-time byte equality for admin-login inputs.
//!
//! `==` on `String` short-circuits on the first mismatching byte — an
//! attacker who can time a login response can learn the username
//! prefix byte-by-byte. The password half of the check already runs
//! through `AdminCredentials::verify` (constant-time in both branches);
//! this helper gives the same guarantee to the username compare so a
//! wrong-user request takes indistinguishable time from a
//! wrong-password request against the correct user.

#![cfg(feature = "ssr")]

use subtle::ConstantTimeEq;

/// Byte-wise equality that does NOT short-circuit on the first
/// mismatch. Mismatched lengths return `false` immediately (length
/// alone is not secret for an admin username), but given same-length
/// inputs every byte is always compared.
pub fn equals_in_constant_time(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).unwrap_u8() == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_strings_compare_equal() {
        assert!(equals_in_constant_time("admin", "admin"));
    }

    #[test]
    fn unequal_strings_of_equal_length_compare_unequal() {
        assert!(!equals_in_constant_time("admin", "admix"));
    }

    #[test]
    fn unequal_length_strings_compare_unequal() {
        assert!(!equals_in_constant_time("admin", "admins"));
        assert!(!equals_in_constant_time("admin", "a"));
    }

    #[test]
    fn empty_strings_compare_equal() {
        // The login path guards against empty admin_username separately
        // (via env default); this asserts only the primitive's shape.
        assert!(equals_in_constant_time("", ""));
    }

    #[test]
    fn non_ascii_bytes_are_compared() {
        assert!(equals_in_constant_time("αβγ", "αβγ"));
        assert!(!equals_in_constant_time("αβγ", "αβδ"));
    }
}
