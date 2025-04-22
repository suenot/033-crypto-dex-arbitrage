# Глава 36: Крипто-арбитраж на DEX — AMM и кросс-биржевые стратегии

## Обзор

Децентрализованные биржи (DEX) используют автоматические маркетмейкеры (AMM), которые создают уникальные арбитражные возможности. В этой главе мы исследуем подходы машинного обучения к обнаружению и использованию арбитража между DEX, учитывая затраты на газ, проскальзывание и MEV (максимально извлекаемую ценность).

## Торговая стратегия

**Суть стратегии:** Машинное обучение для прогнозирования прибыльности арбитражных возможностей с учётом:
- Ценовых расхождений между DEX (Uniswap, Curve, Balancer)
- Динамики цен на газ
- Проскальзывания при исполнении
- Конкуренции за MEV (Maximal Extractable Value)

**Сигнал на вход:**
- Исполнить: Прогнозируемая прибыль > затраты на газ + проскальзывание + премия за риск
- Пропустить: Ожидаемая прибыль отрицательная или слишком рискованно

**Преимущество:** Более точное прогнозирование затрат на исполнение по сравнению с наивными арбитражёрами

## Техническая спецификация

### Ноутбуки для создания

| # | Ноутбук | Описание |
|---|---------|----------|
| 1 | `01_amm_mechanics.ipynb` | Теория AMM: постоянное произведение, концентрированная ликвидность |
| 2 | `02_dex_data_collection.ipynb` | Сбор данных с Uniswap, Curve, Balancer |
| 3 | `03_arbitrage_detection.ipynb` | Обнаружение ценовых расхождений |
| 4 | `04_gas_price_prediction.ipynb` | ML для прогнозирования цен на газ |
| 5 | `05_slippage_modeling.ipynb` | Моделирование проскальзывания в зависимости от размера сделки |
| 6 | `06_profitability_prediction.ipynb` | Сквозное прогнозирование прибыли |
| 7 | `07_mev_analysis.ipynb` | Анализ MEV и рисков фронтраннинга |
| 8 | `08_flashloan_strategies.ipynb` | Использование флэш-займов для капитала |
| 9 | `09_execution_optimization.ipynb` | Оптимизация маршрутизации и тайминга |
| 10 | `10_backtesting.ipynb` | Бэктест на исторических данных блокчейна |
| 11 | `11_risk_management.ipynb` | Риски смарт-контрактов, непостоянные потери |

## Основы AMM

### Что такое AMM?

**Автоматический маркетмейкер (AMM)** — это смарт-контракт, который создаёт ликвидность для торговли токенами без необходимости в традиционной книге ордеров. Вместо сопоставления покупателей и продавцов AMM использует математические формулы для определения цен.

### Формула постоянного произведения (Uniswap V2)

Самая популярная формула AMM:

```
x × y = k
```

Где:
- `x` — резерв токена A в пуле
- `y` — резерв токена B в пуле
- `k` — константа (неизменная после каждой сделки)

**Пример:**
- Пул содержит 1000 ETH и 2,000,000 USDC
- k = 1000 × 2,000,000 = 2,000,000,000
- Цена 1 ETH = 2,000,000 / 1000 = 2000 USDC

```python
class ConstantProductAMM:
    """
    AMM в стиле Uniswap V2: x * y = k
    """
    def __init__(self, reserve_x, reserve_y, fee=0.003):
        self.reserve_x = reserve_x
        self.reserve_y = reserve_y
        self.k = reserve_x * reserve_y
        self.fee = fee

    def get_price(self):
        """Текущая спотовая цена X в единицах Y"""
        return self.reserve_y / self.reserve_x

    def get_amount_out(self, amount_in, token_in='x'):
        """
        Рассчитать выходное количество для заданного входа (с учётом комиссии)
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
        """Рассчитать ценовое воздействие / проскальзывание"""
        spot_price = self.get_price()
        amount_out = self.get_amount_out(amount_in, token_in)
        effective_price = amount_in / amount_out
        slippage = (effective_price - spot_price) / spot_price
        return slippage
```

### Концентрированная ликвидность (Uniswap V3)

