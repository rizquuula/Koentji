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
use actix_web::HttpMessage;

use super::request_id::RequestId;

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
        let request_id = req.extensions().get::<RequestId>().map(|r| r.to_string());

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

            // Structured fields rather than a hand-built JSON string: the
            // JSON subscriber renders these as top-level fields (and adds its
            // own `timestamp`), so the access log stays first-class instead
            // of nesting an escaped JSON blob inside `message`. Absent
            // referer/user-agent collapse to "" (tracing fields can't be
            // conditionally omitted the way `skip_serializing_if` did).
            tracing::info!(
                target: "http_access",
                request_id = request_id.as_deref().unwrap_or("-"),
                remote = remote.as_str(),
                method = method.as_str(),
                path = path.as_str(),
                status,
                bytes,
                duration_ms = duration.as_millis() as u64,
                referer = referer.as_deref().unwrap_or(""),
                user_agent = user_agent.as_deref().unwrap_or(""),
                "http access"
            );

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
}
