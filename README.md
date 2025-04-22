# Chapter 36: Crypto DEX Arbitrage — AMM and Cross-Exchange Strategies

## Overview

Decentralized Exchanges (DEX) используют Automated Market Makers (AMM), создающие уникальные арбитражные возможности. В этой главе мы исследуем ML подходы к обнаружению и эксплуатации арбитража между DEX, с учетом gas costs, slippage и MEV.

## Trading Strategy

**Суть стратегии:** ML для предсказания profitability арбитражных возможностей с учетом:
- Price discrepancies между DEX (Uniswap, Curve, Balancer)
- Gas price dynamics
- Slippage при execution
- MEV (Maximal Extractable Value) competition

**Сигнал на вход:**
- Execute: Predicted profit > gas cost + slippage + risk premium
- Skip: Expected profit negative или слишком рискованно

**Edge:** Лучшее предсказание execution costs vs наивные арбитражеры

## Technical Specification

### Notebooks to Create

| # | Notebook | Description |
|---|----------|-------------|
| 1 | `01_amm_mechanics.ipynb` | Теория AMM: constant product, concentrated liquidity |
| 2 | `02_dex_data_collection.ipynb` | Сбор данных с Uniswap, Curve, Balancer |
| 3 | `03_arbitrage_detection.ipynb` | Обнаружение ценовых расхождений |
| 4 | `04_gas_price_prediction.ipynb` | ML для предсказания gas prices |
| 5 | `05_slippage_modeling.ipynb` | Моделирование slippage по размеру сделки |
| 6 | `06_profitability_prediction.ipynb` | End-to-end profit prediction |
| 7 | `07_mev_analysis.ipynb` | Анализ MEV и frontrunning risks |
| 8 | `08_flashloan_strategies.ipynb` | Использование flashloans для капитала |
| 9 | `09_execution_optimization.ipynb` | Оптимизация routing и timing |
| 10 | `10_backtesting.ipynb` | Backtest на исторических blockchain данных |
| 11 | `11_risk_management.ipynb` | Smart contract risks, impermanent loss |

### AMM Fundamentals

```python
class ConstantProductAMM:
    """
    Uniswap V2 style AMM: x * y = k
    """
    def __init__(self, reserve_x, reserve_y, fee=0.003):
        self.reserve_x = reserve_x
        self.reserve_y = reserve_y
        self.k = reserve_x * reserve_y
        self.fee = fee

    def get_price(self):
        """Current spot price of X in terms of Y"""
        return self.reserve_y / self.reserve_x

    def get_amount_out(self, amount_in, token_in='x'):
        """
        Calculate output amount for given input (with fee)
        """
        amount_in_with_fee = amount_in * (1 - self.fee)

        if token_in == 'x':
            new_reserve_x = self.reserve_x + amount_in_with_fee
            new_reserve_y = self.k / new_reserve_x
            amount_out = self.reserve_y - new_reserve_y
        else:
            new_reserve_y = self.reserve_y + amount_in_with_fee
            new_reserve_x = self.k / new_reserve_y
            amount_out = self.reserve_x - new_reserve_x

        return amount_out

    def get_slippage(self, amount_in, token_in='x'):
        """Calculate price impact / slippage"""
        spot_price = self.get_price()
        amount_out = self.get_amount_out(amount_in, token_in)
        effective_price = amount_in / amount_out
        slippage = (effective_price - spot_price) / spot_price
        return slippage


class ConcentratedLiquidityAMM:
    """
    Uniswap V3 style with concentrated liquidity
    """
    def __init__(self, positions):
        # positions: list of (lower_tick, upper_tick, liquidity)
        self.positions = positions

    def get_amount_out(self, amount_in, price_current):
        # More complex calculation across price ranges
        pass
```

### Arbitrage Detection

```python
def detect_arbitrage(dex_prices, gas_price, trade_size):
    """
    Detect profitable arbitrage between DEXes
    """
    opportunities = []

    for pair in trading_pairs:
        prices = {dex: dex_prices[dex][pair] for dex in dexes}

        # Find best buy and sell venues
        best_buy = min(prices, key=prices.get)
        best_sell = max(prices, key=prices.get)

        price_diff = prices[best_sell] - prices[best_buy]
        price_diff_pct = price_diff / prices[best_buy]

        # Estimate costs
        gas_cost = estimate_gas_cost(gas_price, pair, [best_buy, best_sell])
        slippage = estimate_slippage(trade_size, [best_buy, best_sell])

        # Calculate profit
        gross_profit = trade_size * price_diff_pct
        net_profit = gross_profit - gas_cost - slippage

        if net_profit > 0:
            opportunities.append({
                'pair': pair,
                'buy_dex': best_buy,
                'sell_dex': best_sell,
                'price_diff_pct': price_diff_pct,
                'gross_profit': gross_profit,
                'net_profit': net_profit,
                'gas_cost': gas_cost
            })

    return sorted(opportunities, key=lambda x: x['net_profit'], reverse=True)
```

### Gas Price Prediction

