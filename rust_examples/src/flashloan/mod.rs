//! Flashloan simulation and execution
//!
//! This module provides tools for simulating flashloan-based arbitrage.

use serde::{Deserialize, Serialize};

/// Flashloan provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlashloanProvider {
    /// Aave V3 - 0.09% fee (0.0009)
    Aave,
    /// dYdX - 0 fee (but requires collateral in pool)
    DyDx,
    /// Uniswap V3 - 0.05% fee (0.0005)
    UniswapV3,
    /// Balancer - 0% fee
    Balancer,
}

impl FlashloanProvider {
    /// Get the fee rate for this provider
    pub fn fee_rate(&self) -> f64 {
        match self {
            FlashloanProvider::Aave => 0.0009,      // 0.09%
            FlashloanProvider::DyDx => 0.0,         // 0%
            FlashloanProvider::UniswapV3 => 0.0005, // 0.05%
            FlashloanProvider::Balancer => 0.0,     // 0%
        }
    }

    /// Get max loan amount (approximate, in USD)
    pub fn max_loan_usd(&self) -> f64 {
        match self {
            FlashloanProvider::Aave => 1_000_000_000.0,     // $1B
            FlashloanProvider::DyDx => 500_000_000.0,       // $500M
            FlashloanProvider::UniswapV3 => 100_000_000.0,  // $100M
            FlashloanProvider::Balancer => 50_000_000.0,    // $50M
        }
    }
}

/// Result of a flashloan execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashloanResult {
    /// Provider used
    pub provider: FlashloanProvider,
    /// Loan amount
    pub loan_amount: f64,
    /// Fee paid
    pub fee_paid: f64,
    /// Gross profit from arbitrage
    pub gross_profit: f64,
    /// Gas cost
    pub gas_cost: f64,
    /// Net profit
    pub net_profit: f64,
    /// Success flag
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Flashloan executor for arbitrage
pub struct FlashloanExecutor {
    provider: FlashloanProvider,
    max_gas_price: f64,
    min_profit_threshold: f64,
}

impl FlashloanExecutor {
    /// Create a new executor with specified provider
    pub fn new(provider: FlashloanProvider) -> Self {
        Self {
            provider,
            max_gas_price: 200.0,      // Max 200 Gwei
            min_profit_threshold: 10.0, // Min $10 profit
        }
    }

    /// Set maximum acceptable gas price
    pub fn with_max_gas_price(mut self, gwei: f64) -> Self {
        self.max_gas_price = gwei;
        self
    }

    /// Set minimum profit threshold
    pub fn with_min_profit(mut self, profit: f64) -> Self {
        self.min_profit_threshold = profit;
        self
    }

    /// Get the provider
    pub fn provider(&self) -> FlashloanProvider {
        self.provider
    }

    /// Get fee rate
    pub fn fee_rate(&self) -> f64 {
        self.provider.fee_rate()
    }

    /// Calculate flashloan fee
    pub fn calculate_fee(&self, loan_amount: f64) -> f64 {
        loan_amount * self.provider.fee_rate()
    }

    /// Check if loan amount is within limits
    pub fn is_within_limits(&self, loan_amount: f64) -> bool {
        loan_amount <= self.provider.max_loan_usd()
    }

    /// Simulate a flashloan execution
    ///
    /// # Arguments
    /// * `loan_amount` - Amount to borrow
    /// * `gross_profit` - Expected gross profit from arbitrage
    /// * `gas_cost` - Estimated gas cost
    pub fn simulate_execution(
        &self,
        loan_amount: f64,
        gross_profit: f64,
        gas_cost: f64,
    ) -> FlashloanResult {
        // Check loan limit
        if !self.is_within_limits(loan_amount) {
            return FlashloanResult {
                provider: self.provider,
                loan_amount,
                fee_paid: 0.0,
                gross_profit: 0.0,
                gas_cost: 0.0,
                net_profit: 0.0,
                success: false,
                error: Some("Loan amount exceeds provider limit".to_string()),
            };
        }

        let fee_paid = self.calculate_fee(loan_amount);
        let net_profit = gross_profit - fee_paid - gas_cost;

        // Check if profitable
        if net_profit < self.min_profit_threshold {
            return FlashloanResult {
                provider: self.provider,
                loan_amount,
                fee_paid,
                gross_profit,
                gas_cost,
                net_profit,
                success: false,
                error: Some(format!(
                    "Net profit ${:.2} below threshold ${:.2}",
                    net_profit, self.min_profit_threshold
                )),
            };
        }

        FlashloanResult {
            provider: self.provider,
            loan_amount,
            fee_paid,
            gross_profit,
            gas_cost,
            net_profit,
            success: true,
            error: None,
        }
    }

