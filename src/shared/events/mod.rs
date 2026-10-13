//! Domain event system.
//!
//! Provides the core infrastructure for publishing and handling domain events:
//!
//! - [`DomainEvent`] — trait that all events must implement
//! - [`EventPublisher`] — async port for publishing events
//! - [`EventHandler`] — typed handler trait for specific event types
//! - [`InMemoryEventDispatcher`] — synchronous in-process dispatcher for testing
//!
//! # Example
//!
//! ```ignore
//! use zxcq_money::shared::events::InMemoryEventDispatcher;
//!
//! let dispatcher = InMemoryEventDispatcher::new();
//! dispatcher.register_handler("InvoicePaid", |event| {
//!     // downcast and handle
//! });
//! dispatcher.publish(vec![&event]).await?;
//! ```

use chrono::{DateTime, Utc};
use std::any::Any;

use super::errors::PublishError;
use super::ids::UserID;

/// A domain event that occurred in the system.
///
/// All event structs must implement this trait. The `as_any()` method
/// enables downcasting when handling events through the untyped dispatcher.
pub trait DomainEvent: Any + Send + Sync {
    /// Return the event type name (e.g. `"TransactionRecorded"`).
    fn event_type(&self) -> &'static str;

    /// Return the UTC timestamp when the event occurred.
    fn timestamp(&self) -> DateTime<Utc>;

    /// Upcast to `&dyn Any` for downcasting in handlers.
    fn as_any(&self) -> &dyn Any;
}

/// Wrapper that pairs any domain event with the [`UserID`] of the actor
/// who triggered it.
///
/// Use this when publishing events to enable audit logging:
///
/// ```ignore
/// let event = AuditableEvent::new(principal.user_id, TransactionRecorded { ... });
/// publisher.publish(vec![&event]).await?;
/// ```
#[derive(Debug)]
pub struct AuditableEvent<E: DomainEvent> {
    /// The user who triggered this event.
    pub actor: UserID,
    /// The inner domain event.
    pub event: E,
}

impl<E: DomainEvent> AuditableEvent<E> {
    /// Wraps an event with the given actor.
    pub fn new(actor: UserID, event: E) -> Self {
        Self { actor, event }
    }
}

impl<E: DomainEvent> DomainEvent for AuditableEvent<E> {
    fn event_type(&self) -> &'static str {
        self.event.event_type()
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.event.timestamp()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A boxed closure that handles domain events by type string.
pub type EventHandlerFn = Box<dyn Fn(&dyn DomainEvent) + Send + Sync>;

/// Port for publishing domain events asynchronously.
///
/// Implementations can be synchronous (in-memory dispatcher) or
/// asynchronous (message queue, outbox pattern, etc.).
#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a batch of domain events.
    async fn publish(&self, events: Vec<&dyn DomainEvent>) -> Result<(), PublishError>;
}

/// Typed handler trait for a specific event type.
///
/// Use with [`InMemoryEventDispatcher::register_handler`] for type-safe
/// event handling with automatic downcasting.
pub trait EventHandler<E: DomainEvent>: Send + Sync {
    /// Handle the given event.
    fn handle(&self, event: &E);
}

/// In-memory event dispatcher for single-process use and testing.
///
/// Handlers are registered by event type string. When events are published,
/// all matching handlers are called synchronously in registration order.
///
/// # Example
///
/// ```ignore
/// use zxcq_money::shared::events::InMemoryEventDispatcher;
///
/// let dispatcher = InMemoryEventDispatcher::new();
/// dispatcher.register_handler("TransactionRecorded", |event| {
///     println!("Transaction recorded: {:?}", event);
/// });
/// ```
pub struct InMemoryEventDispatcher {
    handlers: std::sync::RwLock<std::collections::HashMap<&'static str, Vec<EventHandlerFn>>>,
}

impl InMemoryEventDispatcher {
    /// Create a new empty dispatcher with no registered handlers.
    pub fn new() -> Self {
        Self {
            handlers: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Register a handler for events with the given type string.
    ///
    /// Multiple handlers can be registered for the same event type.
    /// They will be called in registration order when the event is published.
    pub fn register_handler<F>(&self, event_type: &'static str, handler: F)
    where
        F: Fn(&dyn DomainEvent) + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().unwrap();
        handlers
            .entry(event_type)
            .or_default()
            .push(Box::new(handler));
    }
}

impl Default for InMemoryEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl EventPublisher for InMemoryEventDispatcher {
    async fn publish(&self, events: Vec<&dyn DomainEvent>) -> Result<(), PublishError> {
        let handlers = self.handlers.read().unwrap();
        for event in &events {
            if let Some(event_handlers) = handlers.get(event.event_type()) {
                for handler in event_handlers {
                    handler(*event);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct TestEvent {
        timestamp: DateTime<Utc>,
        #[allow(dead_code)]
        value: usize,
    }

    impl DomainEvent for TestEvent {
        fn event_type(&self) -> &'static str {
            "TestEvent"
        }

        fn timestamp(&self) -> DateTime<Utc> {
            self.timestamp
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[tokio::test]
    async fn test_dispatcher_calls_handler() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let dispatcher = InMemoryEventDispatcher::new();
        dispatcher.register_handler("TestEvent", move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event = TestEvent {
            timestamp: Utc::now(),
            value: 42,
        };

        dispatcher.publish(vec![&event]).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_dispatcher_multiple_handlers() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = counter.clone();
        let c2 = counter.clone();

        let dispatcher = InMemoryEventDispatcher::new();
        dispatcher.register_handler("TestEvent", move |_| {
            c1.fetch_add(1, Ordering::SeqCst);
        });
        dispatcher.register_handler("TestEvent", move |_| {
            c2.fetch_add(10, Ordering::SeqCst);
        });

        let event = TestEvent {
            timestamp: Utc::now(),
            value: 1,
        };

        dispatcher.publish(vec![&event]).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 11);
    }

    #[tokio::test]
    async fn test_dispatcher_ignores_unregistered() {
        let dispatcher = InMemoryEventDispatcher::new();
        let event = TestEvent {
            timestamp: Utc::now(),
            value: 1,
        };
        dispatcher.publish(vec![&event]).await.unwrap();
    }

    #[tokio::test]
    async fn test_dispatcher_multiple_events() {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();

        let dispatcher = InMemoryEventDispatcher::new();
        dispatcher.register_handler("TestEvent", move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });

        let e1 = TestEvent {
            timestamp: Utc::now(),
            value: 1,
        };
        let e2 = TestEvent {
            timestamp: Utc::now(),
            value: 2,
        };

        dispatcher.publish(vec![&e1, &e2]).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
