//! Curve StableSwap AMM implementation
//!
//! Implements the StableSwap invariant designed for assets
//! that should trade near parity (like stablecoins).
//!
//! The invariant combines constant sum and constant product:
//! An * sum(x_i) + D = AnD + D^(n+1) / (n^n * prod(x_i))

use super::Pool;
use anyhow::{anyhow, Result};

/// Curve StableSwap AMM implementation
///
/// Uses a hybrid invariant optimized for stablecoins with minimal slippage
/// when tokens trade near parity.
#[derive(Debug, Clone)]
pub struct CurveStableSwap {
    /// Reserves of each token in the pool
    reserves: Vec<f64>,
    /// Amplification coefficient (A parameter)
    /// Higher A = closer to constant sum (less slippage near parity)
    amplification: f64,
    /// Fee rate
    fee: f64,
    /// Pool name
    name: String,
}

impl CurveStableSwap {
    /// Create a new Curve StableSwap pool
    ///
    /// # Arguments
    /// * `reserves` - Initial reserves for each token
    /// * `amplification` - The A parameter (typically 50-2000 for stablecoins)
    /// * `fee` - Fee rate (e.g., 0.0004 for 0.04%)
    pub fn new(reserves: Vec<f64>, amplification: f64, fee: f64) -> Self {
        Self {
            reserves,
            amplification,
            fee,
            name: "CurveStableSwap".to_string(),
        }
    }

    /// Create with a custom name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Number of tokens in the pool
    pub fn n_tokens(&self) -> usize {
        self.reserves.len()
    }

    /// Get reserve by index
    pub fn reserve(&self, idx: usize) -> f64 {
        self.reserves.get(idx).copied().unwrap_or(0.0)
    }

    /// Calculate D (the invariant)
    ///
    /// Uses Newton's method to solve for D
    pub fn calculate_d(&self) -> f64 {
        let n = self.reserves.len() as f64;
        let sum: f64 = self.reserves.iter().sum();

        if sum == 0.0 {
            return 0.0;
        }

        let ann = self.amplification * n;

        // Initial guess
        let mut d = sum;
        let mut d_prev: f64;

        // Newton iteration
        for _ in 0..256 {
            let mut d_p = d;
            for reserve in &self.reserves {
                d_p = d_p * d / (reserve * n);
            }

            d_prev = d;
            d = (ann * sum + d_p * n) * d / ((ann - 1.0) * d + (n + 1.0) * d_p);

            if (d - d_prev).abs() <= 1.0 {
                break;
            }
        }

        d
    }

    /// Calculate y given x for a 2-token pool
    ///
    /// Solves for the new reserve of token j after swapping
    fn calculate_y(&self, i: usize, j: usize, new_reserve_i: f64) -> f64 {
        let n = self.reserves.len() as f64;
        let d = self.calculate_d();
        let ann = self.amplification * n;

        let mut c = d;
        let mut s = 0.0;

        for (k, reserve) in self.reserves.iter().enumerate() {
            let x_k = if k == i { new_reserve_i } else { *reserve };

            if k != j {
                s += x_k;
                c = c * d / (x_k * n);
            }
        }

        c = c * d / (ann * n);
        let b = s + d / ann;

        // Newton iteration for y
        let mut y = d;
        let mut y_prev: f64;

        for _ in 0..256 {
            y_prev = y;
            y = (y * y + c) / (2.0 * y + b - d);

            if (y - y_prev).abs() <= 1.0 {
                break;
            }
        }

        y
    }

    /// Get amount out for swapping from token i to token j
    pub fn get_amount_out_ij(&self, amount_in: f64, i: usize, j: usize) -> f64 {
        if i >= self.reserves.len() || j >= self.reserves.len() || i == j {
            return 0.0;
        }

        let amount_in_with_fee = amount_in * (1.0 - self.fee);
        let new_reserve_i = self.reserves[i] + amount_in_with_fee;
        let new_reserve_j = self.calculate_y(i, j, new_reserve_i);

        self.reserves[j] - new_reserve_j
    }

    /// Execute a swap between tokens i and j
    pub fn swap_ij(&mut self, amount_in: f64, i: usize, j: usize) -> Result<f64> {
        if i >= self.reserves.len() || j >= self.reserves.len() {
            return Err(anyhow!("Invalid token index"));
        }
        if i == j {
            return Err(anyhow!("Cannot swap same token"));
        }
        if amount_in <= 0.0 {
            return Err(anyhow!("Amount must be positive"));
        }

        let amount_out = self.get_amount_out_ij(amount_in, i, j);
        if amount_out <= 0.0 || amount_out > self.reserves[j] {
            return Err(anyhow!("Insufficient liquidity"));
        }

        self.reserves[i] += amount_in;
        self.reserves[j] -= amount_out;

        Ok(amount_out)
    }

