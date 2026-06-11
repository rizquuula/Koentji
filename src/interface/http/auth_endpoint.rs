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
//!
//! The wire envelope is frozen: `rate_limit_remaining` stays an integer
//! (ceil shim). The *request* field, however, is `f64` so callers may
//! consume fractional units — existing integer clients are unaffected
//! because JSON `1` deserialises straight into `1.0`. Every request also
//! emits an `AuthEvent` to the `AuthEventSink` for ClickHouse analytics
//! (fire-and-forget; drops on buffer full, never blocks the caller).

use std::sync::Arc;

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::{AuthOutcome, AuthenticateApiKey};
use crate::domain::authentication::{
    AuthEvent, AuthEventDecision, AuthEventSink, AuthKey, DenialReason, DeviceId, RateLimitUsage,
};
use crate::interface::http::i18n::{status_code, DenialEnvelope};

#[derive(ToSchema, Deserialize)]
pub struct AuthRequest {
    /// The API key to authenticate
    pub auth_key: String,
    /// The device ID associated with the key
    pub auth_device: String,
    /// Number of rate limit units to consume (default: 1.0, fractional allowed)
    #[serde(default = "default_rate_limit_usage")]
    #[schema(default = 1.0, example = 1.0)]
    pub rate_limit_usage: f64,
}

fn default_rate_limit_usage() -> f64 {
    1.0
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
    sink: web::Data<dyn AuthEventSink>,
) -> HttpResponse {
    log::debug!("Auth request: device={}", body.auth_device);

    let start = std::time::Instant::now();
    let now = Utc::now();

    // Coerce raw request usage to a finite positive `f64`. NaN, infinite,
    // zero, and negative values collapse to `1.0`. Several client SDKs
    // (Kotlin/Java/Swift/Go) serialise unset numeric fields as `0`; the
    // domain treats `usage == 0` as a no-op consume, which silently stops
    // decrementing the ledger while still stamping
    // `rate_limit_updated_at`. Normalising at the edge keeps the domain's
    // zero-is-a-no-op invariant intact without exposing the footgun on
    // the public envelope.
    let usage_f = coerce_usage(body.rate_limit_usage);

    let key = match AuthKey::parse(body.auth_key.clone()) {
        Ok(k) => k,
        Err(_) => {
            emit_denied(
                sink.get_ref(),
                now,
                0,
                body.auth_key.clone(),
                body.auth_device.clone(),
                usage_f,
                Some("UnknownKey"),
                elapsed_us(start),
            );
            return unknown_key_response();
        }
    };
    let device = match DeviceId::parse(body.auth_device.clone()) {
        Ok(d) => d,
        Err(_) => {
            emit_denied(
                sink.get_ref(),
                now,
                0,
                body.auth_key.clone(),
                body.auth_device.clone(),
                usage_f,
                Some("UnknownKey"),
                elapsed_us(start),
            );
            return unknown_key_response();
        }
    };

    let usage = RateLimitUsage::new(usage_f).expect("coerced to a finite positive value");

    let outcome = handler.execute(key, device, usage, now).await;
    match outcome {
        AuthOutcome::Success { key, remaining } => {
            log::info!(
                "Auth success: device={}, subscription={:?}, remaining={}",
                key.device_id.as_str(),
                key.subscription.as_ref().map(|s| s.as_str()),
                remaining.value()
            );
            sink.record(AuthEvent {
                occurred_at: now,
                auth_key_id: key.id.value() as i64,
                auth_key: key.key.as_str().to_string(),
                device_id: key.device_id.as_str().to_string(),
                usage: usage_f,
                remaining_after: remaining.value(),
                decision: AuthEventDecision::Allowed,
                denial_reason: None,
                latency_us: elapsed_us(start),
            });
            HttpResponse::Ok().json(AuthResponse {
                status: "success".into(),
                data: AuthResponseData {
                    key: key.key.as_str().to_string(),
                    device: key.device_id.as_str().to_string(),
                    subscription: key.subscription.as_ref().map(|s| s.as_str().to_string()),
                    username: key.username.clone(),
                    email: key.email.clone(),
                    valid_until: key.expired_at.map(|d| d.to_rfc3339()),
                    // v1 envelope: `rate_limit_remaining` is integer JSON.
                    // Ceil shim — fractional remainders should not round
                    // down to 0 and signal "exhausted" while quota remains.
                    rate_limit_remaining: remaining.value().ceil() as i32,
                },
            })
        }
        AuthOutcome::Denied { reason } => {
            let env = DenialEnvelope::from_reason(&reason);
            let status = StatusCode::from_u16(status_code(&reason))
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            emit_denied(
                sink.get_ref(),
                now,
                0,
                body.auth_key.clone(),
                body.auth_device.clone(),
                usage_f,
                Some(reason_str(&reason)),
                elapsed_us(start),
            );
            HttpResponse::build(status).json(serde_json::json!({
                "error": { "en": env.en, "id": env.id },
                "message": env.en,
            }))
        }
        AuthOutcome::BackendError => {
            emit_denied(
                sink.get_ref(),
                now,
                0,
                body.auth_key.clone(),
                body.auth_device.clone(),
                usage_f,
                Some("BackendError"),
                elapsed_us(start),
            );
            internal_error_response()
        }
    }
}

