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
        "active" => conditions.push("deleted_at IS NULL AND (expired_at IS NULL OR expired_at > NOW())".to_string()),
        "expired" => conditions.push("expired_at IS NOT NULL AND expired_at <= NOW() AND deleted_at IS NULL".to_string()),
        "deleted" => conditions.push("deleted_at IS NOT NULL".to_string()),
        _ => {}
    }

    let where_clause = conditions.join(" AND ");

    // Build count query
    let count_sql = format!("SELECT COUNT(*) as count FROM authentication_keys WHERE {}", where_clause);
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
        q.fetch_one(pool.get_ref()).await.map_err(|e| ServerFnError::new(e.to_string()))?
    };

    let keys: Vec<AuthenticationKey> = {
        let mut q = sqlx::query_as::<_, AuthenticationKey>(&list_sql);
        for p in &params {
            q = q.bind(p);
        }
        q.fetch_all(pool.get_ref()).await.map_err(|e| ServerFnError::new(e.to_string()))?
    };

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

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let key = generate_api_key();
    let rate_limit = req.rate_limit_daily.unwrap_or(6000);

    let expired_at: Option<chrono::DateTime<chrono::Utc>> = req
        .expired_at
        .as_ref()
        .and_then(|s| {
            if s.is_empty() {
                None
            } else {
                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
                    .ok()
                    .map(|ndt| ndt.and_utc())
            }
        });

    let created = sqlx::query_as::<_, AuthenticationKey>(
        r#"INSERT INTO authentication_keys (key, device_id, subscription, rate_limit_daily, rate_limit_remaining, username, email, expired_at, created_by)
           VALUES ($1, $2, $3, $4, $4, $5, $6, $7, $8)
           RETURNING *"#,
    )
    .bind(&key)
    .bind(&req.device_id)
    .bind(&req.subscription)
    .bind(rate_limit)
    .bind(&req.username)
    .bind(&req.email)
    .bind(expired_at)
    .bind("admin")
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(created)
}

#[server]
pub async fn update_key(id: i32, req: UpdateKeyRequest) -> Result<AuthenticationKey, ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    let expired_at: Option<chrono::DateTime<chrono::Utc>> = req
        .expired_at
        .as_ref()
        .and_then(|s| {
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
            rate_limit_daily = COALESCE($6, rate_limit_daily),
            expired_at = $7,
            updated_by = 'admin',
            updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(id)
    .bind(&req.device_id)
    .bind(&req.username)
    .bind(&req.email)
    .bind(&req.subscription)
    .bind(req.rate_limit_daily)
    .bind(expired_at)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(updated)
}

#[server]
pub async fn delete_key(id: i32) -> Result<(), ServerFnError> {
    use leptos_actix::extract;
    use sqlx::PgPool;

    let pool = extract::<actix_web::web::Data<PgPool>>().await?;

    sqlx::query("UPDATE authentication_keys SET deleted_at = NOW(), deleted_by = 'admin' WHERE id = $1")
        .bind(id)
        .execute(pool.get_ref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

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
    .map_err(|e| ServerFnError::new(e.to_string()))?;

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
