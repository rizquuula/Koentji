//! Postgres implementation of
//! [`crate::domain::authentication::IssuedKeyRepository`].
//!
//! Two responsibilities:
//!
//! - [`find`] hydrates the `IssuedKey` aggregate from a join of
//!   `authentication_keys` with `rate_limit_intervals`. Value-object
//!   parse errors collapse to `None` — if a row is malformed the
//!   domain treats it as "unknown", which the use case maps to
//!   `DenialReason::UnknownKey`.
//!
//! - [`consume_quota`] performs the atomic decrement in SQL, the same
//!   `UPDATE … RETURNING` that landed in commit 0.3. 1.6 will replace
//!   the free-standing helper in `src/rate_limit.rs` with this port
//!   call; for now the helper and the adapter share the SQL verbatim
//!   so behaviour is identical.
//!
//! Note on semantics: the SQL keeps the legacy off-by-one
//! (`daily > usage`, `remaining > usage`) so the public `/v1/auth`
//! envelope stays byte-identical — changing the predicate to `>=`
//! would flip the 401/429 boundary for clients that rely on the
//! "last slot refused" behaviour. The cleanup is deferred to the
//! Phase 1 wiring commit (1.6) alongside any envelope work.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::authentication::{
    AuthKey, ConsumeOutcome, DeviceId, FreeTrialConfig, IssueKeyCommand, IssuedKey, IssuedKeyId,
    IssuedKeyRepository, RateLimitAmount, RateLimitLedger, RateLimitUsage, RateLimitWindow,
    RepositoryError, SubscriptionName,
};

#[derive(Clone)]
pub struct PostgresIssuedKeyRepository {
    pool: PgPool,
}

impl PostgresIssuedKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl IssuedKeyRepository for PostgresIssuedKeyRepository {
    async fn find(
        &self,
        key: &AuthKey,
        device: &DeviceId,
    ) -> Result<Option<IssuedKey>, RepositoryError> {
        let row: Option<IssuedKeyRow> = sqlx::query_as::<_, IssuedKeyRow>(
            r#"
            SELECT
                ak.id,
                ak.key,
                ak.device_id,
                ak.subscription,
                ak.rate_limit_daily,
                ak.rate_limit_remaining,
                ak.rate_limit_updated_at,
                ak.expired_at,
                ak.deleted_at,
                ak.username,
                ak.email,
                COALESCE(
                    (SELECT duration_seconds FROM rate_limit_intervals
                     WHERE id = ak.rate_limit_interval_id),
                    86400
                ) AS window_seconds
            FROM authentication_keys ak
            WHERE ak.key = $1 AND ak.device_id = $2
            "#,
        )
        .bind(key.as_str())
        .bind(device.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Backend(e.to_string()))?;

        Ok(row.and_then(|r| r.into_aggregate().ok()))
    }

    async fn consume_quota(
        &self,
        key: &AuthKey,
        device: &DeviceId,
        usage: RateLimitUsage,
        now: DateTime<Utc>,
    ) -> Result<ConsumeOutcome, RepositoryError> {
        let row: Option<(i32, DateTime<Utc>)> = sqlx::query_as(
            r#"
            UPDATE authentication_keys ak
            SET
                rate_limit_remaining = CASE
                    WHEN ak.rate_limit_updated_at IS NULL
                      OR EXTRACT(EPOCH FROM ($1::timestamptz - ak.rate_limit_updated_at))
                         >= COALESCE(
                                (SELECT duration_seconds FROM rate_limit_intervals
                                 WHERE id = ak.rate_limit_interval_id),
                                86400)
                    THEN ak.rate_limit_daily - $2::int
                    ELSE ak.rate_limit_remaining - $2::int
                END,
                rate_limit_updated_at = $1::timestamptz
            WHERE ak.key = $3
              AND ak.device_id = $4
              AND (
                  ak.rate_limit_updated_at IS NULL
                  OR EXTRACT(EPOCH FROM ($1::timestamptz - ak.rate_limit_updated_at))
                     >= COALESCE(
                            (SELECT duration_seconds FROM rate_limit_intervals
                             WHERE id = ak.rate_limit_interval_id),
                            86400)
                  OR ak.rate_limit_remaining > $2::int
              )
              AND ak.rate_limit_daily > $2::int
            RETURNING ak.rate_limit_remaining, ak.rate_limit_updated_at
            "#,
        )
        .bind(now)
        .bind(usage.value())
        .bind(key.as_str())
        .bind(device.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::Backend(e.to_string()))?;

        Ok(match row {
            Some((remaining, updated_at)) => ConsumeOutcome::Allowed {
                remaining: RateLimitAmount::literal(remaining),
                updated_at,
            },
            None => ConsumeOutcome::RateLimitExceeded,
        })
    }

