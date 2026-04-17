//! Actix handler for `POST /v1/auth`.
//!
//! A thin adapter — parses the request, converts the strings into
//! domain value objects, calls `AuthenticateApiKey`, then renders the
//! `AuthOutcome` back into the legacy envelope using
//! `interface::http::i18n`.
//!
//! The request/response DTOs are `utoipa::ToSchema` so the existing
//! Swagger page keeps working. They live here now instead of
//! `src/main.rs`, where they lived before the extraction.

use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::{AuthOutcome, AuthenticateApiKey};
use crate::domain::authentication::{AuthKey, DeviceId, RateLimitUsage};
use crate::interface::http::i18n::{status_code, DenialEnvelope};

#[derive(ToSchema, Deserialize)]
pub struct AuthRequest {
    /// The API key to authenticate
    pub auth_key: String,
    /// The device ID associated with the key
    pub auth_device: String,
    /// Number of rate limit units to consume (default: 1)
    #[serde(default = "default_rate_limit_usage")]
    #[schema(default = 1, example = 1)]
    pub rate_limit_usage: i32,
}

fn default_rate_limit_usage() -> i32 {
    1
}

#[derive(ToSchema, Serialize)]
pub struct AuthResponse {
    pub status: String,
    pub data: AuthResponseData,
}

#[derive(ToSchema, Serialize)]
pub struct AuthResponseData {
    pub key: String,
    pub device: String,
    pub subscription: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub valid_until: Option<String>,
    pub rate_limit_remaining: i32,
}

#[derive(ToSchema, Serialize)]
pub struct AuthError {
    pub error: serde_json::Value,
    pub message: String,
}

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
pub async fn auth_endpoint(
    body: web::Json<AuthRequest>,
    handler: web::Data<Arc<AuthenticateApiKey>>,
) -> HttpResponse {
    log::debug!("Auth request: device={}", body.auth_device);

    let key = match AuthKey::parse(body.auth_key.clone()) {
        Ok(k) => k,
        Err(_) => return unknown_key_response(),
    };
    let device = match DeviceId::parse(body.auth_device.clone()) {
        Ok(d) => d,
        Err(_) => return unknown_key_response(),
    };
    let usage = match RateLimitUsage::new(body.rate_limit_usage) {
        Ok(u) => u,
        Err(_) => {
            // Negative / invalid usage short-circuits to the
            // rate-limit-exceeded envelope — mirrors the legacy
            // handler's behaviour when the SQL predicate rejected
            // the row.
            return rate_limit_response();
        }
    };

    let outcome = handler.execute(key, device, usage, Utc::now()).await;
    match outcome {
        AuthOutcome::Success { key, remaining } => {
            log::info!(
                "Auth success: device={}, subscription={:?}, remaining={}",
                key.device_id.as_str(),
                key.subscription.as_ref().map(|s| s.as_str()),
                remaining.value()
            );
            HttpResponse::Ok().json(AuthResponse {
                status: "success".into(),
                data: AuthResponseData {
                    key: key.key.as_str().to_string(),
                    device: key.device_id.as_str().to_string(),
                    subscription: key.subscription.as_ref().map(|s| s.as_str().to_string()),
                    username: key.username.clone(),
                    email: key.email.clone(),
                    valid_until: key.expired_at.map(|d| d.to_rfc3339()),
                    rate_limit_remaining: remaining.value(),
                },
            })
        }
        AuthOutcome::Denied { reason } => {
            let env = DenialEnvelope::from_reason(&reason);
            let status = StatusCode::from_u16(status_code(&reason))
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            HttpResponse::build(status).json(serde_json::json!({
                "error": { "en": env.en, "id": env.id },
                "message": env.en,
            }))
        }
        AuthOutcome::BackendError => internal_error_response(),
    }
}

fn unknown_key_response() -> HttpResponse {
    let env = DenialEnvelope::from_reason(&crate::domain::authentication::DenialReason::UnknownKey);
    HttpResponse::build(StatusCode::UNAUTHORIZED).json(serde_json::json!({
        "error": { "en": env.en, "id": env.id },
        "message": env.en,
    }))
}

fn rate_limit_response() -> HttpResponse {
    let env = DenialEnvelope::from_reason(
        &crate::domain::authentication::DenialReason::RateLimitExceeded,
    );
    HttpResponse::build(StatusCode::TOO_MANY_REQUESTS).json(serde_json::json!({
        "error": { "en": env.en, "id": env.id },
        "message": env.en,
    }))
}

fn internal_error_response() -> HttpResponse {
    HttpResponse::InternalServerError().json(serde_json::json!({
        "error": { "en": "Internal server error." },
        "message": "Internal server error."
    }))
}
