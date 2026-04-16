//! Shared test harness.
//!
//! Only compiles under `--features ssr`. Integration tests that want any of
//! the fixtures below declare `mod common;` at the top of their binary file.
//! Dead-code is allowed because each binary will use a subset.

#![cfg(feature = "ssr")]
#![allow(dead_code, unused_imports)]

pub mod clock;
pub mod db;
pub mod key_builder;

pub use clock::{Clock, SystemClock, TestClock};
pub use db::{fresh_pool, reset, test_pool};
pub use key_builder::{a_free_trial_key, a_key, an_expired_key, a_revoked_key, KeyBuilder};
