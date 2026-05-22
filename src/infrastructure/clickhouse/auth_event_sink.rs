use clickhouse::Row;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use crate::domain::authentication::auth_event::{AuthEvent, AuthEventDecision};
use crate::domain::authentication::auth_event_sink::AuthEventSink;

const CHANNEL_CAP: usize = 10_000;
const BATCH_SIZE: usize = 1_000;
const BATCH_TIMEOUT: Duration = Duration::from_secs(1);

impl From<AuthEventDecision> for i8 {
    fn from(d: AuthEventDecision) -> i8 {
        match d {
            AuthEventDecision::Allowed => 1,
            AuthEventDecision::Denied => 2,
        }
    }
}

#[derive(Row, Serialize)]
struct AuthEventRow {
    #[serde(rename = "ts", with = "clickhouse::serde::chrono::datetime64::millis")]
    occurred_at: chrono::DateTime<chrono::Utc>,
    auth_key_id: i64,
    auth_key: String,
    device_id: String,
    usage: f64,
    remaining_after: f64,
    decision: i8,
    denial_reason: String,
    latency_us: u32,
}

impl From<AuthEvent> for AuthEventRow {
    fn from(e: AuthEvent) -> Self {
        Self {
            occurred_at: e.occurred_at,
            auth_key_id: e.auth_key_id,
            auth_key: e.auth_key,
            device_id: e.device_id,
            usage: e.usage,
            remaining_after: e.remaining_after,
            decision: i8::from(e.decision),
            denial_reason: e.denial_reason.unwrap_or("").to_string(),
            latency_us: e.latency_us,
        }
    }
}

pub struct ClickHouseAuthEventSink {
    tx: mpsc::Sender<AuthEvent>,
}

impl ClickHouseAuthEventSink {
    pub fn spawn(client: clickhouse::Client) -> Self {
        let (tx, mut rx) = mpsc::channel::<AuthEvent>(CHANNEL_CAP);

        tokio::spawn(async move {
            loop {
                // Wait for the first event.
                let first = match rx.recv().await {
                    Some(e) => e,
                    None => break,
                };

                let mut batch = Vec::with_capacity(BATCH_SIZE);
                batch.push(first);

                // Drain up to BATCH_SIZE - 1 more within the timeout.
                let deadline = tokio::time::Instant::now() + BATCH_TIMEOUT;
                while batch.len() < BATCH_SIZE {
                    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                    if remaining.is_zero() {
                        break;
                    }
                    match timeout(remaining, rx.recv()).await {
                        Ok(Some(e)) => batch.push(e),
                        Ok(None) => break,
                        Err(_) => break,
                    }
                }

                if let Err(e) = insert_batch(&client, batch).await {
                    log::warn!("ClickHouse auth_events insert failed: {e}");
                }
            }
        });

        Self { tx }
    }
}

async fn insert_batch(
    client: &clickhouse::Client,
    batch: Vec<AuthEvent>,
) -> Result<(), clickhouse::error::Error> {
    let mut insert = client.insert::<AuthEventRow>("auth_events")?;
    for event in batch {
        insert.write(&AuthEventRow::from(event)).await?;
    }
    insert.end().await
}

impl AuthEventSink for ClickHouseAuthEventSink {
    fn record(&self, event: AuthEvent) {
        let _ = self.tx.try_send(event);
    }
}
