use crate::models::*;
use leptos::prelude::*;

#[server]
pub async fn list_keys(
    page: i32,
    search: String,
    subscription: String,
    status: String,
) -> Result<KeyListResponse, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;
    let per_page = 20;
    let offset = (page - 1) * per_page;

    let mut conditions = vec!["1=1".to_string()];
    let mut params: Vec<String> = vec![];

    if !search.is_empty() {
        params.push(format!("%{}%", search));
        let idx = params.len();
        conditions.push(format!(
            "(device_id ILIKE ${idx} OR username ILIKE ${idx} OR email ILIKE ${idx})"
        ));
    }

    if !subscription.is_empty() {
        params.push(subscription.clone());
        let idx = params.len();
        conditions.push(format!("subscription = ${idx}"));
    }

    match status.as_str() {
        "active" => conditions
            .push("deleted_at IS NULL AND (expired_at IS NULL OR expired_at > NOW())".to_string()),
        "expired" => conditions.push(
            "expired_at IS NOT NULL AND expired_at <= NOW() AND deleted_at IS NULL".to_string(),
        ),
        "deleted" => conditions.push("deleted_at IS NOT NULL".to_string()),
        _ => {}
    }

    let where_clause = conditions.join(" AND ");

    // Build count query
    let count_sql = format!(
        "SELECT COUNT(*) as count FROM authentication_keys WHERE {}",
        where_clause
    );
    let list_sql = format!(
        "SELECT * FROM authentication_keys WHERE {} ORDER BY created_at DESC LIMIT {} OFFSET {}",
        where_clause, per_page, offset
    );

    // We need to use raw queries with dynamic params
    // For simplicity, build the query with sqlx::query_as
    let total: i64 = {
        let mut q = sqlx::query_scalar::<_, i64>(&count_sql);
        for p in &params {
            q = q.bind(p);
        }
        q.fetch_one(pool.get_ref()).await.map_err(|e| {
            log::error!("Failed to count keys: {}", e);
            ServerFnError::new(e.to_string())
        })?
    };

    let keys: Vec<AuthenticationKey> = {
        let mut q = sqlx::query_as::<_, AuthenticationKey>(&list_sql);
        for p in &params {
            q = q.bind(p);
        }
        q.fetch_all(pool.get_ref()).await.map_err(|e| {
            log::error!("Failed to list keys: {}", e);
            ServerFnError::new(e.to_string())
        })?
    };

    log::debug!("list_keys: page={}, total={}", page, total);
    Ok(KeyListResponse {
        keys,
        total,
        page,
        per_page,
    })
}

#[server]
pub async fn create_key(req: CreateKeyRequest) -> Result<AuthenticationKey, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;
    use std::sync::Arc;

    use crate::application::IssueKey;
    use crate::domain::authentication::{
        AuthKey, DeviceId, IssueKeyCommand, RateLimitAmount, SubscriptionName,
    };

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;
    let issue_key = extract::<actix_web::web::Data<Arc<IssueKey>>>().await?;

    let key_string = generate_api_key();

    // Look up subscription type to get defaults
    let (rate_limit, subscription_name, rate_limit_interval_id) =
        if let Some(st_id) = req.subscription_type_id {
            let st = sqlx::query_as::<_, crate::models::SubscriptionType>(
                "SELECT * FROM subscription_types WHERE id = $1",
            )
            .bind(st_id)
            .fetch_optional(pool.get_ref())
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

            match st {
                Some(st) => (
                    req.rate_limit_daily.unwrap_or(st.rate_limit_amount),
                    Some(st.name),
                    Some(st.rate_limit_interval_id),
                ),
                None => (
                    req.rate_limit_daily.unwrap_or(6000),
                    req.subscription.clone(),
                    None,
                ),
            }
        } else {
            (
                req.rate_limit_daily.unwrap_or(6000),
                req.subscription.clone(),
                None,
            )
        };

    let expired_at: Option<chrono::DateTime<chrono::Utc>> = req.expired_at.as_ref().and_then(|s| {
        if s.is_empty() {
            None
        } else {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
                .ok()
                .map(|ndt| ndt.and_utc())
        }
    });

    let key = AuthKey::parse(&key_string).map_err(|e| ServerFnError::new(e.to_string()))?;
    let device = DeviceId::parse(&req.device_id).map_err(|e| ServerFnError::new(e.to_string()))?;
    let subscription = match subscription_name.as_deref() {
        Some(s) if !s.is_empty() => {
            Some(SubscriptionName::parse(s).map_err(|e| ServerFnError::new(e.to_string()))?)
        }
        _ => None,
    };
    let rate_limit_daily =
        RateLimitAmount::new(rate_limit).map_err(|e| ServerFnError::new(e.to_string()))?;

    let command = IssueKeyCommand {
        key,
        device,
        subscription,
        subscription_type_id: req.subscription_type_id,
        rate_limit_daily,
        rate_limit_interval_id,
        username: req.username.clone(),
        email: req.email.clone(),
        expired_at,
        issued_by: "admin".to_string(),
    };

    let issued = issue_key.execute(command).await.map_err(|e| {
        log::error!("Failed to issue key: {}", e);
        ServerFnError::new(e.to_string())
    })?;

    // The server-fn contract still returns the full DB row (the frontend
    // expects timestamps / audit fields). Re-fetch by id — the domain
    // aggregate deliberately doesn't carry them.
    let created =
        sqlx::query_as::<_, AuthenticationKey>("SELECT * FROM authentication_keys WHERE id = $1")
            .bind(issued.id.value())
            .fetch_one(pool.get_ref())
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(created)
}

