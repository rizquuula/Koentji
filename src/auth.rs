use leptos::prelude::*;

#[server]
pub async fn login(username: String, password: String) -> Result<bool, ServerFnError> {
    use actix_session::Session;
    use leptos_actix::extract;

    // `ADMIN_USERNAME` is not a secret — straight string compare.
    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    if username != admin_username {
        log::warn!("Admin login failed: username={username}");
        return Ok(false);
    }

    // Password precedence: prefer the argon2id PHC hash
    // (`ADMIN_PASSWORD_HASH`) and fall back to plaintext
    // (`ADMIN_PASSWORD`) for dev / e2e. An invalid hash is a deploy-time
    // mistake and fails closed here — we don't silently fall back to
    // plaintext since that would mask the misconfiguration.
    let creds = load_admin_credentials();

    let Some(creds) = creds else {
        log::error!("Admin login refused: neither ADMIN_PASSWORD_HASH nor ADMIN_PASSWORD is set");
        return Ok(false);
    };

    if creds.verify(&password) {
        let session = extract::<Session>().await?;
        session
            .insert("username", &username)
            .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?;
        log::info!("Admin login success: username={username}");
        Ok(true)
    } else {
        log::warn!("Admin login failed: username={username}");
        Ok(false)
    }
}

#[cfg(feature = "ssr")]
fn load_admin_credentials() -> Option<crate::domain::admin_access::AdminCredentials> {
    use crate::domain::admin_access::AdminCredentials;

    if let Ok(phc) = std::env::var("ADMIN_PASSWORD_HASH") {
        let trimmed = phc.trim();
        if !trimmed.is_empty() {
            match AdminCredentials::from_hash(trimmed.to_string()) {
                Ok(c) => return Some(c),
                Err(e) => {
                    log::error!("ADMIN_PASSWORD_HASH rejected: {e}");
                    return None;
                }
            }
        }
    }
    if let Ok(plaintext) = std::env::var("ADMIN_PASSWORD") {
        if !plaintext.is_empty() {
            return AdminCredentials::from_plaintext(plaintext).ok();
        }
    }
    None
}

#[cfg(not(feature = "ssr"))]
#[allow(dead_code)]
fn load_admin_credentials() -> Option<()> {
    None
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
    log::info!("Admin logout: username={username}");
    Ok(())
}

#[server]
pub async fn get_current_user() -> Result<Option<String>, ServerFnError> {
    use actix_session::Session;
    use leptos_actix::extract;

    let session = extract::<Session>().await?;
    let username = session
        .get::<String>("username")
        .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?;
    Ok(username)
}
