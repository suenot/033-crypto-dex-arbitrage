//! Gas price modeling and prediction
//!
//! This module provides tools for estimating and predicting Ethereum gas prices.

use rand::Rng;
use serde::{Deserialize, Serialize};

/// Gas estimate for a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimate {
    /// Gas limit (units)
    pub gas_limit: u64,
    /// Gas price in Gwei
    pub gas_price_gwei: f64,
    /// Priority fee (tip) in Gwei
    pub priority_fee_gwei: f64,
    /// Max fee per gas in Gwei
    pub max_fee_gwei: f64,
    /// Estimated cost in ETH
    pub cost_eth: f64,
    /// Estimated cost in USD
    pub cost_usd: f64,
}

impl GasEstimate {
    /// Create a new gas estimate
    pub fn new(gas_limit: u64, gas_price_gwei: f64, eth_price_usd: f64) -> Self {
        let cost_eth = (gas_limit as f64 * gas_price_gwei) / 1e9;
        let cost_usd = cost_eth * eth_price_usd;

        Self {
            gas_limit,
            gas_price_gwei,
            priority_fee_gwei: 2.0,
            max_fee_gwei: gas_price_gwei * 1.5,
            cost_eth,
            cost_usd,
        }
    }

    /// Estimate for a DEX swap
    pub fn for_swap(gas_price_gwei: f64, eth_price_usd: f64) -> Self {
        Self::new(150_000, gas_price_gwei, eth_price_usd)
    }

    /// Estimate for a flashloan arbitrage (multiple operations)
    pub fn for_flashloan_arb(gas_price_gwei: f64, eth_price_usd: f64, num_swaps: u32) -> Self {
        // Base cost + per-swap cost
        let gas_limit = 100_000 + (num_swaps as u64 * 150_000);
        Self::new(gas_limit, gas_price_gwei, eth_price_usd)
    }

    /// Set priority fee
    pub fn with_priority_fee(mut self, priority_fee_gwei: f64) -> Self {
        self.priority_fee_gwei = priority_fee_gwei;
        self.max_fee_gwei = self.gas_price_gwei + priority_fee_gwei;
        self
    }
}

/// Gas price predictor using simple time-series features
pub struct GasPricePredictor {
    /// Current gas price baseline
    current_price: f64,
    /// Historical volatility
    volatility: f64,
    /// Mean reversion factor
    mean_reversion: f64,
    /// Long-term average
    long_term_mean: f64,
}

impl GasPricePredictor {
    /// Create a new predictor with current gas price
    pub fn new(current_price: f64) -> Self {
        Self {
            current_price,
            volatility: 0.1,       // 10% volatility
            mean_reversion: 0.05,  // Mean reversion speed
            long_term_mean: 50.0,  // Long-term average in Gwei
        }
    }

    /// Set volatility
    pub fn with_volatility(mut self, volatility: f64) -> Self {
        self.volatility = volatility;
        self
    }

    /// Set long-term mean
    pub fn with_long_term_mean(mut self, mean: f64) -> Self {
        self.long_term_mean = mean;
        self
    }

    /// Predict gas prices for next n blocks
    ///
    /// Uses a mean-reverting random walk model
    pub fn predict_next_blocks(&self, n_blocks: usize) -> Vec<f64> {
        let mut predictions = Vec::with_capacity(n_blocks);
        let mut rng = rand::thread_rng();

        let mut price = self.current_price;

        for _ in 0..n_blocks {
            // Mean reversion component
            let reversion = self.mean_reversion * (self.long_term_mean - price);

            // Random shock
            let shock: f64 = rng.gen_range(-1.0..1.0) * self.volatility * price;

            // Update price
            price = (price + reversion + shock).max(1.0);

            predictions.push(price);
        }

        predictions
    }

