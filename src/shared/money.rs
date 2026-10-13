//! Monetary value object with currency-safe arithmetic.
//!
//! Amounts are stored as [`rust_decimal::Decimal`] in major currency units
//! (e.g. `15.50` for R$ 15,50). The [`Currency`] enum enforces that operations
//! only combine matching currencies.
//!
//! Formatting, rounding, splitting, and exchange-rate conversion are powered by
//! the [`rusty_money`] crate. The [`Currency`] enum implements
//! [`FormattableCurrency`], enabling locale-aware display via [`Money::formatted`].
//!
//! # Example
//!
//! ```ignore
//! use zxcq_money::shared::money::{Money, Currency};
//!
//! let price = Money::new(dec!(49.90), Currency::BRL);  // R$ 49,90
//! let tax   = Money::new(dec!(9.98),  Currency::BRL);  // R$  9,98
//! let total = price.checked_add(tax).unwrap();           // R$ 59,88
//! assert_eq!(total.formatted(), "R$ 59.88");
//! ```

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rusty_money::{Exchange, Locale, Round};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};

/// A monetary value with an associated currency.
///
/// Amounts are stored as [`Decimal`] in major currency units (e.g. `15.50`).
/// This avoids floating-point rounding errors common in financial calculations
/// and provides arbitrary precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Serialize, Deserialize)]
pub struct Money {
    amount: Decimal,
    currency: Currency,
}

/// Supported currencies (ISO 4217 subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Serialize, Deserialize)]
pub enum Currency {
    /// Brazilian Real
    BRL,
    /// United States Dollar
    USD,
    /// Euro
    EUR,
}

impl Currency {
    /// Return the currency symbol (e.g. `"R$"`, `"$"`, `"€"`).
    pub fn symbol(&self) -> &'static str {
        match self {
            Currency::BRL => "R$",
            Currency::USD => "$",
            Currency::EUR => "€",
        }
    }

    /// Return the ISO 4217 currency code (e.g. `"BRL"`, `"USD"`, `"EUR"`).
    pub fn code(&self) -> &'static str {
        match self {
            Currency::BRL => "BRL",
            Currency::USD => "USD",
            Currency::EUR => "EUR",
        }
    }

    /// Return the number of decimal places (exponent) for the currency.
    ///
    /// All supported currencies use 2 decimal places (centavos, cents, cents).
    pub fn exponent(&self) -> u32 {
        2
    }

    /// Return the [`Locale`] used for locale-aware formatting.
    ///
    /// - BRL → [`Locale::EnBy`] (Brazilian: `R$ 1.234,56`)
    /// - USD → [`Locale::EnUs`] (US: `$1,234.56`)
    /// - EUR → [`Locale::EnEu`] (European: `€1.234,56`)
    pub fn locale(&self) -> Locale {
        match self {
            Currency::BRL => Locale::EnBy,
            Currency::USD => Locale::EnUs,
            Currency::EUR => Locale::EnEu,
        }
    }

    /// Return `true` if the currency symbol comes before the amount.
    ///
    /// BRL and USD place the symbol first (`R$ 15,50`); EUR places it after
    /// in some locales, but `EnEu` uses symbol-first as well.
    pub fn symbol_first(&self) -> bool {
        true
    }
}

impl rusty_money::FormattableCurrency for Currency {
    fn to_string(&self) -> String {
        self.code().to_string()
    }

    fn exponent(&self) -> u32 {
        Currency::exponent(self)
    }

    fn code(&self) -> &'static str {
        Currency::code(self)
    }

    fn locale(&self) -> Locale {
        Currency::locale(self)
    }

    fn symbol(&self) -> &'static str {
        Currency::symbol(self)
    }

    fn symbol_first(&self) -> bool {
        Currency::symbol_first(self)
    }
}

impl From<Currency> for rusty_money::iso::Currency {
    fn from(c: Currency) -> Self {
        match c {
            Currency::BRL => *rusty_money::iso::BRL,
            Currency::USD => *rusty_money::iso::USD,
            Currency::EUR => *rusty_money::iso::EUR,
        }
    }
}

