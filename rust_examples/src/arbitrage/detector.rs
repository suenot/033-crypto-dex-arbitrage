//! Arbitrage opportunity detection

use crate::amm::Pool;
use serde::{Deserialize, Serialize};

/// An arbitrage opportunity between two pools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    /// Index of pool to buy from
    pub buy_pool_idx: usize,
    /// Index of pool to sell to
    pub sell_pool_idx: usize,
    /// Optimal trade size
    pub trade_size: f64,
    /// Gross profit before costs
    pub gross_profit: f64,
    /// Estimated gas cost
    pub gas_cost: f64,
    /// Estimated slippage cost
    pub slippage_cost: f64,
    /// Net profit after costs
    pub net_profit: f64,
    /// Return on investment
    pub roi: f64,
    /// Price on buy pool
    pub buy_price: f64,
    /// Price on sell pool
    pub sell_price: f64,
    /// Price difference percentage
    pub price_diff_pct: f64,
}

/// Arbitrage detector across multiple pools
pub struct ArbitrageDetector {
    pools: Vec<Box<dyn Pool>>,
    gas_price_gwei: f64,
    gas_per_swap: u64,
    eth_price_usd: f64,
}

impl ArbitrageDetector {
    /// Create a new arbitrage detector
    ///
    /// # Arguments
    /// * `pools` - Vector of pool implementations
    /// * `gas_price_gwei` - Current gas price in Gwei
    pub fn new(pools: Vec<Box<dyn Pool>>, gas_price_gwei: f64) -> Self {
        Self {
            pools,
            gas_price_gwei,
            gas_per_swap: 150_000, // Typical gas for a swap
            eth_price_usd: 2000.0, // Default ETH price
        }
    }

    /// Set ETH price for gas cost calculation
    pub fn with_eth_price(mut self, eth_price: f64) -> Self {
        self.eth_price_usd = eth_price;
        self
    }

    /// Set gas per swap
    pub fn with_gas_per_swap(mut self, gas: u64) -> Self {
        self.gas_per_swap = gas;
        self
    }

    /// Calculate gas cost in USD for a single swap
    fn gas_cost_usd(&self) -> f64 {
        let gas_cost_eth = (self.gas_price_gwei * self.gas_per_swap as f64) / 1e9;
        gas_cost_eth * self.eth_price_usd
    }

    /// Find arbitrage opportunities
    ///
    /// # Arguments
    /// * `trade_size` - Size of trade in quote currency (e.g., USDC)
    /// * `min_profit_pct` - Minimum profit percentage to consider
    pub fn find_opportunities(
        &self,
        trade_size: f64,
        min_profit_pct: f64,
    ) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        let gas_cost = self.gas_cost_usd() * 2.0; // Two swaps

        // Compare all pairs of pools
        for i in 0..self.pools.len() {
            for j in 0..self.pools.len() {
                if i == j {
                    continue;
                }

                let pool_i = &self.pools[i];
                let pool_j = &self.pools[j];

                let price_i = pool_i.get_price();
                let price_j = pool_j.get_price();

                // Check if there's a price difference
                // Buy from cheaper pool (lower price), sell to more expensive (higher price)
                if price_i < price_j {
                    let price_diff_pct = (price_j - price_i) / price_i;

                    // Calculate amounts
                    // Buy base token with trade_size quote on pool_i
                    let base_amount = pool_i.get_amount_out(trade_size, false); // Y to X

                    // Sell base token for quote on pool_j
                    let quote_received = pool_j.get_amount_out(base_amount, true); // X to Y

                    let gross_profit = quote_received - trade_size;

                    // Calculate slippage costs
                    let slippage_i = pool_i.get_slippage(trade_size, false);
                    let slippage_j = pool_j.get_slippage(base_amount, true);
                    let slippage_cost = trade_size * (slippage_i + slippage_j);

                    let net_profit = gross_profit - gas_cost;
                    let roi = net_profit / trade_size;

                    if roi >= min_profit_pct {
                        opportunities.push(ArbitrageOpportunity {
                            buy_pool_idx: i,
                            sell_pool_idx: j,
                            trade_size,
                            gross_profit,
                            gas_cost,
                            slippage_cost,
                            net_profit,
                            roi,
                            buy_price: price_i,
                            sell_price: price_j,
                            price_diff_pct,
                        });
                    }
                }
            }
        }

