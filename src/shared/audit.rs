/*!
Audit logging infrastructure.

Provides the [`AuditLogger`] port and [`AuditEntry`] struct for recording
who did what, when, and on which resource. The event-driven approach
uses [`AuditEventHandler`] to automatically convert domain events into
audit entries — no handler changes required.

# Architecture

```text
Handler → EventPublisher → AuditEventHandler → AuditLogger
```

The [`AuditableEvent`](super::events::AuditableEvent) wrapper pairs
each domain event with the [`UserID`] of the actor who triggered it.
*/

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::errors::PublishError;
use super::events::DomainEvent;
use super::ids::UserID;

/// A record of a security-relevant action performed by a user.
///
/// Stored by [`AuditLogger`] implementations (append-only table, file, etc.).
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// The user who performed the action.
    pub actor: UserID,
    /// Human-readable action name (e.g. `"record_transaction"`).
    pub action: String,
    /// Type of resource affected (e.g. `"account"`, `"transaction"`).
    pub resource_type: String,
    /// Identifier of the affected resource.
    pub resource_id: String,
    /// When the action occurred.
    pub timestamp: DateTime<Utc>,
    /// Optional additional details (e.g. amount, description).
    pub details: Option<String>,
}

/// Port for persisting audit entries.
///
/// Adapters implement this with an append-only store (database table,
/// file, etc.). The in-memory [`InMemoryAuditLogger`] is used for testing.
#[async_trait]
pub trait AuditLogger: Send + Sync {
    /// Persist an audit entry.
    async fn log(&self, entry: AuditEntry) -> Result<(), PublishError>;
}

/// In-memory audit logger for testing.
///
/// Stores entries in a `Vec` behind a `Mutex`. Not suitable for production.
pub struct InMemoryAuditLogger {
    entries: Mutex<Vec<AuditEntry>>,
}

impl InMemoryAuditLogger {
    /// Creates a new empty logger.
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }

    /// Returns all recorded audit entries.
    pub fn entries(&self) -> Vec<AuditEntry> {
        self.entries.lock().unwrap().clone()
    }

    /// Returns the number of recorded entries.
    pub fn count(&self) -> usize {
        self.entries.lock().unwrap().len()
    }
}

impl Default for InMemoryAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditLogger for InMemoryAuditLogger {
    async fn log(&self, entry: AuditEntry) -> Result<(), PublishError> {
        self.entries.lock().unwrap().push(entry);
        Ok(())
    }
}

/// Event handler that converts domain events into audit entries.
///
/// Buffers entries synchronously and provides a [`flush`](Self::flush) method
/// to persist them via [`AuditLogger`]. This avoids the sync/async mismatch
/// between `EventHandlerFn` (sync) and `AuditLogger::log` (async).
///
/// # Usage
///
/// ```ignore
/// use std::sync::Arc;
/// use zxcq_money::shared::audit::{AuditEventHandler, InMemoryAuditLogger};
///
/// let logger = Arc::new(InMemoryAuditLogger::new());
/// let handler = AuditEventHandler::new(logger.clone());
///
/// // Register for event types — handler_fn returns an owned closure
/// dispatcher.register_handler("TransactionRecorded", handler.handler_fn());
///
/// // After publishing events, flush buffered entries
/// handler.flush().await;
/// ```
pub struct AuditEventHandler<L: AuditLogger> {
    buffer: Arc<Mutex<Vec<AuditEntry>>>,
    logger: Arc<L>,
}

impl<L: AuditLogger> AuditEventHandler<L> {
    /// Creates a new handler that will flush to the given logger.
    pub fn new(logger: Arc<L>) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            logger,
        }
    }

    /// Returns an owned closure suitable for `InMemoryEventDispatcher::register_handler`.
    ///
    /// The closure captures a clone of the internal buffer `Arc` and processes
    /// any `&dyn DomainEvent`, building an [`AuditEntry`] and buffering it
    /// for later flush via [`flush`](Self::flush).
    pub fn handler_fn(&self) -> impl Fn(&dyn DomainEvent) + Send + Sync + Clone + 'static {
        let buffer = self.buffer.clone();
        move |event: &dyn DomainEvent| {
            let entry = build_audit_entry(event);
            buffer.lock().unwrap().push(entry);
        }
    }

    /// Flushes all buffered audit entries to the [`AuditLogger`].
    pub async fn flush(&self) {
        let entries: Vec<AuditEntry> = self.buffer.lock().unwrap().drain(..).collect();
        for entry in entries {
            let _ = self.logger.log(entry).await;
        }
    }

    /// Returns the number of entries currently buffered (not yet flushed).
    pub fn buffered_count(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }
}