impl Money {
    /// Create a new `Money` from a decimal amount and a currency.
    pub fn new(amount: Decimal, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// Create a `Money` from an amount in minor currency units (cents).
    ///
    /// This is a convenience for migrating from the old `i64`-based API.
    /// `Money::from_cents(1500, BRL)` creates R$ 15,00.
    pub fn from_cents(cents: i64, currency: Currency) -> Self {
        Self {
            amount: Decimal::from(cents) / Decimal::from(100),
            currency,
        }
    }

    /// Create a zero-amount `Money` in the given currency.
    pub fn zero(currency: Currency) -> Self {
        Self {
            amount: Decimal::ZERO,
            currency,
        }
    }

    /// Return the amount as a [`Decimal`].
    pub fn amount(&self) -> Decimal {
        self.amount
    }

    /// Return the amount truncated to an integer (no fractional part).
    pub fn amount_trunc(&self) -> Decimal {
        self.amount.trunc()
    }

    /// Return the amount in minor currency units (cents) as `i64`.
    ///
    /// Returns `None` if the amount cannot be exactly represented as `i64` cents.
    pub fn to_cents(&self) -> Option<i64> {
        (self.amount * Decimal::from(100)).to_i64()
    }

    /// Return the currency.
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns `true` if the amount is positive.
    pub fn is_positive(&self) -> bool {
        self.amount > Decimal::ZERO
    }

    /// Returns `true` if the amount is negative.
    pub fn is_negative(&self) -> bool {
        self.amount < Decimal::ZERO
    }

    /// Returns `true` if the amount is zero.
    pub fn is_zero(&self) -> bool {
        self.amount == Decimal::ZERO
    }

    /// Return the absolute value of the amount.
    pub fn abs(&self) -> Self {
        Self {
            amount: self.amount.abs(),
            currency: self.currency,
        }
    }

    /// Safely add two `Money` values.
    ///
    /// Returns [`LedgerError::CurrencyMismatch`] if currencies differ.
    pub fn checked_add(self, other: Money) -> Result<Money, crate::shared::errors::LedgerError> {
        if self.currency != other.currency {
            return Err(crate::shared::errors::LedgerError::CurrencyMismatch {
                expected: self.currency.code().to_string(),
                received: other.currency.code().to_string(),
            });
        }
        Ok(Money {
            amount: self.amount + other.amount,
            currency: self.currency,
        })
    }

    /// Safely subtract two `Money` values.
    ///
    /// Returns [`LedgerError::CurrencyMismatch`] if currencies differ.
    pub fn checked_sub(self, other: Money) -> Result<Money, crate::shared::errors::LedgerError> {
        if self.currency != other.currency {
            return Err(crate::shared::errors::LedgerError::CurrencyMismatch {
                expected: self.currency.code().to_string(),
                received: other.currency.code().to_string(),
            });
        }
        Ok(Money {
            amount: self.amount - other.amount,
            currency: self.currency,
        })
    }

    // ── rusty-money integration ──────────────────────────────────────

    /// Convert to a [`rusty_money::Money`] for formatting and advanced operations.
    ///
    /// The returned value uses the same `Decimal` amount and maps the
    /// [`Currency`] enum to the corresponding ISO 4217 currency.
    pub fn to_rusty(&self) -> rusty_money::Money<'static, rusty_money::iso::Currency> {
        let iso_cur: &'static rusty_money::iso::Currency = match self.currency {
            Currency::BRL => rusty_money::iso::BRL,
            Currency::USD => rusty_money::iso::USD,
            Currency::EUR => rusty_money::iso::EUR,
        };
        rusty_money::Money::from_decimal(self.amount, iso_cur)
    }

    /// Format the amount using locale-aware currency formatting.
    ///
    /// Delegates to [`rusty_money::Money`]'s `Display` implementation.
    ///
    /// # Examples
    ///
    /// ```
    /// use zxcq_money::shared::money::{Currency, Money};
    /// use rust_decimal_macros::dec;
    ///
    /// let m = Money::new(dec!(1234.56), Currency::BRL);
    /// assert_eq!(m.formatted(), "R$1.234,56");
    ///
    /// let m = Money::new(dec!(1234.56), Currency::USD);
    /// assert_eq!(m.formatted(), "$1,234.56");
    /// ```
    pub fn formatted(&self) -> String {
        self.to_rusty().to_string()
    }

    /// Split the amount equally into `n` shares.
    ///
    /// The remainder (in minor units) is distributed to the first shares.
    /// Delegates to [`rusty_money::Money::split`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zxcq_money::shared::money::{Currency, Money};
    /// use rust_decimal_macros::dec;
    ///
    /// let m = Money::new(dec!(10.00), Currency::USD);
    /// let parts = m.split(3).unwrap();
    /// assert_eq!(parts.len(), 3);
    /// assert_eq!(parts[0], Money::new(dec!(3.34), Currency::USD));
    /// ```
    pub fn split(&self, n: u32) -> Result<Vec<Money>, crate::shared::errors::LedgerError> {
        let rusty = self.to_rusty();
        let parts = rusty.split(n).map_err(map_money_error)?;
        Ok(parts
            .into_iter()
            .map(|p| Money::new(*p.amount(), self.currency))
            .collect())
    }

    /// Divide the amount into shares according to the given weights.
    ///
    /// Remainders are distributed to shares with non-zero weights in input order.
    /// Delegates to [`rusty_money::Money::allocate`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zxcq_money::shared::money::{Currency, Money};
    /// use rust_decimal_macros::dec;
    ///
    /// let m = Money::new(dec!(100.00), Currency::USD);
    /// let parts = m.allocate(vec![70, 20, 10]).unwrap();
    /// assert_eq!(parts[0], Money::new(dec!(70.00), Currency::USD));
    /// ```
    pub fn allocate(
        &self,
        weights: Vec<u32>,
    ) -> Result<Vec<Money>, crate::shared::errors::LedgerError> {
        let rusty = self.to_rusty();
        let parts = rusty.allocate(weights).map_err(map_money_error)?;
        Ok(parts
            .into_iter()
            .map(|p| Money::new(*p.amount(), self.currency))
            .collect())
    }

    /// Round the amount to `digits` decimal places using the given strategy.
    ///
    /// The original amount is unchanged. Delegates to
    /// [`rusty_money::Money::round`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zxcq_money::shared::money::{Currency, Money};
    /// use rusty_money::Round;
    /// use rust_decimal_macros::dec;
    ///
    /// let m = Money::new(dec!(10.005), Currency::USD);
    /// assert_eq!(m.round(2, Round::HalfUp), Money::new(dec!(10.01), Currency::USD));
    /// ```
    pub fn round(&self, digits: u32, strategy: Round) -> Money {
        let rounded = self.to_rusty().round(digits, strategy);
        Money::new(*rounded.amount(), self.currency)
    }

    /// Convert this amount to another currency using the provided exchange rates.
    ///
    /// Delegates to [`rusty_money::Money::exchange_to`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zxcq_money::shared::money::{Currency, Money};
    /// use rusty_money::{Exchange, ExchangeRate};
    /// use rust_decimal_macros::dec;
    ///
    /// let mut exchange = Exchange::new();
    /// let rate = ExchangeRate::new(rusty_money::iso::USD, rusty_money::iso::EUR, dec!(0.85)).unwrap();
    /// exchange.set_rate(&rate);
    ///
    /// let usd = Money::new(dec!(100.00), Currency::USD);
    /// let eur = usd.exchange_to(Currency::EUR, &exchange).unwrap();
    /// assert_eq!(eur, Money::new(dec!(85.00), Currency::EUR));
    /// ```
    pub fn exchange_to(
        &self,
        target: Currency,
        exchange: &Exchange<'static, rusty_money::iso::Currency>,
    ) -> Result<Money, crate::shared::errors::LedgerError> {
        let rusty = self.to_rusty();
        let target_iso = rusty_money::iso::Currency::from(target);
        let result = rusty
            .exchange_to(&target_iso, exchange)
            .map_err(map_money_error)?;
        Ok(Money::new(*result.amount(), target))
    }
}

