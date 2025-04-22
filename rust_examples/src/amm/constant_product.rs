//! Constant Product AMM (Uniswap V2 style)
//!
//! Implements the x * y = k formula where:
//! - x is the reserve of token X
//! - y is the reserve of token Y
//! - k is the constant invariant

use super::Pool;
use anyhow::{anyhow, Result};

/// Constant Product AMM implementation
///
/// Uses the formula: x * y = k
/// This is the model used by Uniswap V2, SushiSwap, and many other DEXes.
#[derive(Debug, Clone)]
pub struct ConstantProductAMM {
    /// Reserve of token X (e.g., ETH)
    reserve_x: f64,
    /// Reserve of token Y (e.g., USDC)
    reserve_y: f64,
    /// The constant product invariant
    k: f64,
    /// Fee rate (e.g., 0.003 for 0.3%)
    fee: f64,
    /// Pool name
    name: String,
}

impl ConstantProductAMM {
    /// Create a new Constant Product AMM
    ///
    /// # Arguments
    /// * `reserve_x` - Initial reserve of token X
    /// * `reserve_y` - Initial reserve of token Y
    /// * `fee` - Fee rate (e.g., 0.003 for 0.3%)
    pub fn new(reserve_x: f64, reserve_y: f64, fee: f64) -> Self {
        Self {
            reserve_x,
            reserve_y,
            k: reserve_x * reserve_y,
            fee,
            name: "ConstantProduct".to_string(),
        }
    }

    /// Create with a custom name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Get the constant product invariant k
    pub fn k(&self) -> f64 {
        self.k
    }

    /// Calculate optimal arbitrage amount
    ///
    /// Given a target price, calculate how much to trade to reach that price.
    pub fn optimal_arbitrage_amount(&self, target_price: f64) -> f64 {
        let current_price = self.get_price();

        if (target_price - current_price).abs() < 1e-10 {
            return 0.0;
        }

        // For x * y = k, price = y/x
        // After swap: (x + dx) * (y - dy) = k
        // New price = (y - dy) / (x + dx) = target_price
        //
        // Solving: dx = sqrt(k / target_price) - x

        let optimal_x = (self.k / target_price).sqrt();
        let dx = optimal_x - self.reserve_x;

        if dx > 0.0 {
            // Need to add X (sell X for Y)
            dx
        } else {
            // Need to remove X (buy X with Y)
            // Calculate equivalent Y amount
            let optimal_y = (self.k * target_price).sqrt();
            optimal_y - self.reserve_y
        }
    }

    /// Add liquidity to the pool
    ///
    /// Returns the amount of liquidity tokens that would be minted
    pub fn add_liquidity(&mut self, amount_x: f64, amount_y: f64) -> Result<f64> {
        if amount_x <= 0.0 || amount_y <= 0.0 {
            return Err(anyhow!("Amounts must be positive"));
        }

        // Check if amounts are in correct ratio
        let expected_ratio = self.reserve_y / self.reserve_x;
        let provided_ratio = amount_y / amount_x;
        let ratio_diff = (provided_ratio - expected_ratio).abs() / expected_ratio;

        if ratio_diff > 0.01 {
            return Err(anyhow!(
                "Amounts not in correct ratio. Expected Y/X = {:.4}, got {:.4}",
                expected_ratio,
                provided_ratio
            ));
        }

        // Calculate liquidity tokens (simplified - using geometric mean)
        let total_liquidity = (self.reserve_x * self.reserve_y).sqrt();
        let new_liquidity = (amount_x * amount_y).sqrt();
        let liquidity_share = new_liquidity / total_liquidity;

        self.reserve_x += amount_x;
        self.reserve_y += amount_y;
        self.k = self.reserve_x * self.reserve_y;

        Ok(liquidity_share)
    }

    /// Remove liquidity from the pool
    ///
    /// Returns (amount_x, amount_y) withdrawn
    pub fn remove_liquidity(&mut self, liquidity_share: f64) -> Result<(f64, f64)> {
        if liquidity_share <= 0.0 || liquidity_share > 1.0 {
            return Err(anyhow!("Liquidity share must be between 0 and 1"));
        }

        let amount_x = self.reserve_x * liquidity_share;
        let amount_y = self.reserve_y * liquidity_share;

        self.reserve_x -= amount_x;
        self.reserve_y -= amount_y;
        self.k = self.reserve_x * self.reserve_y;

        Ok((amount_x, amount_y))
    }
}

