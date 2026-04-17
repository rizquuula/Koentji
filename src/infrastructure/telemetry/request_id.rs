//! Request-id middleware.
//!
//! Every request either carries an inbound `X-Request-Id` header
//! (trusted from an upstream proxy / load balancer) or is assigned a
//! freshly minted UUID v7. The id is:
//!
//! - stored as a `RequestId` extension on the `ServiceRequest` so
//!   downstream handlers can pull it out (e.g. the access log),
//! - mirrored on the outbound response as `X-Request-Id` so clients
//!   (and browser devtools) can correlate a failed call with a log
//!   line on the server,
//! - the value type is opaque (`RequestId`) so handlers don't stringify
//!   it by accident.
//!
//! UUID v7 is time-sortable — log lines grouped by id still roughly
//! reflect wall-clock order, which makes `sort | uniq` style triage
//! on a flat log file usable. Inbound ids are accepted only if they
//! pass a cheap sanity check (printable ASCII, ≤128 bytes) — a
//! header echoed unmodified to responses would otherwise be an
//! injection vector into downstream log sinks.

use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{Error, HttpMessage};
use uuid::Uuid;

pub const HEADER: &str = "x-request-id";
const MAX_INBOUND_LEN: usize = 128;

/// Opaque per-request id. `Clone` is cheap (Rc-backed `String`). The
/// `Display` impl gives the canonical stringification used on the
/// outbound header and in log lines.
#[derive(Clone, Debug)]
pub struct RequestId(Rc<String>);

impl RequestId {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn from_string(s: String) -> Self {
        Self(Rc::new(s))
    }

    fn generated() -> Self {
        Self::from_string(Uuid::now_v7().to_string())
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

/// Install with `.wrap(RequestIdMiddleware)` on the top-level App.
#[derive(Clone, Copy, Default)]
pub struct RequestIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RequestIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = RequestIdService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdService { service }))
    }
}

pub struct RequestIdService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestIdService<S>
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
        let id = inbound_id(&req).unwrap_or_else(RequestId::generated);
        req.extensions_mut().insert(id.clone());

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            let mut res = res.map_into_boxed_body();
            if let Ok(value) = HeaderValue::from_str(id.as_str()) {
                res.headers_mut()
                    .insert(HeaderName::from_static(HEADER), value);
            }
            Ok(res)
        })
    }
}

fn inbound_id(req: &ServiceRequest) -> Option<RequestId> {
    let raw = req.headers().get(HeaderName::from_static(HEADER))?;
    let s = raw.to_str().ok()?;
    if s.is_empty() || s.len() > MAX_INBOUND_LEN {
        return None;
    }
    // Printable ASCII only — blocks log-injection (newlines, control
    // chars, raw escape sequences) from rebounding into our log sinks.
    if !s.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return None;
    }
    Some(RequestId::from_string(s.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::{call_service, init_service, TestRequest};
    use actix_web::{web, App, HttpMessage, HttpRequest, HttpResponse};

    #[actix_web::test]
    async fn generates_request_id_when_absent() {
        let app = init_service(App::new().wrap(RequestIdMiddleware).route(
            "/",
            web::get().to(|req: HttpRequest| async move {
                let id = req.extensions().get::<RequestId>().cloned();
                HttpResponse::Ok().body(id.map(|r| r.to_string()).unwrap_or_default())
            }),
        ))
        .await;

        let req = TestRequest::get().uri("/").to_request();
        let res = call_service(&app, req).await;
        assert_eq!(res.status().as_u16(), 200);
        let echoed = res.headers().get(HEADER).unwrap().to_str().unwrap();
        assert!(Uuid::parse_str(echoed).is_ok(), "generated id must be uuid");
    }

    #[actix_web::test]
    async fn preserves_trusted_inbound_request_id() {
        let app = init_service(
            App::new()
                .wrap(RequestIdMiddleware)
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;

        let req = TestRequest::get()
            .uri("/")
            .insert_header(("X-Request-Id", "abc-123"))
            .to_request();
        let res = call_service(&app, req).await;
        assert_eq!(res.headers().get(HEADER).unwrap(), "abc-123");
    }

    #[actix_web::test]
    async fn rejects_inbound_with_control_chars() {
        // Tabs in a header would let a caller inject junk whitespace
        // into our structured access log — refuse and generate a fresh
        // id instead. Newlines and NULs are already blocked at the HTTP
        // parser layer, so tab is the stressful-but-reachable case.
        let app = init_service(
            App::new()
                .wrap(RequestIdMiddleware)
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;

        let req = TestRequest::get()
            .uri("/")
            .insert_header((
                HeaderName::from_static(HEADER),
                HeaderValue::from_bytes(b"ok\tINJECT").unwrap(),
            ))
            .to_request();
        let res = call_service(&app, req).await;
        let echoed = res.headers().get(HEADER).unwrap().to_str().unwrap();
        assert_ne!(echoed, "ok\tINJECT");
        assert!(Uuid::parse_str(echoed).is_ok());
    }

    #[actix_web::test]
    async fn rejects_oversized_inbound() {
        let app = init_service(
            App::new()
                .wrap(RequestIdMiddleware)
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;

        let oversized = "a".repeat(MAX_INBOUND_LEN + 1);
        let req = TestRequest::get()
            .uri("/")
            .insert_header(("X-Request-Id", oversized.clone()))
            .to_request();
        let res = call_service(&app, req).await;
        let echoed = res.headers().get(HEADER).unwrap().to_str().unwrap();
        assert_ne!(echoed, oversized);
    }

    #[actix_web::test]
    async fn stores_request_id_as_extension() {
        // Proves downstream handlers can see the id — the access log
        // relies on this to attach the id to every line.
        let app = init_service(App::new().wrap(RequestIdMiddleware).route(
            "/",
            web::get().to(|req: HttpRequest| async move {
                let id = req
                    .extensions()
                    .get::<RequestId>()
                    .map(|r| r.to_string())
                    .unwrap_or_default();
                HttpResponse::Ok().body(id)
            }),
        ))
        .await;

        let req = TestRequest::get()
            .uri("/")
            .insert_header(("X-Request-Id", "trace-xyz"))
            .to_request();
        let res = call_service(&app, req).await;
        let body = actix_web::test::read_body(res).await;
        assert_eq!(&body[..], b"trace-xyz");
    }
}