/// Map a [`rusty_money::MoneyError`] to the corresponding [`LedgerError`].
fn map_money_error(err: rusty_money::MoneyError) -> crate::shared::errors::LedgerError {
    use rusty_money::MoneyError;
    match err {
        MoneyError::CurrencyMismatch { expected, actual } => {
            crate::shared::errors::LedgerError::CurrencyMismatch {
                expected: expected.to_string(),
                received: actual.to_string(),
            }
        }
        MoneyError::DivisionByZero => {
            crate::shared::errors::LedgerError::InvalidAmount("division by zero".to_string())
        }
        MoneyError::Overflow => {
            crate::shared::errors::LedgerError::InvalidAmount("arithmetic overflow".to_string())
        }
        MoneyError::InvalidRatio => {
            crate::shared::errors::LedgerError::InvalidAmount("invalid split ratio".to_string())
        }
        other => crate::shared::errors::LedgerError::InvalidAmount(other.to_string()),
    }
}

impl Add for Money {
    type Output = Result<Money, crate::shared::errors::LedgerError>;

    fn add(self, rhs: Money) -> Self::Output {
        self.checked_add(rhs)
    }
}

impl Sub for Money {
    type Output = Result<Money, crate::shared::errors::LedgerError>;

    fn sub(self, rhs: Money) -> Self::Output {
        self.checked_sub(rhs)
    }
}

