//! Automated Market Maker (AMM) implementations
//!
//! This module provides implementations of various AMM pricing models:
//! - Constant Product (Uniswap V2)
//! - Concentrated Liquidity (Uniswap V3)
//! - StableSwap (Curve)

mod constant_product;
mod concentrated_liquidity;
mod curve_stableswap;
mod pool;

pub use constant_product::ConstantProductAMM;
pub use concentrated_liquidity::{ConcentratedLiquidityAMM, LiquidityPosition};
pub use curve_stableswap::CurveStableSwap;
pub use pool::Pool;