    /// Find optimal loan amount for given opportunity
    ///
    /// # Arguments
    /// * `profit_func` - Function that returns profit for a given loan amount
    /// * `max_loan` - Maximum loan amount to consider
    /// * `gas_cost` - Fixed gas cost
    pub fn optimize_loan_amount<F>(&self, profit_func: F, max_loan: f64, gas_cost: f64) -> (f64, f64)
    where
        F: Fn(f64) -> f64,
    {
        let mut best_loan = 0.0;
        let mut best_profit = f64::NEG_INFINITY;

        // Grid search
        let steps = 100;
        for i in 1..=steps {
            let loan = max_loan * (i as f64 / steps as f64);
            let loan = loan.min(self.provider.max_loan_usd());

            let gross_profit = profit_func(loan);
            let fee = self.calculate_fee(loan);
            let net_profit = gross_profit - fee - gas_cost;

            if net_profit > best_profit {
                best_profit = net_profit;
                best_loan = loan;
            }
        }

        (best_loan, best_profit)
    }
}

/// Builder for flashloan transactions
pub struct FlashloanTxBuilder {
    provider: FlashloanProvider,
    loan_token: String,
    loan_amount: f64,
    operations: Vec<FlashloanOperation>,
}

/// An operation within a flashloan transaction
#[derive(Debug, Clone)]
pub enum FlashloanOperation {
    /// Swap on a DEX
    Swap {
        dex: String,
        token_in: String,
        token_out: String,
        amount_in: f64,
        min_amount_out: f64,
    },
    /// Transfer tokens
    Transfer {
        token: String,
        to: String,
        amount: f64,
    },
    /// Repay the flashloan
    Repay {
        token: String,
        amount: f64,
    },
}

impl FlashloanTxBuilder {
    /// Create a new transaction builder
    pub fn new(provider: FlashloanProvider, loan_token: &str, loan_amount: f64) -> Self {
        Self {
            provider,
            loan_token: loan_token.to_string(),
            loan_amount,
            operations: Vec::new(),
        }
    }

    /// Add a swap operation
    pub fn swap(
        mut self,
        dex: &str,
        token_in: &str,
        token_out: &str,
        amount_in: f64,
        min_amount_out: f64,
    ) -> Self {
        self.operations.push(FlashloanOperation::Swap {
            dex: dex.to_string(),
            token_in: token_in.to_string(),
            token_out: token_out.to_string(),
            amount_in,
            min_amount_out,
        });
        self
    }

    /// Add the repay operation
    pub fn repay(mut self) -> Self {
        let repay_amount = self.loan_amount * (1.0 + self.provider.fee_rate());
        self.operations.push(FlashloanOperation::Repay {
            token: self.loan_token.clone(),
            amount: repay_amount,
        });
        self
    }

    /// Build and validate the transaction
    pub fn build(self) -> Result<FlashloanTx, String> {
        // Validate that repay is the last operation
        match self.operations.last() {
            Some(FlashloanOperation::Repay { .. }) => {}
            _ => return Err("Transaction must end with repay operation".to_string()),
        }

        Ok(FlashloanTx {
            provider: self.provider,
            loan_token: self.loan_token,
            loan_amount: self.loan_amount,
            operations: self.operations,
        })
    }
}

/// A complete flashloan transaction
#[derive(Debug, Clone)]
pub struct FlashloanTx {
    pub provider: FlashloanProvider,
    pub loan_token: String,
    pub loan_amount: f64,
    pub operations: Vec<FlashloanOperation>,
}

impl FlashloanTx {
    /// Estimate gas for this transaction
    pub fn estimate_gas(&self) -> u64 {
        let base_gas = 100_000; // Flashloan overhead
        let per_swap_gas = 150_000;

        let swap_count = self
            .operations
            .iter()
            .filter(|op| matches!(op, FlashloanOperation::Swap { .. }))
            .count();

        base_gas + (swap_count as u64 * per_swap_gas)
    }

    /// Get required repay amount
    pub fn repay_amount(&self) -> f64 {
        self.loan_amount * (1.0 + self.provider.fee_rate())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_fees() {
        assert!((FlashloanProvider::Aave.fee_rate() - 0.0009).abs() < 1e-10);
        assert!((FlashloanProvider::DyDx.fee_rate()).abs() < 1e-10);
    }

    #[test]
    fn test_executor_simulation() {
        let executor = FlashloanExecutor::new(FlashloanProvider::Aave);

        // Profitable scenario
        let result = executor.simulate_execution(100_000.0, 200.0, 50.0);
        assert!(result.success);
        assert!(result.net_profit > 0.0);

        // Unprofitable scenario
        let result = executor.simulate_execution(100_000.0, 50.0, 50.0);
        assert!(!result.success);
    }

    #[test]
    fn test_tx_builder() {
        let tx = FlashloanTxBuilder::new(FlashloanProvider::Aave, "USDC", 100_000.0)
            .swap("Uniswap", "USDC", "ETH", 100_000.0, 49.0)
            .swap("SushiSwap", "ETH", "USDC", 49.0, 100_100.0)
            .repay()
            .build()
            .unwrap();

        assert_eq!(tx.operations.len(), 3);
        assert!(tx.estimate_gas() > 0);
        assert!(tx.repay_amount() > tx.loan_amount);
    }

    #[test]
    fn test_tx_builder_validation() {
        let result = FlashloanTxBuilder::new(FlashloanProvider::Aave, "USDC", 100_000.0)
            .swap("Uniswap", "USDC", "ETH", 100_000.0, 49.0)
            .build();

        assert!(result.is_err());
    }
}
