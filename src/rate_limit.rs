//! Atomic rate-limit decrement for `POST /v1/auth`.
//!
//! The previous implementation was a read-modify-write split across two
//! round-trips (SELECT then a fire-and-forget UPDATE). Two concurrent
//! requests could both read the same `rate_limit_remaining`, both pass
//! the "quota available" check in Rust, and both write back
//! `remaining - usage` — leaking one quota slot per collision.
//!
//! This module replaces that path with a single SQL statement: `UPDATE
//! … RETURNING`. Postgres locks the row for the duration of the update,
//! so two concurrent consumes serialise. The reset-on-window-elapsed
//! decision is made **inside** SQL using the joined
//! `rate_limit_intervals.duration_seconds`, so racing resets produce a
//! correct total instead of both writing the full-daily minus one.
//!
//! Semantic: deny when `remaining < usage` (or `daily < usage`). The
//! last slot is consumable — a request with `usage == remaining`
//! succeeds and drops remaining to `0`; the next request is denied.
//!
//! Pre-checks for revoked / expired keys still live in the caller — only
//! the rate-limit hot path is in SQL here. Phase 1 will move all of this
//! behind a domain `Authenticator`; for now the function is intentionally
//! small and direct.

#![cfg(feature = "ssr")]

use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub enum ConsumeResult {
    /// Quota was available (or the window had elapsed and was reset).
    /// `remaining` is the post-decrement count.
    Allowed {
        remaining: i32,
        updated_at: DateTime<Utc>,
    },
    /// No rows matched the consume predicate — the window is still open
    /// and `remaining < usage`, or the usage itself exceeds the daily
    /// quota.
    RateLimitExceeded,
}

/// Atomically decrement rate-limit quota for `(key, device_id)`,
/// respecting the interval-based reset window.
///
/// Returns:
/// - `Ok(Allowed { remaining, .. })` if the row was updated.
/// - `Ok(RateLimitExceeded)` if the UPDATE affected zero rows due to
///   insufficient quota inside the active window.
/// - `Err(_)` on any DB error.
///
/// If `(key, device_id)` does not exist, returns `RateLimitExceeded` —
/// callers must short-circuit the "no such key" path before invoking.
pub async fn consume_rate_limit(
    pool: &PgPool,
    key: &str,
    device_id: &str,
    usage: i32,
    now: DateTime<Utc>,
) -> Result<ConsumeResult, sqlx::Error> {
    let row: Option<(i32, DateTime<Utc>)> = sqlx::query_as(
        r#"
        UPDATE authentication_keys ak
        SET
            rate_limit_remaining = CASE
                WHEN ak.rate_limit_updated_at IS NULL
                  OR EXTRACT(EPOCH FROM ($1::timestamptz - ak.rate_limit_updated_at))
                     >= COALESCE(
                            (SELECT duration_seconds FROM rate_limit_intervals
                             WHERE id = ak.rate_limit_interval_id),
                            86400)
                THEN ak.rate_limit_daily - $2::int
                ELSE ak.rate_limit_remaining - $2::int
            END,
            rate_limit_updated_at = $1::timestamptz
        WHERE ak.key = $3
          AND ak.device_id = $4
          AND (
              ak.rate_limit_updated_at IS NULL
              OR EXTRACT(EPOCH FROM ($1::timestamptz - ak.rate_limit_updated_at))
                 >= COALESCE(
                        (SELECT duration_seconds FROM rate_limit_intervals
                         WHERE id = ak.rate_limit_interval_id),
                        86400)
              OR ak.rate_limit_remaining >= $2::int
          )
          AND ak.rate_limit_daily >= $2::int
        RETURNING ak.rate_limit_remaining, ak.rate_limit_updated_at
        "#,
    )
    .bind(now)
    .bind(usage)
    .bind(key)
    .bind(device_id)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some((remaining, updated_at)) => ConsumeResult::Allowed {
            remaining,
            updated_at,
        },
        None => ConsumeResult::RateLimitExceeded,
    })
}
