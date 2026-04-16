//! Test-data builders for authentication keys.
//!
//! A second vocabulary layer over raw DB rows: tests read as
//! `a_key().with_device("dev-1").insert(&pool).await` rather than opaque
//! INSERT SQL. Defaults mirror the production `create_key` server fn so a
//! "vanilla" key matches what the admin dashboard issues.

use chrono::{DateTime, Duration, Utc};
use koentji::models::AuthenticationKey;
use sqlx::PgPool;

const DEFAULT_RATE_LIMIT: i32 = 6000;
const DEFAULT_SUBSCRIPTION: &str = "free";

/// A mutable recipe for an `AuthenticationKey`. Build up with `with_*`
/// methods, terminate with `insert(&pool).await`.
#[derive(Clone, Debug)]
pub struct KeyBuilder {
    key: String,
    device_id: String,
    subscription: Option<String>,
    rate_limit_daily: i32,
    rate_limit_remaining: Option<i32>,
    username: Option<String>,
    email: Option<String>,
    expired_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    rate_limit_interval_name: String,
}

impl Default for KeyBuilder {
    fn default() -> Self {
        Self {
            key: format!("klab_test_{}", rand_slug()),
            device_id: format!("device_{}", rand_slug()),
            subscription: Some(DEFAULT_SUBSCRIPTION.to_string()),
            rate_limit_daily: DEFAULT_RATE_LIMIT,
            rate_limit_remaining: None,
            username: None,
            email: None,
            expired_at: None,
            deleted_at: None,
            rate_limit_interval_name: "daily".to_string(),
        }
    }
}

impl KeyBuilder {
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = key.into();
        self
    }

    pub fn with_device(mut self, device_id: impl Into<String>) -> Self {
        self.device_id = device_id.into();
        self
    }

    pub fn with_subscription(mut self, name: impl Into<String>) -> Self {
        self.subscription = Some(name.into());
        self
    }

    pub fn with_rate_limit(mut self, daily: i32) -> Self {
        self.rate_limit_daily = daily;
        self.rate_limit_remaining.get_or_insert(daily);
        self
    }

    pub fn with_remaining(mut self, remaining: i32) -> Self {
        self.rate_limit_remaining = Some(remaining);
        self
    }

    pub fn exhausted(mut self) -> Self {
        self.rate_limit_remaining = Some(0);
        self
    }

    pub fn expired(mut self) -> Self {
        self.expired_at = Some(Utc::now() - Duration::hours(1));
        self
    }

    pub fn expires_at(mut self, at: DateTime<Utc>) -> Self {
        self.expired_at = Some(at);
        self
    }

    pub fn revoked(mut self) -> Self {
        self.deleted_at = Some(Utc::now());
        self
    }

    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn with_interval(mut self, name: impl Into<String>) -> Self {
        self.rate_limit_interval_name = name.into();
        self
    }

    pub fn key_string(&self) -> &str {
        &self.key
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub async fn insert(self, pool: &PgPool) -> AuthenticationKey {
        let remaining = self.rate_limit_remaining.unwrap_or(self.rate_limit_daily);

        sqlx::query_as::<_, AuthenticationKey>(
            r#"
            INSERT INTO authentication_keys
                (key, device_id, subscription, rate_limit_daily, rate_limit_remaining,
                 username, email, expired_at, deleted_at,
                 subscription_type_id, rate_limit_interval_id, created_by)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9,
                 (SELECT id FROM subscription_types WHERE name = $3),
                 (SELECT id FROM rate_limit_intervals WHERE name = $10),
                 'test')
            RETURNING *
            "#,
        )
        .bind(&self.key)
        .bind(&self.device_id)
        .bind(&self.subscription)
        .bind(self.rate_limit_daily)
        .bind(remaining)
        .bind(&self.username)
        .bind(&self.email)
        .bind(self.expired_at)
        .bind(self.deleted_at)
        .bind(&self.rate_limit_interval_name)
        .fetch_one(pool)
        .await
        .expect("insert test key")
    }
}

/// An ordinary active key under quota. Vanilla starting point.
pub fn a_key() -> KeyBuilder {
    KeyBuilder::default()
}

/// A key whose `expired_at` is already in the past.
pub fn an_expired_key() -> KeyBuilder {
    KeyBuilder::default().expired()
}

/// A soft-deleted (revoked) key.
pub fn a_revoked_key() -> KeyBuilder {
    KeyBuilder::default().revoked()
}

/// A free-trial key — the one the public endpoint upserts on first use.
pub fn a_free_trial_key() -> KeyBuilder {
    KeyBuilder::default()
        .with_key("FREE_TRIAL")
        .with_subscription("free")
}

fn rand_slug() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", n)
}
