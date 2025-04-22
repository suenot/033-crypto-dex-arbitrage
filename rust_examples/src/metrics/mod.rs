//! Performance metrics and analysis
//!
//! This module provides tools for measuring and analyzing arbitrage performance.

use serde::{Deserialize, Serialize};

/// Metrics for arbitrage performance
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArbitrageMetrics {
    /// Total opportunities found
    pub total_opportunities: u64,
    /// Opportunities executed
    pub executed: u64,
    /// Successful executions
    pub successful: u64,
    /// Failed executions
    pub failed: u64,
    /// Total gross profit
    pub total_gross_profit: f64,
    /// Total net profit
    pub total_net_profit: f64,
    /// Total gas spent
    pub total_gas_cost: f64,
    /// Total flashloan fees
    pub total_flashloan_fees: f64,
    /// Total slippage cost
    pub total_slippage_cost: f64,
    /// Profits by time bucket (hour of day)
    pub hourly_profits: [f64; 24],
    /// Execution times in milliseconds
    pub execution_times: Vec<u64>,
}

impl ArbitrageMetrics {
    /// Create new metrics tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an opportunity found
    pub fn record_opportunity(&mut self) {
        self.total_opportunities += 1;
    }

    /// Record a successful execution
    pub fn record_success(
        &mut self,
        gross_profit: f64,
        net_profit: f64,
        gas_cost: f64,
        flashloan_fee: f64,
        slippage: f64,
        execution_time_ms: u64,
        hour: usize,
    ) {
        self.executed += 1;
        self.successful += 1;
        self.total_gross_profit += gross_profit;
        self.total_net_profit += net_profit;
        self.total_gas_cost += gas_cost;
        self.total_flashloan_fees += flashloan_fee;
        self.total_slippage_cost += slippage;
        self.hourly_profits[hour % 24] += net_profit;
        self.execution_times.push(execution_time_ms);
    }

    /// Record a failed execution
    pub fn record_failure(&mut self, gas_cost: f64) {
        self.executed += 1;
        self.failed += 1;
        self.total_gas_cost += gas_cost;
        self.total_net_profit -= gas_cost;
    }

    /// Success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.executed == 0 {
            return 0.0;
        }
        self.successful as f64 / self.executed as f64
    }

    /// Average profit per opportunity
    pub fn avg_profit_per_opportunity(&self) -> f64 {
        if self.total_opportunities == 0 {
            return 0.0;
        }
        self.total_net_profit / self.total_opportunities as f64
    }

    /// Average profit per successful execution
    pub fn avg_profit_per_success(&self) -> f64 {
        if self.successful == 0 {
            return 0.0;
        }
        self.total_net_profit / self.successful as f64
    }

    /// Average execution time in milliseconds
    pub fn avg_execution_time(&self) -> f64 {
        if self.execution_times.is_empty() {
            return 0.0;
        }
        self.execution_times.iter().sum::<u64>() as f64 / self.execution_times.len() as f64
    }

    /// Find the most profitable hour
    pub fn best_hour(&self) -> (usize, f64) {
        self.hourly_profits
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(h, p)| (h, *p))
            .unwrap_or((0, 0.0))
    }

    /// Cost efficiency (net profit / total costs)
    pub fn cost_efficiency(&self) -> f64 {
        let total_costs = self.total_gas_cost + self.total_flashloan_fees + self.total_slippage_cost;
        if total_costs == 0.0 {
            return 0.0;
        }
        self.total_net_profit / total_costs
    }
}

/// Performance report for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Report period start
    pub period_start: String,
    /// Report period end
    pub period_end: String,
    /// Aggregated metrics
    pub metrics: ArbitrageMetrics,
    /// Key performance indicators
    pub kpis: ReportKPIs,
}

/// Key performance indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportKPIs {
    /// Total return percentage
    pub total_return_pct: f64,
    /// Win rate
    pub win_rate: f64,
    /// Average win size
    pub avg_win: f64,
    /// Average loss size
    pub avg_loss: f64,
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    /// Maximum drawdown
    pub max_drawdown: f64,
    /// Sharpe ratio (annualized)
    pub sharpe_ratio: f64,
}

impl PerformanceReport {
    /// Generate a performance report
    pub fn generate(metrics: ArbitrageMetrics, period_start: &str, period_end: &str) -> Self {
        let kpis = ReportKPIs {
            total_return_pct: if metrics.total_gross_profit > 0.0 {
                (metrics.total_net_profit / metrics.total_gross_profit) * 100.0
            } else {
                0.0
            },
            win_rate: metrics.success_rate() * 100.0,
            avg_win: metrics.avg_profit_per_success(),
            avg_loss: if metrics.failed > 0 {
                metrics.total_gas_cost / metrics.failed as f64
            } else {
                0.0
            },
            profit_factor: if metrics.total_gas_cost > 0.0 {
                metrics.total_gross_profit / metrics.total_gas_cost
            } else {
                0.0
            },
            max_drawdown: 0.0,  // Would need trade history to calculate
            sharpe_ratio: 0.0, // Would need daily returns to calculate
        };

        Self {
            period_start: period_start.to_string(),
            period_end: period_end.to_string(),
            metrics,
            kpis,
        }
    }

