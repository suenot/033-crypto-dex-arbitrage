//! Example: Arbitrage Detection
//!
//! This example demonstrates how to detect arbitrage opportunities
//! between multiple DEX pools.
//!
//! Run with: cargo run --example arbitrage_detection

use anyhow::Result;
use dex_arbitrage::{
    ArbitrageDetector, BybitClient, ConstantProductAMM, CurveStableSwap,
    GasPricePredictor, Interval, Pool,
};

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════╗");
    println!("║       Arbitrage Detection Example          ║");
    println!("╚════════════════════════════════════════════╝\n");

    // Fetch current ETH price from Bybit
    println!("Fetching current ETH price from Bybit...");
    let client = BybitClient::new();
    let ticker = client.get_ticker("ETHUSDT")?;
    let eth_price: f64 = ticker.last_price.parse()?;
    println!("Current ETH/USDT: ${:.2}\n", eth_price);

    // ============================================
    // 1. Simple Two-Pool Arbitrage
    // ============================================
    println!("1. TWO-POOL ARBITRAGE DETECTION");
    println!("{:-<60}", "");

    // Create pools with price discrepancy
    // Pool 1: ETH is cheaper
    let pool1 = ConstantProductAMM::new(1000.0, eth_price * 1000.0, 0.003)
        .with_name("Uniswap V2");

    // Pool 2: ETH is more expensive (2% premium)
    let premium = 1.02;
    let pool2 = ConstantProductAMM::new(800.0, eth_price * premium * 800.0, 0.003)
        .with_name("SushiSwap");

    println!("Pool configurations:");
    println!("  {} - Price: ${:.2}", pool1.name(), pool1.get_price());
    println!("  {} - Price: ${:.2}", pool2.name(), pool2.get_price());
    println!(
        "  Price difference: {:.2}%",
        (pool2.get_price() - pool1.get_price()) / pool1.get_price() * 100.0
    );

    let pools: Vec<Box<dyn Pool>> = vec![
        Box::new(pool1.clone()),
        Box::new(pool2.clone()),
    ];

    let detector = ArbitrageDetector::new(pools, 50.0) // 50 Gwei gas
        .with_eth_price(eth_price);

    // Find opportunities
    let opportunities = detector.find_opportunities(10_000.0, 0.0);

    println!("\nDetected {} opportunities:", opportunities.len());

    for (i, opp) in opportunities.iter().enumerate() {
        println!("\n  Opportunity #{}:", i + 1);
        println!("    Buy from:  Pool {} at ${:.2}", opp.buy_pool_idx, opp.buy_price);
        println!("    Sell to:   Pool {} at ${:.2}", opp.sell_pool_idx, opp.sell_price);
        println!("    Price diff: {:.2}%", opp.price_diff_pct * 100.0);
        println!("    Trade size: ${:.2}", opp.trade_size);
        println!("    Gross profit: ${:.2}", opp.gross_profit);
        println!("    Gas cost: ${:.2}", opp.gas_cost);
        println!("    Slippage cost: ${:.2}", opp.slippage_cost);
        println!("    Net profit: ${:.2}", opp.net_profit);
        println!("    ROI: {:.4}%", opp.roi * 100.0);

        // MEV risk
        let mev_risk = detector.estimate_mev_risk(opp);
        println!("    MEV Risk: {:.1}%", mev_risk * 100.0);
    }

    // Optimize trade size
    if !opportunities.is_empty() {
        println!("\n  Optimizing trade size...");
        let (optimal_size, max_profit) = detector.optimize_trade_size(0, 1, 100_000.0);
        println!("    Optimal size: ${:.2}", optimal_size);
        println!("    Maximum profit: ${:.2}", max_profit);
    }

    // ============================================
    // 2. Multiple Pool Arbitrage
    // ============================================
    println!("\n\n2. MULTI-POOL SCENARIO");
    println!("{:-<60}", "");

    // Create multiple pools with varying prices
    let pools: Vec<Box<dyn Pool>> = vec![
        Box::new(ConstantProductAMM::new(1000.0, eth_price * 1000.0, 0.003)
            .with_name("Uniswap")),
        Box::new(ConstantProductAMM::new(800.0, eth_price * 1.015 * 800.0, 0.003)
            .with_name("SushiSwap")),
        Box::new(ConstantProductAMM::new(1200.0, eth_price * 0.99 * 1200.0, 0.003)
            .with_name("PancakeSwap")),
        Box::new(ConstantProductAMM::new(600.0, eth_price * 1.025 * 600.0, 0.003)
            .with_name("Shibaswap")),
    ];

    println!("Pool prices:");
    for (i, pool) in pools.iter().enumerate() {
        println!("  {}: {} - ${:.2}", i, pool.name(), pool.get_price());
    }

    let detector = ArbitrageDetector::new(pools, 50.0).with_eth_price(eth_price);
    let opportunities = detector.find_opportunities(10_000.0, 0.001); // Min 0.1% profit

    println!("\nBest opportunities (min 0.1% profit):");
    for (i, opp) in opportunities.iter().take(5).enumerate() {
        let buy_pool = detector.pool(opp.buy_pool_idx).unwrap();
        let sell_pool = detector.pool(opp.sell_pool_idx).unwrap();
        println!(
            "  {}. {} -> {}: Net profit ${:.2} (ROI: {:.2}%)",
            i + 1,
            buy_pool.name(),
            sell_pool.name(),
            opp.net_profit,
            opp.roi * 100.0
        );
    }

    // ============================================
    // 3. Gas Price Timing
    // ============================================
    println!("\n\n3. GAS PRICE OPTIMIZATION");
    println!("{:-<60}", "");

    let predictor = GasPricePredictor::new(50.0)
        .with_volatility(0.15)
        .with_long_term_mean(45.0);

    println!("Current gas price: {} Gwei", predictor.current_price());
    println!("\nGas price forecast for next 10 blocks:");

    let predictions = predictor.predict_next_blocks(10);
    for (i, pred) in predictions.iter().enumerate() {
        let bar_len = (*pred / 5.0) as usize;
        let bar = "█".repeat(bar_len.min(20));
        println!("  Block +{:>2}: {:>6.1} Gwei {}", i + 1, pred, bar);
    }

    let optimal = predictor.optimal_execution_block(&predictions, 10);
    println!(
        "\n  Optimal execution: Block +{} (expected gas: {:.1} Gwei)",
        optimal + 1,
        predictions[optimal]
    );

    // Calculate savings
    let current_gas = predictor.current_price();
    let optimal_gas = predictions[optimal];
    let gas_units = 300_000.0; // Two swaps
    let savings = (current_gas - optimal_gas) * gas_units / 1e9 * eth_price;
    if savings > 0.0 {
        println!("  Potential gas savings: ${:.2}", savings);
    }

    // ============================================
    // 4. Stablecoin Arbitrage
    // ============================================
    println!("\n\n4. STABLECOIN ARBITRAGE");
    println!("{:-<60}", "");

    // Simulate stablecoin pools with slight mispricing
    let curve_pool = CurveStableSwap::new(
        vec![10_000_000.0, 9_950_000.0, 10_050_000.0], // Slightly imbalanced
        100.0,
        0.0004,
    ).with_name("Curve 3pool");

    let usdc_pool = ConstantProductAMM::new(5_000_000.0, 5_010_000.0, 0.003)
        .with_name("Uniswap USDC/DAI");

    println!("Curve 3pool (USDC/USDT/DAI):");
    println!("  USDC: {:.0}", curve_pool.reserve(0));
    println!("  USDT: {:.0}", curve_pool.reserve(1));
    println!("  DAI:  {:.0}", curve_pool.reserve(2));

    println!("\nUniswap USDC/DAI:");
    println!("  Price: {:.6} DAI/USDC", usdc_pool.get_price());

    // Check for stablecoin arb
    let curve_rate = curve_pool.get_amount_out_ij(10_000.0, 0, 2); // USDC -> DAI
    let uni_rate = usdc_pool.get_amount_out(10_000.0, true);

    println!("\nSwap 10,000 USDC:");
    println!("  Curve: {:.2} DAI", curve_rate);
    println!("  Uniswap: {:.2} DAI", uni_rate);
    println!("  Difference: {:.2} DAI ({:.4}%)",
        (curve_rate - uni_rate).abs(),
        (curve_rate - uni_rate).abs() / 10_000.0 * 100.0
    );

    if curve_rate > uni_rate {
        println!("\n  Strategy: Swap on Curve, sell on Uniswap");
    } else {
        println!("\n  Strategy: Swap on Uniswap, sell on Curve");
    }

    println!("\n╔════════════════════════════════════════════╗");
    println!("║       Detection Complete!                  ║");
    println!("╚════════════════════════════════════════════╝");

    Ok(())
}
