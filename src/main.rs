#![recursion_limit = "512"]

#[cfg(feature = "ssr")]
use utoipa::OpenApi;

#[cfg(feature = "ssr")]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Koentji API",
        version = "1.0.0",
        description = "Public API for authenticating Koentji API keys"
    ),
    paths(auth_endpoint),
    components(schemas(AuthRequest, AuthResponse, AuthResponseData, AuthError))
)]
struct ApiDoc;

#[cfg(feature = "ssr")]
#[derive(utoipa::ToSchema, serde::Deserialize)]
struct AuthRequest {
    /// The API key to authenticate
    auth_key: String,
    /// The device ID associated with the key
    auth_device: String,
    /// Number of rate limit units to consume (default: 1)
    #[serde(default = "default_rate_limit_usage")]
    rate_limit_usage: i32,
}

#[cfg(feature = "ssr")]
fn default_rate_limit_usage() -> i32 {
    1
}

#[cfg(feature = "ssr")]
#[derive(utoipa::ToSchema, serde::Serialize)]
struct AuthResponse {
    status: String,
    data: AuthResponseData,
}

#[cfg(feature = "ssr")]
#[derive(utoipa::ToSchema, serde::Serialize)]
struct AuthResponseData {
    key: String,
    device: String,
    subscription: Option<String>,
    username: Option<String>,
    email: Option<String>,
    valid_until: Option<String>,
    rate_limit_remaining: i32,
}

#[cfg(feature = "ssr")]
#[derive(utoipa::ToSchema, serde::Serialize)]
struct AuthError {
    error: serde_json::Value,
    message: String,
}

