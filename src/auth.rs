use leptos::prelude::*;

#[server]
pub async fn login(username: String, password: String) -> Result<bool, ServerFnError> {
    use actix_session::Session;
    use leptos_actix::extract;

    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

    if username == admin_username && password == admin_password {
        let session = extract::<Session>().await?;
        session
            .insert("username", &username)
            .map_err(|e| ServerFnError::new(format!("Session error: {}", e)))?;
        log::info!("Admin login success: username={}", username);
        Ok(true)
    } else {
        log::warn!("Admin login failed: username={}", username);
        Ok(false)
    }
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    use actix_session::Session;
    use leptos_actix::extract;

    let session = extract::<Session>().await?;
    let username = session
        .get::<String>("username")
        .ok()
        .flatten()
        .unwrap_or_default();
    session.purge();
    log::info!("Admin logout: username={}", username);
    Ok(())
}

#[server]
pub async fn get_current_user() -> Result<Option<String>, ServerFnError> {
    use actix_session::Session;
    use leptos_actix::extract;

    let session = extract::<Session>().await?;
    let username = session
        .get::<String>("username")
        .map_err(|e| ServerFnError::new(format!("Session error: {}", e)))?;
    Ok(username)
}
