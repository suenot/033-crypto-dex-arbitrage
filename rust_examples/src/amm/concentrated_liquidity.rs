//! Concentrated Liquidity AMM (Uniswap V3 style)
//!
//! Implements concentrated liquidity where LPs can provide liquidity
//! within specific price ranges (ticks).

use super::Pool;
use anyhow::{anyhow, Result};

/// A liquidity position in a concentrated liquidity AMM
#[derive(Debug, Clone)]
pub struct LiquidityPosition {
    /// Lower price bound (tick)
    pub price_lower: f64,
    /// Upper price bound (tick)
    pub price_upper: f64,
    /// Amount of liquidity in this position
    pub liquidity: f64,
}

impl LiquidityPosition {
    /// Create a new liquidity position
    pub fn new(price_lower: f64, price_upper: f64, liquidity: f64) -> Self {
        Self {
            price_lower,
            price_upper,
            liquidity,
        }
    }

    /// Check if a price is within this position's range
    pub fn is_in_range(&self, price: f64) -> bool {
        price >= self.price_lower && price <= self.price_upper
    }

    /// Get the effective liquidity at a given price
    pub fn effective_liquidity(&self, price: f64) -> f64 {
        if self.is_in_range(price) {
            self.liquidity
        } else {
            0.0
        }
    }
}

/// Concentrated Liquidity AMM implementation
///
/// Based on Uniswap V3's concentrated liquidity model where
/// liquidity providers can specify price ranges.
#[derive(Debug, Clone)]
pub struct ConcentratedLiquidityAMM {
    /// Current price (Y per X)
    current_price: f64,
    /// List of liquidity positions
    positions: Vec<LiquidityPosition>,
    /// Fee rate
    fee: f64,
    /// Pool name
    name: String,
    /// Virtual reserve X (for compatibility)
    virtual_reserve_x: f64,
    /// Virtual reserve Y (for compatibility)
    virtual_reserve_y: f64,
}

impl ConcentratedLiquidityAMM {
    /// Create a new Concentrated Liquidity AMM
    pub fn new(initial_price: f64, fee: f64) -> Self {
        Self {
            current_price: initial_price,
            positions: Vec::new(),
            fee,
            name: "ConcentratedLiquidity".to_string(),
            virtual_reserve_x: 1000.0,
            virtual_reserve_y: initial_price * 1000.0,
        }
    }

    /// Create with a custom name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Add a liquidity position
    pub fn add_position(&mut self, position: LiquidityPosition) {
        self.positions.push(position);
        self.update_virtual_reserves();
    }

    /// Get total liquidity at current price
    pub fn liquidity_at_price(&self, price: f64) -> f64 {
        self.positions
            .iter()
            .map(|p| p.effective_liquidity(price))
            .sum()
    }

    /// Update virtual reserves based on positions
    fn update_virtual_reserves(&mut self) {
        let total_liquidity = self.liquidity_at_price(self.current_price);
        if total_liquidity > 0.0 {
            // Simplified calculation
            self.virtual_reserve_x = total_liquidity / self.current_price.sqrt();
            self.virtual_reserve_y = total_liquidity * self.current_price.sqrt();
        }
    }

    /// Calculate the amount out considering concentrated liquidity
    fn calculate_amount_out(&self, amount_in: f64, is_x_to_y: bool) -> f64 {
        let amount_in_with_fee = amount_in * (1.0 - self.fee);
        let current_liquidity = self.liquidity_at_price(self.current_price);

        if current_liquidity == 0.0 {
            return 0.0;
        }

        // Simplified calculation - in reality this would iterate through price ranges
        if is_x_to_y {
            // Selling X for Y
            // Use virtual reserves for approximation
            let k = self.virtual_reserve_x * self.virtual_reserve_y;
            let new_x = self.virtual_reserve_x + amount_in_with_fee;
            let new_y = k / new_x;
            self.virtual_reserve_y - new_y
        } else {
            // Buying X with Y
            let k = self.virtual_reserve_x * self.virtual_reserve_y;
            let new_y = self.virtual_reserve_y + amount_in_with_fee;
            let new_x = k / new_y;
            self.virtual_reserve_x - new_x
        }
    }