#[server]
pub async fn update_key(
    id: i32,
    req: UpdateKeyRequest,
) -> Result<AuthenticationKey, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    // Look up subscription type for name + interval if changing subscription
    let (subscription_name, rate_limit_interval_id) = if let Some(st_id) = req.subscription_type_id
    {
        let st = sqlx::query_as::<_, crate::models::SubscriptionType>(
            "SELECT * FROM subscription_types WHERE id = $1",
        )
        .bind(st_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        match st {
            Some(st) => (Some(st.name), Some(st.rate_limit_interval_id)),
            None => (req.subscription.clone(), None),
        }
    } else {
        (req.subscription.clone(), None)
    };

    let expired_at: Option<chrono::DateTime<chrono::Utc>> = req.expired_at.as_ref().and_then(|s| {
        if s.is_empty() {
            None
        } else {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
                .ok()
                .map(|ndt| ndt.and_utc())
        }
    });

    let updated = sqlx::query_as::<_, AuthenticationKey>(
        r#"UPDATE authentication_keys SET
            device_id = COALESCE($2, device_id),
            username = COALESCE($3, username),
            email = COALESCE($4, email),
            subscription = COALESCE($5, subscription),
            subscription_type_id = COALESCE($6, subscription_type_id),
            rate_limit_daily = COALESCE($7, rate_limit_daily),
            rate_limit_interval_id = COALESCE($8, rate_limit_interval_id),
            expired_at = $9,
            updated_by = 'admin',
            updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(id)
    .bind(&req.device_id)
    .bind(&req.username)
    .bind(&req.email)
    .bind(&subscription_name)
    .bind(req.subscription_type_id)
    .bind(req.rate_limit_daily)
    .bind(rate_limit_interval_id)
    .bind(expired_at)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| {
        log::error!("Failed to update key id={}: {}", id, e);
        ServerFnError::new(e.to_string())
    })?;

    log::info!("Key updated: id={}", id);
    // Invalidate auth cache for this key
    invalidate_cache_for_key(pool.get_ref(), id).await;

    Ok(updated)
}

#[cfg(feature = "ssr")]
async fn invalidate_cache_for_key(pool: &sqlx::PgPool, id: i32) {
    if let Ok((key, device_id)) = sqlx::query_as::<_, (String, String)>(
        "SELECT key, device_id FROM authentication_keys WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    {
        if let Some(cache) = GLOBAL_AUTH_CACHE.get() {
            cache.invalidate(&key, &device_id).await;
        }
    }
}

#[cfg(feature = "ssr")]
static GLOBAL_AUTH_CACHE: std::sync::OnceLock<std::sync::Arc<crate::cache::AuthCache>> =
    std::sync::OnceLock::new();

#[cfg(feature = "ssr")]
pub fn set_global_auth_cache(cache: std::sync::Arc<crate::cache::AuthCache>) {
    let _ = GLOBAL_AUTH_CACHE.set(cache);
}

#[server]
pub async fn delete_key(id: i32) -> Result<(), ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    sqlx::query(
        "UPDATE authentication_keys SET deleted_at = NOW(), deleted_by = 'admin' WHERE id = $1",
    )
    .bind(id)
    .execute(pool.get_ref())
    .await
    .map_err(|e| {
        log::error!("Failed to delete key id={}: {}", id, e);
        ServerFnError::new(e.to_string())
    })?;

    log::info!("Key revoked: id={}", id);
    invalidate_cache_for_key(pool.get_ref(), id).await;

    Ok(())
}

#[server]
pub async fn reveal_key(id: i32) -> Result<String, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let key: String = sqlx::query_scalar("SELECT key FROM authentication_keys WHERE id = $1")
        .bind(id)
        .fetch_one(pool.get_ref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(key)
}

#[server]
pub async fn reset_rate_limit(id: i32) -> Result<(), ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    sqlx::query(
        "UPDATE authentication_keys SET rate_limit_remaining = rate_limit_daily, rate_limit_updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(pool.get_ref())
    .await
    .map_err(|e| {
        log::error!("Failed to reset rate limit for key id={}: {}", id, e);
        ServerFnError::new(e.to_string())
    })?;

    log::info!("Rate limit reset: key id={}", id);
    invalidate_cache_for_key(pool.get_ref(), id).await;

    Ok(())
}

#[cfg(feature = "ssr")]
fn generate_api_key() -> String {
    use base64::Engine;
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    format!("klab_{}", token)
}
