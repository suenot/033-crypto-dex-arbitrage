//! Example: Full DEX Arbitrage Pipeline
//!
//! This example demonstrates a complete arbitrage workflow:
//! 1. Fetch market data from Bybit
//! 2. Simulate DEX pools based on real prices
//! 3. Detect arbitrage opportunities
//! 4. Simulate flashloan execution
//! 5. Track performance metrics
//!
//! Run with: cargo run --example full_pipeline

use anyhow::Result;
use dex_arbitrage::{
    ArbitrageDetector, ArbitrageMetrics, BybitClient, ConstantProductAMM,
    FlashloanExecutor, FlashloanProvider, GasEstimate, GasPricePredictor,
    Interval, PerformanceReport, Pool,
};
use std::time::Instant;

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║           FULL DEX ARBITRAGE PIPELINE                      ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let pipeline_start = Instant::now();
    let mut metrics = ArbitrageMetrics::new();

    // ============================================
    // STEP 1: Fetch Market Data
    // ============================================
    println!("STEP 1: FETCHING MARKET DATA");
    println!("{:-<60}", "");

    let client = BybitClient::new();

    // Fetch ETH price
    let eth_ticker = client.get_ticker("ETHUSDT")?;
    let eth_price: f64 = eth_ticker.last_price.parse()?;
    println!("  ETH/USDT: ${:.2}", eth_price);

    // Fetch recent volatility
    let klines = client.get_klines("ETHUSDT", Interval::Hour1, Some(24), None, None)?;
    let returns: Vec<f64> = klines.iter().map(|k| k.return_pct()).collect();
    let volatility = {
        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let var: f64 = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
        var.sqrt()
    };
    println!("  24h Volatility: {:.2}%", volatility * 100.0);

    // Fetch other tokens for multi-pair analysis
    let tokens = ["BTCUSDT", "SOLUSDT", "BNBUSDT"];
    for symbol in &tokens {
        if let Ok(t) = client.get_ticker(symbol) {
            println!("  {}: ${}", symbol, t.last_price);
        }
    }

    // ============================================
    // STEP 2: Simulate DEX Pools
    // ============================================
    println!("\n\nSTEP 2: SIMULATING DEX POOLS");
    println!("{:-<60}", "");

    // Create pools with realistic parameters
    // Simulate different liquidity depths and slight price variations

    // Deep liquidity pool (Uniswap)
    let uniswap = ConstantProductAMM::new(
        5000.0,                     // 5000 ETH
        eth_price * 5000.0,         // Matching USDC
        0.003                       // 0.3% fee
    ).with_name("Uniswap V2");

    // Medium liquidity pool (SushiSwap) - slight premium
    let sushi_premium = 1.0 + (rand::random::<f64>() * 0.02); // 0-2% premium
    let sushiswap = ConstantProductAMM::new(
        2000.0,
        eth_price * sushi_premium * 2000.0,
        0.003
    ).with_name("SushiSwap");

    // Lower liquidity pool (Shibaswap) - might have bigger discrepancy
    let shiba_adjustment = 0.98 + (rand::random::<f64>() * 0.04); // -2% to +2%
    let shibaswap = ConstantProductAMM::new(
        500.0,
        eth_price * shiba_adjustment * 500.0,
        0.003
    ).with_name("ShibaSwap");

    println!("  Pool configurations:");
    println!("    {} - Liq: ${}M, Price: ${:.2}",
        uniswap.name(),
        (uniswap.liquidity_usd(eth_price) / 1_000_000.0) as i32,
        uniswap.get_price()
    );
    println!("    {} - Liq: ${}M, Price: ${:.2}",
        sushiswap.name(),
        (sushiswap.liquidity_usd(eth_price) / 1_000_000.0) as i32,
        sushiswap.get_price()
    );
    println!("    {} - Liq: ${}K, Price: ${:.2}",
        shibaswap.name(),
        (shibaswap.liquidity_usd(eth_price) / 1_000.0) as i32,
        shibaswap.get_price()
    );

    // ============================================
    // STEP 3: Gas Price Analysis
    // ============================================
    println!("\n\nSTEP 3: GAS PRICE ANALYSIS");
    println!("{:-<60}", "");

    let base_gas = 50.0 + rand::random::<f64>() * 50.0; // 50-100 Gwei
    let predictor = GasPricePredictor::new(base_gas)
        .with_volatility(0.2);

    println!("  Current gas: {:.1} Gwei", predictor.current_price());

    let predictions = predictor.predict_next_blocks(10);
    let optimal_block = predictor.optimal_execution_block(&predictions, 10);
    println!("  Optimal block: +{} ({:.1} Gwei)", optimal_block + 1, predictions[optimal_block]);

    let gas_estimate = GasEstimate::for_flashloan_arb(predictions[optimal_block], eth_price, 2);
    println!("  Estimated gas cost: ${:.2}", gas_estimate.cost_usd);

    // ============================================
    // STEP 4: Detect Arbitrage Opportunities
    // ============================================
    println!("\n\nSTEP 4: DETECTING ARBITRAGE OPPORTUNITIES");
    println!("{:-<60}", "");

    let pools: Vec<Box<dyn Pool>> = vec![
        Box::new(uniswap),
        Box::new(sushiswap),
        Box::new(shibaswap),
    ];

    let detector = ArbitrageDetector::new(pools, predictions[optimal_block])
        .with_eth_price(eth_price);

    // Try different trade sizes
    let trade_sizes = [10_000.0, 50_000.0, 100_000.0];
    let mut best_opportunity = None;
    let mut best_profit = f64::NEG_INFINITY;

    for size in trade_sizes {
        let opportunities = detector.find_opportunities(size, 0.0);
        metrics.total_opportunities += opportunities.len() as u64;

        for opp in &opportunities {
            if opp.net_profit > best_profit {
                best_profit = opp.net_profit;
                best_opportunity = Some((size, opp.clone()));
            }
        }
    }

    if let Some((size, opp)) = &best_opportunity {
        let buy_pool = detector.pool(opp.buy_pool_idx).unwrap();
        let sell_pool = detector.pool(opp.sell_pool_idx).unwrap();

        println!("  Best opportunity found:");
        println!("    Trade size: ${:.2}", size);
        println!("    Route: {} -> {}", buy_pool.name(), sell_pool.name());
        println!("    Price spread: {:.2}%", opp.price_diff_pct * 100.0);
        println!("    Gross profit: ${:.2}", opp.gross_profit);
        println!("    Gas cost: ${:.2}", opp.gas_cost);
        println!("    Net profit: ${:.2}", opp.net_profit);
        println!("    ROI: {:.4}%", opp.roi * 100.0);

        let mev_risk = detector.estimate_mev_risk(&opp);
        println!("    MEV risk: {:.1}%", mev_risk * 100.0);
    } else {
        println!("  No profitable opportunities found");
    }

    // ============================================
    // STEP 5: Simulate Flashloan Execution
    // ============================================
    println!("\n\nSTEP 5: FLASHLOAN EXECUTION SIMULATION");
    println!("{:-<60}", "");

    let executor = FlashloanExecutor::new(FlashloanProvider::Aave)
        .with_min_profit(5.0); // Min $5 profit

    if let Some((size, opp)) = best_opportunity {
        let exec_start = Instant::now();

        // Simulate execution
        let result = executor.simulate_execution(size, opp.gross_profit, opp.gas_cost);

        let exec_time = exec_start.elapsed().as_millis() as u64;

        println!("  Execution simulation:");
        println!("    Provider: {:?}", result.provider);
        println!("    Loan amount: ${:.2}", result.loan_amount);
        println!("    Fee paid: ${:.2}", result.fee_paid);
        println!("    Gross profit: ${:.2}", result.gross_profit);
        println!("    Gas cost: ${:.2}", result.gas_cost);
        println!("    Net profit: ${:.2}", result.net_profit);
        println!("    Status: {}", if result.success { "SUCCESS" } else { "FAILED" });

        if let Some(err) = &result.error {
            println!("    Error: {}", err);
        }

        // Record metrics
        if result.success {
            let hour = chrono::Utc::now().hour() as usize;
            metrics.record_success(
                result.gross_profit,
                result.net_profit,
                result.gas_cost,
                result.fee_paid,
                opp.slippage_cost,
                exec_time,
                hour,
            );
        } else {
            metrics.record_failure(opp.gas_cost);
        }
    } else {
        println!("  No opportunity to execute");
    }

    // ============================================
    // STEP 6: Performance Report
    // ============================================
    println!("\n\nSTEP 6: PERFORMANCE REPORT");
    println!("{:-<60}", "");

    let report = PerformanceReport::generate(
        metrics,
        &chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string(),
        &chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string(),
    );

    report.print_summary();

    // ============================================
    // Pipeline Summary
    // ============================================
    let pipeline_duration = pipeline_start.elapsed();

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                    PIPELINE COMPLETE                       ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║  Total time: {:>10.2?}                                ║", pipeline_duration);
    println!("║  Opportunities scanned: {:>6}                            ║", report.metrics.total_opportunities);
    println!("║  Executions simulated: {:>6}                             ║", report.metrics.executed);
    println!("║  Success rate: {:>10.1}%                              ║", report.kpis.win_rate);
    println!("╚════════════════════════════════════════════════════════════╝");

    // ============================================
    // Production Recommendations
    // ============================================
    println!("\nPRODUCTION RECOMMENDATIONS:");
    println!("{:-<60}", "");
    println!("  1. Connect to real DEX contracts via web3/ethers");
    println!("  2. Use WebSocket for real-time price feeds");
    println!("  3. Implement Flashbots bundle submission");
    println!("  4. Add mempool monitoring for MEV protection");
    println!("  5. Deploy monitoring and alerting");
    println!("  6. Start with small trade sizes to test");
    println!("  7. Consider running on dedicated infrastructure");

    Ok(())
}

// Helper to get current hour
mod chrono {
    pub struct Utc;

    impl Utc {
        pub fn now() -> DateTime {
            DateTime {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            }
        }
    }

    pub struct DateTime {
        timestamp: i64,
    }

    impl DateTime {
        pub fn hour(&self) -> u32 {
            ((self.timestamp / 3600) % 24) as u32
        }

        pub fn format(&self, _fmt: &str) -> String {
            // Simplified format
            let secs = self.timestamp;
            let hours = (secs / 3600) % 24;
            let mins = (secs / 60) % 60;
            format!("2024-01-01 {:02}:{:02}", hours, mins)
        }
    }
}
