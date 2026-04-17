//! User-interface layer, organised by feature rather than by technical kind.
//!
//! Each sub-module corresponds to a bounded context on the admin dashboard —
//! pages, their supporting widgets, and any feature-local view state live
//! together under one path. Cross-cutting concerns live under `design/` (the
//! token-backed primitive library) and `shell/` (the nav + layout frame).

pub mod admin_access;
pub mod dashboard;
pub mod design;
pub mod keys;
pub mod marketing;
pub mod rate_limits;
pub mod shell;
pub mod subscriptions;
