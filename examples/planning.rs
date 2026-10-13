//! Financial simulators example: mortgage, loan, retirement, and net salary.
//!
//! Run with: `cargo run --example planning`

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use zxcq_money::planning::loan::simulate_loan;
use zxcq_money::planning::mortgage::{AmortizationMethod, simulate_mortgage};
use zxcq_money::planning::net_salary::{TaxRegime, calculate_net_salary};
use zxcq_money::planning::retirement::{required_contribution, simulate_retirement};
use zxcq_money::shared::money::{Currency, Money};

fn main() {
    let brl = Currency::BRL;

    // ── Mortgage: SAC vs Price ────────────────────────────────
    println!("═══ Mortgage Simulation ═══");
    let principal = Money::from_cents(50000000, brl); // R$ 500.000,00
    let months = 360; // 30 years
    let rate = Decimal::from(10); // 10% annual

    let sac = simulate_mortgage(principal, months, rate, AmortizationMethod::SAC);
    let price = simulate_mortgage(principal, months, rate, AmortizationMethod::Price);

    println!(
        "  Principal: R$ {:.2}",
        principal.amount().to_f64().unwrap() / 100.0
    );
    println!("  Term: {months} months ({} years)", months / 12);
    println!("  Rate: {rate}% p.a.\n");

    println!("  SAC:");
    println!(
        "    First payment: R$ {:.2}",
        sac.entries[0].payment.amount().to_f64().unwrap() / 100.0
    );
    println!(
        "    Last payment:  R$ {:.2}",
        sac.entries
            .last()
            .unwrap()
            .payment
            .amount()
            .to_f64()
            .unwrap()
            / 100.0
    );
    println!(
        "    Total interest: R$ {:.2}",
        sac.total_interest.amount().to_f64().unwrap() / 100.0
    );

    println!("  Price:");
    println!(
        "    Monthly payment: R$ {:.2}",
        price.entries[0].payment.amount().to_f64().unwrap() / 100.0
    );
    println!(
        "    Total interest:  R$ {:.2}",
        price.total_interest.amount().to_f64().unwrap() / 100.0
    );
    println!();

    // ── Loan ──────────────────────────────────────────────────
    println!("═══ Loan Simulation ═══");
    let loan_principal = Money::from_cents(5000000, brl); // R$ 50.000,00
    let loan = simulate_loan(loan_principal, 24, Decimal::from(12));

    println!(
        "  Principal: R$ {:.2}",
        loan_principal.amount().to_f64().unwrap() / 100.0
    );
    println!("  Term: 24 months, Rate: 12% p.a.");
    println!(
        "  Monthly payment: R$ {:.2}",
        loan.entries[0].payment.amount().to_f64().unwrap() / 100.0
    );
    println!(
        "  Total interest:  R$ {:.2}",
        loan.total_interest.amount().to_f64().unwrap() / 100.0
    );
    println!();

    // ── Retirement ────────────────────────────────────────────
    println!("═══ Retirement Projection ═══");
    let current = Money::from_cents(10000000, brl); // R$ 100.000,00
    let monthly = Money::from_cents(200000, brl); // R$ 2.000,00/month
    let years = 20;
    let annual_return = Decimal::from(8);

    let projection = simulate_retirement(current, monthly, years, annual_return);
    println!(
        "  Current savings: R$ {:.2}",
        current.amount().to_f64().unwrap() / 100.0
    );
    println!(
        "  Monthly contribution: R$ {:.2}",
        monthly.amount().to_f64().unwrap() / 100.0
    );
    println!("  Annual return: {annual_return}%");
    println!("  Projection ({years} years):");
    for entry in &projection.timeline {
        if entry.year % 5 == 0 || entry.year == 1 {
            println!(
                "    Year {}: R$ {:.2} (contributions: R$ {:.2}, growth: R$ {:.2})",
                entry.year,
                entry.balance.amount().to_f64().unwrap() / 100.0,
                entry.contributions.amount().to_f64().unwrap() / 100.0,
                entry.growth.amount().to_f64().unwrap() / 100.0,
            );
        }
    }
    println!(
        "  Final balance: R$ {:.2}",
        projection.final_balance.amount().to_f64().unwrap() / 100.0
    );

    let target = Money::from_cents(200000000, brl); // R$ 2.000.000,00
    let needed = required_contribution(target, current, years, annual_return);
    println!(
        "\n  To reach R$ {:.2} in {years} years:",
        target.amount().to_f64().unwrap() / 100.0
    );
    println!(
        "    Required monthly contribution: R$ {:.2}",
        needed.amount().to_f64().unwrap() / 100.0
    );
    println!();

    // ── Net Salary ────────────────────────────────────────────
    println!("═══ Net Salary Calculator ═══");
    let gross = Money::from_cents(1200000, brl); // R$ 12.000,00

    let clt = calculate_net_salary(gross, 2, TaxRegime::CLT);
    println!(
        "  Gross: R$ {:.2}",
        gross.amount().to_f64().unwrap() / 100.0
    );
    println!("  CLT (2 dependents):");
    println!(
        "    INSS:    R$ {:.2}",
        clt.inss.amount().to_f64().unwrap() / 100.0
    );
    println!(
        "    IRRF:    R$ {:.2}",
        clt.irrf.amount().to_f64().unwrap() / 100.0
    );
    println!(
        "    Net:     R$ {:.2}",
        clt.net.amount().to_f64().unwrap() / 100.0
    );

    let pj = calculate_net_salary(gross, 0, TaxRegime::PJ);
    println!("  PJ:");
    println!(
        "    DAS:     R$ {:.2}",
        pj.total_discounts.amount().to_f64().unwrap() / 100.0
    );
    println!(
        "    Net:     R$ {:.2}",
        pj.net.amount().to_f64().unwrap() / 100.0
    );
}
