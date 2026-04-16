//! Authentication bounded context.
//!
//! Houses the value objects, the `AuthDecision` pure-output enum, and
//! the `IssuedKey` aggregate that decides whether a `(key, device)`
//! pair may consume quota right now. Later slices (1.4+) will bolt on
//! the repository port + Postgres adapter.

pub mod auth_cache_port;
pub mod auth_decision;
pub mod auth_key;
pub mod device_id;
pub mod issued_key;
pub mod issued_key_repository;
pub mod rate_limit;
pub mod subscription_name;

#[cfg(feature = "ssr")]
pub use auth_cache_port::AuthCachePort;
pub use auth_decision::{AuthDecision, DenialReason};
pub use auth_key::AuthKey;
pub use device_id::DeviceId;
pub use issued_key::{IssuedKey, IssuedKeyId, RateLimitLedger, FREE_TRIAL_MARKER_DEFAULT};
#[cfg(feature = "ssr")]
pub use issued_key_repository::IssuedKeyRepository;
pub use issued_key_repository::{ConsumeOutcome, FreeTrialConfig, RepositoryError};
pub use rate_limit::{RateLimitAmount, RateLimitUsage, RateLimitWindow};
pub use subscription_name::SubscriptionName;
