use crate::credit_card::domain::card::CreditCard;
use crate::credit_card::domain::invoice::Invoice;
use crate::credit_card::domain::repository::{CreditCardRepository, InvoiceRepository};
use crate::shared::ids::{AssetID, BudgetID, CreditCardID, GoalID, InvoiceID, PortfolioID};
use crate::shared::period::{Period, YearMonth};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::bills::domain::bill::Bill;
use crate::bills::domain::repository::BillRepository;
use crate::budgeting::domain::budget::Budget;
use crate::budgeting::domain::goal::FinancialGoal;
use crate::budgeting::domain::repository::{BudgetRepository, GoalRepository};
use crate::identity::domain::repository::UserRepository;
use crate::identity::domain::user::User;
use crate::investment::domain::asset::Asset;
use crate::investment::domain::portfolio::Portfolio;
use crate::investment::domain::repository::{AssetRepository, PortfolioRepository};
use crate::shared::errors::RepositoryError;
use crate::shared::ids::{AccountID, BillID, IdempotencyKey, TransactionID, UserID};
use crate::shared::repository::IdempotencyRepository;

use crate::ledger::domain::account::Account;
use crate::ledger::domain::recurring_transaction::RecurringTransaction;
use crate::ledger::domain::repository::{
    AccountRepository, RecurringTransactionRepository, TransactionFilter, TransactionRepository,
};
use crate::ledger::domain::transaction::Transaction;
use crate::shared::ids::RecurringTransactionID;

/// In-memory mock implementation of [`AccountRepository`].
///
/// Stores accounts in a `HashMap` behind a `Mutex`. Suitable for unit tests
/// and integration tests where no persistence is needed.
pub struct MockAccountRepository {
    accounts: Mutex<HashMap<AccountID, Account>>,
}

