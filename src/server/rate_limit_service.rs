use crate::models::*;
use leptos::prelude::*;

#[server]
pub async fn list_rate_limit_intervals() -> Result<Vec<RateLimitInterval>, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let intervals = sqlx::query_as::<_, RateLimitInterval>(
        "SELECT * FROM rate_limit_intervals WHERE is_active = true ORDER BY duration_seconds ASC",
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(intervals)
}

#[server]
pub async fn list_all_rate_limit_intervals() -> Result<Vec<RateLimitInterval>, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let intervals = sqlx::query_as::<_, RateLimitInterval>(
        "SELECT * FROM rate_limit_intervals ORDER BY duration_seconds ASC",
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(intervals)
}

#[server]
pub async fn create_rate_limit_interval(
    req: CreateRateLimitIntervalRequest,
) -> Result<RateLimitInterval, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let created = sqlx::query_as::<_, RateLimitInterval>(
        r#"INSERT INTO rate_limit_intervals (name, display_name, duration_seconds)
           VALUES ($1, $2, $3)
           RETURNING *"#,
    )
    .bind(&req.name)
    .bind(&req.display_name)
    .bind(req.duration_seconds)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| {
        log::error!("Failed to create rate limit interval '{}': {}", req.name, e);
        ServerFnError::new(e.to_string())
    })?;

    log::info!(
        "Rate limit interval created: id={}, name={}",
        created.id,
        created.name
    );
    Ok(created)
}

#[server]
pub async fn update_rate_limit_interval(
    id: i32,
    req: UpdateRateLimitIntervalRequest,
) -> Result<RateLimitInterval, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let updated = sqlx::query_as::<_, RateLimitInterval>(
        r#"UPDATE rate_limit_intervals SET
            name = COALESCE($2, name),
            display_name = COALESCE($3, display_name),
            duration_seconds = COALESCE($4, duration_seconds),
            is_active = COALESCE($5, is_active),
            updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.display_name)
    .bind(req.duration_seconds)
    .bind(req.is_active)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| {
        log::error!("Failed to update rate limit interval id={}: {}", id, e);
        ServerFnError::new(e.to_string())
    })?;

    log::info!("Rate limit interval updated: id={}", id);
    Ok(updated)
}

#[server]
pub async fn delete_rate_limit_interval(id: i32) -> Result<(), ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    // Check if any subscription types reference this interval
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscription_types WHERE rate_limit_interval_id = $1",
    )
    .bind(id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    if count > 0 {
        log::warn!(
            "Cannot delete rate limit interval id={} — in use by {} subscription type(s)",
            id,
            count
        );
        return Err(ServerFnError::new(
            "Cannot delete interval that is in use by subscription types. Deactivate it instead.",
        ));
    }

    sqlx::query("DELETE FROM rate_limit_intervals WHERE id = $1")
        .bind(id)
        .execute(pool.get_ref())
        .await
        .map_err(|e| {
            log::error!("Failed to delete rate limit interval id={}: {}", id, e);
            ServerFnError::new(e.to_string())
        })?;

    log::info!("Rate limit interval deleted: id={}", id);
    Ok(())
}