impl std::ops::Mul<Decimal> for Money {
    type Output = Money;

    fn mul(self, rhs: Decimal) -> Self::Output {
        Money {
            amount: self.amount * rhs,
            currency: self.currency,
        }
    }
}

impl std::ops::Mul<i64> for Money {
    type Output = Money;

    fn mul(self, rhs: i64) -> Self::Output {
        Money {
            amount: self.amount * Decimal::from(rhs),
            currency: self.currency,
        }
    }
}

impl std::ops::Div<Decimal> for Money {
    type Output = Money;

    fn div(self, rhs: Decimal) -> Self::Output {
        Money {
            amount: self.amount / rhs,
            currency: self.currency,
        }
    }
}

impl std::ops::Div<i64> for Money {
    type Output = Money;

    fn div(self, rhs: i64) -> Self::Output {
        Money {
            amount: self.amount / Decimal::from(rhs),
            currency: self.currency,
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use rusty_money::ExchangeRate;

    #[test]
    fn test_money_creation() {
        let m = Money::new(dec!(15.00), Currency::BRL);
        assert_eq!(m.amount(), dec!(15.00));
        assert_eq!(m.currency(), Currency::BRL);
    }

    #[test]
    fn test_from_cents() {
        let m = Money::from_cents(1500, Currency::BRL);
        assert_eq!(m.amount(), dec!(15.00));
        assert_eq!(m.to_cents(), Some(1500));
    }

    #[test]
    fn test_money_zero() {
        let m = Money::zero(Currency::USD);
        assert!(m.is_zero());
        assert!(!m.is_positive());
        assert!(!m.is_negative());
    }

    #[test]
    fn test_add_same_currency() {
        let a = Money::new(dec!(10.00), Currency::BRL);
        let b = Money::new(dec!(5.00), Currency::BRL);
        let result = (a + b).unwrap();
        assert_eq!(result.amount(), dec!(15.00));
    }

    #[test]
    fn test_add_different_currency_fails() {
        let a = Money::new(dec!(10.00), Currency::BRL);
        let b = Money::new(dec!(5.00), Currency::USD);
        assert!((a + b).is_err());
    }

    #[test]
    fn test_sub() {
        let a = Money::new(dec!(10.00), Currency::BRL);
        let b = Money::new(dec!(3.00), Currency::BRL);
        let result = (a - b).unwrap();
        assert_eq!(result.amount(), dec!(7.00));
    }

    #[test]
    fn test_mul_decimal() {
        let m = Money::new(dec!(100.00), Currency::BRL);
        let result = m * dec!(1.5);
        assert_eq!(result.amount(), dec!(150.00));
    }

    #[test]
    fn test_mul_i64() {
        let m = Money::new(dec!(50.00), Currency::BRL);
        let result = m * 3;
        assert_eq!(result.amount(), dec!(150.00));
    }

    #[test]
    fn test_div_decimal() {
        let m = Money::new(dec!(150.00), Currency::BRL);
        let result = m / dec!(3);
        assert_eq!(result.amount(), dec!(50.00));
    }

    #[test]
    fn test_div_i64() {
        let m = Money::new(dec!(150.00), Currency::BRL);
        let result = m / 3;
        assert_eq!(result.amount(), dec!(50.00));
    }

    #[test]
    fn test_display() {
        let m = Money::new(dec!(15.50), Currency::BRL);
        assert_eq!(format!("{m}"), "15.50");
    }

    #[test]
    fn test_abs() {
        let m = Money::new(dec!(-5.00), Currency::EUR);
        assert_eq!(m.abs().amount(), dec!(5.00));
    }

    #[test]
    fn test_currency_symbol() {
        assert_eq!(Currency::BRL.symbol(), "R$");
        assert_eq!(Currency::USD.symbol(), "$");
        assert_eq!(Currency::EUR.symbol(), "€");
    }

    #[test]
    fn test_serde_roundtrip() {
        let m = Money::new(dec!(123.45), Currency::BRL);
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(m, deserialized);
    }

    #[test]
    fn test_to_cents() {
        let m = Money::new(dec!(15.50), Currency::BRL);
        assert_eq!(m.to_cents(), Some(1550));
    }

    #[test]
    fn test_amount_trunc() {
        let m = Money::new(dec!(15.99), Currency::BRL);
        assert_eq!(m.amount_trunc(), dec!(15));
    }

    // ── rusty-money integration tests ────────────────────────────────

    #[test]
    fn test_formatted_brl() {
        let m = Money::new(dec!(1234.56), Currency::BRL);
        assert_eq!(m.formatted(), "R$1.234,56");
    }

    #[test]
    fn test_formatted_usd() {
        let m = Money::new(dec!(1234.56), Currency::USD);
        assert_eq!(m.formatted(), "$1,234.56");
    }

    #[test]
    fn test_formatted_eur() {
        let m = Money::new(dec!(1234.56), Currency::EUR);
        assert_eq!(m.formatted(), "€1.234,56");
    }

    #[test]
    fn test_formatted_negative() {
        let m = Money::new(dec!(-1234.56), Currency::USD);
        assert_eq!(m.formatted(), "-$1,234.56");
    }

    #[test]
    fn test_formatted_zero() {
        let m = Money::zero(Currency::BRL);
        assert_eq!(m.formatted(), "R$0,00");
    }

    #[test]
    fn test_to_rusty_preserves_amount() {
        let m = Money::new(dec!(42.50), Currency::EUR);
        let rusty = m.to_rusty();
        assert_eq!(rusty.amount(), &dec!(42.50));
        assert_eq!(rusty.currency().iso_alpha_code, "EUR");
    }

    #[test]
    fn test_split_equal() {
        let m = Money::new(dec!(10.00), Currency::USD);
        let parts = m.split(3).unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], Money::new(dec!(3.34), Currency::USD));
        assert_eq!(parts[1], Money::new(dec!(3.33), Currency::USD));
        assert_eq!(parts[2], Money::new(dec!(3.33), Currency::USD));
    }

