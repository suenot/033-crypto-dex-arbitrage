//! Pool trait for AMM implementations

use anyhow::Result;

/// Common trait for all AMM pool implementations
pub trait Pool: Send + Sync {
    /// Get the name of this pool type
    fn name(&self) -> &str;

    /// Get the current spot price (token Y per token X)
    fn get_price(&self) -> f64;

    /// Calculate amount out for a given amount in
    ///
    /// # Arguments
    /// * `amount_in` - Amount of input token
    /// * `is_x_to_y` - True if swapping X for Y, false if Y for X
    fn get_amount_out(&self, amount_in: f64, is_x_to_y: bool) -> f64;

    /// Calculate the slippage for a given trade
    fn get_slippage(&self, amount_in: f64, is_x_to_y: bool) -> f64;

    /// Calculate the price impact of a trade
    fn get_price_impact(&self, amount_in: f64, is_x_to_y: bool) -> f64;

    /// Execute a swap (mutates pool state)
    fn swap(&mut self, amount_in: f64, is_x_to_y: bool) -> Result<f64>;

    /// Get reserve of token X
    fn reserve_x(&self) -> f64;

    /// Get reserve of token Y
    fn reserve_y(&self) -> f64;

    /// Get the fee rate (e.g., 0.003 for 0.3%)
    fn fee_rate(&self) -> f64;

    /// Get total liquidity in USD terms
    fn liquidity_usd(&self, price_x_usd: f64) -> f64 {
        self.reserve_x() * price_x_usd + self.reserve_y()
    }

    /// Clone this pool into a Box
    fn clone_box(&self) -> Box<dyn Pool>;
}

impl Clone for Box<dyn Pool> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
