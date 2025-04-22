//! Arbitrage detection and execution
//!
//! This module provides tools for detecting and analyzing arbitrage
//! opportunities across multiple DEXes.

mod detector;
mod triangular;

pub use detector::{ArbitrageDetector, ArbitrageOpportunity};
pub use triangular::TriangularPath;
