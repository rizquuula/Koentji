use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Defaults tuned for the dashboard + `/v1/auth` mix:
/// - `acquire_timeout(5s)`: a request waiting longer than this for a
///   connection is stuck behind a saturated pool, not just slow —
///   fail loud so the client retries and the incident is visible.
/// - `idle_timeout(10m)`: reclaim cold connections so Postgres isn't
///   carrying ~20 idle backends forever, but keep enough warmth that
///   a burst of traffic doesn't pay reconnect latency.
/// - `test_before_acquire(true)`: a Postgres restart or network blip
///   silently invalidates pooled sockets; a cheap `SELECT 1` before
///   handing a connection out prevents the first-request-after-blip
///   from 500-ing on every pooled socket in turn.
pub async fn create_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .test_before_acquire(true)
        .connect(&database_url)
        .await
        .expect("Failed to create database pool")
}

include!(concat!(env!("OUT_DIR"), "/migrations.rs"));

pub async fn run_migrations(pool: &PgPool) {
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            filename    VARCHAR(255) PRIMARY KEY,
            applied_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create schema_migrations table");

    for (filename, sql) in MIGRATIONS {
        let already_applied: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE filename = $1)",
        )
        .bind(*filename)
        .fetch_one(pool)
        .await
        .expect("Failed to query schema_migrations");

        if already_applied {
            log::debug!("Migration already applied, skipping: {}", filename);
            continue;
        }

        log::info!("Running migration: {}", filename);
        sqlx::raw_sql(sql)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("Failed to run migration {}: {}", filename, e));

        sqlx::query("INSERT INTO schema_migrations (filename) VALUES ($1)")
            .bind(*filename)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("Failed to record migration {}: {}", filename, e));

        log::info!("Migration applied: {}", filename);
    }
}