    /// Get positions
    pub fn positions(&self) -> &[LiquidityPosition] {
        &self.positions
    }

    /// Calculate impermanent loss for a position
    pub fn impermanent_loss(&self, position: &LiquidityPosition, new_price: f64) -> f64 {
        if !position.is_in_range(new_price) {
            // If price is out of range, position is fully in one asset
            return 0.0; // Simplified - actual IL calculation is more complex
        }

        let initial_price = (position.price_lower * position.price_upper).sqrt();
        let price_ratio = new_price / initial_price;

        // IL formula: 2 * sqrt(price_ratio) / (1 + price_ratio) - 1
        2.0 * price_ratio.sqrt() / (1.0 + price_ratio) - 1.0
    }
}

impl Pool for ConcentratedLiquidityAMM {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_price(&self) -> f64 {
        self.current_price
    }

    fn get_amount_out(&self, amount_in: f64, is_x_to_y: bool) -> f64 {
        self.calculate_amount_out(amount_in, is_x_to_y)
    }

    fn get_slippage(&self, amount_in: f64, is_x_to_y: bool) -> f64 {
        let amount_out = self.get_amount_out(amount_in, is_x_to_y);
        if amount_out == 0.0 {
            return 1.0; // 100% slippage if no output
        }

        if is_x_to_y {
            let effective_price = amount_out / amount_in;
            (self.current_price - effective_price) / self.current_price
        } else {
            let effective_price = amount_in / amount_out;
            (effective_price - self.current_price) / self.current_price
        }
    }

    fn get_price_impact(&self, amount_in: f64, is_x_to_y: bool) -> f64 {
        // Estimate new price after swap
        let amount_out = self.get_amount_out(amount_in, is_x_to_y);
        if amount_out == 0.0 {
            return 1.0;
        }

        let new_price = if is_x_to_y {
            (self.virtual_reserve_y - amount_out) / (self.virtual_reserve_x + amount_in)
        } else {
            (self.virtual_reserve_y + amount_in) / (self.virtual_reserve_x - amount_out)
        };

        (new_price - self.current_price).abs() / self.current_price
    }

    fn swap(&mut self, amount_in: f64, is_x_to_y: bool) -> Result<f64> {
        if amount_in <= 0.0 {
            return Err(anyhow!("Amount must be positive"));
        }

        let amount_out = self.get_amount_out(amount_in, is_x_to_y);
        if amount_out <= 0.0 {
            return Err(anyhow!("Insufficient liquidity"));
        }

        // Update virtual reserves
        if is_x_to_y {
            self.virtual_reserve_x += amount_in;
            self.virtual_reserve_y -= amount_out;
        } else {
            self.virtual_reserve_y += amount_in;
            self.virtual_reserve_x -= amount_out;
        }

        // Update current price
        self.current_price = self.virtual_reserve_y / self.virtual_reserve_x;

        Ok(amount_out)
    }

    fn reserve_x(&self) -> f64 {
        self.virtual_reserve_x
    }

    fn reserve_y(&self) -> f64 {
        self.virtual_reserve_y
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
    fn test_new_concentrated_liquidity() {
        let amm = ConcentratedLiquidityAMM::new(2000.0, 0.003);
        assert!((amm.get_price() - 2000.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_position() {
        let mut amm = ConcentratedLiquidityAMM::new(2000.0, 0.003);

        let position = LiquidityPosition::new(1800.0, 2200.0, 100_000.0);
        amm.add_position(position);

        assert_eq!(amm.positions().len(), 1);
        assert!(amm.liquidity_at_price(2000.0) > 0.0);
    }

    #[test]
    fn test_position_out_of_range() {
        let amm = ConcentratedLiquidityAMM::new(2000.0, 0.003);
        let position = LiquidityPosition::new(1800.0, 1900.0, 100_000.0);

        assert!(!position.is_in_range(2000.0));
        assert!(position.is_in_range(1850.0));
    }
}