#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("run-migrations") {
        dotenvy::dotenv().ok();
        let pool = koentji_lab::db::create_pool().await;
        koentji_lab::db::run_migrations(&pool).await;
        println!("All migrations done.");
        return Ok(());
    }

    use actix_files::Files;
    use actix_session::config::{CookieContentSecurity, PersistentSession};
    use actix_session::storage::CookieSessionStore;
    use actix_session::SessionMiddleware;
    use actix_web::cookie::time::Duration;
    use actix_web::cookie::Key;
    use actix_web::*;
    use koentji_lab::app::*;
    use leptos::config::get_configuration;
    use leptos::prelude::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use leptos_meta::MetaTags;
    use utoipa_swagger_ui::SwaggerUi;

    dotenvy::dotenv().ok();

    let pool = koentji_lab::db::create_pool().await;
    koentji_lab::db::run_migrations(&pool).await;

    let cache_ttl: u64 = std::env::var("AUTH_CACHE_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(900); // 15 minutes default
    let auth_cache = std::sync::Arc::new(koentji_lab::cache::AuthCache::new(cache_ttl));
    koentji_lab::server::key_service::set_global_auth_cache(auth_cache.clone());

    let secret_key = std::env::var("SECRET_KEY").unwrap_or_else(|_| {
        "a-very-secret-key-that-should-be-at-least-64-bytes-long-for-security-purposes-change-me"
            .to_string()
    });
    let cookie_key = Key::from(secret_key.as_bytes());

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;

    println!("listening on http://{}", &addr);

    HttpServer::new(move || {
        let routes = generate_route_list(App);
        let leptos_options = &conf.leptos_options;
        let site_root = leptos_options.site_root.clone().to_string();
        let pool = pool.clone();
        let auth_cache = auth_cache.clone();

        actix_web::App::new()
            .app_data(web::Data::from(auth_cache))
            .service(auth_endpoint)
            .service(
                web::resource("/docs").route(web::get().to(|| async {
                    actix_web::HttpResponse::PermanentRedirect()
                        .append_header(("Location", "/docs/"))
                        .finish()
                })),
            )
            .service(
                SwaggerUi::new("/docs/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), cookie_key.clone())
                    .cookie_name("koentjilab_session".to_string())
                    .cookie_http_only(true)
                    .cookie_same_site(actix_web::cookie::SameSite::Lax)
                    .cookie_content_security(CookieContentSecurity::Private)
                    .session_lifecycle(
                        PersistentSession::default()
                            .session_ttl(Duration::hours(24)),
                    )
                    .build(),
            )
            .app_data(web::Data::new(pool))
            .service(Files::new("/pkg", format!("{site_root}/pkg")))
            .service(Files::new("/assets", &site_root))
            .service(favicon)
            .leptos_routes(routes, {
                let leptos_options = leptos_options.clone();
                move || {
                    view! {
                        <!DOCTYPE html>
                        <html lang="en">
                            <head>
                                <meta charset="utf-8"/>
                                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                                <AutoReload options=leptos_options.clone() />
                                <HydrationScripts options=leptos_options.clone()/>
                                <MetaTags/>
                            </head>
                            <body class="bg-gray-50 min-h-screen">
                                <App/>
                            </body>
                        </html>
                    }
                }
            })
            .app_data(web::Data::new(leptos_options.to_owned()))
    })
    .bind(&addr)?
    .run()
    .await
}

#[cfg(feature = "ssr")]
#[utoipa::path(
    post,
    path = "/v1/auth",
    tag = "Authentication",
    request_body = AuthRequest,
    responses(
        (status = 200, description = "Authentication successful", body = AuthResponse),
        (status = 401, description = "Invalid, revoked, or expired key", body = AuthError),
        (status = 429, description = "Rate limit exceeded", body = AuthError),
        (status = 500, description = "Internal server error", body = AuthError),
    )
)]
#[actix_web::post("/v1/auth")]
async fn auth_endpoint(
    body: actix_web::web::Json<AuthRequest>,
    pool: actix_web::web::Data<sqlx::PgPool>,
    auth_cache: actix_web::web::Data<std::sync::Arc<koentji_lab::cache::AuthCache>>,
) -> actix_web::HttpResponse {
    use actix_web::http::StatusCode;
    use chrono::Utc;
    use serde_json::json;

    let free_trial_key = std::env::var("FREE_TRIAL_KEY")
        .unwrap_or_else(|_| "FREE_TRIAL_SERPUL_PINTAR".to_string());

    // 1. Check cache first
    let cached = auth_cache.get(&body.auth_key, &body.auth_device).await;

    let (key, interval_seconds) = if let Some(entry) = cached {
        (entry.key_data, entry.interval_seconds)
    } else {
        // Cache miss — query DB with JOIN for interval
        let row = sqlx::query_as::<_, AuthKeyWithInterval>(
            r#"SELECT ak.*, COALESCE(rli.duration_seconds, 86400) as interval_seconds
               FROM authentication_keys ak
               LEFT JOIN rate_limit_intervals rli ON ak.rate_limit_interval_id = rli.id
               WHERE ak.key = $1 AND ak.device_id = $2"#,
        )
        .bind(&body.auth_key)
        .bind(&body.auth_device)
        .fetch_optional(pool.get_ref())
        .await;

        match row {
            Err(_) => {
                return actix_web::HttpResponse::InternalServerError().json(json!({
                    "error": { "en": "Internal server error." },
                    "message": "Internal server error."
                }));
            }
            Ok(Some(r)) => {
                let key = r.to_auth_key();
                let interval_seconds = r.interval_seconds;
                // Populate cache
                auth_cache
                    .insert(
                        &body.auth_key,
                        &body.auth_device,
                        koentji_lab::cache::CachedAuthEntry {
                            key_data: key.clone(),
                            interval_seconds,
                            cached_at: Utc::now(),
                        },
                    )
                    .await;
                (key, interval_seconds)
            }
            Ok(None) => {
                // Attempt free trial upsert
                let upserted = try_upsert_free_trial(
                    pool.get_ref(),
                    &body.auth_key,
                    &body.auth_device,
                    &free_trial_key,
                )
                .await;

                match upserted {
                    Ok(Some(k)) => (k, 86400i64), // free trial defaults to daily
                    _ => {
                        return actix_web::HttpResponse::build(StatusCode::UNAUTHORIZED).json(json!({
                            "error": {
                                "en": "Authentication key invalid or not exists in our system.",
                                "id": "Authentication key tidak valid atau tidak ditemukan di sistem kami."
                            },
                            "message": "Authentication key invalid or not exists in our system."
                        }));
                    }
                }
            }
        }
    };

    // 2. Check revoked
    if key.deleted_at.is_some() {
        let deleted_at = key.deleted_at.unwrap().to_rfc3339();
        return actix_web::HttpResponse::build(StatusCode::UNAUTHORIZED).json(json!({
            "error": {
                "en": format!("Authentication key already revoked and can't be used since {}.", deleted_at),
                "id": format!("Authentication key sudah tidak bisa digunakan sejak {}.", deleted_at)
            },
            "message": format!("Authentication key already revoked and can't be used since {}.", deleted_at)
        }));
    }

    // 3. Check expired
    if key.is_expired() {
        let expired_at = key.expired_at.unwrap().to_rfc3339();
        return actix_web::HttpResponse::build(StatusCode::UNAUTHORIZED).json(json!({
            "error": {
                "en": format!("Authentication key expired and need renewal per {}.", expired_at),
                "id": format!("Authentication key kadaluwarsa dan butuh pembaruan per tanggal {}.", expired_at)
            },
            "message": format!("Authentication key expired and need renewal per {}.", expired_at)
        }));
    }

    // 4. Compute new rate limit (reset based on interval)
    let now = Utc::now();
    let should_reset = match key.rate_limit_updated_at {
        None => true,
        Some(updated_at) => (now - updated_at).num_seconds() >= interval_seconds,
    };
    let new_remaining = if should_reset {
        key.rate_limit_daily - body.rate_limit_usage
    } else {
        key.rate_limit_remaining - body.rate_limit_usage
    };

    if new_remaining <= 0 {
        return actix_web::HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).json(json!({
            "error": {
                "en": "Rate limit exceeded. Please try again later or upgrade your subscription.",
                "id": "Batas rate limit terlampaui. Silakan coba lagi nanti atau upgrade langganan Anda."
            },
            "message": "Rate limit exceeded. Please try again later or upgrade your subscription."
        }));
    }

    // 5. Update rate limit in DB (write-through)
    let update_result = sqlx::query(
        "UPDATE authentication_keys SET rate_limit_remaining = $1, rate_limit_updated_at = $2 WHERE key = $3 AND device_id = $4",
    )
    .bind(new_remaining)
    .bind(now)
    .bind(&body.auth_key)
    .bind(&body.auth_device)
    .execute(pool.get_ref())
    .await;

    if update_result.is_err() {
        return actix_web::HttpResponse::InternalServerError().json(json!({
            "error": { "en": "Internal server error." },
            "message": "Internal server error."
        }));
    }

    // Update cache with new rate limit values
    auth_cache
        .insert(
            &body.auth_key,
            &body.auth_device,
            koentji_lab::cache::CachedAuthEntry {
                key_data: koentji_lab::models::AuthenticationKey {
                    rate_limit_remaining: new_remaining,
                    rate_limit_updated_at: Some(now),
                    ..key.clone()
                },
                interval_seconds,
                cached_at: Utc::now(),
            },
        )
        .await;

    // 6. Return success
    actix_web::HttpResponse::Ok().json(json!({
        "status": "success",
        "data": {
            "key": key.key,
            "device": key.device_id,
            "subscription": key.subscription,
            "username": key.username,
            "email": key.email,
            "valid_until": key.expired_at.map(|d| d.to_rfc3339()),
            "rate_limit_remaining": new_remaining,
        }
    }))
}

