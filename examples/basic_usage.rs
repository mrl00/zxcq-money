//! Basic usage example: creating accounts and recording transactions.
//!
//! Run with: `cargo run --example basic_usage`

use zxcq_money::ledger::domain::account::AccountType;
use zxcq_money::ledger::domain::repository::{AccountRepository, TransactionRepository};
use zxcq_money::ledger::domain::transaction::TransactionType;
use zxcq_money::shared::ids::{AccountID, TransactionID, UserID};
use zxcq_money::shared::mock::{MockAccountRepository, MockTransactionRepository};
use zxcq_money::shared::money::{Currency, Money};

#[tokio::main]
async fn main() {
    let account_repo = MockAccountRepository::new();
    let tx_repo = MockTransactionRepository::new();
    let owner = UserID::new();

    // Create a checking account
    let account = zxcq_money::ledger::domain::account::Account::new(
        AccountID::new(),
        owner,
        "Nubank Checking".into(),
        AccountType::Checking,
        Currency::BRL,
        Money::from_cents(100000, Currency::BRL), // R$ 1.000,00
    )
    .unwrap();

    account_repo.save(&account).await.unwrap();
    println!("Created account: {} (id: {})", account.name, account.id);

    // Record a salary income
    let salary = zxcq_money::ledger::domain::transaction::Transaction::new(
        TransactionID::new(),
        account.id,
        TransactionType::Income,
        Money::from_cents(800000, Currency::BRL), // R$ 8.000,00
        "Monthly Salary".into(),
        chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
    )
    .unwrap();

    tx_repo.save(&salary).await.unwrap();
    println!(
        "Recorded income: {} ({})",
        salary.description, salary.amount
    );

    // Record an expense
    let grocery = zxcq_money::ledger::domain::transaction::Transaction::new(
        TransactionID::new(),
        account.id,
        TransactionType::Expense,
        Money::from_cents(45000, Currency::BRL), // R$ 450,00
        "Supermarket".into(),
        chrono::NaiveDate::from_ymd_opt(2026, 1, 20).unwrap(),
    )
    .unwrap();

    tx_repo.save(&grocery).await.unwrap();
    println!(
        "Recorded expense: {} ({})",
        grocery.description, grocery.amount
    );

    // Query transactions for the account
    let period = zxcq_money::shared::period::Period::new(
        chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
    );

    let transactions = tx_repo.find_by_account(account.id, period).await.unwrap();
    println!("\nTransactions in January 2026:");
    for tx in &transactions {
        println!("  {}: {} ({})", tx.date, tx.description, tx.amount);
    }
}
