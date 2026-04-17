#![recursion_limit = "512"]

#[cfg(feature = "ssr")]
use utoipa::OpenApi;

#[cfg(feature = "ssr")]
use koentji::interface::http::auth_endpoint::{
    auth_endpoint, AuthError, AuthRequest, AuthResponse, AuthResponseData,
};

#[cfg(feature = "ssr")]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Koentji API",
        version = "1.0.0",
        description = "Public API for authenticating Koentji API keys"
    ),
    paths(koentji::interface::http::auth_endpoint::auth_endpoint),
    components(schemas(AuthRequest, AuthResponse, AuthResponseData, AuthError))
)]
struct ApiDoc;

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

    use koentji::application::{
        AuthenticateApiKey, ExtendExpiration, IssueKey, ReassignDevice, ResetRateLimit, RevokeKey,
    };
    use koentji::domain::admin_access::{LockoutPolicy, LoginAttemptLedger};
    use koentji::domain::authentication::FreeTrialConfig;
    use koentji::infrastructure::cache::MokaAuthCache;
    use koentji::infrastructure::postgres::{
        PostgresAuditEventRepository, PostgresIssuedKeyRepository,
    };
    use koentji::infrastructure::telemetry::{AccessLog, RequestIdMiddleware};

    dotenvy::dotenv().ok();
    env_logger::init();

    log::info!("Starting Koentji server...");

    let pool = koentji::db::create_pool().await;
    koentji::db::run_migrations(&pool).await;

    let cache_ttl: u64 = std::env::var("AUTH_CACHE_TTL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(900);
    log::info!("Auth cache TTL: {}s", cache_ttl);

    let free_trial = FreeTrialConfig::new(
        std::env::var("FREE_TRIAL_KEY").unwrap_or_else(|_| "FREE_TRIAL".to_string()),
        std::env::var("FREE_TRIAL_SUBSCRIPTION_NAME").unwrap_or_else(|_| "free".to_string()),
    );

    let issued_key_repo: std::sync::Arc<dyn koentji::domain::authentication::IssuedKeyRepository> =
        std::sync::Arc::new(PostgresIssuedKeyRepository::new(pool.clone()));
    let auth_cache_port: std::sync::Arc<dyn koentji::domain::authentication::AuthCachePort> =
        std::sync::Arc::new(MokaAuthCache::new(cache_ttl));
    let audit_port: std::sync::Arc<dyn koentji::domain::authentication::AuditEventPort> =
        std::sync::Arc::new(PostgresAuditEventRepository::new(pool.clone()));
    let auth_handler = std::sync::Arc::new(AuthenticateApiKey::new(
        issued_key_repo.clone(),
        auth_cache_port.clone(),
        free_trial,
    ));
    let issue_key = std::sync::Arc::new(IssueKey::new(issued_key_repo.clone(), audit_port.clone()));
    let revoke_key = std::sync::Arc::new(RevokeKey::new(
        issued_key_repo.clone(),
        auth_cache_port.clone(),
        audit_port.clone(),
    ));
    let reassign_device = std::sync::Arc::new(ReassignDevice::new(
        issued_key_repo.clone(),
        auth_cache_port.clone(),
        audit_port.clone(),
    ));
    let reset_rate_limit = std::sync::Arc::new(ResetRateLimit::new(
        issued_key_repo.clone(),
        auth_cache_port.clone(),
        audit_port.clone(),
    ));
    let extend_expiration = std::sync::Arc::new(ExtendExpiration::new(
        issued_key_repo.clone(),
        auth_cache_port.clone(),
        audit_port.clone(),
    ));

    let login_ledger = std::sync::Arc::new(LoginAttemptLedger::new(LockoutPolicy::default_admin()));

    let secret_key = std::env::var("SECRET_KEY").unwrap_or_else(|_| {
        log::warn!("SECRET_KEY not set, using insecure default — set SECRET_KEY in production");
        "a-very-secret-key-that-should-be-at-least-64-bytes-long-for-security-purposes-change-me"
            .to_string()
    });
    let cookie_key = Key::from(secret_key.as_bytes());

    // Session cookie `Secure` flag. Defaults to `true` — shipping an
    // admin session cookie over plain HTTP in production is a
    // credential-theft vector. Dev and e2e run on `http://localhost`
    // and must opt out with `COOKIE_SECURE=false`. Misconfiguration is
    // visible at boot via the warn line.
    let cookie_secure = match std::env::var("COOKIE_SECURE").as_deref() {
        Ok("false") | Ok("0") | Ok("no") => false,
        Ok(_) => true,
        Err(_) => {
            log::warn!(
                "COOKIE_SECURE not set, defaulting to true — set COOKIE_SECURE=false for local dev"
            );
            true
        }
    };

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
        let auth_handler = auth_handler.clone();
        let issue_key = issue_key.clone();
        let revoke_key = revoke_key.clone();
        let reassign_device = reassign_device.clone();
        let reset_rate_limit = reset_rate_limit.clone();
        let extend_expiration = extend_expiration.clone();
        let cache_port_data = auth_cache_port.clone();
        let login_ledger = login_ledger.clone();

        actix_web::App::new()
            .app_data(web::Data::new(auth_handler))
            .app_data(web::Data::new(issue_key))
            .app_data(web::Data::new(revoke_key))
            .app_data(web::Data::new(reassign_device))
            .app_data(web::Data::new(reset_rate_limit))
            .app_data(web::Data::new(extend_expiration))
            .app_data(web::Data::new(cache_port_data))
            .app_data(web::Data::new(login_ledger))
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
                    .cookie_secure(cookie_secure)
                    .cookie_same_site(actix_web::cookie::SameSite::Lax)
                    .cookie_content_security(CookieContentSecurity::Private)
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(Duration::hours(24)),
                    )
                    .build(),
            )
            // AccessLog sits above Session so every request — including
            // auth-failed ones — produces a line. Actix runs wraps in
            // reverse order (last `.wrap` runs first on request), so
            // RequestIdMiddleware must be added *after* AccessLog to
            // set the extension before AccessLog reads it.
            .wrap(AccessLog)
            .wrap(RequestIdMiddleware)
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
