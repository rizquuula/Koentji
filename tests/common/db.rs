use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Executor, PgPool};
use std::str::FromStr;
use tokio::sync::OnceCell;

/// Maintenance DB URL — used to `CREATE DATABASE` if the shared test DB is
/// missing. Defaults to the dev-compose cluster.
const MAINTENANCE_URL_ENV: &str = "KOENTJI_TEST_MAINTENANCE_URL";
const DEFAULT_MAINTENANCE_URL: &str = "postgres://koentji:koentji@127.0.0.1:5432/postgres";

/// Shared test DB. Created once per process if missing, then reused.
const TEST_DB_NAME: &str = "koentji_rs_test";

static SHARED_POOL: OnceCell<PgPool> = OnceCell::const_new();

/// Return a pool to the shared test DB. On first call per process, this
/// creates the DB if missing and runs migrations. Subsequent calls are cheap.
///
/// Tests share the DB — to avoid cross-test pollution, call [`reset`] at
/// the top of each test.
pub async fn test_pool() -> PgPool {
    SHARED_POOL
        .get_or_init(|| async { init_pool().await })
        .await
        .clone()
}

async fn init_pool() -> PgPool {
    let maintenance_url = std::env::var(MAINTENANCE_URL_ENV)
        .unwrap_or_else(|_| DEFAULT_MAINTENANCE_URL.to_string());

    ensure_database_exists(&maintenance_url, TEST_DB_NAME).await;

    let test_opts = PgConnectOptions::from_str(&maintenance_url)
        .expect("valid maintenance URL")
        .database(TEST_DB_NAME);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(test_opts)
        .await
        .expect("connect to shared test DB");

    koentji::db::run_migrations(&pool).await;
    pool
}

async fn ensure_database_exists(maintenance_url: &str, db_name: &str) {
    let opts = PgConnectOptions::from_str(maintenance_url).expect("valid maintenance URL");
    let mut admin = opts.connect().await.expect("connect to maintenance DB");

    let exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(db_name)
            .fetch_one(&mut admin)
            .await
            .expect("probe pg_database");

    if !exists.0 {
        admin
            .execute(format!(r#"CREATE DATABASE "{}""#, db_name).as_str())
            .await
            .expect("create shared test DB");
    }
}

/// Wipe all domain data so each test starts from a known state.
/// Preserves the static catalogue rows (`subscription_types`,
/// `rate_limit_intervals`) that migrations seed.
pub async fn reset(pool: &PgPool) {
    sqlx::query("TRUNCATE authentication_keys RESTART IDENTITY CASCADE")
        .execute(pool)
        .await
        .expect("truncate authentication_keys");
}

/// Convenience for tests that want the pool + an already-reset state.
pub async fn fresh_pool() -> PgPool {
    let pool = test_pool().await;
    reset(&pool).await;
    pool
}