    async fn claim_free_trial(
        &self,
        key: &AuthKey,
        device: &DeviceId,
        config: &FreeTrialConfig,
    ) -> Result<Option<IssuedKey>, RepositoryError> {
        use chrono::{Datelike, Utc};

        let backend = |e: sqlx::Error| RepositoryError::Backend(e.to_string());

        // Branch A — client presented the FREE_TRIAL magic marker.
        if key.as_str() == config.marker {
            let sub: Option<(i32, i32, i32)> = sqlx::query_as(
                "SELECT id, rate_limit_amount, rate_limit_interval_id
                 FROM subscription_types
                 WHERE name = $1 AND is_active = true
                 LIMIT 1",
            )
            .bind(&config.subscription_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;

            let (sub_name, sub_type_id, rate_limit, interval_id) = if let Some((
                id,
                quota,
                interval_id,
            )) = sub
            {
                (
                    config.subscription_name.clone(),
                    Some(id),
                    quota,
                    Some(interval_id),
                )
            } else {
                log::warn!(
                    "Free trial subscription type '{}' not found or inactive, using hardcoded fallback (6000 daily)",
                    config.subscription_name
                );
                ("free_trial".to_string(), None, 6000, None)
            };

            let now = Utc::now();
            let d = now.date_naive();
            let (y, m) = if d.month() == 12 {
                (d.year() + 1, 1u32)
            } else {
                (d.year(), d.month() + 1)
            };
            let next_month = chrono::NaiveDate::from_ymd_opt(y, m, 1)
                .and_then(|nd| nd.and_hms_opt(0, 0, 0))
                .map(|ndt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));

            sqlx::query(
                r#"INSERT INTO authentication_keys
                    (key, device_id, subscription, subscription_type_id,
                     rate_limit_interval_id, created_by, created_at, updated_at,
                     expired_at, rate_limit_daily, rate_limit_remaining)
                   VALUES ($1, $2, $3, $4, $5, 'system', $6, $6, $7, $8, $8)"#,
            )
            .bind(key.as_str())
            .bind(device.as_str())
            .bind(&sub_name)
            .bind(sub_type_id)
            .bind(interval_id)
            .bind(now)
            .bind(next_month)
            .bind(rate_limit)
            .execute(&self.pool)
            .await
            .map_err(backend)?;

            log::info!(
                "Free trial created: device={}, subscription={}",
                device.as_str(),
                sub_name
            );

            return self.find(key, device).await;
        }

        // Branch B — a pre-issued key is waiting with `device_id = '-'`.
        let rebinding: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM authentication_keys
             WHERE key = $1 AND device_id = '-'
             LIMIT 1",
        )
        .bind(key.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;

        if rebinding.is_some() {
            sqlx::query(
                "UPDATE authentication_keys
                 SET device_id = $1, updated_at = NOW()
                 WHERE key = $2 AND device_id = '-'",
            )
            .bind(device.as_str())
            .bind(key.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;

            log::info!("Device bound to existing key: device={}", device.as_str());
            return self.find(key, device).await;
        }

        Ok(None)
    }

    async fn issue_key(&self, command: IssueKeyCommand) -> Result<IssuedKey, RepositoryError> {
        let backend = |e: sqlx::Error| RepositoryError::Backend(e.to_string());

        let subscription_name = command
            .subscription
            .as_ref()
            .map(|s| s.as_str().to_string());
        let daily = command.rate_limit_daily.value();

        let (id,): (i32,) = sqlx::query_as(
            r#"INSERT INTO authentication_keys
                (key, device_id, subscription, subscription_type_id, rate_limit_daily,
                 rate_limit_remaining, rate_limit_interval_id, username, email,
                 expired_at, created_by)
               VALUES ($1, $2, $3, $4, $5, $5, $6, $7, $8, $9, $10)
               RETURNING id"#,
        )
        .bind(command.key.as_str())
        .bind(command.device.as_str())
        .bind(&subscription_name)
        .bind(command.subscription_type_id)
        .bind(daily)
        .bind(command.rate_limit_interval_id)
        .bind(&command.username)
        .bind(&command.email)
        .bind(command.expired_at)
        .bind(&command.issued_by)
        .fetch_one(&self.pool)
        .await
        .map_err(backend)?;

        log::info!(
            "Key issued: id={}, device={}, issued_by={}",
            id,
            command.device.as_str(),
            command.issued_by
        );

        self.find(&command.key, &command.device)
            .await
            .and_then(|o| {
                o.ok_or_else(|| {
                    RepositoryError::Backend(format!("inserted key id={} vanished", id))
                })
            })
    }
}

/// Wire row — the raw columns we pull back before re-assembling the
/// domain aggregate. Keeping this private so the `#[derive(FromRow)]`
/// never leaks onto an `IssuedKey`.
#[derive(sqlx::FromRow)]
struct IssuedKeyRow {
    id: i32,
    key: String,
    device_id: String,
    subscription: Option<String>,
    rate_limit_daily: i32,
    rate_limit_remaining: i32,
    rate_limit_updated_at: Option<DateTime<Utc>>,
    expired_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    username: Option<String>,
    email: Option<String>,
    window_seconds: i64,
}

impl IssuedKeyRow {
    fn into_aggregate(self) -> Result<IssuedKey, crate::domain::errors::DomainError> {
        let key = AuthKey::parse(self.key)?;
        let device_id = DeviceId::parse(self.device_id)?;
        let subscription = match self.subscription {
            Some(s) => Some(SubscriptionName::parse(s)?),
            None => None,
        };
        let daily = RateLimitAmount::new(self.rate_limit_daily)?;
        let remaining = RateLimitAmount::new(self.rate_limit_remaining)?;
        let window = RateLimitWindow::from_seconds(self.window_seconds)?;

        let is_free_trial = subscription
            .as_ref()
            .is_some_and(|s| s.as_str().eq_ignore_ascii_case("free"));

        Ok(IssuedKey {
            id: IssuedKeyId::new(self.id),
            key,
            device_id,
            subscription,
            rate_limit: RateLimitLedger {
                daily,
                remaining,
                window,
                last_updated_at: self.rate_limit_updated_at,
            },
            expired_at: self.expired_at,
            revoked_at: self.deleted_at,
            is_free_trial,
            username: self.username,
            email: self.email,
        })
    }
}
