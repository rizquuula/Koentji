//! Structured JSON access log middleware.
//!
//! Emits one line per completed HTTP request, as a single-line JSON
//! object. Mirrors the classic Apache combined log fields (remote
//! address, method, path, status, bytes, latency, referer, user-agent)
//! but in a shape log aggregators (Loki, Elasticsearch, Datadog) can
//! index without a regex parser on the hot path.
//!
//! Implemented as an actix-web middleware (Transform + Service)
//! rather than using `actix_web::middleware::Logger` — the built-in
//! Logger takes a format string and is very awkward to coerce into a
//! JSON-escaped shape (timestamps come bracketed, headers aren't
//! escaped). Hand-rolling the middleware is ~80 lines and gives us
//! serde-correct escaping.

use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use serde::Serialize;

/// Install with `.wrap(AccessLog)` on the top-level Actix app.
#[derive(Clone, Copy, Default)]
pub struct AccessLog;

impl<S, B> Transform<S, ServiceRequest> for AccessLog
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AccessLogMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AccessLogMiddleware { service }))
    }
}

pub struct AccessLogMiddleware<S> {
    service: S,
}

#[derive(Serialize)]
struct AccessLogLine<'a> {
    ts: String,
    remote: &'a str,
    method: &'a str,
    path: &'a str,
    status: u16,
    bytes: u64,
    duration_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    referer: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_agent: Option<&'a str>,
}

impl<S, B> Service<ServiceRequest> for AccessLogMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let method = req.method().clone();
        let path = req.path().to_string();
        let remote = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();
        let referer = req
            .headers()
            .get(actix_web::http::header::REFERER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let user_agent = req
            .headers()
            .get(actix_web::http::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            let status = res.status().as_u16();
            let duration = start.elapsed();
            let bytes = res.response().body().size();
            let bytes = match bytes {
                actix_web::body::BodySize::Sized(n) => n,
                _ => 0,
            };

            let line = AccessLogLine {
                ts: chrono::Utc::now().to_rfc3339(),
                remote: &remote,
                method: method.as_str(),
                path: &path,
                status,
                bytes,
                duration_ms: duration.as_millis(),
                referer: referer.as_deref(),
                user_agent: user_agent.as_deref(),
            };

            match serde_json::to_string(&line) {
                Ok(json) => log::info!(target: "http_access", "{json}"),
                Err(e) => log::warn!(target: "http_access", "access log serialise failed: {e}"),
            }

            Ok(res.map_into_boxed_body())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    use actix_web::{web, App, HttpResponse};

    #[actix_web::test]
    async fn middleware_passes_requests_through_unchanged() {
        // The middleware wraps responses — make sure the wrapped body
        // is preserved and the status propagates out.
        let app = actix_web::test::init_service(App::new().wrap(AccessLog).route(
            "/ok",
            web::get().to(|| async { HttpResponse::Ok().body("hello") }),
        ))
        .await;

        let req = TestRequest::get().uri("/ok").to_request();
        let res = actix_web::test::call_service(&app, req).await;
        assert_eq!(res.status().as_u16(), 200);
        let body = actix_web::test::read_body(res).await;
        assert_eq!(&body[..], b"hello");
    }

    #[actix_web::test]
    async fn middleware_preserves_non_2xx_status() {
        let app = actix_web::test::init_service(App::new().wrap(AccessLog).route(
            "/nope",
            web::get().to(|| async { HttpResponse::NotFound().finish() }),
        ))
        .await;

        let req = TestRequest::get().uri("/nope").to_request();
        let res = actix_web::test::call_service(&app, req).await;
        assert_eq!(res.status().as_u16(), 404);
    }

    #[test]
    fn access_log_line_serialises_to_single_line_json() {
        // Lock in the JSON shape so future refactors of the struct don't
        // silently break downstream log parsers.
        let line = AccessLogLine {
            ts: "2026-04-17T12:00:00Z".to_string(),
            remote: "1.2.3.4",
            method: "POST",
            path: "/v1/auth",
            status: 200,
            bytes: 42,
            duration_ms: 5,
            referer: None,
            user_agent: Some("curl/8.0"),
        };
        let json = serde_json::to_string(&line).unwrap();
        assert!(json.starts_with('{'));
        assert!(!json.contains('\n'));
        // Only the `user_agent` optional field should appear —
        // `referer` is None and skip_serializing_if trims it.
        assert!(json.contains("\"user_agent\":\"curl/8.0\""));
        assert!(!json.contains("\"referer\""));
    }
}
