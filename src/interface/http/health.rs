//! Liveness and readiness probes.
//!
//! - `GET /healthz` — **liveness**. Returns 200 as long as the
//!   process is up and the event loop is turning. Orchestrators
//!   (Docker `HEALTHCHECK`, k8s livenessProbe) use this to decide
//!   *restart the container*. It must not depend on downstream state,
//!   or a transient Postgres blip would cause the orchestrator to
//!   kill a process that is perfectly capable of serving cached or
//!   degraded traffic.
//!
//! - `GET /readyz` — **readiness**. Returns 200 only when the
//!   process can actually serve production traffic, which here means
//!   the Postgres pool can issue a query. A 503 tells the load
//!   balancer *stop sending new traffic*, not *restart me*. This is
//!   the right hook for rolling deploys (mark unready → drain → stop).
//!
//! Probes are cheap on purpose: a `SELECT 1` with an acquire
//! deadline short enough that a stuck pool is visible to the probe
//! before the probe itself times out.

use std::time::Duration;

use actix_web::{web, HttpResponse, Responder};
use sqlx::PgPool;

const READINESS_TIMEOUT: Duration = Duration::from_secs(2);

#[actix_web::get("/healthz")]
pub async fn healthz() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"status":"ok"}"#)
}

#[actix_web::get("/readyz")]
pub async fn readyz(pool: web::Data<PgPool>) -> HttpResponse {
    match check_database(pool.get_ref()).await {
        Ok(()) => HttpResponse::Ok()
            .content_type("application/json")
            .body(r#"{"status":"ready","database":"ok"}"#),
        Err(reason) => {
            log::warn!(target: "readiness", "readyz failing: {reason}");
            HttpResponse::ServiceUnavailable()
                .content_type("application/json")
                .body(format!(
                    r#"{{"status":"unavailable","database":"{reason}"}}"#
                ))
        }
    }
}

async fn check_database(pool: &PgPool) -> Result<(), &'static str> {
    // `timeout` wraps the whole acquire+query dance: a pool that has
    // deadlocked on connection acquisition is just as unready as one
    // that returns an error.
    let work = async {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(pool)
            .await
    };
    match tokio::time::timeout(READINESS_TIMEOUT, work).await {
        Ok(Ok(1)) => Ok(()),
        Ok(Ok(_)) => Err("unexpected"),
        Ok(Err(_)) => Err("query_failed"),
        Err(_) => Err("timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::{call_service, init_service, TestRequest};
    use actix_web::App;

    #[actix_web::test]
    async fn healthz_returns_ok_without_a_pool() {
        // Liveness must not depend on the DB — if the probe tried to
        // acquire a connection, a stuck pool would cause the container
        // to be restarted unnecessarily. The test enforces the
        // no-pool-required contract at the type level.
        let app = init_service(App::new().service(healthz)).await;
        let req = TestRequest::get().uri("/healthz").to_request();
        let res = call_service(&app, req).await;
        assert_eq!(res.status().as_u16(), 200);
        let body = actix_web::test::read_body(res).await;
        assert_eq!(&body[..], br#"{"status":"ok"}"#);
    }
}
