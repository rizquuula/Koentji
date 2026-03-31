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

        actix_web::App::new()
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
) -> actix_web::HttpResponse {
    use actix_web::http::StatusCode;
    use chrono::Utc;
    use serde_json::json;

    let free_trial_key = std::env::var("FREE_TRIAL_KEY")
        .unwrap_or_else(|_| "FREE_TRIAL_SERPUL_PINTAR".to_string());

    // 1. Look up key by auth_key + auth_device
    let row = sqlx::query_as::<_, koentji_lab::models::AuthenticationKey>(
        "SELECT * FROM authentication_keys WHERE key = $1 AND device_id = $2",
    )
    .bind(&body.auth_key)
    .bind(&body.auth_device)
    .fetch_optional(pool.get_ref())
    .await;

    let key = match row {
        Err(_) => {
            return actix_web::HttpResponse::InternalServerError().json(json!({
                "error": { "en": "Internal server error." },
                "message": "Internal server error."
            }));
        }
        Ok(Some(k)) => k,
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
                Ok(Some(k)) => k,
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

    // 4. Compute new rate limit (reset daily if needed)
    let now = Utc::now();
    let new_remaining = match key.rate_limit_updated_at {
        None => key.rate_limit_daily - body.rate_limit_usage,
        Some(updated_at) if updated_at.date_naive() < now.date_naive() => {
            key.rate_limit_daily - body.rate_limit_usage
        }
        Some(_) => key.rate_limit_remaining - body.rate_limit_usage,
    };

    if new_remaining <= 0 {
        return actix_web::HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).json(json!({
            "error": {
                "en": "Rate limit exceeded. Please try again tomorrow or upgrade your subscription.",
                "id": "Batas rate limit terlampaui. Silakan coba lagi besok atau upgrade langganan Anda."
            },
            "message": "Rate limit exceeded. Please try again tomorrow or upgrade your subscription."
        }));
    }

    // 5. Update rate limit in DB
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