impl MockAccountRepository {
    /// Creates a new empty mock repository.
    pub fn new() -> Self {
        Self {
            accounts: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockAccountRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AccountRepository for MockAccountRepository {
    async fn save(&self, account: &Account) -> Result<(), RepositoryError> {
        let mut accounts = self.accounts.lock().unwrap();
        accounts.insert(account.id, account.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: AccountID) -> Result<Option<Account>, RepositoryError> {
        let accounts = self.accounts.lock().unwrap();
        Ok(accounts.get(&id).cloned())
    }

    async fn find_by_owner(&self, owner: UserID) -> Result<Vec<Account>, RepositoryError> {
        let accounts = self.accounts.lock().unwrap();
        let result: Vec<Account> = accounts
            .values()
            .filter(|a| a.owner_id == owner)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn delete(&self, id: AccountID) -> Result<(), RepositoryError> {
        let mut accounts = self.accounts.lock().unwrap();
        accounts.remove(&id);
        Ok(())
    }
}

/// In-memory mock implementation of [`TransactionRepository`].
pub struct MockTransactionRepository {
    transactions: Mutex<HashMap<TransactionID, Transaction>>,
}

impl MockTransactionRepository {
    /// Creates a new empty mock repository.
    pub fn new() -> Self {
        Self {
            transactions: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockTransactionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TransactionRepository for MockTransactionRepository {
    async fn save(&self, transaction: &Transaction) -> Result<(), RepositoryError> {
        let mut transactions = self.transactions.lock().unwrap();
        transactions.insert(transaction.id, transaction.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: TransactionID) -> Result<Option<Transaction>, RepositoryError> {
        let transactions = self.transactions.lock().unwrap();
        Ok(transactions.get(&id).cloned())
    }

    async fn find_by_account(
        &self,
        account_id: AccountID,
        period: Period,
    ) -> Result<Vec<Transaction>, RepositoryError> {
        let transactions = self.transactions.lock().unwrap();
        let result: Vec<Transaction> = transactions
            .values()
            .filter(|t| t.account_id == account_id && period.contains(t.date))
            .cloned()
            .collect();
        Ok(result)
    }

    async fn find_by_account_filtered(
        &self,
        account_id: AccountID,
        period: Period,
        filter: &TransactionFilter,
    ) -> Result<Vec<Transaction>, RepositoryError> {
        let transactions = self.transactions.lock().unwrap();
        let result: Vec<Transaction> = transactions
            .values()
            .filter(|t| {
                t.account_id == account_id
                    && period.contains(t.date)
                    && filter.tx_type.is_none_or(|ty| t.tx_type == ty)
                    && filter
                        .category_id
                        .is_none_or(|cid| t.category_id == Some(cid))
                    && filter.reconciled.is_none_or(|r| t.reconciled == r)
                    && filter.tags.as_ref().is_none_or(|required_tags| {
                        required_tags.iter().any(|tag| t.tags.contains(tag))
                    })
            })
            .cloned()
            .collect();
        Ok(result)
    }

    async fn has_transactions(&self, account_id: AccountID) -> Result<bool, RepositoryError> {
        let transactions = self.transactions.lock().unwrap();
        Ok(transactions.values().any(|t| t.account_id == account_id))
    }

    async fn delete(&self, id: TransactionID) -> Result<(), RepositoryError> {
        let mut transactions = self.transactions.lock().unwrap();
        transactions.remove(&id);
        Ok(())
    }
}

/// In-memory mock implementation of [`RecurringTransactionRepository`].
pub struct MockRecurringTransactionRepository {
    recurring: Mutex<HashMap<RecurringTransactionID, RecurringTransaction>>,
}

impl MockRecurringTransactionRepository {
    /// Creates a new empty mock repository.
    pub fn new() -> Self {
        Self {
            recurring: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockRecurringTransactionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RecurringTransactionRepository for MockRecurringTransactionRepository {
    async fn save(&self, recurring: &RecurringTransaction) -> Result<(), RepositoryError> {
        let mut map = self.recurring.lock().unwrap();
        map.insert(recurring.id, recurring.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: RecurringTransactionID,
    ) -> Result<Option<RecurringTransaction>, RepositoryError> {
        let recurring = self.recurring.lock().unwrap();
        Ok(recurring.get(&id).cloned())
    }

    async fn find_by_owner(
        &self,
        owner: UserID,
    ) -> Result<Vec<RecurringTransaction>, RepositoryError> {
        let recurring = self.recurring.lock().unwrap();
        let result: Vec<RecurringTransaction> = recurring
            .values()
            .filter(|r| r.owner_id == owner)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn find_due(
        &self,
        today: chrono::NaiveDate,
    ) -> Result<Vec<RecurringTransaction>, RepositoryError> {
        let recurring = self.recurring.lock().unwrap();
        let result: Vec<RecurringTransaction> = recurring
            .values()
            .filter(|r| r.is_due(today))
            .cloned()
            .collect();
        Ok(result)
    }

    async fn delete(&self, id: RecurringTransactionID) -> Result<(), RepositoryError> {
        let mut recurring = self.recurring.lock().unwrap();
        recurring.remove(&id);
        Ok(())
    }
}

/// In-memory mock implementation of [`CreditCardRepository`].
pub struct MockCreditCardRepository {
    cards: Mutex<HashMap<CreditCardID, CreditCard>>,
}

impl MockCreditCardRepository {
    /// Creates a new empty mock repository.
    pub fn new() -> Self {
        Self {
            cards: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockCreditCardRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CreditCardRepository for MockCreditCardRepository {
    async fn save(&self, card: &CreditCard) -> Result<(), RepositoryError> {
        let mut cards = self.cards.lock().unwrap();
        cards.insert(card.id, card.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: CreditCardID) -> Result<Option<CreditCard>, RepositoryError> {
        let cards = self.cards.lock().unwrap();
        Ok(cards.get(&id).cloned())
    }

    async fn find_by_owner(&self, owner: UserID) -> Result<Vec<CreditCard>, RepositoryError> {
        let cards = self.cards.lock().unwrap();
        let result: Vec<CreditCard> = cards
            .values()
            .filter(|c| c.owner_id == owner)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn delete(&self, id: CreditCardID) -> Result<(), RepositoryError> {
        let mut cards = self.cards.lock().unwrap();
        cards.remove(&id);
        Ok(())
    }
}

/// In-memory mock implementation of [`InvoiceRepository`].
pub struct MockInvoiceRepository {
    invoices: Mutex<HashMap<InvoiceID, Invoice>>,
}

impl MockInvoiceRepository {
    /// Creates a new empty mock repository.
    pub fn new() -> Self {
        Self {
            invoices: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockInvoiceRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl InvoiceRepository for MockInvoiceRepository {
    async fn save(&self, invoice: &Invoice) -> Result<(), RepositoryError> {
        let mut invoices = self.invoices.lock().unwrap();
        invoices.insert(invoice.id, invoice.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: InvoiceID) -> Result<Option<Invoice>, RepositoryError> {
        let invoices = self.invoices.lock().unwrap();
        Ok(invoices.get(&id).cloned())
    }

    async fn find_open(
        &self,
        credit_card_id: CreditCardID,
    ) -> Result<Option<Invoice>, RepositoryError> {
        let invoices = self.invoices.lock().unwrap();
        let result = invoices
            .values()
            .find(|i| i.credit_card_id == credit_card_id && i.is_open())
            .cloned();
        Ok(result)
    }

    async fn find_by_card_and_month(
        &self,
        credit_card_id: CreditCardID,
        reference_month: YearMonth,
    ) -> Result<Option<Invoice>, RepositoryError> {
        let invoices = self.invoices.lock().unwrap();
        let result = invoices
            .values()
            .find(|i| i.credit_card_id == credit_card_id && i.reference_month == reference_month)
            .cloned();
        Ok(result)
    }
}

/// In-memory mock implementation of [`BudgetRepository`].
pub struct MockBudgetRepository {
    budgets: Mutex<HashMap<BudgetID, Budget>>,
}

impl MockBudgetRepository {
    /// Creates a new empty mock repository.
    pub fn new() -> Self {
        Self {
            budgets: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockBudgetRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl BudgetRepository for MockBudgetRepository {
    async fn save(&self, budget: &Budget) -> Result<(), RepositoryError> {
        let mut budgets = self.budgets.lock().unwrap();
        budgets.insert(budget.id, budget.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: BudgetID) -> Result<Option<Budget>, RepositoryError> {
        let budgets = self.budgets.lock().unwrap();
        Ok(budgets.get(&id).cloned())
    }

    async fn find_by_owner(&self, owner: UserID) -> Result<Vec<Budget>, RepositoryError> {
        let budgets = self.budgets.lock().unwrap();
        let result: Vec<Budget> = budgets
            .values()
            .filter(|b| b.owner_id == owner)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn find_by_category_and_period(
        &self,
        owner: UserID,
        category_id: crate::shared::ids::CategoryID,
        period: Period,
    ) -> Result<Option<Budget>, RepositoryError> {
        let budgets = self.budgets.lock().unwrap();
        let result = budgets
            .values()
            .find(|b| b.owner_id == owner && b.category_id == category_id && b.period == period)
            .cloned();
        Ok(result)
    }

    async fn delete(&self, id: BudgetID) -> Result<(), RepositoryError> {
        let mut budgets = self.budgets.lock().unwrap();
        budgets.remove(&id);
        Ok(())
    }
}

/// In-memory mock implementation of [`GoalRepository`].
pub struct MockGoalRepository {
    goals: Mutex<HashMap<GoalID, FinancialGoal>>,
}

impl MockGoalRepository {
    /// Creates a new empty mock repository.
    pub fn new() -> Self {
        Self {
            goals: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockGoalRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl GoalRepository for MockGoalRepository {
    async fn save(&self, goal: &FinancialGoal) -> Result<(), RepositoryError> {
        let mut goals = self.goals.lock().unwrap();
        goals.insert(goal.id, goal.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: GoalID) -> Result<Option<FinancialGoal>, RepositoryError> {
        let goals = self.goals.lock().unwrap();
        Ok(goals.get(&id).cloned())
    }

    async fn find_by_owner(&self, owner: UserID) -> Result<Vec<FinancialGoal>, RepositoryError> {
        let goals = self.goals.lock().unwrap();
        let result: Vec<FinancialGoal> = goals
            .values()
            .filter(|g| g.owner_id == owner)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn find_by_linked_account(
        &self,
        account_id: crate::shared::ids::AccountID,
    ) -> Result<Vec<FinancialGoal>, RepositoryError> {
        let goals = self.goals.lock().unwrap();
        let result: Vec<FinancialGoal> = goals
            .values()
            .filter(|g| g.linked_account_id == Some(account_id))
            .cloned()
            .collect();
        Ok(result)
    }

    async fn delete(&self, id: GoalID) -> Result<(), RepositoryError> {
        let mut goals = self.goals.lock().unwrap();
        goals.remove(&id);
        Ok(())
    }
}

/// In-memory mock implementation of [`BillRepository`].
pub struct MockBillRepository {
    bills: Mutex<HashMap<BillID, Bill>>,
}

impl MockBillRepository {
    /// Creates a new empty mock repository.
    pub fn new() -> Self {
        Self {
            bills: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockBillRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl BillRepository for MockBillRepository {
    async fn save(&self, bill: &Bill) -> Result<(), RepositoryError> {
        let mut bills = self.bills.lock().unwrap();
        bills.insert(bill.id, bill.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: BillID) -> Result<Option<Bill>, RepositoryError> {
        let bills = self.bills.lock().unwrap();
        Ok(bills.get(&id).cloned())
    }

    async fn find_by_owner(&self, owner: UserID) -> Result<Vec<Bill>, RepositoryError> {
        let bills = self.bills.lock().unwrap();
        let result: Vec<Bill> = bills
            .values()
            .filter(|b| b.owner_id == owner)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn find_pending(&self) -> Result<Vec<Bill>, RepositoryError> {
        let bills = self.bills.lock().unwrap();
        let result: Vec<Bill> = bills
            .values()
            .filter(|b| b.status == crate::bills::domain::bill::BillStatus::Pending)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn find_overdue(&self) -> Result<Vec<Bill>, RepositoryError> {
        let bills = self.bills.lock().unwrap();
        let result: Vec<Bill> = bills
            .values()
            .filter(|b| b.status == crate::bills::domain::bill::BillStatus::Overdue)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn delete(&self, id: BillID) -> Result<(), RepositoryError> {
        let mut bills = self.bills.lock().unwrap();
        bills.remove(&id);
        Ok(())
    }
}

/// In-memory mock implementation of [`AssetRepository`].
pub struct MockAssetRepository {
    assets: Mutex<HashMap<AssetID, Asset>>,
}

impl MockAssetRepository {
    /// Creates a new empty mock repository.
    pub fn new() -> Self {
        Self {
            assets: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockAssetRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AssetRepository for MockAssetRepository {
    async fn save(&self, asset: &Asset) -> Result<(), RepositoryError> {
        let mut assets = self.assets.lock().unwrap();
        assets.insert(asset.id, asset.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: AssetID) -> Result<Option<Asset>, RepositoryError> {
        let assets = self.assets.lock().unwrap();
        Ok(assets.get(&id).cloned())
    }

    async fn find_by_ticker(&self, ticker: &str) -> Result<Option<Asset>, RepositoryError> {
        let assets = self.assets.lock().unwrap();
        Ok(assets.values().find(|a| a.ticker == ticker).cloned())
    }

    async fn delete(&self, id: AssetID) -> Result<(), RepositoryError> {
        let mut assets = self.assets.lock().unwrap();
        assets.remove(&id);
        Ok(())
    }
}

/// In-memory mock implementation of [`PortfolioRepository`].
pub struct MockPortfolioRepository {
    portfolios: Mutex<HashMap<PortfolioID, Portfolio>>,
}

impl MockPortfolioRepository {
    /// Creates a new empty mock repository.
    pub fn new() -> Self {
        Self {
            portfolios: Mutex::new(HashMap::new()),
        }
    }

    /// Returns all portfolios (test helper, not part of the trait).
    pub async fn find_all(&self) -> Result<Vec<Portfolio>, RepositoryError> {
        let portfolios = self.portfolios.lock().unwrap();
        Ok(portfolios.values().cloned().collect())
    }
}

impl Default for MockPortfolioRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PortfolioRepository for MockPortfolioRepository {
    async fn save(&self, portfolio: &Portfolio) -> Result<(), RepositoryError> {
        let mut portfolios = self.portfolios.lock().unwrap();
        portfolios.insert(portfolio.id, portfolio.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: PortfolioID) -> Result<Option<Portfolio>, RepositoryError> {
        let portfolios = self.portfolios.lock().unwrap();
        Ok(portfolios.get(&id).cloned())
    }

    async fn find_by_owner(&self, owner: UserID) -> Result<Vec<Portfolio>, RepositoryError> {
        let portfolios = self.portfolios.lock().unwrap();
        Ok(portfolios
            .values()
            .filter(|p| p.owner_id == owner)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: PortfolioID) -> Result<(), RepositoryError> {
        let mut portfolios = self.portfolios.lock().unwrap();
        portfolios.remove(&id);
        Ok(())
    }
}

/// In-memory mock implementation of [`UserRepository`].
pub struct MockUserRepository {
    users: Mutex<HashMap<UserID, User>>,
    email_index: Mutex<HashMap<String, UserID>>,
}

impl MockUserRepository {
    pub fn new() -> Self {
        Self {
            users: Mutex::new(HashMap::new()),
            email_index: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl UserRepository for MockUserRepository {
    async fn save(&self, user: &User) -> Result<(), RepositoryError> {
        let mut users = self.users.lock().unwrap();
        let mut email_idx = self.email_index.lock().unwrap();
        email_idx.insert(user.email.clone(), user.id);
        users.insert(user.id, user.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: UserID) -> Result<Option<User>, RepositoryError> {
        let users = self.users.lock().unwrap();
        Ok(users.get(&id).cloned())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepositoryError> {
        let email_idx = self.email_index.lock().unwrap();
        let users = self.users.lock().unwrap();
        Ok(email_idx.get(email).and_then(|id| users.get(id).cloned()))
    }

    async fn delete(&self, id: UserID) -> Result<(), RepositoryError> {
        let mut users = self.users.lock().unwrap();
        let mut email_idx = self.email_index.lock().unwrap();
        if let Some(user) = users.remove(&id) {
            email_idx.remove(&user.email);
        }
        Ok(())
    }
}

/// In-memory mock implementation of [`IdempotencyRepository`].
pub struct MockIdempotencyRepository {
    used_keys: Mutex<HashMap<IdempotencyKey, ()>>,
}

impl MockIdempotencyRepository {
    pub fn new() -> Self {
        Self {
            used_keys: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MockIdempotencyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl IdempotencyRepository for MockIdempotencyRepository {
    async fn exists(&self, key: IdempotencyKey) -> Result<bool, RepositoryError> {
        Ok(self.used_keys.lock().unwrap().contains_key(&key))
    }

    async fn mark_used(&self, key: IdempotencyKey) -> Result<(), RepositoryError> {
        self.used_keys.lock().unwrap().insert(key, ());
        Ok(())
    }
}

// ── AuditLogger mock ─────────────────────────────────────────────

use crate::shared::audit::{AuditEntry, AuditLogger};
use crate::shared::errors::PublishError;

/// In-memory mock for [`AuditLogger`]. Stores entries in a `Vec`.
pub struct MockAuditLogger {
    entries: Mutex<Vec<AuditEntry>>,
}

impl MockAuditLogger {
    /// Creates a new empty mock.
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

impl Default for MockAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AuditLogger for MockAuditLogger {
    async fn log(&self, entry: AuditEntry) -> Result<(), PublishError> {
        self.entries.lock().unwrap().push(entry);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ids::CategoryID;
    use crate::shared::money::{Currency, Money};

    #[tokio::test]
    async fn test_mock_account_repository() {
        let repo = MockAccountRepository::new();
        let owner_id = UserID::new();
        let account = Account::new(
            AccountID::new(),
            owner_id,
            "Test Account".into(),
            crate::ledger::domain::account::AccountType::Checking,
            Currency::BRL,
            Money::from_cents(1000, Currency::BRL),
        )
        .unwrap();

        repo.save(&account).await.unwrap();
        let found = repo.find_by_id(account.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Account");

        repo.delete(account.id).await.unwrap();
        let found = repo.find_by_id(account.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_mock_account_find_by_owner() {
        let repo = MockAccountRepository::new();
        let owner1 = UserID::new();
        let owner2 = UserID::new();

        let account1 = Account::new(
            AccountID::new(),
            owner1,
            "Account 1".into(),
            crate::ledger::domain::account::AccountType::Checking,
            Currency::BRL,
            Money::from_cents(1000, Currency::BRL),
        )
        .unwrap();
        let account2 = Account::new(
            AccountID::new(),
            owner1,
            "Account 2".into(),
            crate::ledger::domain::account::AccountType::Savings,
            Currency::BRL,
            Money::from_cents(2000, Currency::BRL),
        )
        .unwrap();
        let account3 = Account::new(
            AccountID::new(),
            owner2,
            "Account 3".into(),
            crate::ledger::domain::account::AccountType::Checking,
            Currency::BRL,
            Money::from_cents(3000, Currency::BRL),
        )
        .unwrap();

        repo.save(&account1).await.unwrap();
        repo.save(&account2).await.unwrap();
        repo.save(&account3).await.unwrap();

        let owner1_accounts = repo.find_by_owner(owner1).await.unwrap();
        assert_eq!(owner1_accounts.len(), 2);

        let owner2_accounts = repo.find_by_owner(owner2).await.unwrap();
        assert_eq!(owner2_accounts.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_transaction_repository() {
        let repo = MockTransactionRepository::new();
        let account_id = AccountID::new();
        let transaction = Transaction::new(
            TransactionID::new(),
            account_id,
            crate::ledger::domain::transaction::TransactionType::Income,
            Money::from_cents(500, Currency::BRL),
            "Salary".into(),
            chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
        )
        .unwrap();

        repo.save(&transaction).await.unwrap();
        let found = repo.find_by_id(transaction.id).await.unwrap();
        assert!(found.is_some());

        let period = Period::new(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
        );
        let txns = repo.find_by_account(account_id, period).await.unwrap();
        assert_eq!(txns.len(), 1);

        repo.delete(transaction.id).await.unwrap();
        let found = repo.find_by_id(transaction.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_mock_bill_repository() {
        let repo = MockBillRepository::new();
        let owner_id = UserID::new();
        let bill = Bill::new(
            BillID::new(),
            owner_id,
            "Internet".into(),
            Some(Money::from_cents(99_90, Currency::BRL)),
            chrono::NaiveDate::from_ymd_opt(2026, 2, 10).unwrap(),
            Some(crate::bills::domain::bill::RecurrenceRule::Monthly),
            CategoryID::new(),
        );

        repo.save(&bill).await.unwrap();
        let found = repo.find_by_id(bill.id).await.unwrap();
        assert!(found.is_some());

        let owner_bills = repo.find_by_owner(owner_id).await.unwrap();
        assert_eq!(owner_bills.len(), 1);

        let pending = repo.find_pending().await.unwrap();
        assert_eq!(pending.len(), 1);

        let overdue = repo.find_overdue().await.unwrap();
        assert_eq!(overdue.len(), 0);

        repo.delete(bill.id).await.unwrap();
        let found = repo.find_by_id(bill.id).await.unwrap();
        assert!(found.is_none());
    }
}
