//! Telemetry — the slice of infrastructure that lets an operator watch
//! the running server. Phase 5.1 seeds it with a structured JSON
//! access-log middleware; 5.2 will add request-id propagation;
//! /healthz, /readyz and graceful shutdown land in their own slices.

#![cfg(feature = "ssr")]

pub mod access_log;

pub use access_log::AccessLog;