impl Pool for ConstantProductAMM {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_price(&self) -> f64 {
        self.reserve_y / self.reserve_x
    }

    fn get_amount_out(&self, amount_in: f64, is_x_to_y: bool) -> f64 {
        let amount_in_with_fee = amount_in * (1.0 - self.fee);

        if is_x_to_y {
            // Swapping X for Y
            let new_reserve_x = self.reserve_x + amount_in_with_fee;
            let new_reserve_y = self.k / new_reserve_x;
            self.reserve_y - new_reserve_y
        } else {
            // Swapping Y for X
            let new_reserve_y = self.reserve_y + amount_in_with_fee;
            let new_reserve_x = self.k / new_reserve_y;
            self.reserve_x - new_reserve_x
        }
    }

    fn get_slippage(&self, amount_in: f64, is_x_to_y: bool) -> f64 {
        let spot_price = self.get_price();
        let amount_out = self.get_amount_out(amount_in, is_x_to_y);

        if is_x_to_y {
            // Selling X for Y: effective price should be close to spot_price
            let effective_price = amount_out / amount_in; // Y per X
            (spot_price - effective_price) / spot_price
        } else {
            // Buying X with Y: effective price is amount_in / amount_out
            let effective_price = amount_in / amount_out; // Y per X
            (effective_price - spot_price) / spot_price
        }
    }

    fn get_price_impact(&self, amount_in: f64, is_x_to_y: bool) -> f64 {
        let price_before = self.get_price();

        // Simulate the swap
        let amount_in_with_fee = amount_in * (1.0 - self.fee);
        let price_after = if is_x_to_y {
            let new_x = self.reserve_x + amount_in_with_fee;
            let new_y = self.k / new_x;
            new_y / new_x
        } else {
            let new_y = self.reserve_y + amount_in_with_fee;
            let new_x = self.k / new_y;
            new_y / new_x
        };

        (price_after - price_before).abs() / price_before
    }

    fn swap(&mut self, amount_in: f64, is_x_to_y: bool) -> Result<f64> {
        if amount_in <= 0.0 {
            return Err(anyhow!("Amount must be positive"));
        }

        let amount_out = self.get_amount_out(amount_in, is_x_to_y);

        if is_x_to_y {
            if amount_out > self.reserve_y {
                return Err(anyhow!("Insufficient liquidity"));
            }
            self.reserve_x += amount_in;
            self.reserve_y -= amount_out;
        } else {
            if amount_out > self.reserve_x {
                return Err(anyhow!("Insufficient liquidity"));
            }
            self.reserve_y += amount_in;
            self.reserve_x -= amount_out;
        }

        // Note: k changes slightly due to fees accumulating
        // In real Uniswap, k increases over time as fees are collected

        Ok(amount_out)
    }

    fn reserve_x(&self) -> f64 {
        self.reserve_x
    }

    fn reserve_y(&self) -> f64 {
        self.reserve_y
    }

    fn fee_rate(&self) -> f64 {
        self.fee
    }

    fn clone_box(&self) -> Box<dyn Pool> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_amm() {
        let amm = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);
        assert_eq!(amm.reserve_x(), 1000.0);
        assert_eq!(amm.reserve_y(), 2_000_000.0);
        assert_eq!(amm.k(), 2_000_000_000.0);
    }

    #[test]
    fn test_price() {
        let amm = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);
        assert!((amm.get_price() - 2000.0).abs() < 1e-10);
    }

    #[test]
    fn test_swap_x_to_y() {
        let mut amm = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);

        // Swap 10 ETH for USDC
        let amount_out = amm.swap(10.0, true).unwrap();

        // Should get slightly less than 20000 USDC due to slippage and fees
        assert!(amount_out > 19000.0 && amount_out < 20000.0);

        // Price should have increased (ETH is more scarce)
        assert!(amm.get_price() > 2000.0);
    }

    #[test]
    fn test_slippage_increases_with_size() {
        let amm = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);

        let slippage_small = amm.get_slippage(1.0, true);
        let slippage_large = amm.get_slippage(100.0, true);

        assert!(slippage_large > slippage_small);
    }

    #[test]
    fn test_add_remove_liquidity() {
        let mut amm = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);

        // Add 10% more liquidity
        let share = amm.add_liquidity(100.0, 200_000.0).unwrap();
        assert!(share > 0.0);

        assert!((amm.reserve_x() - 1100.0).abs() < 1e-10);
        assert!((amm.reserve_y() - 2_200_000.0).abs() < 1e-10);
    }
}