В Uniswap V3 провайдеры ликвидности могут сосредоточить свои средства в определённом ценовом диапазоне:

```python
class ConcentratedLiquidityAMM:
    """
    Стиль Uniswap V3 с концентрированной ликвидностью
    """
    def __init__(self, positions):
        # positions: список (нижний_тик, верхний_тик, ликвидность)
        self.positions = positions

    def get_liquidity_at_price(self, price):
        """Получить доступную ликвидность по заданной цене"""
        tick = self.price_to_tick(price)
        return sum(
            pos['liquidity']
            for pos in self.positions
            if pos['lower_tick'] <= tick <= pos['upper_tick']
        )

    def get_amount_out(self, amount_in, price_current):
        # Более сложный расчёт по ценовым диапазонам
        pass
```

### StableSwap (Curve)

Curve использует специальную формулу для стейблкоинов с минимальным проскальзыванием:

```
A × n^n × sum(x_i) + D = A × D × n^n + D^(n+1) / (n^n × prod(x_i))
```

Где:
- `A` — коэффициент усиления (чем выше, тем ближе к постоянной сумме)
- `D` — инвариант
- `n` — количество токенов в пуле

## Обнаружение арбитража

### Основной алгоритм

```python
def detect_arbitrage(dex_prices, gas_price, trade_size):
    """
    Обнаружение прибыльного арбитража между DEX
    """
    opportunities = []

    for pair in trading_pairs:
        prices = {dex: dex_prices[dex][pair] for dex in dexes}

        # Найти лучшие площадки для покупки и продажи
        best_buy = min(prices, key=prices.get)
        best_sell = max(prices, key=prices.get)

        price_diff = prices[best_sell] - prices[best_buy]
        price_diff_pct = price_diff / prices[best_buy]

        # Оценить затраты
        gas_cost = estimate_gas_cost(gas_price, pair, [best_buy, best_sell])
        slippage = estimate_slippage(trade_size, [best_buy, best_sell])

        # Рассчитать прибыль
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

### Треугольный арбитраж

```python
def triangular_arbitrage(dex, pairs):
    """
    Поиск треугольного арбитража на одной DEX
    Например: ETH → USDC → DAI → ETH
    """
    triangles = find_triangular_paths(pairs)

    opportunities = []
    for path in triangles:
        amount = 1.0  # Начинаем с 1 единицы

        for i, hop in enumerate(path):
            pool = dex.get_pool(hop['pair'])
            amount = pool.get_amount_out(amount, hop['direction'])

        profit_pct = (amount - 1.0) * 100

        if profit_pct > 0:
            opportunities.append({
                'path': path,
                'profit_pct': profit_pct,
                'start_token': path[0]['token_in']
            })

    return opportunities
```

## Прогнозирование цены газа

### ML-модель для газа

```python
class GasPricePredictor:
    """
    Прогнозирование цен на газ для оптимального тайминга исполнения
    """
    def __init__(self):
        self.model = LGBMRegressor()
        self.features = [
            'hour_of_day',        # Час дня
            'day_of_week',        # День недели
            'pending_tx_count',   # Количество ожидающих транзакций
            'block_utilization',  # Использование блока
            'eth_price',          # Цена ETH
            'gas_price_ma_1h',    # Скользящее среднее газа за 1 час
            'gas_price_ma_24h',   # Скользящее среднее газа за 24 часа
            'mempool_size'        # Размер мемпула
        ]

    def predict_next_blocks(self, current_state, n_blocks=10):
        """Прогноз цен на газ для следующих n блоков"""
        predictions = []
        for i in range(n_blocks):
            x = self._prepare_features(current_state, lookahead=i)
            pred = self.model.predict([x])[0]
            predictions.append(pred)
        return predictions

    def optimal_execution_block(self, predictions, deadline=50):
        """Найти лучший блок для исполнения в пределах дедлайна"""
        valid_preds = predictions[:deadline]
        return np.argmin(valid_preds)