    /// Find the optimal block to execute within a deadline
    ///
    /// Returns the index of the block with lowest predicted gas price
    pub fn optimal_execution_block(&self, predictions: &[f64], deadline: usize) -> usize {
        let valid_range = predictions.len().min(deadline);
        if valid_range == 0 {
            return 0;
        }

        predictions[..valid_range]
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Estimate probability of gas being below a threshold
    pub fn probability_below(&self, threshold: f64, blocks_ahead: usize) -> f64 {
        // Simple Monte Carlo simulation
        let n_simulations = 1000;
        let mut count_below = 0;

        for _ in 0..n_simulations {
            let predictions = self.predict_next_blocks(blocks_ahead);
            if predictions.iter().any(|&p| p < threshold) {
                count_below += 1;
            }
        }

        count_below as f64 / n_simulations as f64
    }

    /// Get current price
    pub fn current_price(&self) -> f64 {
        self.current_price
    }

    /// Update current price
    pub fn update_price(&mut self, price: f64) {
        self.current_price = price;
    }
}

/// Time-of-day gas price patterns
pub struct GasTimePatterns {
    /// Average gas price by hour (0-23)
    hourly_averages: [f64; 24],
    /// Day of week multipliers (0=Sunday, 6=Saturday)
    day_multipliers: [f64; 7],
}

impl Default for GasTimePatterns {
    fn default() -> Self {
        Self::new()
    }
}

impl GasTimePatterns {
    /// Create with typical Ethereum patterns
    pub fn new() -> Self {
        // Typical pattern: lower at night (UTC), higher during business hours
        let hourly_averages = [
            35.0, 30.0, 28.0, 25.0, 25.0, 28.0,  // 0-5 (night)
            35.0, 45.0, 55.0, 60.0, 65.0, 70.0,  // 6-11 (morning)
            75.0, 80.0, 85.0, 80.0, 75.0, 70.0,  // 12-17 (afternoon)
            65.0, 60.0, 55.0, 50.0, 45.0, 40.0,  // 18-23 (evening)
        ];

        // Weekends typically have lower gas
        let day_multipliers = [
            0.7,  // Sunday
            1.0,  // Monday
            1.1,  // Tuesday
            1.1,  // Wednesday
            1.0,  // Thursday
            0.9,  // Friday
            0.7,  // Saturday
        ];

        Self {
            hourly_averages,
            day_multipliers,
        }
    }

    /// Get expected gas price for a given hour and day
    pub fn expected_price(&self, hour: usize, day_of_week: usize) -> f64 {
        let hour = hour % 24;
        let day = day_of_week % 7;

        self.hourly_averages[hour] * self.day_multipliers[day]
    }

    /// Find best time to execute in next n hours
    pub fn best_execution_time(&self, current_hour: usize, current_day: usize, hours_ahead: usize) -> (usize, usize, f64) {
        let mut best_hour = current_hour;
        let mut best_day = current_day;
        let mut best_price = f64::MAX;

        for i in 0..hours_ahead {
            let hour = (current_hour + i) % 24;
            let day = (current_day + (current_hour + i) / 24) % 7;
            let price = self.expected_price(hour, day);

            if price < best_price {
                best_price = price;
                best_hour = hour;
                best_day = day;
            }
        }

        (best_hour, best_day, best_price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_estimate() {
        let estimate = GasEstimate::for_swap(50.0, 2000.0);

        assert_eq!(estimate.gas_limit, 150_000);
        assert!((estimate.cost_eth - 0.0075).abs() < 0.0001);
        assert!((estimate.cost_usd - 15.0).abs() < 0.1);
    }

    #[test]
    fn test_predictor() {
        let predictor = GasPricePredictor::new(50.0);
        let predictions = predictor.predict_next_blocks(10);

        assert_eq!(predictions.len(), 10);
        // All predictions should be positive
        assert!(predictions.iter().all(|&p| p > 0.0));
    }

    #[test]
    fn test_optimal_block() {
        let predictor = GasPricePredictor::new(50.0);
        let predictions = vec![60.0, 55.0, 45.0, 50.0, 55.0];

        let optimal = predictor.optimal_execution_block(&predictions, 5);
        assert_eq!(optimal, 2); // Index of 45.0
    }

    #[test]
    fn test_time_patterns() {
        let patterns = GasTimePatterns::new();

        // Night should be cheaper than midday
        let night_price = patterns.expected_price(3, 1); // 3 AM Monday
        let midday_price = patterns.expected_price(14, 1); // 2 PM Monday

        assert!(night_price < midday_price);

        // Weekend should be cheaper than weekday
        let weekday = patterns.expected_price(12, 2); // Noon Tuesday
        let weekend = patterns.expected_price(12, 0); // Noon Sunday

        assert!(weekend < weekday);
    }
}
