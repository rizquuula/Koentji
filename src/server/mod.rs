pub mod analytics_service;
pub mod insights_service;
pub mod key_service;
pub mod rate_limit_service;
pub mod stats_service;
pub mod subscription_service;

/// Reject any server-fn invocation lacking an authenticated admin session.
/// Leptos `#[server]` fns are individually POST-invocable at their generated
/// endpoints regardless of the client router, so every admin verb must gate
/// itself — there is no route-level guard.
#[cfg(feature = "ssr")]
pub(crate) async fn require_admin() -> Result<(), leptos::prelude::ServerFnError> {
    use actix_session::Session;
    use leptos_actix::extract;

    let session = extract::<Session>().await?;
    let username = session
        .get::<String>("username")
        .map_err(|e| leptos::prelude::ServerFnError::new(format!("Session error: {e}")))?;
    if username.is_none() {
        return Err(leptos::prelude::ServerFnError::ServerError(
            "unauthorized".into(),
        ));
    }
    Ok(())
}
