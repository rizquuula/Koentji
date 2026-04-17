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
    use std::sync::Arc;

    use crate::domain::authentication::{AuthCachePort, AuthKey, DeviceId};

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;
    let cache = extract::<actix_web::web::Data<Arc<dyn AuthCachePort>>>().await?;

    // Snapshot the pre-update (key, device_id) so we can evict the
    // *previous* cache entry if the device was reassigned — the
    // legacy helper only ever evicted the post-update device, which
    // left the prior entry stale (B9).
    let previous: Option<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT key, device_id FROM authentication_keys WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

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

    // Evict the prior `(key, old_device)` entry always (any field
    // change stales the cached snapshot); when the device was
    // reassigned, also evict the new `(key, new_device)` entry in
    // case something raced and populated it post-UPDATE.
    if let Some((raw_prev_key, raw_prev_device)) = previous {
        let prev_key = AuthKey::parse(raw_prev_key.clone())
            .map_err(|e| ServerFnError::new(format!("stored key: {:?}", e)))?;
        let prev_device = DeviceId::parse(raw_prev_device.clone())
            .map_err(|e| ServerFnError::new(format!("stored device: {:?}", e)))?;
        cache.invalidate(&prev_key, &prev_device).await;

        if raw_prev_device != updated.device_id {
            let new_device = DeviceId::parse(updated.device_id.clone())
                .map_err(|e| ServerFnError::new(format!("stored device: {:?}", e)))?;
            cache.invalidate(&prev_key, &new_device).await;
        }
    }

    Ok(updated)
}

#[server]
pub async fn delete_key(id: i32) -> Result<(), ServerFnError> {
    use leptos_actix::extract;
    use std::sync::Arc;

    use crate::application::RevokeKey;
    use crate::domain::authentication::IssuedKeyId;

    let revoke = extract::<actix_web::web::Data<Arc<RevokeKey>>>().await?;

    revoke
        .execute(IssuedKeyId::new(id), "admin")
        .await
        .map_err(|e| {
            log::error!("Failed to revoke key id={}: {}", id, e);
            ServerFnError::new(e.to_string())
        })?;

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
    use std::sync::Arc;

    use crate::application::ResetRateLimit;
    use crate::domain::authentication::IssuedKeyId;

    let reset = extract::<actix_web::web::Data<Arc<ResetRateLimit>>>().await?;

    reset
        .execute(IssuedKeyId::new(id), chrono::Utc::now(), "admin")
        .await
        .map_err(|e| {
            log::error!("Failed to reset rate limit for key id={}: {}", id, e);
            ServerFnError::new(e.to_string())
        })?;

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
