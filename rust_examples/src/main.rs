//! DEX Arbitrage CLI Application
//!
//! A command-line tool for simulating DEX arbitrage opportunities
//! using real market data from Bybit.

use anyhow::Result;
use clap::{Parser, Subcommand};
use dex_arbitrage::{
    ArbitrageDetector, BybitClient, ConstantProductAMM, CurveStableSwap, FlashloanExecutor,
    FlashloanProvider, GasPricePredictor, Interval, Pool,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "dex-arbitrage")]
#[command(about = "DEX Arbitrage Simulation Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbosity level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch market data from Bybit
    Fetch {
        /// Trading symbol (e.g., ETHUSDT)
        #[arg(short, long, default_value = "ETHUSDT")]
        symbol: String,

        /// Number of candles to fetch
        #[arg(short, long, default_value = "100")]
        limit: u32,

        /// Interval (1m, 5m, 15m, 1h, 4h, 1d)
        #[arg(short, long, default_value = "1h")]
        interval: String,
    },

    /// Simulate AMM trading
    Simulate {
        /// Initial ETH reserve
        #[arg(long, default_value = "1000.0")]
        eth_reserve: f64,

        /// Initial USDC reserve
        #[arg(long, default_value = "2000000.0")]
        usdc_reserve: f64,

        /// Trade amount in ETH
        #[arg(long, default_value = "10.0")]
        trade_amount: f64,

        /// Fee percentage (e.g., 0.003 for 0.3%)
        #[arg(long, default_value = "0.003")]
        fee: f64,
    },

    /// Detect arbitrage opportunities
    Arbitrage {
        /// Trade size in quote currency
        #[arg(long, default_value = "10000.0")]
        trade_size: f64,

        /// Minimum profit threshold (percentage)
        #[arg(long, default_value = "0.1")]
        min_profit: f64,

        /// Gas price in Gwei
        #[arg(long, default_value = "50.0")]
        gas_price: f64,
    },

    /// Simulate flashloan arbitrage
    Flashloan {
        /// Loan amount
        #[arg(long, default_value = "100000.0")]
        loan_amount: f64,

        /// Flashloan provider (aave, dydx)
        #[arg(long, default_value = "aave")]
        provider: String,
    },
}