/// Helper struct for JOIN query that includes interval_seconds
#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow)]
struct AuthKeyWithInterval {
    pub id: i32,
    pub key: String,
    pub device_id: String,
    pub subscription: Option<String>,
    pub rate_limit_daily: i32,
    pub rate_limit_remaining: i32,
    pub rate_limit_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub deleted_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub expired_at: Option<chrono::DateTime<chrono::Utc>>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub subscription_type_id: Option<i32>,
    pub rate_limit_interval_id: Option<i32>,
    pub interval_seconds: i64,
}

#[cfg(feature = "ssr")]
impl AuthKeyWithInterval {
    fn to_auth_key(&self) -> koentji_lab::models::AuthenticationKey {
        koentji_lab::models::AuthenticationKey {
            id: self.id,
            key: self.key.clone(),
            device_id: self.device_id.clone(),
            subscription: self.subscription.clone(),
            rate_limit_daily: self.rate_limit_daily,
            rate_limit_remaining: self.rate_limit_remaining,
            rate_limit_updated_at: self.rate_limit_updated_at,
            username: self.username.clone(),
            email: self.email.clone(),
            created_by: self.created_by.clone(),
            updated_by: self.updated_by.clone(),
            deleted_by: self.deleted_by.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            expired_at: self.expired_at,
            deleted_at: self.deleted_at,
            subscription_type_id: self.subscription_type_id,
            rate_limit_interval_id: self.rate_limit_interval_id,
        }
    }
}

