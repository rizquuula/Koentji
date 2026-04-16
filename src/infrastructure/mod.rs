//! Infrastructure layer — the adapters that plug the domain's ports
//! into concrete technology (Postgres, Moka, argon2, clocks). The
//! domain never imports from here.
//!
//! Phase 1.4 adds the Postgres adapter for
//! [`crate::domain::authentication::IssuedKeyRepository`]. 1.5 will
//! add the cache adapter; later phases add argon2 + telemetry.

#![cfg(feature = "ssr")]

pub mod cache;
pub mod postgres;
