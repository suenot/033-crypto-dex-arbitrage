# DEX Arbitrage Rust Library

A modular Rust implementation of DEX arbitrage simulation with AMM pricing models and Bybit market data integration.

## Features

- **AMM Implementations**: Constant Product (Uniswap V2), Concentrated Liquidity (V3), Curve StableSwap
- **Arbitrage Detection**: Cross-DEX opportunity finder with profit optimization
- **Flashloan Simulation**: Multi-provider support (Aave, dYdX, Uniswap V3, Balancer)
- **Gas Price Modeling**: Prediction and timing optimization
- **Real Market Data**: Bybit API integration for OHLCV and ticker data
- **Performance Metrics**: PnL tracking, win rate, Sharpe ratio, drawdown

## Project Structure

```
rust_examples/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # Library exports
│   ├── main.rs             # CLI application
│   ├── api/
│   │   ├── mod.rs
│   │   └── bybit.rs        # Bybit API client
│   ├── amm/
│   │   ├── mod.rs
│   │   ├── pool.rs         # Pool trait
│   │   ├── constant_product.rs    # Uniswap V2
│   │   ├── concentrated_liquidity.rs  # Uniswap V3
│   │   └── curve_stableswap.rs    # Curve
│   ├── arbitrage/
│   │   ├── mod.rs
│   │   ├── detector.rs     # Opportunity detection
│   │   └── triangular.rs   # Triangle arbitrage
│   ├── gas/
│   │   └── mod.rs          # Gas price prediction
│   ├── flashloan/
│   │   └── mod.rs          # Flashloan simulation
│   ├── data/
│   │   └── mod.rs          # Data structures
│   └── metrics/
│       └── mod.rs          # Performance metrics
└── examples/
    ├── fetch_bybit_data.rs     # Market data fetching
    ├── amm_simulation.rs       # AMM trading simulation
    ├── arbitrage_detection.rs  # Find opportunities
    ├── flashloan_simulation.rs # Flashloan arb
    └── full_pipeline.rs        # Complete workflow
```

## Installation

```bash
# Clone the repository
cd 36_crypto_dex_arbitrage/rust_examples

# Build the project
cargo build --release

# Run tests
cargo test
```

## Quick Start

### Fetch Market Data from Bybit

```rust
use dex_arbitrage::{BybitClient, Interval};

fn main() {
    let client = BybitClient::new();

    // Get ETH/USDT price
    let ticker = client.get_ticker("ETHUSDT").unwrap();
    println!("ETH Price: ${}", ticker.last_price);

    // Get OHLCV candles
    let klines = client.get_klines(
        "ETHUSDT",
        Interval::Hour1,
        Some(100),
        None,
        None
    ).unwrap();

    println!("Fetched {} candles", klines.len());
}
```

### Simulate AMM Trading

```rust
use dex_arbitrage::{ConstantProductAMM, Pool};

fn main() {
    // Create Uniswap V2 style pool
    let mut pool = ConstantProductAMM::new(
        1000.0,       // 1000 ETH
        2_000_000.0,  // 2M USDC
        0.003         // 0.3% fee
    );

    println!("Spot Price: ${:.2}", pool.get_price());

    // Simulate a swap
    let eth_out = pool.get_amount_out(10_000.0, false); // USDC -> ETH
    println!("10k USDC -> {:.4} ETH", eth_out);

    // Check slippage
    let slippage = pool.get_slippage(10_000.0, false);
    println!("Slippage: {:.2}%", slippage * 100.0);
}
```

### Detect Arbitrage Opportunities

```rust
use dex_arbitrage::{ArbitrageDetector, ConstantProductAMM, Pool};

fn main() {
    // Create pools with different prices
    let pool1 = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);
    let pool2 = ConstantProductAMM::new(800.0, 1_640_000.0, 0.003);

    let pools: Vec<Box<dyn Pool>> = vec![
        Box::new(pool1),
        Box::new(pool2),
    ];

    let detector = ArbitrageDetector::new(pools, 50.0);
    let opportunities = detector.find_opportunities(10_000.0, 0.001);

    for opp in opportunities {
        println!("Arbitrage: Buy from Pool {} -> Sell to Pool {}",
            opp.buy_pool_idx, opp.sell_pool_idx);
        println!("Net Profit: ${:.2}", opp.net_profit);
    }
}
```