```

## Моделирование проскальзывания

### Зависимость от размера сделки

```python
def model_slippage(dex, pool, trade_size, direction):
    """
    Прогнозирование проскальзывания для заданной сделки
    """
    # Получить текущее состояние пула
    reserves = get_pool_reserves(dex, pool)
    liquidity_depth = calculate_liquidity_depth(reserves)

    # Проскальзывание растёт с размером сделки относительно ликвидности
    size_ratio = trade_size / liquidity_depth

    if dex == 'uniswap_v2':
        # Постоянное произведение: slippage ≈ size_ratio для малых сделок
        slippage = size_ratio / (1 + size_ratio)
    elif dex == 'uniswap_v3':
        # Концентрированная ликвидность: зависит от распределения позиций
        slippage = calculate_v3_slippage(reserves, trade_size)
    elif dex == 'curve':
        # StableSwap: значительно меньшее проскальзывание для стейблкоинов
        slippage = size_ratio * 0.01  # Коэффициент усиления

    return slippage
```

## MEV и фронтраннинг

### Что такое MEV?

**MEV (Maximal Extractable Value)** — это прибыль, которую майнеры/валидаторы могут извлечь путём манипулирования порядком транзакций в блоке.

Виды MEV-атак:
- **Фронтраннинг** — размещение транзакции перед жертвой
- **Бэкраннинг** — размещение транзакции после жертвы
- **Сэндвич-атака** — комбинация фронтраннинга и бэкраннинга

```python
class MEVAnalyzer:
    """
    Анализ и снижение рисков MEV
    """
    def estimate_frontrun_risk(self, opportunity, mempool_data):
        """
        Оценка вероятности фронтраннинга
        """
        # Факторы, увеличивающие риск фронтраннинга:
        # - Высокая прибыль (привлекает искателей)
        # - Простой арбитраж (легко обнаружить)
        # - Низкая цена газа (легко перебить)

        profit = opportunity['net_profit']
        gas_price = opportunity['gas_price']

        # Проверить мемпул на конкурирующие транзакции
        competing_txs = self._find_competing_txs(mempool_data, opportunity)

        risk_score = (
            0.3 * min(profit / 1000, 1) +  # Выше прибыль = выше риск
            0.3 * (1 - gas_price / mempool_data['max_gas']) +  # Ниже газ = выше риск
            0.4 * len(competing_txs) / 10  # Больше конкурентов = выше риск
        )

        return risk_score

    def private_transaction(self, tx, flashbots_relay):
        """
        Отправка транзакции через Flashbots для обхода публичного мемпула
        """
        bundle = {
            'txs': [tx],
            'blockNumber': target_block,
            'minTimestamp': 0,
            'maxTimestamp': int(time.time()) + 120
        }
        return flashbots_relay.send_bundle(bundle)
```

## Флэш-займы (Flashloans)

### Что такое флэш-займ?

**Флэш-займ** — это займ без залога, который должен быть возвращён в пределах одной транзакции. Если займ не возвращён, вся транзакция откатывается.

### Преимущества для арбитража:
- Не требуется начальный капитал
- Нет риска ликвидации
- Масштабируемость до миллионов долларов

```python
class FlashloanArbitrage:
    """
    Исполнение арбитража с использованием флэш-займов
    """
    def build_flashloan_tx(self, opportunity, loan_amount):
        """
        Построение транзакции для флэш-займ арбитража
        """
        # 1. Взять займ у Aave/dYdX
        borrow_call = aave.flashLoan(loan_amount, self.address)

        # 2. Купить на дешёвой DEX
        buy_call = opportunity['buy_dex'].swap(
            loan_amount,
            opportunity['pair'],
            min_out=opportunity['expected_out'] * 0.99  # 1% допуск на проскальзывание
        )

        # 3. Продать на дорогой DEX
        sell_call = opportunity['sell_dex'].swap(
            opportunity['expected_out'],
            opportunity['pair'],
            direction='reverse'
        )

        # 4. Вернуть флэш-займ + комиссию
        repay_call = aave.repay(loan_amount * 1.0009)  # 0.09% комиссия

        # 5. Прибыль остаётся в контракте
        return [borrow_call, buy_call, sell_call, repay_call]

    def execute_with_flashbots(self, tx_bundle, max_priority_fee):
        """
        Исполнение через Flashbots для защиты от MEV
        """
        signed_bundle = self.sign_bundle(tx_bundle)
        return flashbots.send_bundle(
            signed_bundle,
            target_block=self.get_next_block() + 1,
            max_priority_fee=max_priority_fee
        )
