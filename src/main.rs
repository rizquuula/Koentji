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
        let pool = koentji::db::create_pool().await;
        koentji::db::run_migrations(&pool).await;
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
    use koentji::app::*;
    use leptos::config::get_configuration;
    use leptos::prelude::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};
    use leptos_meta::MetaTags;
    use utoipa_swagger_ui::SwaggerUi;

    dotenvy::dotenv().ok();
    env_logger::init();

    log::info!("Starting Koentji server...");

    let pool = koentji::db::create_pool().await;
    koentji::db::run_migrations(&pool).await;

    let cache_ttl: u64 = std::env::var("AUTH_CACHE_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(900); // 15 minutes default
    log::info!("Auth cache TTL: {}s", cache_ttl);
    let auth_cache = std::sync::Arc::new(koentji::cache::AuthCache::new(cache_ttl));
    koentji::server::key_service::set_global_auth_cache(auth_cache.clone());

    let secret_key = std::env::var("SECRET_KEY").unwrap_or_else(|_| {
        log::warn!("SECRET_KEY not set, using insecure default — set SECRET_KEY in production");
        "a-very-secret-key-that-should-be-at-least-64-bytes-long-for-security-purposes-change-me"
            .to_string()
    });
    let cookie_key = Key::from(secret_key.as_bytes());

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;

    log::info!("Listening on http://{}", &addr);

    let workers: usize = std::env::var("WORKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);

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
    .workers(workers)
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
    auth_cache: actix_web::web::Data<koentji::cache::AuthCache>,
) -> actix_web::HttpResponse {
    use actix_web::http::StatusCode;
    use chrono::Utc;
    use serde_json::json;

    let free_trial_key =
        std::env::var("FREE_TRIAL_KEY").unwrap_or_else(|_| "FREE_TRIAL".to_string());
    let free_trial_subscription_name =
        std::env::var("FREE_TRIAL_SUBSCRIPTION_NAME").unwrap_or_else(|_| "free".to_string());

    log::debug!("Auth request: device={}", body.auth_device);

    // 1. Check cache first
    let cached = auth_cache.get(&body.auth_key, &body.auth_device).await;

    let (key, interval_seconds) = if let Some(entry) = cached {
        log::debug!("Cache hit: device={}", body.auth_device);
        (entry.key_data, entry.interval_seconds)
    } else {
        log::debug!("Cache miss: device={}, querying DB", body.auth_device);
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
            Err(e) => {
                log::error!("DB query failed for device={}: {}", body.auth_device, e);
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
                        koentji::cache::CachedAuthEntry {
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
                    &free_trial_subscription_name,
                )
                .await;

                match upserted {
                    Ok(Some((k, interval_secs))) => (k, interval_secs),
                    Ok(None) => {
                        log::warn!("Auth failed - unknown key for device={}", body.auth_device);
                        return actix_web::HttpResponse::build(StatusCode::UNAUTHORIZED).json(json!({
                            "error": {
                                "en": "Authentication key invalid or not exists in our system.",
                                "id": "Authentication key tidak valid atau tidak ditemukan di sistem kami."
                            },
                            "message": "Authentication key invalid or not exists in our system."
                        }));
                    }
                    Err(e) => {
                        log::error!(
                            "Free trial upsert failed for device={}: {}",
                            body.auth_device,
                            e
                        );
                        return actix_web::HttpResponse::InternalServerError().json(json!({
                            "error": { "en": "Internal server error." },
                            "message": "Internal server error."
                        }));
                    }
                }
            }
        }
    };

    // 2. Check revoked
    if key.deleted_at.is_some() {
        let deleted_at = key.deleted_at.unwrap().to_rfc3339();
        log::warn!(
            "Auth failed - revoked key: device={}, revoked_at={}",
            key.device_id,
            deleted_at
        );
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
        log::warn!(
            "Auth failed - expired key: device={}, expired_at={}",
            key.device_id,
            expired_at
        );
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
        log::warn!(
            "Auth failed - rate limit exceeded: device={}, subscription={:?}",
            key.device_id,
            key.subscription
        );
        return actix_web::HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).json(json!({
            "error": {
                "en": "Rate limit exceeded. Please try again later or upgrade your subscription.",
                "id": "Batas rate limit terlampaui. Silakan coba lagi nanti atau upgrade langganan Anda."
            },
            "message": "Rate limit exceeded. Please try again later or upgrade your subscription."
        }));
    }

    // 5. Update rate limit in DB (fire-and-forget — don't block the response)
    {
        let pool = pool.clone();
        let auth_key = body.auth_key.clone();
        let auth_device = body.auth_device.clone();
        let device_id = key.device_id.clone();
        actix_web::rt::spawn(async move {
            if let Err(e) = sqlx::query(
                "UPDATE authentication_keys SET rate_limit_remaining = $1, rate_limit_updated_at = $2 WHERE key = $3 AND device_id = $4",
            )
            .bind(new_remaining)
            .bind(now)
            .bind(&auth_key)
            .bind(&auth_device)
            .execute(pool.get_ref())
            .await
            {
                log::error!("Failed to update rate limit for device={}: {}", device_id, e);
            }
        });
    }

    // Update cache with new rate limit values
    auth_cache
        .insert(
            &body.auth_key,
            &body.auth_device,
            koentji::cache::CachedAuthEntry {
                key_data: koentji::models::AuthenticationKey {
                    rate_limit_remaining: new_remaining,
                    rate_limit_updated_at: Some(now),
                    ..key.clone()
                },
                interval_seconds,
                cached_at: Utc::now(),
            },
        )
        .await;

    log::info!(
        "Auth success: device={}, subscription={:?}, remaining={}",
        key.device_id,
        key.subscription,
        new_remaining
    );

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
    fn to_auth_key(&self) -> koentji::models::AuthenticationKey {
        koentji::models::AuthenticationKey {
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

/// Attempt to upsert a free trial record or bind a device to an existing key.
/// - If auth_key == FREE_TRIAL_KEY: insert new free trial record for this device
/// - If auth_key exists (any device) with device_id "-": bind this device to it
/// - Otherwise: return None (key not recognized)
#[cfg(feature = "ssr")]
async fn try_upsert_free_trial(
    pool: &sqlx::PgPool,
    auth_key: &str,
    device_id: &str,
    free_trial_key: &str,
    free_trial_subscription_name: &str,
) -> Result<Option<(koentji::models::AuthenticationKey, i64)>, sqlx::Error> {
    use chrono::{Datelike, Utc};

    if auth_key == free_trial_key {
        // Look up the subscription type for free trial
        let sub = sqlx::query_as::<_, koentji::models::SubscriptionType>(
            "SELECT * FROM subscription_types WHERE name = $1 AND is_active = true LIMIT 1",
        )
        .bind(free_trial_subscription_name)
        .fetch_optional(pool)
        .await?;

        let (sub_name, sub_type_id, rate_limit, interval_id, interval_seconds) = if let Some(s) =
            sub
        {
            // Look up the interval duration
            let dur: Option<(i64,)> =
                sqlx::query_as("SELECT duration_seconds FROM rate_limit_intervals WHERE id = $1")
                    .bind(s.rate_limit_interval_id)
                    .fetch_optional(pool)
                    .await?;
            (
                s.name,
                Some(s.id),
                s.rate_limit_amount,
                Some(s.rate_limit_interval_id),
                dur.map(|d| d.0).unwrap_or(86400),
            )
        } else {
            log::warn!(
                "Free trial subscription type '{}' not found or inactive, using hardcoded fallback (6000 daily)",
                free_trial_subscription_name
            );
            ("free_trial".to_string(), None, 6000, None, 86400i64)
        };

        let now = Utc::now();
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

        let row = sqlx::query_as::<_, koentji::models::AuthenticationKey>(
            r#"INSERT INTO authentication_keys
                (key, device_id, subscription, subscription_type_id, rate_limit_interval_id,
                 created_by, created_at, updated_at, expired_at, rate_limit_daily, rate_limit_remaining)
               VALUES ($1, $2, $3, $4, $5, 'system', $6, $6, $7, $8, $8)
               RETURNING *"#,
        )
        .bind(auth_key)
        .bind(device_id)
        .bind(&sub_name)
        .bind(sub_type_id)
        .bind(interval_id)
        .bind(now)
        .bind(next_month)
        .bind(rate_limit)
        .fetch_optional(pool)
        .await?;

        if row.is_some() {
            log::info!(
                "Free trial created: device={}, subscription={}",
                device_id,
                sub_name
            );
        }
        return Ok(row.map(|r| (r, interval_seconds)));
    }

    // Check if the key exists with a placeholder device_id ("-")
    let exists: Option<(i32,)> = sqlx::query_as(
        "SELECT id FROM authentication_keys WHERE key = $1 AND device_id = '-' LIMIT 1",
    )
    .bind(auth_key)
    .fetch_optional(pool)
    .await?;

    if exists.is_some() {
        // Bind this device to the existing key (only update device_id)
        let row = sqlx::query_as::<_, AuthKeyWithInterval>(
            r#"UPDATE authentication_keys ak
               SET device_id = $1, updated_at = NOW()
               FROM (SELECT COALESCE(rli.duration_seconds, 86400) as interval_seconds
                     FROM authentication_keys ak2
                     LEFT JOIN rate_limit_intervals rli ON ak2.rate_limit_interval_id = rli.id
                     WHERE ak2.key = $2 AND ak2.device_id = '-' LIMIT 1) sub
               WHERE ak.key = $2 AND ak.device_id = '-'
               RETURNING ak.*, sub.interval_seconds"#,
        )
        .bind(device_id)
        .bind(auth_key)
        .fetch_optional(pool)
        .await?;

        if row.is_some() {
            log::info!("Device bound to existing key: device={}", device_id);
        }
        return Ok(row.map(|r| (r.to_auth_key(), r.interval_seconds)));
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
    use koentji::app::*;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