fn parse_interval(s: &str) -> Result<Interval> {
    match s {
        "1m" => Ok(Interval::Min1),
        "3m" => Ok(Interval::Min3),
        "5m" => Ok(Interval::Min5),
        "15m" => Ok(Interval::Min15),
        "30m" => Ok(Interval::Min30),
        "1h" => Ok(Interval::Hour1),
        "2h" => Ok(Interval::Hour2),
        "4h" => Ok(Interval::Hour4),
        "6h" => Ok(Interval::Hour6),
        "12h" => Ok(Interval::Hour12),
        "1d" => Ok(Interval::Day1),
        "1w" => Ok(Interval::Week1),
        "1M" => Ok(Interval::Month1),
        _ => anyhow::bail!("Invalid interval: {}", s),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let level = match cli.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    match cli.command {
        Commands::Fetch {
            symbol,
            limit,
            interval,
        } => {
            info!("Fetching {} candles for {} at {} interval", limit, symbol, interval);

            let client = BybitClient::new();
            let interval = parse_interval(&interval)?;
            let klines = client.get_klines(&symbol, interval, Some(limit), None, None)?;

            println!("\n{} Market Data ({} candles)", symbol, klines.len());
            println!("{:-<80}", "");
            println!(
                "{:>20} {:>12} {:>12} {:>12} {:>12}",
                "Timestamp", "Open", "High", "Low", "Close"
            );
            println!("{:-<80}", "");

            for kline in klines.iter().take(10) {
                println!(
                    "{:>20} {:>12.2} {:>12.2} {:>12.2} {:>12.2}",
                    kline.datetime().format("%Y-%m-%d %H:%M"),
                    kline.open,
                    kline.high,
                    kline.low,
                    kline.close
                );
            }

            if klines.len() > 10 {
                println!("... and {} more candles", klines.len() - 10);
            }

            // Calculate some stats
            let avg_price: f64 = klines.iter().map(|k| k.close).sum::<f64>() / klines.len() as f64;
            let total_volume: f64 = klines.iter().map(|k| k.volume).sum();
            let avg_return: f64 =
                klines.iter().map(|k| k.return_pct()).sum::<f64>() / klines.len() as f64;

            println!("\nStatistics:");
            println!("  Average Price: ${:.2}", avg_price);
            println!("  Total Volume: {:.2}", total_volume);
            println!("  Avg Return: {:.4}%", avg_return * 100.0);
        }

        Commands::Simulate {
            eth_reserve,
            usdc_reserve,
            trade_amount,
            fee,
        } => {
            info!("Simulating AMM trade");

            let mut amm = ConstantProductAMM::new(eth_reserve, usdc_reserve, fee);

            println!("\nConstant Product AMM Simulation");
            println!("{:-<60}", "");
            println!("Initial State:");
            println!("  ETH Reserve: {:.4}", amm.reserve_x());
            println!("  USDC Reserve: {:.2}", amm.reserve_y());
            println!("  Spot Price: ${:.2} per ETH", amm.get_price());
            println!("  K (invariant): {:.0}", amm.k());

            // Simulate buying ETH
            let amount_out = amm.get_amount_out(trade_amount, true);
            let slippage = amm.get_slippage(trade_amount, true);
            let price_impact = amm.get_price_impact(trade_amount, true);

            println!("\nTrade: Buy ETH with {} USDC", trade_amount);
            println!("  Amount Out: {:.6} ETH", amount_out);
            println!("  Effective Price: ${:.2}", trade_amount / amount_out);
            println!("  Slippage: {:.4}%", slippage * 100.0);
            println!("  Price Impact: {:.4}%", price_impact * 100.0);

            // Execute the trade
            amm.swap(trade_amount, true)?;

            println!("\nAfter Trade:");
            println!("  ETH Reserve: {:.4}", amm.reserve_x());
            println!("  USDC Reserve: {:.2}", amm.reserve_y());
            println!("  New Spot Price: ${:.2} per ETH", amm.get_price());
        }

        Commands::Arbitrage {
            trade_size,
            min_profit,
            gas_price,
        } => {
            info!("Detecting arbitrage opportunities");

            // Create simulated DEX pools with different prices
            // Simulating price discrepancies between exchanges
            let uniswap = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);
            let sushiswap = ConstantProductAMM::new(800.0, 1_620_000.0, 0.003); // Slightly different price
            let curve = CurveStableSwap::new(vec![1_000_000.0, 1_000_000.0, 1_000_000.0], 100.0, 0.0004);

            let pools: Vec<Box<dyn Pool>> = vec![
                Box::new(uniswap),
                Box::new(sushiswap),
            ];

            let detector = ArbitrageDetector::new(pools, gas_price);
            let opportunities = detector.find_opportunities(trade_size, min_profit / 100.0);

            println!("\nArbitrage Opportunity Detection");
            println!("{:-<80}", "");
            println!("Parameters:");
            println!("  Trade Size: ${:.2}", trade_size);
            println!("  Min Profit: {:.2}%", min_profit);
            println!("  Gas Price: {} Gwei", gas_price);

            println!("\nPool Prices:");
            println!("  Uniswap:   ${:.2} per ETH", 2_000_000.0 / 1000.0);
            println!("  SushiSwap: ${:.2} per ETH", 1_620_000.0 / 800.0);
            println!("  Curve:     Stableswap pool");

            if opportunities.is_empty() {
                println!("\nNo profitable opportunities found with current parameters.");
            } else {
                println!("\nFound {} opportunities:", opportunities.len());
                for (i, opp) in opportunities.iter().enumerate() {
                    println!("\n  Opportunity #{}:", i + 1);
                    println!("    Buy on: Pool {}", opp.buy_pool_idx);
                    println!("    Sell on: Pool {}", opp.sell_pool_idx);
                    println!("    Gross Profit: ${:.2}", opp.gross_profit);
                    println!("    Gas Cost: ${:.2}", opp.gas_cost);
                    println!("    Slippage Cost: ${:.2}", opp.slippage_cost);
                    println!("    Net Profit: ${:.2}", opp.net_profit);
                    println!("    ROI: {:.2}%", opp.roi * 100.0);
                }
            }

            // Show gas price prediction
            println!("\nGas Price Forecast (next 10 blocks):");
            let predictor = GasPricePredictor::new(gas_price);
            let predictions = predictor.predict_next_blocks(10);
            for (i, pred) in predictions.iter().enumerate() {
                let bar = "█".repeat((pred / 10.0) as usize);
                println!("  Block +{}: {:>6.1} Gwei {}", i + 1, pred, bar);
            }

            let optimal_block = predictor.optimal_execution_block(&predictions, 10);
            println!("\n  Optimal execution: Block +{}", optimal_block + 1);
        }

        Commands::Flashloan {
            loan_amount,
            provider,
        } => {
            info!("Simulating flashloan arbitrage");

            let provider = match provider.as_str() {
                "aave" => FlashloanProvider::Aave,
                "dydx" => FlashloanProvider::DyDx,
                _ => FlashloanProvider::Aave,
            };

            let executor = FlashloanExecutor::new(provider);

            println!("\nFlashloan Arbitrage Simulation");
            println!("{:-<60}", "");
            println!("Provider: {:?}", executor.provider());
            println!("Loan Amount: ${:.2}", loan_amount);
            println!("Fee: {:.2}%", executor.fee_rate() * 100.0);
            println!("Fee Amount: ${:.2}", loan_amount * executor.fee_rate());

            // Simulate an arbitrage opportunity
            let buy_price = 2000.0;
            let sell_price = 2025.0;
            let price_diff_pct = (sell_price - buy_price) / buy_price;

            let eth_bought = loan_amount / buy_price;
            let usdc_received = eth_bought * sell_price;
            let gross_profit = usdc_received - loan_amount;
            let flashloan_fee = loan_amount * executor.fee_rate();
            let gas_cost = 100.0; // Estimated gas cost
            let net_profit = gross_profit - flashloan_fee - gas_cost;

            println!("\nTrade Execution:");
            println!("  Buy {} ETH at ${:.2} on DEX A", eth_bought, buy_price);
            println!("  Sell {} ETH at ${:.2} on DEX B", eth_bought, sell_price);
            println!("  Price Difference: {:.2}%", price_diff_pct * 100.0);

            println!("\nProfit Breakdown:");
            println!("  Gross Profit: ${:.2}", gross_profit);
            println!("  Flashloan Fee: -${:.2}", flashloan_fee);
            println!("  Gas Cost: -${:.2}", gas_cost);
            println!("  Net Profit: ${:.2}", net_profit);
            println!("  ROI: {:.4}%", (net_profit / loan_amount) * 100.0);

            if net_profit > 0.0 {
                println!("\n  Status: PROFITABLE");
            } else {
                println!("\n  Status: NOT PROFITABLE");
            }

            // Simulate the flashloan execution
            let result = executor.simulate_execution(loan_amount, gross_profit, gas_cost);
            println!("\nExecution Result: {:?}", result);
        }
    }

    Ok(())
}