        // Sort by net profit descending
        opportunities.sort_by(|a, b| b.net_profit.partial_cmp(&a.net_profit).unwrap());

        opportunities
    }

    /// Find optimal trade size for an opportunity
    ///
    /// Uses binary search to find the trade size that maximizes profit
    pub fn optimize_trade_size(
        &self,
        buy_pool_idx: usize,
        sell_pool_idx: usize,
        max_size: f64,
    ) -> (f64, f64) {
        let gas_cost = self.gas_cost_usd() * 2.0;

        let mut best_size = 0.0;
        let mut best_profit = f64::NEG_INFINITY;

        // Simple grid search (could be improved with gradient methods)
        let steps = 100;
        for i in 1..=steps {
            let size = max_size * (i as f64 / steps as f64);

            let pool_buy = &self.pools[buy_pool_idx];
            let pool_sell = &self.pools[sell_pool_idx];

            let base_amount = pool_buy.get_amount_out(size, false);
            let quote_received = pool_sell.get_amount_out(base_amount, true);
            let net_profit = quote_received - size - gas_cost;

            if net_profit > best_profit {
                best_profit = net_profit;
                best_size = size;
            }
        }

        (best_size, best_profit)
    }

    /// Get number of pools
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    /// Get pool by index
    pub fn pool(&self, idx: usize) -> Option<&dyn Pool> {
        self.pools.get(idx).map(|p| p.as_ref())
    }

    /// Estimate MEV risk for an opportunity
    ///
    /// Returns a risk score from 0 to 1
    pub fn estimate_mev_risk(&self, opportunity: &ArbitrageOpportunity) -> f64 {
        // Factors that increase MEV risk:
        // 1. High profit attracts more searchers
        // 2. Simple two-pool arbitrage is easy to detect
        // 3. Low gas price makes it easy to frontrun

        let profit_factor = (opportunity.net_profit / 1000.0).min(1.0) * 0.4;
        let gas_factor = (1.0 - self.gas_price_gwei / 200.0).max(0.0).min(1.0) * 0.3;
        let simplicity_factor = 0.3; // Two-pool arb is simple

        profit_factor + gas_factor + simplicity_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amm::ConstantProductAMM;

    #[test]
    fn test_detect_arbitrage() {
        // Pool 1: 1 ETH = 2000 USDC
        let pool1 = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003)
            .with_name("Pool1");

        // Pool 2: 1 ETH = 2050 USDC (2.5% higher)
        let pool2 = ConstantProductAMM::new(800.0, 1_640_000.0, 0.003)
            .with_name("Pool2");

        let pools: Vec<Box<dyn Pool>> = vec![
            Box::new(pool1),
            Box::new(pool2),
        ];

        let detector = ArbitrageDetector::new(pools, 50.0);
        let opportunities = detector.find_opportunities(10000.0, 0.0);

        // Should find at least one opportunity
        assert!(!opportunities.is_empty());
    }

    #[test]
    fn test_no_arbitrage_same_price() {
        let pool1 = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);
        let pool2 = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);

        let pools: Vec<Box<dyn Pool>> = vec![
            Box::new(pool1),
            Box::new(pool2),
        ];

        let detector = ArbitrageDetector::new(pools, 50.0);
        let opportunities = detector.find_opportunities(10000.0, 0.001);

        // Should find no profitable opportunities (fees eat the profit)
        assert!(opportunities.is_empty());
    }

    #[test]
    fn test_optimize_trade_size() {
        let pool1 = ConstantProductAMM::new(1000.0, 2_000_000.0, 0.003);
        let pool2 = ConstantProductAMM::new(800.0, 1_680_000.0, 0.003);

        let pools: Vec<Box<dyn Pool>> = vec![
            Box::new(pool1),
            Box::new(pool2),
        ];

        let detector = ArbitrageDetector::new(pools, 50.0);
        let (optimal_size, profit) = detector.optimize_trade_size(0, 1, 100000.0);

        assert!(optimal_size > 0.0);
        // Profit might be negative due to gas costs
        println!("Optimal size: {}, profit: {}", optimal_size, profit);
    }
}