/// Coerce raw request usage to a finite positive `f64`.
///
/// NaN, infinite, zero, and negative values collapse to `1.0`. Clients
/// shipping `0`, `-1`, or `NaN` for an unset field would otherwise
/// silently no-op the consume or get rejected by `RateLimitUsage::new`.
fn coerce_usage(raw: f64) -> f64 {
    if raw.is_nan() || raw.is_infinite() || raw <= 0.0 {
        1.0
    } else {
        raw
    }
}

fn elapsed_us(start: std::time::Instant) -> u32 {
    let micros = start.elapsed().as_micros();
    if micros > u32::MAX as u128 {
        u32::MAX
    } else {
        micros as u32
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_denied(
    sink: &dyn AuthEventSink,
    now: chrono::DateTime<Utc>,
    auth_key_id: i64,
    auth_key: String,
    device_id: String,
    usage: f64,
    denial_reason: Option<&'static str>,
    latency_us: u32,
) {
    sink.record(AuthEvent {
        occurred_at: now,
        auth_key_id,
        auth_key,
        device_id,
        usage,
        remaining_after: 0.0,
        decision: AuthEventDecision::Denied,
        denial_reason,
        latency_us,
    });
}

fn reason_str(reason: &DenialReason) -> &'static str {
    match reason {
        DenialReason::UnknownKey => "UnknownKey",
        DenialReason::Revoked { .. } => "Revoked",
        DenialReason::Expired { .. } => "Expired",
        DenialReason::FreeTrialEnded { .. } => "FreeTrialEnded",
        DenialReason::RateLimitExceeded => "RateLimitExceeded",
    }
}

fn unknown_key_response() -> HttpResponse {
    let env = DenialEnvelope::from_reason(&DenialReason::UnknownKey);
    HttpResponse::build(StatusCode::UNAUTHORIZED).json(serde_json::json!({
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn coerce_usage_collapses_non_positive_and_non_finite_to_one() {
        assert_eq!(coerce_usage(f64::NAN), 1.0);
        assert_eq!(coerce_usage(f64::INFINITY), 1.0);
        assert_eq!(coerce_usage(f64::NEG_INFINITY), 1.0);
        assert_eq!(coerce_usage(0.0), 1.0);
        assert_eq!(coerce_usage(-0.0), 1.0);
        assert_eq!(coerce_usage(-1.0), 1.0);
        assert_eq!(coerce_usage(-0.5), 1.0);
    }

    #[test]
    fn coerce_usage_passes_through_finite_positive() {
        assert_eq!(coerce_usage(1.0), 1.0);
        assert_eq!(coerce_usage(0.5), 0.5);
        assert_eq!(coerce_usage(2.75), 2.75);
        assert_eq!(coerce_usage(f64::MIN_POSITIVE), f64::MIN_POSITIVE);
        assert_eq!(coerce_usage(f64::MAX), f64::MAX);
    }

    #[test]
    fn reason_str_maps_every_variant() {
        assert_eq!(reason_str(&DenialReason::UnknownKey), "UnknownKey");
        let at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(reason_str(&DenialReason::Revoked { at }), "Revoked");
        assert_eq!(reason_str(&DenialReason::Expired { at }), "Expired");
        assert_eq!(
            reason_str(&DenialReason::FreeTrialEnded { at }),
            "FreeTrialEnded"
        );
        assert_eq!(
            reason_str(&DenialReason::RateLimitExceeded),
            "RateLimitExceeded"
        );
    }
}
