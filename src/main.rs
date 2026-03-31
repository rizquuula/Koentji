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
            .service(validate_key)
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
#[actix_web::get("/api/validate")]
async fn validate_key(
    req: actix_web::HttpRequest,
    pool: actix_web::web::Data<sqlx::PgPool>,
) -> actix_web::HttpResponse {
    use actix_web::http::StatusCode;
    use serde_json::json;

    let api_key = match req.headers().get("X-API-Key").and_then(|v| v.to_str().ok()) {
        Some(k) => k.to_string(),
        None => {
            return actix_web::HttpResponse::build(StatusCode::UNAUTHORIZED).json(
                json!({ "error": "missing_api_key", "message": "X-API-Key header is required" }),
            );
        }
    };

    let row = sqlx::query_as::<_, koentji_lab::models::AuthenticationKey>(
        "SELECT * FROM authentication_keys WHERE key = $1 AND deleted_at IS NULL",
    )
    .bind(&api_key)
    .fetch_optional(pool.get_ref())
    .await;

    match row {
        Ok(Some(key)) => {
            if key.is_expired() {
                return actix_web::HttpResponse::build(StatusCode::UNAUTHORIZED).json(
                    json!({ "error": "key_expired", "message": "This API key has expired" }),
                );
            }
            if key.rate_limit_remaining <= 0 {
                return actix_web::HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).json(
                    json!({
                        "error": "rate_limit_exceeded",
                        "message": "Daily rate limit reached",
                        "rate_limit_remaining": 0
                    }),
                );
            }
            actix_web::HttpResponse::Ok().json(json!({
                "valid": true,
                "subscription": key.subscription,
                "rate_limit_daily": key.rate_limit_daily,
                "rate_limit_remaining": key.rate_limit_remaining,
            }))
        }
        Ok(None) => actix_web::HttpResponse::build(StatusCode::UNAUTHORIZED).json(
            json!({ "error": "invalid_api_key", "message": "The provided API key is not valid" }),
        ),
        Err(_) => actix_web::HttpResponse::InternalServerError()
            .json(json!({ "error": "server_error", "message": "Internal server error" })),
    }
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
