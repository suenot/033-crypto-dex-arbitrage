//! Example: Fetching market data from Bybit
//!
//! This example demonstrates how to fetch OHLCV data from the Bybit API.
//!
//! Run with: cargo run --example fetch_bybit_data

use anyhow::Result;
use dex_arbitrage::{BybitClient, Interval};

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    println!("╔════════════════════════════════════════════╗");
    println!("║     Bybit Market Data Fetcher Example      ║");
    println!("╚════════════════════════════════════════════╝\n");

    // Create Bybit client
    let client = BybitClient::new();

    // Fetch ETH/USDT klines
    println!("Fetching ETH/USDT hourly candles...\n");
    let klines = client.get_klines("ETHUSDT", Interval::Hour1, Some(24), None, None)?;

    println!("Last 24 hours of ETH/USDT:");
    println!("{:-<80}", "");
    println!(
        "{:>20} {:>12} {:>12} {:>12} {:>12} {:>10}",
        "Time", "Open", "High", "Low", "Close", "Volume"
    );
    println!("{:-<80}", "");

    for kline in &klines {
        println!(
            "{:>20} {:>12.2} {:>12.2} {:>12.2} {:>12.2} {:>10.1}",
            kline.datetime().format("%Y-%m-%d %H:%M"),
            kline.open,
            kline.high,
            kline.low,
            kline.close,
            kline.volume
        );
    }

    // Calculate statistics
    println!("\n{:-<80}", "");
    println!("Statistics:");

    let avg_price: f64 = klines.iter().map(|k| k.close).sum::<f64>() / klines.len() as f64;
    let max_price = klines.iter().map(|k| k.high).fold(f64::MIN, f64::max);
    let min_price = klines.iter().map(|k| k.low).fold(f64::MAX, f64::min);
    let total_volume: f64 = klines.iter().map(|k| k.volume).sum();

    let returns: Vec<f64> = klines.iter().map(|k| k.return_pct()).collect();
    let avg_return = returns.iter().sum::<f64>() / returns.len() as f64;
    let volatility = {
        let variance: f64 = returns.iter().map(|r| (r - avg_return).powi(2)).sum::<f64>()
            / (returns.len() - 1) as f64;
        variance.sqrt()
    };

    println!("  Average Price:    ${:.2}", avg_price);
    println!("  24h High:         ${:.2}", max_price);
    println!("  24h Low:          ${:.2}", min_price);
    println!("  24h Range:        ${:.2} ({:.2}%)", max_price - min_price, (max_price - min_price) / min_price * 100.0);
    println!("  Total Volume:     {:.2} ETH", total_volume);
    println!("  Avg Hourly Return: {:.4}%", avg_return * 100.0);
    println!("  Hourly Volatility: {:.4}%", volatility * 100.0);
    println!("  Annualized Vol:    {:.2}%", volatility * (24.0 * 365.0_f64).sqrt() * 100.0);

    // Fetch ticker for current price
    println!("\n{:-<80}", "");
    println!("Current Ticker:");

    let ticker = client.get_ticker("ETHUSDT")?;
    println!("  Symbol:     {}", ticker.symbol);
    println!("  Last Price: ${}", ticker.last_price);
    println!("  24h Change: {}%", ticker.price_24h_pcnt);
    println!("  24h Volume: {} ETH", ticker.volume_24h);

    // Fetch multiple tokens
    println!("\n{:-<80}", "");
    println!("Top Crypto Prices:");

    let tokens = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "XRPUSDT"];

    for symbol in &tokens {
        if let Ok(t) = client.get_ticker(symbol) {
            let change: f64 = t.price_24h_pcnt.parse().unwrap_or(0.0);
            let arrow = if change >= 0.0 { "↑" } else { "↓" };
            println!(
                "  {:10} ${:>12} {:>8.2}% {}",
                symbol, t.last_price, change * 100.0, arrow
            );
        }
    }

    println!("\nDone!");
    Ok(())
}
