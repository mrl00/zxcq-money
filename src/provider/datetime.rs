//! DateTime provider port.

use chrono::{DateTime, Utc};

/// Port for obtaining the current datetime.
///
/// Abstracts time so domain logic and handlers can be tested deterministically.
///
/// # Example
///
/// ```ignore
/// use zxcq_money::provider::datetime::{DateTimeProvider, SystemDateTime};
///
/// let provider = SystemDateTime;
/// let now = provider.now();
/// ```
pub trait DateTimeProvider: Send + Sync {
    /// Return the current UTC datetime.
    fn now(&self) -> DateTime<Utc>;
}

/// Production datetime provider using the system clock.
pub struct SystemDateTime;

impl DateTimeProvider for SystemDateTime {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Deterministic datetime provider for tests.
///
/// Always returns the same fixed datetime.
#[cfg(test)]
pub struct MockDateTime {
    /// The datetime that will be returned by [`DateTimeProvider::now`].
    pub fixed_now: DateTime<Utc>,
}

#[cfg(test)]
impl MockDateTime {
    /// Create a mock that always returns `fixed_now`.
    pub fn new(fixed_now: DateTime<Utc>) -> Self {
        Self { fixed_now }
    }
}

#[cfg(test)]
impl DateTimeProvider for MockDateTime {
    fn now(&self) -> DateTime<Utc> {
        self.fixed_now
    }
}
