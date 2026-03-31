use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(20)
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
            println!("Migration already applied, skipping: {}", filename);
            continue;
        }

        println!("Running migration: {}", filename);
        sqlx::raw_sql(sql)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("Failed to run migration {}: {}", filename, e));

        sqlx::query("INSERT INTO schema_migrations (filename) VALUES ($1)")
            .bind(*filename)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("Failed to record migration {}: {}", filename, e));

        println!("Migration applied: {}", filename);
    }
}
