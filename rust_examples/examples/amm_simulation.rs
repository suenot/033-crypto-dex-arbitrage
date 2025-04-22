//! Example: AMM Simulation
//!
//! This example demonstrates how to simulate trading on different AMM types.
//!
//! Run with: cargo run --example amm_simulation

use anyhow::Result;
use dex_arbitrage::{ConcentratedLiquidityAMM, ConstantProductAMM, CurveStableSwap, LiquidityPosition, Pool};

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════╗");
    println!("║         AMM Simulation Example             ║");
    println!("╚════════════════════════════════════════════╝\n");

    // ============================================
    // 1. Constant Product AMM (Uniswap V2 style)
    // ============================================
    println!("1. CONSTANT PRODUCT AMM (Uniswap V2)");
    println!("{:-<60}", "");

    let mut uniswap = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003)
        .with_name("Uniswap V2 ETH/USDC");

    println!("Pool: {}", uniswap.name());
    println!("  ETH Reserve:  {:>15.4} ETH", uniswap.reserve_x());
    println!("  USDC Reserve: {:>15.2} USDC", uniswap.reserve_y());
    println!("  Spot Price:   {:>15.2} USDC/ETH", uniswap.get_price());
    println!("  K (invariant): {:>14.0}", uniswap.k());
    println!("  Fee Rate:     {:>15.2}%", uniswap.fee_rate() * 100.0);

    // Simulate trades of different sizes
    println!("\n  Trade Simulations:");
    println!("  {:>10} {:>15} {:>12} {:>12}", "Size", "USDC Out", "Eff. Price", "Slippage");

    for size in [1.0, 10.0, 50.0, 100.0] {
        let amount_out = uniswap.get_amount_out(size, true);
        let eff_price = amount_out / size;
        let slippage = uniswap.get_slippage(size, true);
        println!(
            "  {:>10.1} ETH {:>12.2} USDC {:>10.2} {:>11.4}%",
            size, amount_out, eff_price, slippage * 100.0
        );
    }

    // Execute a trade
    println!("\n  Executing swap: 50 ETH -> USDC");
    let usdc_received = uniswap.swap(50.0, true)?;
    println!("  Received: {:.2} USDC", usdc_received);
    println!("  New Price: {:.2} USDC/ETH", uniswap.get_price());
    println!("  Price increased by {:.2}%", (uniswap.get_price() - 2000.0) / 2000.0 * 100.0);

    // ============================================
    // 2. Concentrated Liquidity AMM (Uniswap V3)
    // ============================================
    println!("\n\n2. CONCENTRATED LIQUIDITY AMM (Uniswap V3)");
    println!("{:-<60}", "");

    let mut uni_v3 = ConcentratedLiquidityAMM::new(2000.0, 0.003)
        .with_name("Uniswap V3 ETH/USDC");

    // Add liquidity positions
    uni_v3.add_position(LiquidityPosition::new(1800.0, 2200.0, 500_000.0));
    uni_v3.add_position(LiquidityPosition::new(1900.0, 2100.0, 300_000.0));
    uni_v3.add_position(LiquidityPosition::new(1950.0, 2050.0, 200_000.0));

    println!("Pool: {}", uni_v3.name());
    println!("  Current Price: {:.2} USDC/ETH", uni_v3.get_price());
    println!("  Positions: {}", uni_v3.positions().len());

    for (i, pos) in uni_v3.positions().iter().enumerate() {
        let in_range = pos.is_in_range(uni_v3.get_price());
        let status = if in_range { "IN RANGE" } else { "OUT OF RANGE" };
        println!(
            "    Position {}: [{:.0} - {:.0}] Liquidity: {:>10.0} [{}]",
            i + 1, pos.price_lower, pos.price_upper, pos.liquidity, status
        );
    }

    println!("\n  Liquidity at current price: {:.0}", uni_v3.liquidity_at_price(2000.0));

    // Compare slippage with V2
    println!("\n  Slippage comparison with V2:");
    println!("  {:>10} {:>15} {:>15}", "Trade Size", "V2 Slippage", "V3 Slippage");

    let uniswap_v2 = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);
    for size in [1.0, 10.0, 50.0] {
        let v2_slip = uniswap_v2.get_slippage(size * 2000.0, false);
        let v3_slip = uni_v3.get_slippage(size * 2000.0, false);
        println!(
            "  {:>10.1} ETH {:>14.4}% {:>14.4}%",
            size, v2_slip * 100.0, v3_slip * 100.0
        );
    }

    // ============================================
    // 3. Curve StableSwap
    // ============================================
    println!("\n\n3. CURVE STABLESWAP");
    println!("{:-<60}", "");

    let mut curve = CurveStableSwap::new(
        vec![1_000_000.0, 1_000_000.0, 1_000_000.0], // USDC, USDT, DAI
        100.0, // Amplification factor
        0.0004 // 0.04% fee
    ).with_name("Curve 3pool");

    println!("Pool: {}", curve.name());
    println!("  Tokens: 3 (USDC, USDT, DAI)");
    println!("  USDC Reserve:  {:>12.2}", curve.reserve(0));
    println!("  USDT Reserve:  {:>12.2}", curve.reserve(1));
    println!("  DAI Reserve:   {:>12.2}", curve.reserve(2));
    println!("  Amplification: {:>12.0}", curve.amplification());
    println!("  Virtual Price: {:>12.6}", curve.virtual_price());
    println!("  D (invariant): {:>12.2}", curve.calculate_d());

    // Compare slippage with constant product
    println!("\n  Stablecoin swap comparison:");
    println!("  {:>12} {:>18} {:>18}", "Trade Size", "Curve Slippage", "Const.Prod Slip");

    let cp_stable = ConstantProductAMM::new(1_000_000.0, 1_000_000.0, 0.003);

    for size in [100.0, 1000.0, 10000.0, 100000.0] {
        let curve_slip = curve.get_slippage(size, true);
        let cp_slip = cp_stable.get_slippage(size, true);
        println!(
            "  {:>12.0} {:>17.6}% {:>17.4}%",
            size, curve_slip * 100.0, cp_slip * 100.0
        );
    }

    println!("\n  Notice: Curve has MUCH lower slippage for stablecoins!");

    // Swap demonstration
    println!("\n  Executing swap: 10,000 USDC -> USDT");
    let usdt_out = curve.swap(10_000.0, true)?;
    println!("  Received: {:.2} USDT", usdt_out);
    println!("  Slippage: {:.4}%", (10_000.0 - usdt_out) / 10_000.0 * 100.0);

    // ============================================
    // 4. Price Impact Analysis
    // ============================================
    println!("\n\n4. PRICE IMPACT ANALYSIS");
    println!("{:-<60}", "");

    let pool = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);

    println!("Trade size vs Price Impact on Constant Product AMM:");
    println!("{:>15} {:>15} {:>15} {:>15}", "Trade (ETH)", "Trade ($)", "Price Impact", "New Price");

    for pct in [0.1, 0.5, 1.0, 2.0, 5.0, 10.0] {
        let trade_size = pool.reserve_x() * pct / 100.0;
        let trade_usd = trade_size * pool.get_price();
        let impact = pool.get_price_impact(trade_size, true);
        let new_price = pool.get_price() * (1.0 + impact);

        println!(
            "{:>15.2} {:>15.2} {:>14.4}% {:>15.2}",
            trade_size, trade_usd, impact * 100.0, new_price
        );
    }

    println!("\n  Rule of thumb: Price impact ≈ Trade size / Pool liquidity");

    println!("\n╔════════════════════════════════════════════╗");
    println!("║         Simulation Complete!               ║");
    println!("╚════════════════════════════════════════════╝");

    Ok(())
}
