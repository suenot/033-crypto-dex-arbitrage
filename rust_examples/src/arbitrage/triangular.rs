//! Triangular arbitrage detection
//!
//! Finds arbitrage opportunities across three trading pairs on a single DEX.
//! Example: ETH -> USDC -> DAI -> ETH

use serde::{Deserialize, Serialize};

/// A hop in a triangular path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathHop {
    /// Token to swap from
    pub token_in: String,
    /// Token to swap to
    pub token_out: String,
    /// Pool/pair identifier
    pub pool_id: String,
    /// Expected amount out per unit in
    pub rate: f64,
}

/// A triangular arbitrage path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriangularPath {
    /// Starting token
    pub start_token: String,
    /// The three hops in the path
    pub hops: Vec<PathHop>,
    /// Expected profit percentage (before costs)
    pub profit_pct: f64,
    /// Estimated gas cost
    pub gas_cost: f64,
    /// Net profit percentage
    pub net_profit_pct: f64,
}

impl TriangularPath {
    /// Create a new triangular path
    pub fn new(start_token: &str, hops: Vec<PathHop>) -> Self {
        let profit_pct = Self::calculate_profit(&hops);
        Self {
            start_token: start_token.to_string(),
            hops,
            profit_pct,
            gas_cost: 0.0,
            net_profit_pct: profit_pct,
        }
    }

    /// Set gas cost and update net profit
    pub fn with_gas_cost(mut self, gas_cost: f64, trade_size: f64) -> Self {
        self.gas_cost = gas_cost;
        let gas_pct = gas_cost / trade_size;
        self.net_profit_pct = self.profit_pct - gas_pct;
        self
    }

    /// Calculate profit from hop rates
    fn calculate_profit(hops: &[PathHop]) -> f64 {
        let mut amount = 1.0;
        for hop in hops {
            amount *= hop.rate;
        }
        // Profit is amount - 1 (we started with 1)
        amount - 1.0
    }

    /// Check if path is profitable
    pub fn is_profitable(&self) -> bool {
        self.net_profit_pct > 0.0
    }

    /// Get the path as a string (e.g., "ETH -> USDC -> DAI -> ETH")
    pub fn path_string(&self) -> String {
        let mut path = self.start_token.clone();
        for hop in &self.hops {
            path.push_str(" -> ");
            path.push_str(&hop.token_out);
        }
        path
    }

    /// Calculate required trade size for target profit
    pub fn required_size_for_profit(&self, target_profit: f64) -> f64 {
        if self.profit_pct <= 0.0 {
            return f64::INFINITY;
        }
        // Solve: trade_size * profit_pct - gas_cost = target_profit
        // trade_size = (target_profit + gas_cost) / profit_pct
        (target_profit + self.gas_cost) / self.profit_pct
    }
}

/// Find all triangular paths starting from a token
pub fn find_triangular_paths(
    start_token: &str,
    pairs: &[(String, String, f64)], // (token_a, token_b, rate_a_to_b)
) -> Vec<TriangularPath> {
    let mut paths = Vec::new();

    // Find first hop options (from start_token)
    let first_hops: Vec<_> = pairs
        .iter()
        .filter(|(a, _, _)| a == start_token)
        .collect();

    for (_, token_b, rate1) in &first_hops {
        // Find second hop options (from token_b, not back to start)
        let second_hops: Vec<_> = pairs
            .iter()
            .filter(|(a, b, _)| a == token_b && b != start_token)
            .collect();

        for (_, token_c, rate2) in &second_hops {
            // Find third hop back to start
            let third_hop = pairs
                .iter()
                .find(|(a, b, _)| a == token_c && b == start_token);

            if let Some((_, _, rate3)) = third_hop {
                let hops = vec![
                    PathHop {
                        token_in: start_token.to_string(),
                        token_out: token_b.clone(),
                        pool_id: format!("{}/{}", start_token, token_b),
                        rate: *rate1,
                    },
                    PathHop {
                        token_in: token_b.clone(),
                        token_out: token_c.clone(),
                        pool_id: format!("{}/{}", token_b, token_c),
                        rate: *rate2,
                    },
                    PathHop {
                        token_in: token_c.clone(),
                        token_out: start_token.to_string(),
                        pool_id: format!("{}/{}", token_c, start_token),
                        rate: *rate3,
                    },
                ];

                paths.push(TriangularPath::new(start_token, hops));
            }
        }
    }

    // Sort by profit descending
    paths.sort_by(|a, b| b.profit_pct.partial_cmp(&a.profit_pct).unwrap());

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_triangular_paths() {
        // Create some pairs with slight mispricing
        let pairs = vec![
            ("ETH".to_string(), "USDC".to_string(), 2000.0),
            ("USDC".to_string(), "DAI".to_string(), 1.001),  // Slight premium
            ("DAI".to_string(), "ETH".to_string(), 0.000501), // Slight discount
        ];

        let paths = find_triangular_paths("ETH", &pairs);

        assert!(!paths.is_empty());
        println!("Found {} paths", paths.len());
        for path in &paths {
            println!("  {} profit: {:.4}%", path.path_string(), path.profit_pct * 100.0);
        }
    }

    #[test]
    fn test_path_profit_calculation() {
        let hops = vec![
            PathHop {
                token_in: "A".to_string(),
                token_out: "B".to_string(),
                pool_id: "A/B".to_string(),
                rate: 1.01, // 1% gain
            },
            PathHop {
                token_in: "B".to_string(),
                token_out: "C".to_string(),
                pool_id: "B/C".to_string(),
                rate: 1.01, // 1% gain
            },
            PathHop {
                token_in: "C".to_string(),
                token_out: "A".to_string(),
                pool_id: "C/A".to_string(),
                rate: 1.01, // 1% gain
            },
        ];

        let path = TriangularPath::new("A", hops);

        // 1.01 * 1.01 * 1.01 - 1 ≈ 0.0303
        assert!(path.profit_pct > 0.03 && path.profit_pct < 0.031);
        assert!(path.is_profitable());
    }

    #[test]
    fn test_path_with_gas_cost() {
        let hops = vec![
            PathHop {
                token_in: "A".to_string(),
                token_out: "B".to_string(),
                pool_id: "A/B".to_string(),
                rate: 1.01,
            },
            PathHop {
                token_in: "B".to_string(),
                token_out: "C".to_string(),
                pool_id: "B/C".to_string(),
                rate: 1.01,
            },
            PathHop {
                token_in: "C".to_string(),
                token_out: "A".to_string(),
                pool_id: "C/A".to_string(),
                rate: 1.01,
            },
        ];

        let path = TriangularPath::new("A", hops)
            .with_gas_cost(100.0, 1000.0); // Gas is 10% of trade

        // Profit should be reduced by gas cost percentage
        assert!(path.net_profit_pct < path.profit_pct);
    }
}
