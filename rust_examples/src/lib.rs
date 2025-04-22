//! # DEX Arbitrage Library
//!
//! A modular library for simulating DEX arbitrage with AMM pricing models.
//! Uses Bybit market data for realistic cryptocurrency price simulations.
//!
//! ## Modules
//!
//! - `api` - Bybit API client for fetching market data
//! - `amm` - Automated Market Maker implementations (Uniswap V2/V3, Curve)
//! - `arbitrage` - Arbitrage opportunity detection and analysis
//! - `gas` - Gas price modeling and prediction
//! - `flashloan` - Flashloan simulation and execution
//! - `data` - Data structures and utilities
//! - `metrics` - Performance metrics and analysis
//!
//! ## Example
//!
//! ```rust,no_run
//! use dex_arbitrage::{BybitClient, ConstantProductAMM, ArbitrageDetector};
//!
//! // Fetch market data
//! let client = BybitClient::new();
//! let klines = client.get_klines("ETHUSDT", Interval::Hour1, Some(100), None, None)?;
//!
//! // Simulate AMM pools
//! let uniswap = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);
//! let sushiswap = ConstantProductAMM::new(800.0, 1_620_000.0, 0.003);
//!
//! // Detect arbitrage
//! let detector = ArbitrageDetector::new(vec![uniswap, sushiswap]);
//! let opportunities = detector.find_opportunities(100.0);
//! ```

pub mod amm;
pub mod api;
pub mod arbitrage;
pub mod data;
pub mod flashloan;
pub mod gas;
pub mod metrics;

// Re-export commonly used types
pub use amm::{
    ConcentratedLiquidityAMM, ConstantProductAMM, CurveStableSwap, LiquidityPosition, Pool,
};
pub use api::{BybitClient, BybitError, Interval, Kline, TickerInfo};
pub use arbitrage::{ArbitrageDetector, ArbitrageOpportunity, TriangularPath};
pub use data::{PriceData, TokenPair, TradingPair};
pub use flashloan::{FlashloanExecutor, FlashloanProvider, FlashloanResult};
pub use gas::{GasEstimate, GasPricePredictor};
pub use metrics::{ArbitrageMetrics, PerformanceReport};
