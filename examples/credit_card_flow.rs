//! Credit card flow example: register card, add purchases, close and pay invoice.
//!
//! Run with: `cargo run --example credit_card_flow`

use rust_decimal::prelude::ToPrimitive;
use zxcq_money::credit_card::domain::card::CreditCard;
use zxcq_money::credit_card::domain::invoice::Invoice;
use zxcq_money::credit_card::domain::purchase::Purchase;
use zxcq_money::shared::ids::{CategoryID, CreditCardID, InvoiceID, PurchaseID, UserID};
use zxcq_money::shared::money::{Currency, Money};
use zxcq_money::shared::period::YearMonth;

#[tokio::main]
async fn main() {
    let owner = UserID::new();

    // Register a credit card
    let card = CreditCard::new(
        CreditCardID::new(),
        owner,
        "Nubank".into(),
        "Mastercard".into(),
        Money::from_cents(1000000, Currency::BRL), // R$ 10.000,00 limit
        20,                                        // closing day
        27,                                        // due day
    );

    println!("Registered card: {} ({})", card.name, card.brand);
    println!(
        "  Limit: R$ {:.2}",
        card.limit.amount().to_f64().unwrap() / 100.0
    );

    // Create an open invoice for January 2026
    let mut invoice = Invoice::new(InvoiceID::new(), card.id, YearMonth::new(2026, 1));

    // Add purchases
    let netflix = Purchase::new(
        PurchaseID::new(),
        "Netflix".into(),
        Money::from_cents(39_90, Currency::BRL),
        1,
        CategoryID::new(),
        chrono::NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
    );

    let supermarket = Purchase::new(
        PurchaseID::new(),
        "Supermarket".into(),
        Money::from_cents(35000, Currency::BRL),
        1,
        CategoryID::new(),
        chrono::NaiveDate::from_ymd_opt(2026, 1, 10).unwrap(),
    );

    let tv = Purchase::new(
        PurchaseID::new(),
        "Smart TV".into(),
        Money::from_cents(300000, Currency::BRL),
        3, // 3 installments
        CategoryID::new(),
        chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
    );

    invoice.add_purchase(netflix).unwrap();
    invoice.add_purchase(supermarket).unwrap();
    invoice.add_purchase(tv).unwrap();

    println!(
        "\nInvoice {} - {}",
        invoice.reference_month.year, invoice.reference_month.month
    );
    println!("  Purchases: {}", invoice.purchases.len());
    println!(
        "  Total: R$ {:.2}",
        invoice.total().amount().to_f64().unwrap() / 100.0
    );

    // Close the invoice
    invoice.close().unwrap();
    println!("\nInvoice closed at: {:?}", invoice.closed_at);

    // Pay the invoice
    invoice.pay().unwrap();
    println!("Invoice paid! Status: {:?}", invoice.status);

    // Check available limit
    let used = Money::from_cents(338990, Currency::BRL);
    let available = card.available_limit(used).unwrap();
    println!(
        "\nAvailable limit: R$ {:.2}",
        available.amount().to_f64().unwrap() / 100.0
    );
}