    /// Print a summary to console
    pub fn print_summary(&self) {
        println!("\n╔════════════════════════════════════════════╗");
        println!("║       ARBITRAGE PERFORMANCE REPORT         ║");
        println!("╠════════════════════════════════════════════╣");
        println!("║ Period: {} to {}", self.period_start, self.period_end);
        println!("╠════════════════════════════════════════════╣");
        println!("║ ACTIVITY                                   ║");
        println!(
            "║   Opportunities Found:   {:>18} ║",
            self.metrics.total_opportunities
        );
        println!("║   Executed:              {:>18} ║", self.metrics.executed);
        println!("║   Successful:            {:>18} ║", self.metrics.successful);
        println!("║   Failed:                {:>18} ║", self.metrics.failed);
        println!("╠════════════════════════════════════════════╣");
        println!("║ FINANCIAL                                  ║");
        println!(
            "║   Gross Profit:         ${:>17.2} ║",
            self.metrics.total_gross_profit
        );
        println!(
            "║   Net Profit:           ${:>17.2} ║",
            self.metrics.total_net_profit
        );
        println!(
            "║   Gas Cost:             ${:>17.2} ║",
            self.metrics.total_gas_cost
        );
        println!(
            "║   Flashloan Fees:       ${:>17.2} ║",
            self.metrics.total_flashloan_fees
        );
        println!(
            "║   Slippage Cost:        ${:>17.2} ║",
            self.metrics.total_slippage_cost
        );
        println!("╠════════════════════════════════════════════╣");
        println!("║ KPIs                                       ║");
        println!(
            "║   Win Rate:              {:>17.1}% ║",
            self.kpis.win_rate
        );
        println!(
            "║   Avg Win:              ${:>17.2} ║",
            self.kpis.avg_win
        );
        println!(
            "║   Avg Loss:             ${:>17.2} ║",
            self.kpis.avg_loss
        );
        println!(
            "║   Profit Factor:         {:>17.2}x ║",
            self.kpis.profit_factor
        );
        println!("╚════════════════════════════════════════════╝\n");
    }
}

/// Track cumulative PnL over time
#[derive(Debug, Clone)]
pub struct PnLTracker {
    /// Trade results (timestamp, pnl)
    trades: Vec<(i64, f64)>,
    /// Cumulative PnL
    cumulative: f64,
    /// High water mark
    hwm: f64,
    /// Maximum drawdown
    max_drawdown: f64,
}

impl Default for PnLTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl PnLTracker {
    /// Create a new tracker
    pub fn new() -> Self {
        Self {
            trades: Vec::new(),
            cumulative: 0.0,
            hwm: 0.0,
            max_drawdown: 0.0,
        }
    }

    /// Record a trade
    pub fn record_trade(&mut self, timestamp: i64, pnl: f64) {
        self.cumulative += pnl;
        self.trades.push((timestamp, pnl));

        // Update high water mark and drawdown
        if self.cumulative > self.hwm {
            self.hwm = self.cumulative;
        } else {
            let drawdown = (self.hwm - self.cumulative) / self.hwm.max(1.0);
            if drawdown > self.max_drawdown {
                self.max_drawdown = drawdown;
            }
        }
    }

    /// Get current cumulative PnL
    pub fn cumulative_pnl(&self) -> f64 {
        self.cumulative
    }

    /// Get maximum drawdown
    pub fn max_drawdown(&self) -> f64 {
        self.max_drawdown
    }

    /// Get number of trades
    pub fn trade_count(&self) -> usize {
        self.trades.len()
    }

    /// Get winning trades count
    pub fn winning_trades(&self) -> usize {
        self.trades.iter().filter(|(_, pnl)| *pnl > 0.0).count()
    }

    /// Calculate Sharpe ratio (simplified, assuming zero risk-free rate)
    pub fn sharpe_ratio(&self) -> f64 {
        if self.trades.len() < 2 {
            return 0.0;
        }

        let pnls: Vec<f64> = self.trades.iter().map(|(_, p)| *p).collect();
        let mean = pnls.iter().sum::<f64>() / pnls.len() as f64;
        let variance =
            pnls.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / (pnls.len() - 1) as f64;
        let std_dev = variance.sqrt();

        if std_dev == 0.0 {
            return 0.0;
        }

        mean / std_dev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics() {
        let mut metrics = ArbitrageMetrics::new();

        metrics.record_opportunity();
        metrics.record_success(100.0, 80.0, 10.0, 5.0, 5.0, 100, 14);

        assert_eq!(metrics.total_opportunities, 1);
        assert_eq!(metrics.successful, 1);
        assert!((metrics.success_rate() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_pnl_tracker() {
        let mut tracker = PnLTracker::new();

        tracker.record_trade(1, 100.0);
        tracker.record_trade(2, 50.0);
        tracker.record_trade(3, -30.0);

        assert!((tracker.cumulative_pnl() - 120.0).abs() < 1e-10);
        assert_eq!(tracker.trade_count(), 3);
        assert_eq!(tracker.winning_trades(), 2);
    }

    #[test]
    fn test_max_drawdown() {
        let mut tracker = PnLTracker::new();

        tracker.record_trade(1, 100.0);
        tracker.record_trade(2, 50.0);   // HWM = 150
        tracker.record_trade(3, -50.0);  // Down to 100
        tracker.record_trade(4, -50.0);  // Down to 50

        // Drawdown = (150 - 50) / 150 = 0.667
        assert!(tracker.max_drawdown() > 0.65 && tracker.max_drawdown() < 0.68);
    }
}
