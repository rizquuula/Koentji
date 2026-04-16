//! HTTP adapter — the shape of what Actix sends back on `/v1/auth`.
//!
//! 1.3 introduces the en/id message mapper for
//! [`crate::domain::authentication::DenialReason`]. 1.6 will route the
//! endpoint through [`crate::application`] and emit these envelopes.

pub mod i18n;
