use crate::models::*;
use leptos::prelude::*;

#[server]
pub async fn list_subscription_types() -> Result<Vec<SubscriptionType>, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let types = sqlx::query_as::<_, SubscriptionType>(
        "SELECT * FROM subscription_types WHERE is_active = true ORDER BY rate_limit_amount ASC",
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(types)
}

#[server]
pub async fn list_all_subscription_types() -> Result<Vec<SubscriptionType>, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let types = sqlx::query_as::<_, SubscriptionType>(
        "SELECT * FROM subscription_types ORDER BY rate_limit_amount ASC",
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(types)
}

#[server]
pub async fn create_subscription_type(
    req: CreateSubscriptionTypeRequest,
) -> Result<SubscriptionType, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let created = sqlx::query_as::<_, SubscriptionType>(
        r#"INSERT INTO subscription_types (name, display_name, rate_limit_amount, rate_limit_interval_id)
           VALUES ($1, $2, $3, $4)
           RETURNING *"#,
    )
    .bind(&req.name)
    .bind(&req.display_name)
    .bind(req.rate_limit_amount)
    .bind(req.rate_limit_interval_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| {
        log::error!("Failed to create subscription type '{}': {}", req.name, e);
        ServerFnError::new(e.to_string())
    })?;

    log::info!(
        "Subscription type created: id={}, name={}",
        created.id,
        created.name
    );
    Ok(created)
}

#[server]
pub async fn update_subscription_type(
    id: i32,
    req: UpdateSubscriptionTypeRequest,
) -> Result<SubscriptionType, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let updated = sqlx::query_as::<_, SubscriptionType>(
        r#"UPDATE subscription_types SET
            name = COALESCE($2, name),
            display_name = COALESCE($3, display_name),
            rate_limit_amount = COALESCE($4, rate_limit_amount),
            rate_limit_interval_id = COALESCE($5, rate_limit_interval_id),
            is_active = COALESCE($6, is_active),
            updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.display_name)
    .bind(req.rate_limit_amount)
    .bind(req.rate_limit_interval_id)
    .bind(req.is_active)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| {
        log::error!("Failed to update subscription type id={}: {}", id, e);
        ServerFnError::new(e.to_string())
    })?;

    log::info!("Subscription type updated: id={}", id);
    Ok(updated)
}

#[server]
pub async fn delete_subscription_type(id: i32) -> Result<(), ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    // Check if any keys reference this subscription type
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM authentication_keys WHERE subscription_type_id = $1",
    )
    .bind(id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    if count > 0 {
        log::warn!(
            "Cannot delete subscription type id={} — in use by {} key(s)",
            id,
            count
        );
        return Err(ServerFnError::new(
            "Cannot delete subscription type that is in use by API keys. Deactivate it instead.",
        ));
    }

    sqlx::query("DELETE FROM subscription_types WHERE id = $1")
        .bind(id)
        .execute(pool.get_ref())
        .await
        .map_err(|e| {
            log::error!("Failed to delete subscription type id={}: {}", id, e);
            ServerFnError::new(e.to_string())
        })?;

    log::info!("Subscription type deleted: id={}", id);
    Ok(())
}
