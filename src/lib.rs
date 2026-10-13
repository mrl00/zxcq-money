/*!
# zxc-money

A pure-Rust personal finance core library built with Domain-Driven Design.

This crate provides the domain model and application layer for personal finance
management. It is designed as a **modular monolith** consumed by any frontend
(TUI, web, mobile) via well-defined ports/traits. The core knows nothing about
HTTP, terminals, or UI frameworks.

## Architecture

Each bounded context follows DDD layered architecture:

```text
module/
├── domain/
│   ├── {aggregate}.rs     → Aggregate root / entity
│   ├── events.rs          → Domain events
│   └── repository.rs      → Repository traits (ports)
└── application/
    └── {use_case}.rs       → Command handler (one per operation)
```

Cross-module communication happens exclusively through domain events.
Modules never access each other's repositories directly.

## Modules

- [`shared`] — Value objects, error types, event system, ID types, mocks
- [`provider`] — Port traits for ID generation, datetime, and statement parsing
- [`ledger`] — Accounts, transactions, transfers, recurring transactions
- [`credit_card`] — Credit cards, invoices, purchases, installments
- [`bills`] — Bills reminder (scheduled payments)
- [`budgeting`] — Budgets by category, financial goals
- [`investment`] — Portfolios, assets, positions
- [`importing`] — Statement parsing, preview, and import pipeline
- [`planning`] — Loan, mortgage, retirement, net salary simulators
- [`reporting`] — Account balance projections, net worth (read side)
*/

pub mod bills;
pub mod budgeting;
pub mod credit_card;
pub mod identity;
pub mod importing;
pub mod investment;
pub mod ledger;
pub mod planning;
pub mod provider;
pub mod reporting;
pub mod shared;

// Re-exports: Facades
pub use bills::facade::BillsFacade;
pub use budgeting::facade::BudgetingFacade;
pub use credit_card::facade::CreditCardFacade;
pub use importing::facade::ImportingFacade;
pub use investment::facade::InvestmentFacade;
pub use ledger::facade::LedgerFacade;
pub use reporting::facade::ReportingFacade;

// Re-exports: Provider ports
pub use provider::{
    ColumnMapping, DateTimeProvider, IdGenerator, StatementParser, SystemDateTime, UuidGenerator,
};

// Re-exports: Security types
pub use shared::ids::{IdempotencyKey, Principal};