### Simulate Flashloan Arbitrage

```rust
use dex_arbitrage::{FlashloanExecutor, FlashloanProvider};

fn main() {
    let executor = FlashloanExecutor::new(FlashloanProvider::Aave)
        .with_min_profit(10.0);

    let result = executor.simulate_execution(
        100_000.0,  // Loan amount
        500.0,      // Gross profit
        50.0        // Gas cost
    );

    println!("Success: {}", result.success);
    println!("Net Profit: ${:.2}", result.net_profit);
}
```

## Running Examples

```bash
# Fetch market data from Bybit
cargo run --example fetch_bybit_data

# Simulate AMM trading
cargo run --example amm_simulation

# Detect arbitrage opportunities
cargo run --example arbitrage_detection

# Simulate flashloan arbitrage
cargo run --example flashloan_simulation

# Run complete pipeline
cargo run --example full_pipeline
```

## CLI Usage

```bash
# Build CLI
cargo build --release

# Fetch market data
./target/release/dex-arbitrage fetch --symbol ETHUSDT --limit 24 --interval 1h

# Simulate AMM
./target/release/dex-arbitrage simulate --eth-reserve 1000 --usdc-reserve 2000000 --trade-amount 10

# Detect arbitrage
./target/release/dex-arbitrage arbitrage --trade-size 10000 --min-profit 0.1 --gas-price 50

# Simulate flashloan
./target/release/dex-arbitrage flashloan --loan-amount 100000 --provider aave
```

## AMM Types Supported

### Constant Product (Uniswap V2)
- Formula: `x * y = k`
- Simple and widely used
- Higher slippage for large trades

### Concentrated Liquidity (Uniswap V3)
- LPs can concentrate liquidity in price ranges
- More capital efficient
- Lower slippage within active ranges

### StableSwap (Curve)
- Optimized for pegged assets (stablecoins)
- Much lower slippage near parity
- Uses amplification coefficient (A)

## Gas Price Optimization

The library includes a gas price predictor for optimal execution timing:

```rust
use dex_arbitrage::GasPricePredictor;

let predictor = GasPricePredictor::new(50.0);
let predictions = predictor.predict_next_blocks(10);
let optimal = predictor.optimal_execution_block(&predictions, 10);

println!("Best block to execute: +{}", optimal + 1);
```

## Performance Metrics

Track your arbitrage performance:

```rust
use dex_arbitrage::{ArbitrageMetrics, PerformanceReport};

let mut metrics = ArbitrageMetrics::new();

// Record trades
metrics.record_success(100.0, 80.0, 10.0, 5.0, 5.0, 100, 14);
metrics.record_failure(10.0);

// Generate report
let report = PerformanceReport::generate(metrics, "2024-01-01", "2024-01-02");
report.print_summary();
```

## Dependencies

- `reqwest` - HTTP client for API requests
- `serde` / `serde_json` - JSON serialization
- `tokio` - Async runtime
- `ndarray` - Numerical arrays
- `statrs` - Statistical functions
- `chrono` - Date/time handling
- `clap` - CLI argument parsing
- `tracing` - Logging

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_constant_product

# Run benchmarks
cargo bench
```

## Disclaimer

This library is for **educational purposes only**. DEX arbitrage involves significant risks:

- **MEV Risk**: Transactions can be front-run by sophisticated actors
- **Smart Contract Risk**: Bugs in contracts can lead to fund loss
- **Gas Risk**: Unpredictable gas prices can eliminate profits
- **Liquidity Risk**: Slippage may exceed expectations
- **Regulatory Risk**: Legal status varies by jurisdiction

Do not use real funds without thorough understanding of these risks.

## License

MIT License

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.
