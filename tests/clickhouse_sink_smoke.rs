//! ClickHouse sink smoke test.
//!
//! Requires a live ClickHouse instance. Run with:
//!   CLICKHOUSE_URL=http://user:pass@host:8123/db \
//!   cargo test --features ssr clickhouse_sink -- --ignored

#[cfg(feature = "ssr")]
mod tests {
    use chrono::Utc;
    use koentji::domain::authentication::auth_event::{AuthEvent, AuthEventDecision};
    use koentji::domain::authentication::auth_event_sink::AuthEventSink;
    use koentji::infrastructure::clickhouse::auth_event_sink::ClickHouseAuthEventSink;

    fn make_event(auth_key: &str) -> AuthEvent {
        AuthEvent {
            occurred_at: Utc::now(),
            auth_key_id: 99,
            auth_key: auth_key.to_string(),
            device_id: "smoke-dev".to_string(),
            usage: 1.0,
            remaining_after: 9.0,
            decision: AuthEventDecision::Allowed,
            denial_reason: None,
            latency_us: 42,
        }
    }

    #[tokio::test]
    #[ignore = "requires CLICKHOUSE_URL — run with: cargo test --features ssr clickhouse_sink -- --ignored"]
    async fn sink_records_events_to_clickhouse() {
        let url = match std::env::var("CLICKHOUSE_URL") {
            Ok(u) => u,
            Err(_) => {
                eprintln!("CLICKHOUSE_URL not set — skipping smoke test");
                return;
            }
        };

        let client = clickhouse::Client::default().with_url(&url);
        let sink = ClickHouseAuthEventSink::spawn(client.clone());

        // Unique key per run so counts don't collide across reruns.
        let test_key = format!("smoke-{}", uuid::Uuid::now_v7());

        for _ in 0..10 {
            sink.record(make_event(&test_key));
        }

        // Wait for the batch (1-second timeout) to flush.
        tokio::time::sleep(std::time::Duration::from_millis(1_500)).await;

        let count: u64 = client
            .query("SELECT count() FROM auth_events WHERE auth_key = ?")
            .bind(&test_key)
            .fetch_one()
            .await
            .expect("count query failed");

        assert_eq!(count, 10, "expected 10 rows, got {count}");
    }
}