/// Attempt to upsert a free trial record.
/// - If auth_key == FREE_TRIAL_KEY: insert new record for this device
/// - If auth_key exists (any device): update it to bind this device (free trial)
/// - Otherwise: return None (key not recognized)
#[cfg(feature = "ssr")]
async fn try_upsert_free_trial(
    pool: &sqlx::PgPool,
    auth_key: &str,
    device_id: &str,
    free_trial_key: &str,
) -> Result<Option<koentji_lab::models::AuthenticationKey>, sqlx::Error> {
    use chrono::{Datelike, Utc};

    let now = Utc::now();
    // Expire at the first day of next month
    let next_month = {
        let d = now.date_naive();
        let (y, m) = if d.month() == 12 {
            (d.year() + 1, 1u32)
        } else {
            (d.year(), d.month() + 1)
        };
        chrono::NaiveDate::from_ymd_opt(y, m, 1)
            .map(|nd| nd.and_hms_opt(0, 0, 0).unwrap())
            .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
    };

    if auth_key == free_trial_key {
        // Insert new free trial row for this device
        let row = sqlx::query_as::<_, koentji_lab::models::AuthenticationKey>(
            r#"INSERT INTO authentication_keys
                (key, device_id, subscription, created_by, created_at, updated_at, expired_at, rate_limit_daily, rate_limit_remaining)
               VALUES ($1, $2, 'free_trial', 'system', $3, $3, $4, 6000, 6000)
               RETURNING *"#,
        )
        .bind(auth_key)
        .bind(device_id)
        .bind(now)
        .bind(next_month)
        .fetch_optional(pool)
        .await?;
        return Ok(row);
    }

    // Check if the key exists under any device
    let exists: Option<(i32,)> = sqlx::query_as(
        "SELECT id FROM authentication_keys WHERE key = $1 LIMIT 1",
    )
    .bind(auth_key)
    .fetch_optional(pool)
    .await?;

    if exists.is_some() {
        // Bind this device to the existing key (free trial)
        let row = sqlx::query_as::<_, koentji_lab::models::AuthenticationKey>(
            r#"UPDATE authentication_keys
               SET device_id = $1, subscription = 'free_trial', created_by = 'system',
                   created_at = $2, expired_at = $3
               WHERE key = $4
               RETURNING *"#,
        )
        .bind(device_id)
        .bind(now)
        .bind(next_month)
        .bind(auth_key)
        .fetch_optional(pool)
        .await?;
        return Ok(row);
    }

    Ok(None)
}

#[cfg(feature = "ssr")]
#[actix_web::get("favicon.ico")]
async fn favicon(
    leptos_options: actix_web::web::Data<leptos::config::LeptosOptions>,
) -> actix_web::Result<actix_files::NamedFile> {
    let leptos_options = leptos_options.into_inner();
    let site_root = &leptos_options.site_root;
    Ok(actix_files::NamedFile::open(format!(
        "{site_root}/favicon.ico"
    ))?)
}

#[cfg(not(any(feature = "ssr", feature = "csr")))]
pub fn main() {}

#[cfg(all(not(feature = "ssr"), feature = "csr"))]
pub fn main() {
    use koentji_lab::app::*;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