    #[test]
    fn test_split_exact() {
        let m = Money::new(dec!(9.00), Currency::BRL);
        let parts = m.split(3).unwrap();
        assert_eq!(parts.len(), 3);
        for p in &parts {
            assert_eq!(*p, Money::new(dec!(3.00), Currency::BRL));
        }
    }

    #[test]
    fn test_split_zero_parts_fails() {
        let m = Money::new(dec!(10.00), Currency::USD);
        assert!(m.split(0).is_err());
    }

    #[test]
    fn test_allocate_weighted() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let parts = m.allocate(vec![70, 20, 10]).unwrap();
        assert_eq!(parts[0], Money::new(dec!(70.00), Currency::USD));
        assert_eq!(parts[1], Money::new(dec!(20.00), Currency::USD));
        assert_eq!(parts[2], Money::new(dec!(10.00), Currency::USD));
    }

    #[test]
    fn test_allocate_with_remainder() {
        let m = Money::new(dec!(10.00), Currency::USD);
        let parts = m.allocate(vec![1, 1, 1]).unwrap();
        let total = parts
            .iter()
            .fold(Money::zero(Currency::USD), |acc, p| (acc + *p).unwrap());
        assert_eq!(total, Money::new(dec!(10.00), Currency::USD));
    }

    #[test]
    fn test_allocate_empty_weights_fails() {
        let m = Money::new(dec!(10.00), Currency::USD);
        assert!(m.allocate(vec![]).is_err());
    }

    #[test]
    fn test_round_half_up() {
        let m = Money::new(dec!(10.005), Currency::USD);
        assert_eq!(
            m.round(2, Round::HalfUp),
            Money::new(dec!(10.01), Currency::USD)
        );
    }

    #[test]
    fn test_round_half_down() {
        let m = Money::new(dec!(10.005), Currency::USD);
        assert_eq!(
            m.round(2, Round::HalfDown),
            Money::new(dec!(10.00), Currency::USD)
        );
    }

    #[test]
    fn test_round_half_even() {
        let m = Money::new(dec!(10.015), Currency::USD);
        assert_eq!(
            m.round(2, Round::HalfEven),
            Money::new(dec!(10.02), Currency::USD)
        );
    }

    #[test]
    fn test_round_does_not_mutate() {
        let m = Money::new(dec!(10.005), Currency::USD);
        let _ = m.round(2, Round::HalfUp);
        assert_eq!(m, Money::new(dec!(10.005), Currency::USD));
    }

    #[test]
    fn test_exchange_to() {
        let mut exchange = Exchange::new();
        let rate =
            ExchangeRate::new(rusty_money::iso::USD, rusty_money::iso::EUR, dec!(0.85)).unwrap();
        exchange.set_rate(&rate);

        let usd = Money::new(dec!(100.00), Currency::USD);
        let eur = usd.exchange_to(Currency::EUR, &exchange).unwrap();
        assert_eq!(eur, Money::new(dec!(85.00), Currency::EUR));
    }

    #[test]
    fn test_exchange_missing_rate_fails() {
        let exchange = Exchange::new();
        let usd = Money::new(dec!(100.00), Currency::USD);
        assert!(usd.exchange_to(Currency::EUR, &exchange).is_err());
    }

    #[test]
    fn test_currency_locale_mapping() {
        assert_eq!(Currency::BRL.locale(), rusty_money::Locale::EnBy);
        assert_eq!(Currency::USD.locale(), rusty_money::Locale::EnUs);
        assert_eq!(Currency::EUR.locale(), rusty_money::Locale::EnEu);
    }

    #[test]
    fn test_currency_exponent() {
        assert_eq!(Currency::BRL.exponent(), 2);
        assert_eq!(Currency::USD.exponent(), 2);
        assert_eq!(Currency::EUR.exponent(), 2);
    }

    #[test]
    fn test_currency_symbol_first() {
        assert!(Currency::BRL.symbol_first());
        assert!(Currency::USD.symbol_first());
        assert!(Currency::EUR.symbol_first());
    }

    #[test]
    fn test_currency_to_iso() {
        let iso: rusty_money::iso::Currency = Currency::BRL.into();
        assert_eq!(iso.iso_alpha_code, "BRL");
        let iso: rusty_money::iso::Currency = Currency::USD.into();
        assert_eq!(iso.iso_alpha_code, "USD");
    }
}
