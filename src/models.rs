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
