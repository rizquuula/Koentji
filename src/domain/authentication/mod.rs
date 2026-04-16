//! Authentication bounded context.
//!
//! Houses the value objects and (in later slices) the `Authenticator`
//! aggregate that decides whether a `(key, device)` pair may consume
//! quota right now. Phase 1.1 seeds only the value objects.

pub mod auth_key;
pub mod device_id;
pub mod rate_limit;
pub mod subscription_name;

pub use auth_key::AuthKey;
pub use device_id::DeviceId;
pub use rate_limit::{RateLimitAmount, RateLimitUsage, RateLimitWindow};
pub use subscription_name::SubscriptionName;
