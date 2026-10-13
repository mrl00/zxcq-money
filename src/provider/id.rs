//! ID generation port.

use uuid::Uuid;

/// Port for generating unique identifiers.
///
/// Implementations can use any UUID strategy (v4, v7, etc.) or return
/// deterministic IDs for testing.
///
/// # Example
///
/// ```ignore
/// use zxcq_money::provider::id::{IdGenerator, UuidGenerator};
///
/// let gen = UuidGenerator;
/// let id = gen.new_id(); // random UUID v4
/// ```
pub trait IdGenerator: Send + Sync {
    /// Generate a new unique identifier.
    fn new_id(&self) -> Uuid;
}

/// Production ID generator using UUID v4.
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn new_id(&self) -> Uuid {
        Uuid::new_v4()
    }
}

/// Deterministic ID generator for tests.
///
/// Always returns the same UUID, making assertions predictable.
#[cfg(test)]
pub struct MockIdGenerator {
    /// The UUID that will be returned by [`IdGenerator::new_id`].
    pub next_id: Uuid,
}

#[cfg(test)]
impl MockIdGenerator {
    /// Create a mock that always returns `next_id`.
    pub fn new(next_id: Uuid) -> Self {
        Self { next_id }
    }
}

#[cfg(test)]
impl IdGenerator for MockIdGenerator {
    fn new_id(&self) -> Uuid {
        self.next_id
    }
}