```python
class GasPricePredictor:
    """
    Predict gas prices for optimal execution timing
    """
    def __init__(self):
        self.model = LGBMRegressor()
        self.features = [
            'hour_of_day',
            'day_of_week',
            'pending_tx_count',
            'block_utilization',
            'eth_price',
            'gas_price_ma_1h',
            'gas_price_ma_24h',
            'mempool_size'
        ]

    def predict_next_blocks(self, current_state, n_blocks=10):
        """Predict gas prices for next n blocks"""
        predictions = []
        for i in range(n_blocks):
            x = self._prepare_features(current_state, lookahead=i)
            pred = self.model.predict([x])[0]
            predictions.append(pred)
        return predictions

    def optimal_execution_block(self, predictions, deadline=50):
        """Find best block to execute within deadline"""
        valid_preds = predictions[:deadline]
        return np.argmin(valid_preds)
```

### Slippage Modeling

```python
def model_slippage(dex, pool, trade_size, direction):
    """
    Predict slippage for given trade
    """
    # Get current pool state
    reserves = get_pool_reserves(dex, pool)
    liquidity_depth = calculate_liquidity_depth(reserves)

    # Slippage increases with trade size relative to liquidity
    size_ratio = trade_size / liquidity_depth

    if dex == 'uniswap_v2':
        # Constant product: slippage ≈ size_ratio for small trades
        slippage = size_ratio / (1 + size_ratio)
    elif dex == 'uniswap_v3':
        # Concentrated liquidity: depends on position distribution
        slippage = calculate_v3_slippage(reserves, trade_size)
    elif dex == 'curve':
        # StableSwap: much lower slippage for stablecoins
        slippage = size_ratio * 0.01  # Amplification factor

    return slippage
```

### MEV Considerations

```python
class MEVAnalyzer:
    """
    Analyze and mitigate MEV risks
    """
    def estimate_frontrun_risk(self, opportunity, mempool_data):
        """
        Estimate probability of being frontrun
        """
        # Factors increasing frontrun risk:
        # - Large profit (attracts searchers)
        # - Simple arbitrage (easy to detect)
        # - Low gas price (easy to outbid)

        profit = opportunity['net_profit']
        gas_price = opportunity['gas_price']

        # Check mempool for competing transactions
        competing_txs = self._find_competing_txs(mempool_data, opportunity)

        risk_score = (
            0.3 * min(profit / 1000, 1) +  # Higher profit = higher risk
            0.3 * (1 - gas_price / mempool_data['max_gas']) +  # Lower gas = higher risk
            0.4 * len(competing_txs) / 10  # More competitors = higher risk
        )

        return risk_score

    def private_transaction(self, tx, flashbots_relay):
        """
        Submit transaction via Flashbots to avoid public mempool
        """
        bundle = {
            'txs': [tx],
            'blockNumber': target_block,
            'minTimestamp': 0,
            'maxTimestamp': int(time.time()) + 120
        }
        return flashbots_relay.send_bundle(bundle)
```

### Flashloan Integration

```python
class FlashloanArbitrage:
    """
    Execute arbitrage using flashloans (no capital required)
    """
    def build_flashloan_tx(self, opportunity, loan_amount):
        """
        Build transaction for flashloan arbitrage
        """
        # 1. Borrow from Aave/dYdX
        borrow_call = aave.flashLoan(loan_amount, self.address)

        # 2. Buy on cheaper DEX
        buy_call = opportunity['buy_dex'].swap(
            loan_amount,
            opportunity['pair'],
            min_out=opportunity['expected_out'] * 0.99  # 1% slippage tolerance
        )

        # 3. Sell on expensive DEX
        sell_call = opportunity['sell_dex'].swap(
            opportunity['expected_out'],
            opportunity['pair'],
            direction='reverse'
        )

        # 4. Repay flashloan + fee
        repay_call = aave.repay(loan_amount * 1.0009)  # 0.09% fee

        # 5. Profit remains in contract
        return [borrow_call, buy_call, sell_call, repay_call]
```

### Key Metrics

- **Opportunity Detection:** # opportunities/day, Avg profit per opp
- **Execution:** Success rate, Actual vs predicted profit
- **Costs:** Avg gas cost, Slippage accuracy
- **Risk:** Frontrun rate, Failed transactions

### Dependencies

```python
web3>=6.0.0
eth-abi>=4.0.0
pandas>=1.5.0
numpy>=1.23.0
lightgbm>=4.0.0
requests>=2.28.0
```

## Expected Outcomes

1. **AMM simulator** для Uniswap V2/V3, Curve
2. **Arbitrage detection engine** с real-time pricing
3. **Gas price predictor** для optimal timing
4. **Slippage model** для accurate cost estimation
5. **MEV-aware execution** с Flashbots integration
6. **Backtest results** на исторических blockchain данных

## References

- [Uniswap V2 Whitepaper](https://uniswap.org/whitepaper.pdf)
- [Uniswap V3 Whitepaper](https://uniswap.org/whitepaper-v3.pdf)
- [Flashbots Documentation](https://docs.flashbots.net/)
- [MEV Research](https://research.paradigm.xyz/MEV)
- [Curve StableSwap Whitepaper](https://curve.fi/files/stableswap-paper.pdf)

## Difficulty Level

⭐⭐⭐⭐⭐ (Expert)

Требуется понимание: DeFi protocols, Smart contracts, Blockchain, MEV, Gas optimization

## Disclaimers

- Crypto arbitrage is highly competitive
- MEV extraction requires sophisticated infrastructure
- Smart contract risks are significant
- Regulatory status varies by jurisdiction
- This chapter is educational, not investment advice
