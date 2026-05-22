use chrono::{DateTime, Utc};

/// One observable `/v2/auth` outcome, ready for downstream analytics
/// storage. Fire-and-forget: emitting must never block the hot path.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthEvent {
    pub occurred_at: DateTime<Utc>,
    /// Matches `IssuedKeyId(i32)` — stored as i64 for ClickHouse Int64.
    pub auth_key_id: i64,
    pub auth_key: String,
    pub device_id: String,
    pub usage: f64,
    pub remaining_after: f64,
    pub decision: AuthEventDecision,
    pub denial_reason: Option<&'static str>,
    pub latency_us: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthEventDecision {
    Allowed,
    Denied,
}

impl AuthEventDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_as_str_round_trip() {
        assert_eq!(AuthEventDecision::Allowed.as_str(), "allowed");
        assert_eq!(AuthEventDecision::Denied.as_str(), "denied");
    }
}
