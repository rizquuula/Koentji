//! Interface layer — the thin adapters that translate between the
//! outside world (HTTP requests, Leptos server functions, future
//! schedulers) and the application layer. Nothing here carries
//! business rules; everything defers to `application` / `domain`.
//!
//! Phase 1.3 seeds only the HTTP i18n mapper. 1.6 will mount the new
//! `auth_endpoint` through this module.

pub mod http;
