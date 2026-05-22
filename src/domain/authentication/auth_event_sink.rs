use super::auth_event::AuthEvent;

/// Non-blocking sink for `AuthEvent`s. Implementations MUST NOT block
/// the calling task — drop on full buffer is acceptable; the analytics
/// path is best-effort, not ledger-grade.
pub trait AuthEventSink: Send + Sync {
    fn record(&self, event: AuthEvent);
}

/// A no-op sink for tests and for boot paths where ClickHouse is
/// unavailable. Records are silently dropped.
pub struct NoopAuthEventSink;

impl AuthEventSink for NoopAuthEventSink {
    fn record(&self, _event: AuthEvent) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::authentication::auth_event::AuthEventDecision;
    use chrono::Utc;

    fn sample_event() -> AuthEvent {
        AuthEvent {
            occurred_at: Utc::now(),
            auth_key_id: 1,
            auth_key: "klab_test".to_string(),
            device_id: "dev-1".to_string(),
            usage: 1.0,
            remaining_after: 9.0,
            decision: AuthEventDecision::Allowed,
            denial_reason: None,
            latency_us: 100,
        }
    }

    #[test]
    fn noop_sink_does_not_panic() {
        let sink = NoopAuthEventSink;
        sink.record(sample_event());
    }
}
