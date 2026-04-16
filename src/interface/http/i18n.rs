//! Translate a pure-domain [`DenialReason`] into the bilingual
//! envelope the `/v1/auth` endpoint has always returned.
//!
//! The envelope is frozen for backward-compat:
//!
//! ```json
//! {
//!   "error": { "en": "...", "id": "..." },
//!   "message": "..."   // same string as error.en
//! }
//! ```
//!
//! Keeping the mapper here (and pure) means the domain never knows
//! about languages, JSON shapes, or HTTP status codes — 1.6 will
//! decide the status code from the reason kind, not from the text.
//!
//! Every string below is byte-identical to the inline `json!` blocks
//! that lived in `src/main.rs` before the extraction, so e2e specs
//! that pattern-match the text keep passing.

use chrono::{DateTime, Utc};

use crate::domain::authentication::DenialReason;

/// Bilingual payload the HTTP layer puts under `error` on a failed
/// auth. `message` mirrors `en` for clients that ignore the object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenialEnvelope {
    pub en: String,
    pub id: String,
}

impl DenialEnvelope {
    pub fn from_reason(reason: &DenialReason) -> Self {
        match reason {
            DenialReason::UnknownKey => Self::unknown_key(),
            DenialReason::Revoked { at } => Self::revoked(*at),
            DenialReason::Expired { at } => Self::expired(*at),
            DenialReason::FreeTrialEnded { .. } => Self::free_trial_ended(),
            DenialReason::RateLimitExceeded => Self::rate_limit_exceeded(),
        }
    }

    fn unknown_key() -> Self {
        Self {
            en: "Authentication key invalid or not exists in our system.".into(),
            id: "Authentication key tidak valid atau tidak ditemukan di sistem kami.".into(),
        }
    }

    fn revoked(at: DateTime<Utc>) -> Self {
        let at = at.to_rfc3339();
        Self {
            en: format!(
                "Authentication key already revoked and can't be used since {}.",
                at
            ),
            id: format!(
                "Authentication key sudah tidak bisa digunakan sejak {}.",
                at
            ),
        }
    }

    fn expired(at: DateTime<Utc>) -> Self {
        let at = at.to_rfc3339();
        Self {
            en: format!("Authentication key expired and need renewal per {}.", at),
            id: format!(
                "Authentication key kadaluwarsa dan butuh pembaruan per tanggal {}.",
                at
            ),
        }
    }

    fn free_trial_ended() -> Self {
        Self {
            en: "Free trial period has ended. Please upgrade your subscription to continue.".into(),
            id: "Masa percobaan gratis telah berakhir. Silakan tingkatkan langganan Anda untuk melanjutkan.".into(),
        }
    }

    fn rate_limit_exceeded() -> Self {
        Self {
            en: "Rate limit exceeded. Please try again later or upgrade your subscription.".into(),
            id: "Batas rate limit terlampaui. Silakan coba lagi nanti atau upgrade langganan Anda."
                .into(),
        }
    }
}

/// The HTTP status code that matches a denial. Split out so the
/// endpoint adapter (1.6) reads it from one source of truth rather
/// than inlining the decision next to the text.
///
/// Mirrors the legacy handler exactly:
/// - `UnknownKey` / `Revoked` / `Expired` / `FreeTrialEnded` → 401
/// - `RateLimitExceeded` → 429
pub fn status_code(reason: &DenialReason) -> u16 {
    match reason {
        DenialReason::RateLimitExceeded => 429,
        DenialReason::UnknownKey
        | DenialReason::Revoked { .. }
        | DenialReason::Expired { .. }
        | DenialReason::FreeTrialEnded { .. } => 401,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-02-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn unknown_key_matches_legacy_envelope() {
        let env = DenialEnvelope::from_reason(&DenialReason::UnknownKey);
        assert_eq!(
            env.en,
            "Authentication key invalid or not exists in our system."
        );
        assert_eq!(
            env.id,
            "Authentication key tidak valid atau tidak ditemukan di sistem kami."
        );
        assert_eq!(status_code(&DenialReason::UnknownKey), 401);
    }

    #[test]
    fn revoked_includes_the_revocation_timestamp() {
        let env = DenialEnvelope::from_reason(&DenialReason::Revoked { at: at() });
        assert!(env.en.contains("2026-02-01T00:00:00+00:00"));
        assert!(env.id.contains("2026-02-01T00:00:00+00:00"));
        assert_eq!(status_code(&DenialReason::Revoked { at: at() }), 401);
    }

    #[test]
    fn expired_includes_the_expiry_timestamp() {
        let env = DenialEnvelope::from_reason(&DenialReason::Expired { at: at() });
        assert!(env.en.starts_with("Authentication key expired"));
        assert!(env.en.contains("2026-02-01T00:00:00+00:00"));
        assert_eq!(status_code(&DenialReason::Expired { at: at() }), 401);
    }

    #[test]
    fn free_trial_has_its_own_message_and_drops_the_date() {
        // Legacy envelope intentionally omitted the date for trial keys.
        let env = DenialEnvelope::from_reason(&DenialReason::FreeTrialEnded { at: at() });
        assert_eq!(
            env.en,
            "Free trial period has ended. Please upgrade your subscription to continue."
        );
        assert!(!env.en.contains("2026"));
        assert_eq!(status_code(&DenialReason::FreeTrialEnded { at: at() }), 401);
    }

    #[test]
    fn rate_limit_is_the_only_429() {
        let env = DenialEnvelope::from_reason(&DenialReason::RateLimitExceeded);
        assert!(env.en.starts_with("Rate limit exceeded"));
        assert!(env.id.starts_with("Batas rate limit"));
        assert_eq!(status_code(&DenialReason::RateLimitExceeded), 429);
    }
}