```

### Пример полного цикла

```python
def execute_flashloan_arbitrage():
    """Полный цикл флэш-займ арбитража"""

    # 1. Обнаружить возможность
    opportunity = detect_best_opportunity()
    if not opportunity:
        return None

    # 2. Рассчитать оптимальный размер
    optimal_size = calculate_optimal_size(
        opportunity,
        max_slippage=0.005  # 0.5%
    )

    # 3. Оценить затраты
    gas_estimate = estimate_gas(opportunity)
    expected_profit = opportunity['profit_pct'] * optimal_size
    net_profit = expected_profit - gas_estimate - flashloan_fee(optimal_size)

    # 4. Проверить прибыльность
    if net_profit < MIN_PROFIT_THRESHOLD:
        return None

    # 5. Проверить риски MEV
    mev_risk = analyze_mev_risk(opportunity)
    if mev_risk > MAX_MEV_RISK:
        return None

    # 6. Построить и исполнить транзакцию
    tx = build_flashloan_tx(opportunity, optimal_size)
    result = submit_via_flashbots(tx)

    return result
```

## Ключевые метрики

| Метрика | Описание |
|---------|----------|
| **Обнаружение возможностей** | Количество возможностей в день, средняя прибыль |
| **Исполнение** | Успешность, фактическая vs прогнозируемая прибыль |
| **Затраты** | Средние затраты на газ, точность прогноза проскальзывания |
| **Риски** | Частота фронтраннинга, неуспешные транзакции |

## Зависимости

```python
web3>=6.0.0          # Взаимодействие с блокчейном
eth-abi>=4.0.0       # Кодирование/декодирование ABI
pandas>=1.5.0        # Анализ данных
numpy>=1.23.0        # Численные вычисления
lightgbm>=4.0.0      # ML для прогнозирования
requests>=2.28.0     # HTTP запросы
```

## Ожидаемые результаты

1. **Симулятор AMM** для Uniswap V2/V3, Curve
2. **Движок обнаружения арбитража** с ценами в реальном времени
3. **Предиктор цены газа** для оптимального тайминга
4. **Модель проскальзывания** для точной оценки затрат
5. **MEV-aware исполнение** с интеграцией Flashbots
6. **Результаты бэктеста** на исторических данных блокчейна

## Риски и предупреждения

### Технические риски
- **Риск смарт-контрактов** — баги в коде могут привести к потере средств
- **Риск ликвидности** — низкая ликвидность увеличивает проскальзывание
- **Риск сети** — задержки и сбои могут сорвать арбитраж

### Экономические риски
- **Высокая конкуренция** — профессиональные арбитражёры с лучшей инфраструктурой
- **Волатильность газа** — непредсказуемые всплески цен на газ
- **Impermanent loss** — для LP стратегий

### Регуляторные риски
- Статус криптовалют различается по юрисдикциям
- DeFi находится в правовой серой зоне

## Литература

- [Uniswap V2 Whitepaper](https://uniswap.org/whitepaper.pdf)
- [Uniswap V3 Whitepaper](https://uniswap.org/whitepaper-v3.pdf)
- [Flashbots Documentation](https://docs.flashbots.net/)
- [MEV Research — Paradigm](https://research.paradigm.xyz/MEV)
- [Curve StableSwap Whitepaper](https://curve.fi/files/stableswap-paper.pdf)
- [Flash Boys 2.0: Frontrunning, Transaction Reordering](https://arxiv.org/abs/1904.05234)

## Уровень сложности

⭐⭐⭐⭐⭐ (Эксперт)

Требуется понимание: DeFi протоколов, смарт-контрактов, блокчейна, MEV, оптимизации газа

## Дисклеймер

- Крипто-арбитраж высококонкурентен
- Извлечение MEV требует сложной инфраструктуры
- Риски смарт-контрактов значительны
- Регуляторный статус различается по юрисдикциям
- Эта глава носит образовательный характер, а не является инвестиционным советом
