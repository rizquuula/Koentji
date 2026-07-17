use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct AuthenticationKey {
    pub id: i32,
    pub key: String,
    pub device_id: String,
    pub subscription: Option<String>,
    pub rate_limit_daily: f64,
    pub rate_limit_remaining: f64,
    pub rate_limit_updated_at: Option<DateTime<Utc>>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub deleted_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expired_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub subscription_type_id: Option<i32>,
    pub rate_limit_interval_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct RateLimitInterval {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub duration_seconds: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct SubscriptionType {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub rate_limit_amount: f64,
    pub rate_limit_interval_id: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AuthenticationKey {
    pub fn is_expired(&self) -> bool {
        self.expired_at.map(|exp| exp < Utc::now()).unwrap_or(false)
    }

    pub fn is_active(&self) -> bool {
        self.deleted_at.is_none() && !self.is_expired()
    }

    pub fn status(&self) -> &str {
        if self.deleted_at.is_some() {
            "deleted"
        } else if self.is_expired() {
            "expired"
        } else {
            "active"
        }
    }

    pub fn rate_limit_percentage(&self) -> f64 {
        if self.rate_limit_daily == 0.0 {
            return 0.0;
        }
        let used = self.rate_limit_daily - self.rate_limit_remaining;
        (used / self.rate_limit_daily) * 100.0
    }

    /// A content hash over every field the key table renders, used as
    /// the second half of the `<For>` remount key so a row with the same
    /// `id` but changed content is torn down and rebuilt (its child
    /// signals are captured at construction time and are otherwise
    /// non-reactive). `f64` fields hash via `.to_bits()` — the exact
    /// stored value, not an epsilon compare. Deterministic across the
    /// SSR (native) and hydrate (wasm) builds because `DefaultHasher` is
    /// seeded identically and both compute from the same resource data.
    pub fn row_version(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.key.hash(&mut hasher);
        self.device_id.hash(&mut hasher);
        self.subscription.hash(&mut hasher);
        self.rate_limit_daily.to_bits().hash(&mut hasher);
        self.rate_limit_remaining.to_bits().hash(&mut hasher);
        self.username.hash(&mut hasher);
        self.email.hash(&mut hasher);
        self.expired_at.hash(&mut hasher);
        self.deleted_at.hash(&mut hasher);
        self.updated_at.hash(&mut hasher);
        hasher.finish()
    }

    pub fn masked_key(&self) -> String {
        if self.key.len() <= 8 {
            return self.key.clone();
        }
        let prefix = &self.key[..5]; // "klab_"
        let suffix = &self.key[self.key.len() - 4..];
        format!("{}****...****{}", prefix, suffix)
    }

    /// Visual mask for the device id, mirroring `masked_key`. Unlike the API
    /// key this is purely cosmetic (the device id is not a secret and is
    /// already shipped to the client) — char-based slicing keeps it safe for
    /// non-ASCII ids. Short ids and the `-` unclaimed sentinel pass through.
    pub fn masked_device_id(&self) -> String {
        let chars: Vec<char> = self.device_id.chars().collect();
        if chars.len() <= 8 {
            return self.device_id.clone();
        }
        let prefix: String = chars[..4].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{}****...****{}", prefix, suffix)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKeyRequest {
    pub device_id: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub subscription: Option<String>,
    pub subscription_type_id: Option<i32>,
    pub rate_limit_daily: Option<f64>,
    pub expired_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateKeyRequest {
    pub device_id: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub subscription: Option<String>,
    pub subscription_type_id: Option<i32>,
    pub rate_limit_daily: Option<f64>,
    pub expired_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionTypeRequest {
    pub name: String,
    pub display_name: String,
    pub rate_limit_amount: f64,
    pub rate_limit_interval_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubscriptionTypeRequest {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub rate_limit_amount: Option<f64>,
    pub rate_limit_interval_id: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRateLimitIntervalRequest {
    pub name: String,
    pub display_name: String,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRateLimitIntervalRequest {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub duration_seconds: Option<i64>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyListResponse {
    pub keys: Vec<AuthenticationKey>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total: i64,
    pub active: i64,
    pub expired: i64,
    pub deleted: i64,
    pub subscription_distribution: Vec<(String, i64)>,
    pub rate_limit_buckets: Vec<(String, i64)>,
    pub daily_trend: Vec<(String, i64)>,
}

/// One active key that lapses within the early-warning window, carrying the
/// owner identifiers the dashboard renders plus a precomputed `days_left` so
/// the view stays presentation-only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiringKey {
    pub key: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub device_id: String,
    pub expired_at: DateTime<Utc>,
    pub days_left: i64,
}

/// One entry in the "Recent Admin Activity" feed — a single `audit_log` row
/// projected for display. `summary` is a human sentence built server-side
/// from the event type and its JSONB payload, so the view stays
/// presentation-only. Unknown event types degrade to the bare verb.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub event_type: String,
    pub aggregate_id: Option<i32>,
    pub actor: String,
    pub summary: String,
    pub occurred_at: DateTime<Utc>,
}

/// One subscription tier's health snapshot for the dashboard "Tier Health"
/// table: its quota and interval, whether the catalogue still lists it as
/// active, and how many *live* keys (not deleted, not expired) it currently
/// carries. An inactive tier with a non-zero `active_keys` is the anomaly the
/// widget exists to surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierHealth {
    pub display_name: String,
    pub rate_limit_amount: i64,
    pub interval: String,
    pub is_active: bool,
    pub active_keys: i64,
}

/// One issued-but-unused key in the dashboard "Key Hygiene" panel — either an
/// unclaimed pre-issued row (still on the `-` sentinel) or a dormant claimed
/// key (full quota, never touched). `device_id` is `None` for the unclaimed
/// population (the sentinel carries no real device) and `Some` for dormant
/// rows. `age_days` is precomputed server-side so the view stays
/// presentation-only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HygieneKey {
    pub key: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub device_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub age_days: i64,
}

/// One issued-but-unused population for the "Key Hygiene" panel: the capped
/// display `rows` plus the `total` count across the whole population, so the
/// view can render "Showing 10 of N" when the list is truncated.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HygieneSet {
    pub rows: Vec<HygieneKey>,
    pub total: i64,
}

/// The dashboard "Key Hygiene" panel's two issued-but-unused populations:
/// `unclaimed` pre-issued rows (still on the `-` sentinel) and `dormant`
/// claimed keys (full quota, never used). Each carries its capped rows and the
/// total population count.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyHygiene {
    pub unclaimed: Vec<HygieneKey>,
    pub unclaimed_total: i64,
    pub dormant: Vec<HygieneKey>,
    pub dormant_total: i64,
}

/// Current-state dashboard insights, independent of the date-range picker.
/// Holds the "Expiring Soon" list, the "Recent Admin Activity" feed, the
/// "Tier Health" table, and the "Key Hygiene" panel; later steps fold in more
/// at-a-glance signals, so callers should treat extra fields as expected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardInsights {
    pub expiring_keys: Vec<ExpiringKey>,
    pub recent_activity: Vec<AuditEntry>,
    pub tier_health: Vec<TierHealth>,
    pub key_hygiene: KeyHygiene,
}
