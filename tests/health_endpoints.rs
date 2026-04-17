//! Integration tests for `/healthz` and `/readyz`.
//!
//! `healthz` is covered by a unit test in-module; this file pins the
//! readiness contract against a real Postgres pool, because the
//! interesting edge cases (pool present vs. closed) don't exist
//! without one.

#![cfg(feature = "ssr")]

mod common;

use actix_web::test::{call_service, init_service, TestRequest};
use actix_web::{web, App};
use koentji::interface::http::health::{healthz, readyz};

#[actix_web::test]
async fn readyz_returns_200_when_database_is_reachable() {
    let pool = common::fresh_pool().await;
    let app = init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(readyz),
    )
    .await;

    let req = TestRequest::get().uri("/readyz").to_request();
    let res = call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);
    let body = actix_web::test::read_body(res).await;
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("\"status\":\"ready\""));
    assert!(body_str.contains("\"database\":\"ok\""));
}

#[actix_web::test]
async fn readyz_returns_503_when_pool_is_closed() {
    // A closed pool simulates the failure mode we actually care
    // about: a transient DB outage. The load balancer should see 503
    // and stop sending traffic, not restart the process.
    let pool = common::fresh_pool().await;
    pool.close().await;
    let app = init_service(App::new().app_data(web::Data::new(pool)).service(readyz)).await;

    let req = TestRequest::get().uri("/readyz").to_request();
    let res = call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 503);
    let body = actix_web::test::read_body(res).await;
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("\"status\":\"unavailable\""));
}

#[actix_web::test]
async fn healthz_does_not_require_database() {
    // Re-verified as an integration test: even with no pool registered
    // at all, `/healthz` must answer 200. Liveness probes run in
    // environments where Postgres might be temporarily gone.
    let app = init_service(App::new().service(healthz)).await;
    let req = TestRequest::get().uri("/healthz").to_request();
    let res = call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);
}
