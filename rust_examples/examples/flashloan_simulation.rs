//! Example: Flashloan Arbitrage Simulation
//!
//! This example demonstrates how to simulate flashloan-based arbitrage.
//!
//! Run with: cargo run --example flashloan_simulation

use anyhow::Result;
use dex_arbitrage::{
    ConstantProductAMM, FlashloanExecutor, FlashloanProvider,
    FlashloanTxBuilder, GasEstimate, Pool,
};

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════╗");
    println!("║     Flashloan Arbitrage Simulation         ║");
    println!("╚════════════════════════════════════════════╝\n");

    let eth_price = 2000.0; // Current ETH price

    // ============================================
    // 1. Flashloan Provider Comparison
    // ============================================
    println!("1. FLASHLOAN PROVIDER COMPARISON");
    println!("{:-<60}", "");

    let providers = [
        FlashloanProvider::Aave,
        FlashloanProvider::DyDx,
        FlashloanProvider::UniswapV3,
        FlashloanProvider::Balancer,
    ];

    println!("{:>15} {:>12} {:>15} {:>15}", "Provider", "Fee Rate", "Fee on $100k", "Max Loan");

    for provider in &providers {
        let fee = provider.fee_rate() * 100_000.0;
        println!(
            "{:>15} {:>11.3}% {:>15.2} {:>12.0}M",
            format!("{:?}", provider),
            provider.fee_rate() * 100.0,
            fee,
            provider.max_loan_usd() / 1_000_000.0
        );
    }

    // ============================================
    // 2. Profitability Analysis
    // ============================================
    println!("\n\n2. PROFITABILITY ANALYSIS");
    println!("{:-<60}", "");

    let executor = FlashloanExecutor::new(FlashloanProvider::Aave)
        .with_min_profit(10.0);

    // Different scenarios
    let scenarios = [
        ("Small trade", 10_000.0, 50.0, 30.0),
        ("Medium trade", 100_000.0, 300.0, 100.0),
        ("Large trade", 1_000_000.0, 1500.0, 300.0),
        ("Whale trade", 10_000_000.0, 5000.0, 500.0),
    ];

    println!("{:>15} {:>12} {:>12} {:>10} {:>12} {:>10}",
             "Scenario", "Loan", "Gross Prof", "Gas", "Net Prof", "Status");
    println!("{:-<75}", "");

    for (name, loan, gross, gas) in scenarios {
        let result = executor.simulate_execution(loan, gross, gas);
        let status = if result.success { "PROFIT" } else { "NO-GO" };
        println!(
            "{:>15} {:>12.0} {:>12.2} {:>10.2} {:>12.2} {:>10}",
            name, loan, gross, gas, result.net_profit, status
        );
    }

    // ============================================
    // 3. Complete Flashloan Arbitrage Example
    // ============================================
    println!("\n\n3. COMPLETE FLASHLOAN ARBITRAGE");
    println!("{:-<60}", "");

    // Setup pools with price discrepancy
    let pool_a = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003)
        .with_name("Pool A");
    let pool_b = ConstantProductAMM::new(800.0, 1_640_000.0, 0.003)
        .with_name("Pool B");

    let price_a = pool_a.get_price();
    let price_b = pool_b.get_price();

    println!("Pool Prices:");
    println!("  Pool A: ${:.2} (cheaper, we buy here)", price_a);
    println!("  Pool B: ${:.2} (expensive, we sell here)", price_b);
    println!("  Spread: {:.2}%", (price_b - price_a) / price_a * 100.0);

    // Calculate arbitrage
    let loan_amount = 100_000.0; // Borrow 100k USDC

    println!("\nFlashloan Execution:");
    println!("  1. Borrow {} USDC from Aave", loan_amount);

    // Buy ETH on cheaper pool
    let eth_bought = pool_a.get_amount_out(loan_amount, false); // USDC -> ETH
    let buy_slippage = pool_a.get_slippage(loan_amount, false);
    println!("  2. Buy {:.4} ETH on Pool A (slippage: {:.2}%)", eth_bought, buy_slippage * 100.0);

    // Sell ETH on expensive pool
    let usdc_received = pool_b.get_amount_out(eth_bought, true); // ETH -> USDC
    let sell_slippage = pool_b.get_slippage(eth_bought, true);
    println!("  3. Sell {:.4} ETH on Pool B (slippage: {:.2}%)", eth_bought, sell_slippage * 100.0);
    println!("     Received: {:.2} USDC", usdc_received);

    // Calculate costs and profit
    let flashloan_fee = loan_amount * FlashloanProvider::Aave.fee_rate();
    let gas_estimate = GasEstimate::for_flashloan_arb(50.0, eth_price, 2);
    let repay_amount = loan_amount + flashloan_fee;

    println!("  4. Repay {} USDC + {:.2} fee", loan_amount, flashloan_fee);

    let gross_profit = usdc_received - loan_amount;
    let net_profit = usdc_received - repay_amount - gas_estimate.cost_usd;

    println!("\nProfit Breakdown:");
    println!("  Gross Profit:     ${:>12.2}", gross_profit);
    println!("  Flashloan Fee:    ${:>12.2}", flashloan_fee);
    println!("  Gas Cost:         ${:>12.2}", gas_estimate.cost_usd);
    println!("  {:-<30}", "");
    println!("  Net Profit:       ${:>12.2}", net_profit);
    println!("  ROI:              {:>12.4}%", net_profit / loan_amount * 100.0);

    if net_profit > 0.0 {
        println!("\n  Status: PROFITABLE!");
    } else {
        println!("\n  Status: NOT PROFITABLE (costs exceed spread)");
    }

    // ============================================
    // 4. Build Flashloan Transaction
    // ============================================
    println!("\n\n4. TRANSACTION BUILDER");
    println!("{:-<60}", "");

    let tx = FlashloanTxBuilder::new(FlashloanProvider::Aave, "USDC", loan_amount)
        .swap("PoolA", "USDC", "ETH", loan_amount, eth_bought * 0.99)
        .swap("PoolB", "ETH", "USDC", eth_bought, usdc_received * 0.99)
        .repay()
        .build()?;

    println!("Transaction built:");
    println!("  Provider: {:?}", tx.provider);
    println!("  Loan Token: {}", tx.loan_token);
    println!("  Loan Amount: ${:.2}", tx.loan_amount);
    println!("  Repay Amount: ${:.2}", tx.repay_amount());
    println!("  Operations: {}", tx.operations.len());
    println!("  Estimated Gas: {} units", tx.estimate_gas());

    // ============================================
    // 5. Optimal Loan Size
    // ============================================
    println!("\n\n5. OPTIMAL LOAN SIZE ANALYSIS");
    println!("{:-<60}", "");

    // Define profit function based on our pools
    let profit_func = |loan: f64| {
        let eth = pool_a.get_amount_out(loan, false);
        let usdc = pool_b.get_amount_out(eth, true);
        usdc - loan // Gross profit
    };

    let gas_cost = GasEstimate::for_flashloan_arb(50.0, eth_price, 2).cost_usd;

    let (optimal_loan, max_profit) = executor.optimize_loan_amount(
        profit_func,
        500_000.0, // Max loan to consider
        gas_cost,
    );

    println!("Optimization Results:");
    println!("  Optimal Loan Size: ${:.2}", optimal_loan);
    println!("  Maximum Net Profit: ${:.2}", max_profit);
    println!("  Gas Cost (fixed): ${:.2}", gas_cost);

    // Show profit curve
    println!("\nProfit Curve:");
    println!("{:>15} {:>15} {:>15}", "Loan Size", "Gross Profit", "Net Profit");
    println!("{:-<50}", "");

    for multiplier in [0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0] {
        let loan = optimal_loan * multiplier;
        let gross = profit_func(loan);
        let fee = loan * FlashloanProvider::Aave.fee_rate();
        let net = gross - fee - gas_cost;
        let indicator = if (loan - optimal_loan).abs() < 1.0 { " <-- optimal" } else { "" };
        println!(
            "{:>15.2} {:>15.2} {:>15.2}{}",
            loan, gross, net, indicator
        );
    }

    // ============================================
    // 6. Risk Assessment
    // ============================================
    println!("\n\n6. RISK ASSESSMENT");
    println!("{:-<60}", "");

    println!("Execution Risks:");
    println!("  [ ] Price movement during execution");
    println!("  [ ] Higher than expected slippage");
    println!("  [ ] Gas price spike");
    println!("  [ ] MEV/frontrunning");
    println!("  [ ] Smart contract bugs");
    println!("  [ ] Network congestion");

    println!("\nMitigation Strategies:");
    println!("  - Set tight slippage tolerance (0.5-1%)");
    println!("  - Use Flashbots for private transactions");
    println!("  - Monitor mempool for competing transactions");
    println!("  - Set maximum gas price limits");
    println!("  - Test on forked mainnet before production");

    println!("\n╔════════════════════════════════════════════╗");
    println!("║       Simulation Complete!                 ║");
    println!("╚════════════════════════════════════════════╝");

    Ok(())
}
