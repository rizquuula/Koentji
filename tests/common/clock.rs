use chrono::{DateTime, Duration, Utc};
use std::sync::{Arc, RwLock};

/// Abstraction over wall-clock time so the domain can be exercised
/// deterministically in tests. Production code uses `SystemClock`;
/// tests construct a `TestClock` pinned at a known instant and advance it
/// explicitly.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Clone)]
pub struct TestClock {
    inner: Arc<RwLock<DateTime<Utc>>>,
}

impl TestClock {
    pub fn at(t: DateTime<Utc>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(t)),
        }
    }

    pub fn advance(&self, by: Duration) {
        let mut guard = self.inner.write().expect("clock poisoned");
        *guard += by;
    }

    pub fn set(&self, t: DateTime<Utc>) {
        *self.inner.write().expect("clock poisoned") = t;
    }
}

impl Clock for TestClock {
    fn now(&self) -> DateTime<Utc> {
        *self.inner.read().expect("clock poisoned")
    }
}
