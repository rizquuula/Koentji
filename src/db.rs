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

pub async fn run_migrations(pool: &PgPool) {
    let migration_sql = include_str!("../migrations/001_create_auth_keys.sql");
    sqlx::raw_sql(migration_sql)
        .execute(pool)
        .await
        .expect("Failed to run migrations");
}