/// Builds an [`AuditEntry`] from a domain event.
///
/// The actor is set to a nil UUID by default. Use [`AuditableEvent`](super::events::AuditableEvent)
/// to wrap events with actor information, or construct `AuditEntry` directly
/// in handlers for full control over all fields.
pub fn build_audit_entry(event: &dyn DomainEvent) -> AuditEntry {
    AuditEntry {
        actor: UserID::from_uuid(uuid::Uuid::nil()),
        action: event.event_type().to_string(),
        resource_type: resource_type_from_event(event.event_type()),
        resource_id: String::new(),
        timestamp: event.timestamp(),
        details: None,
    }
}

/// Maps an event type name to a human-readable resource type.
pub fn resource_type_from_event(event_type: &str) -> String {
    if event_type.contains("Account") {
        "account".into()
    } else if event_type.contains("Recurring") {
        "recurring_transaction".into()
    } else if event_type.contains("Import") {
        "import".into()
    } else if event_type.contains("Transaction") || event_type.contains("Transfer") {
        "transaction".into()
    } else if event_type.contains("Budget") {
        "budget".into()
    } else if event_type.contains("Goal") {
        "financial_goal".into()
    } else if event_type.contains("Card") || event_type.contains("Purchase") {
        "credit_card".into()
    } else if event_type.contains("Invoice") {
        "invoice".into()
    } else if event_type.contains("Bill") {
        "bill".into()
    } else if event_type.contains("Asset") {
        "asset".into()
    } else {
        "unknown".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_logger_records_entry() {
        let logger = InMemoryAuditLogger::new();
        assert_eq!(logger.count(), 0);

        let entry = AuditEntry {
            actor: UserID::new(),
            action: "test_action".into(),
            resource_type: "account".into(),
            resource_id: "acc-123".into(),
            timestamp: Utc::now(),
            details: Some("test details".into()),
        };

        logger.log(entry.clone()).await.unwrap();
        assert_eq!(logger.count(), 1);

        let stored = logger.entries();
        assert_eq!(stored[0].action, "test_action");
        assert_eq!(stored[0].resource_type, "account");
        assert_eq!(stored[0].resource_id, "acc-123");
        assert_eq!(stored[0].actor, entry.actor);
    }

    #[tokio::test]
    async fn test_in_memory_logger_multiple_entries() {
        let logger = InMemoryAuditLogger::new();

        for i in 0..5 {
            logger
                .log(AuditEntry {
                    actor: UserID::new(),
                    action: format!("action_{i}"),
                    resource_type: "test".into(),
                    resource_id: i.to_string(),
                    timestamp: Utc::now(),
                    details: None,
                })
                .await
                .unwrap();
        }

        assert_eq!(logger.count(), 5);
    }

    #[test]
    fn test_resource_type_mapping() {
        assert_eq!(resource_type_from_event("AccountOpened"), "account");
        assert_eq!(resource_type_from_event("AccountRenamed"), "account");
        assert_eq!(
            resource_type_from_event("TransactionRecorded"),
            "transaction"
        );
        assert_eq!(resource_type_from_event("TransferCompleted"), "transaction");
        assert_eq!(
            resource_type_from_event("RecurringTransactionCreated"),
            "recurring_transaction"
        );
        assert_eq!(resource_type_from_event("BudgetDefined"), "budget");
        assert_eq!(
            resource_type_from_event("GoalContributed"),
            "financial_goal"
        );
        assert_eq!(resource_type_from_event("PurchaseAdded"), "credit_card");
        assert_eq!(resource_type_from_event("InvoiceClosed"), "invoice");
        assert_eq!(resource_type_from_event("BillScheduled"), "bill");
        assert_eq!(resource_type_from_event("AssetBought"), "asset");
        assert_eq!(resource_type_from_event("TransactionsImported"), "import");
        assert_eq!(resource_type_from_event("SomethingElse"), "unknown");
    }

    #[test]
    fn test_build_audit_entry() {
        use crate::ledger::domain::events::TransactionRecorded;
        use crate::shared::money::{Currency, Money};

        let event = TransactionRecorded {
            transaction_id: crate::shared::ids::TransactionID::new(),
            account_id: crate::shared::ids::AccountID::new(),
            owner_id: UserID::new(),
            tx_type: crate::ledger::domain::transaction::TransactionType::Income,
            amount: Money::from_cents(100, Currency::BRL),
            category_id: None,
            description: "test".into(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            timestamp: Utc::now(),
        };

        let entry = build_audit_entry(&event);
        assert_eq!(entry.action, "TransactionRecorded");
        assert_eq!(entry.resource_type, "transaction");
        assert_eq!(entry.actor, UserID::from_uuid(uuid::Uuid::nil()));
    }

    #[tokio::test]
    async fn test_audit_event_handler_buffer_and_flush() {
        use crate::ledger::domain::events::TransactionRecorded;
        use crate::shared::events::{EventPublisher, InMemoryEventDispatcher};
        use crate::shared::money::{Currency, Money};

        let logger = Arc::new(InMemoryAuditLogger::new());
        let handler = AuditEventHandler::new(logger.clone());

        let dispatcher = InMemoryEventDispatcher::new();
        dispatcher.register_handler("TransactionRecorded", handler.handler_fn());

        let event = TransactionRecorded {
            transaction_id: crate::shared::ids::TransactionID::new(),
            account_id: crate::shared::ids::AccountID::new(),
            owner_id: UserID::new(),
            tx_type: crate::ledger::domain::transaction::TransactionType::Income,
            amount: Money::from_cents(5000, Currency::BRL),
            category_id: None,
            description: "salary".into(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            timestamp: Utc::now(),
        };

        // Publish event — entry goes to buffer, not logger yet
        dispatcher.publish(vec![&event]).await.unwrap();
        assert_eq!(handler.buffered_count(), 1);
        assert_eq!(logger.count(), 0);

        // Flush — entry moves to logger
        handler.flush().await;
        assert_eq!(handler.buffered_count(), 0);
        assert_eq!(logger.count(), 1);

        let entries = logger.entries();
        assert_eq!(entries[0].action, "TransactionRecorded");
        assert_eq!(entries[0].resource_type, "transaction");
    }

    #[tokio::test]
    async fn test_audit_event_handler_multiple_events() {
        use crate::ledger::domain::events::{AccountOpened, TransactionRecorded};
        use crate::shared::events::{EventPublisher, InMemoryEventDispatcher};
        use crate::shared::money::{Currency, Money};

        let logger = Arc::new(InMemoryAuditLogger::new());
        let handler = AuditEventHandler::new(logger.clone());

        let dispatcher = InMemoryEventDispatcher::new();
        dispatcher.register_handler("AccountOpened", handler.handler_fn());
        dispatcher.register_handler("TransactionRecorded", handler.handler_fn());

        let account_event = AccountOpened {
            account_id: crate::shared::ids::AccountID::new(),
            owner_id: crate::shared::ids::UserID::new(),
            name: "Checking".into(),
            currency: Currency::BRL,
            opening_balance: Money::from_cents(1000, Currency::BRL),
            timestamp: Utc::now(),
        };

        let tx_event = TransactionRecorded {
            transaction_id: crate::shared::ids::TransactionID::new(),
            account_id: crate::shared::ids::AccountID::new(),
            owner_id: UserID::new(),
            tx_type: crate::ledger::domain::transaction::TransactionType::Expense,
            amount: Money::from_cents(250, Currency::BRL),
            category_id: None,
            description: "coffee".into(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 3, 10).unwrap(),
            timestamp: Utc::now(),
        };

        dispatcher
            .publish(vec![&account_event as &dyn DomainEvent, &tx_event])
            .await
            .unwrap();
        assert_eq!(handler.buffered_count(), 2);

        handler.flush().await;
        assert_eq!(logger.count(), 2);

        let entries = logger.entries();
        assert_eq!(entries[0].resource_type, "account");
        assert_eq!(entries[1].resource_type, "transaction");
    }
}