    /// Calculate virtual price (price of LP token in terms of underlying)
    pub fn virtual_price(&self) -> f64 {
        let d = self.calculate_d();
        let n = self.reserves.len() as f64;
        d / (self.reserves.iter().sum::<f64>() / n)
    }

    /// Get the amplification coefficient
    pub fn amplification(&self) -> f64 {
        self.amplification
    }

    /// Set amplification (for ramping)
    pub fn set_amplification(&mut self, a: f64) {
        self.amplification = a;
    }
}

impl Pool for CurveStableSwap {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_price(&self) -> f64 {
        // For a 2-token pool, price is approximately 1:1 for stablecoins
        // We return the marginal price of token 1 in terms of token 0
        if self.reserves.len() >= 2 {
            // Approximate marginal price
            let dy = self.get_amount_out_ij(1.0, 0, 1);
            dy
        } else {
            1.0
        }
    }

    fn get_amount_out(&self, amount_in: f64, is_x_to_y: bool) -> f64 {
        if self.reserves.len() < 2 {
            return 0.0;
        }

        if is_x_to_y {
            self.get_amount_out_ij(amount_in, 0, 1)
        } else {
            self.get_amount_out_ij(amount_in, 1, 0)
        }
    }

    fn get_slippage(&self, amount_in: f64, is_x_to_y: bool) -> f64 {
        let amount_out = self.get_amount_out(amount_in, is_x_to_y);
        if amount_out == 0.0 {
            return 1.0;
        }

        // For stablecoins, ideal rate is 1:1
        // Slippage is deviation from 1:1
        let effective_rate = amount_out / amount_in;
        (1.0 - effective_rate).abs()
    }

    fn get_price_impact(&self, amount_in: f64, is_x_to_y: bool) -> f64 {
        let price_before = self.get_price();

        // Simulate the swap
        let mut clone = self.clone();
        let _ = if is_x_to_y {
            clone.swap_ij(amount_in, 0, 1)
        } else {
            clone.swap_ij(amount_in, 1, 0)
        };

        let price_after = clone.get_price();
        (price_after - price_before).abs() / price_before
    }

    fn swap(&mut self, amount_in: f64, is_x_to_y: bool) -> Result<f64> {
        if self.reserves.len() < 2 {
            return Err(anyhow!("Pool needs at least 2 tokens"));
        }

        if is_x_to_y {
            self.swap_ij(amount_in, 0, 1)
        } else {
            self.swap_ij(amount_in, 1, 0)
        }
    }

    fn reserve_x(&self) -> f64 {
        self.reserve(0)
    }

    fn reserve_y(&self) -> f64 {
        self.reserve(1)
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
    fn test_new_stableswap() {
        let pool = CurveStableSwap::new(vec![1_000_000.0, 1_000_000.0], 100.0, 0.0004);
        assert_eq!(pool.n_tokens(), 2);
        assert_eq!(pool.reserve(0), 1_000_000.0);
    }

    #[test]
    fn test_calculate_d() {
        let pool = CurveStableSwap::new(vec![1_000_000.0, 1_000_000.0], 100.0, 0.0004);
        let d = pool.calculate_d();
        // D should be close to 2 * 1_000_000 for balanced pool
        assert!(d > 1_900_000.0 && d < 2_100_000.0);
    }

    #[test]
    fn test_low_slippage() {
        let pool = CurveStableSwap::new(vec![1_000_000.0, 1_000_000.0], 100.0, 0.0004);

        // Swap 1000 tokens - should have very low slippage for stablecoins
        let slippage = pool.get_slippage(1000.0, true);
        assert!(slippage < 0.001); // Less than 0.1% slippage
    }

    #[test]
    fn test_swap() {
        let mut pool = CurveStableSwap::new(vec![1_000_000.0, 1_000_000.0], 100.0, 0.0004);

        let amount_out = pool.swap(1000.0, true).unwrap();
        // Should get nearly 1000 out (minus small fee and slippage)
        assert!(amount_out > 999.0 && amount_out < 1000.0);
    }

    #[test]
    fn test_virtual_price() {
        let pool = CurveStableSwap::new(vec![1_000_000.0, 1_000_000.0], 100.0, 0.0004);
        let vp = pool.virtual_price();
        // Virtual price should be close to 1 for balanced pool
        assert!(vp > 0.99 && vp < 1.01);
    }
}
