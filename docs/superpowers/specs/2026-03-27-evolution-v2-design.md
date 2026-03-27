# Evolution V2 — Breeding, Aggressive Capital, Self-Deciding Params, Data Persistence

**Date:** 2026-03-27
**Status:** Approved
**Goal:** Transform the evolution engine from simple mutation into a full genetic algorithm with crossover breeding, aggressive capital deployment, self-deciding parameters, and permanent data storage for AI analysis.

---

## 1. Strategy Breeding (Crossover)

### Current: Mutation Only
Clone a winner, tweak one parameter ±10-30%.

### New: Mutation + Crossover + Random Exploration

**Spawn allocation per evolution cycle:**
- 40% Mutations — tweak one param from a top performer
- 40% Crossovers — combine params from two top performers
- 20% Random Explorers — completely random params within sane bounds

### Crossover Mechanics

```
Parent A: CS-momentum-fast (ema_fast=5, ema_slow=13, rsi=50)
Parent B: CS-meanrev-tight (bb_period=10, bb_std=1.5)

Child: CS-hybrid-v1
  Params: union of both parents' params
  ema_fast=5 (from A), ema_slow=13 (from A),
  rsi=50 (from A), bb_period=10 (from B), bb_std=1.5 (from B)
  strategy_type: "hybrid"
```

If both parents share a parameter (e.g., both have `ema_fast`), pick randomly from either parent.

### Random Explorer Bounds

| Parameter | Min | Max |
|-----------|-----|-----|
| ema_fast | 2 | 15 |
| ema_slow | 5 | 50 |
| rsi_threshold | 30 | 70 |
| bb_period | 5 | 40 |
| bb_std | 1.0 | 3.0 |
| spacing_pct | 0.1 | 5.0 |
| grid_count | 3 | 25 |
| leverage | 1 | 20 |
| min_edge | 0.5 | 10.0 |
| spread_bps | 3 | 50 |

---

## 2. Aggressive Capital Deployment

### Problem
Strategies risk 1% ($1) per trade. After 1 hour with 10 trades, PnL ranges from -$0.50 to +$0.50 — too small to differentiate.

### Fix: Deploy 80% of Capital Within First Cycle

Each strategy auto-selects based on its type:

| Strategy Type | Capital Usage | Position Count | Default Leverage |
|--------------|---------------|----------------|-----------------|
| scalp | 90% | 3-5 per asset | 10-20x |
| momentum | 80% | 5-10 | 5-10x |
| dca | 95% | 10-20 (spread) | 1-3x |
| grid | 85% | 10-15 levels | 3-5x |
| meanrev | 70% | 3-5 | 5-10x |
| arb/mispricing | 60% | 2-3 | 10-15x |
| market_making | 80% | 6-10 | 3-5x |
| copy | 75% | matches source | matches source |
| value | 70% | 5-10 | 1x |

### Self-Deciding Parameters

Each strategy's params NOW include evolvable execution params:

```
StrategyParams {
  // Strategy-specific (existing)
  ema_fast: 5,
  ema_slow: 13,
  rsi_threshold: 50,

  // NEW: Self-deciding execution params (also evolvable)
  auto_leverage: 10,         // 1-20, mutatable
  auto_timeframe: 5,         // minutes: 1/5/15/60/240, mutatable
  auto_position_count: 5,    // 1-20, mutatable
  capital_usage_pct: 80,     // 50-95%, mutatable
  stop_loss_atr_mult: 2.0,   // 1.0-4.0, mutatable
  take_profit_rr: 2.0,       // 1.5-5.0, mutatable
}
```

These execution params are ALSO subject to mutation and crossover. A strategy might evolve to discover that 15x leverage on 5m timeframe with tight stops works better than 5x on 1h.

### Evaluation Penalty

Strategies that deployed <50% of their capital within the hour get FITNESS PENALIZED:

```
if capital_deployed_pct < 50% {
    fitness_score *= 0.5  // Halved — timidity is punished in dry mode
}
```

---

## 3. Data Persistence

### Storage Structure

```
data/
├── trades/
│   └── YYYY-MM-DD_trades.jsonl     # Append-only, one JSON per line
├── evolution/
│   └── YYYY-MM-DD_evolution.jsonl  # Kill/spawn/crossover events
├── snapshots/
│   └── YYYY-MM-DD_HHh_snapshot.json # Hourly full system state
└── analysis/
    └── winning_patterns.json        # Auto-generated from data
```

### Trade Record (saved per trade, appended to JSONL)

```json
{
  "timestamp": "2026-03-27T05:14:17Z",
  "strategy": "CS-momentum-fast",
  "generation": 3,
  "parent": "CS-momentum-v2",
  "market": "crypto_spot",
  "symbol": "BTCUSDT",
  "side": "Buy",
  "entry_price": 69110.01,
  "exit_price": null,
  "pnl": null,
  "leverage": 10,
  "timeframe_mins": 5,
  "hold_duration_mins": null,
  "stop_loss": 67850,
  "take_profit": 71500,
  "close_reason": null,
  "capital_usage_pct": 80,
  "indicators_at_entry": {
    "ema_fast": 69050.0,
    "ema_slow": 68900.0,
    "rsi": 62.0,
    "atr": 1500.0,
    "bb_upper": 70200.0,
    "bb_middle": 69100.0,
    "bb_lower": 68000.0
  },
  "strategy_params": {
    "ema_fast": 5,
    "ema_slow": 13,
    "auto_leverage": 10,
    "auto_timeframe": 5,
    "capital_usage_pct": 80
  }
}
```

### Hourly Snapshot (saved every evolution cycle)

```json
{
  "hour": 47,
  "timestamp": "2026-03-27T05:00:00Z",
  "strategies": [
    {
      "name": "CS-momentum-fast",
      "market": "crypto_spot",
      "generation": 1,
      "equity": 108.50,
      "pnl": 8.50,
      "trades": 45,
      "win_rate": 0.68,
      "sharpe": 1.4,
      "fitness": 0.82,
      "params": { ... },
      "positions": [ ... ]
    }
  ],
  "evolution_events": [ ... ],
  "market_prices": {
    "BTCUSDT": 69110.01,
    "ETHUSDT": 2065.0
  },
  "health": {
    "alive": 118,
    "dead": 42,
    "total_capital": 11800
  }
}
```

### Why This Matters

All historical data enables future AI to:
- Find parameter ranges that consistently win across different market conditions
- Detect market regime changes and which strategies thrive in each
- Auto-generate optimized strategy configurations
- Predict strategy failure before it happens
- Build a meta-strategy that allocates capital based on learned patterns

---

## 4. Upgraded Evolution Cycle

### Per-Market Hourly Cycle

```
1. EVALUATE
   - Score by composite fitness (40% Sharpe + 25% PnL + 20% WR + 15% Activity)
   - PENALTY: fitness *= 0.5 if capital_deployed < 50%
   - BONUS: fitness *= 1.2 if strategy is a v2+ crossover that outperforms parents

2. KILL bottom 10% (min 5 survivors per market)
   - Close all positions
   - Save death record + full trade history to data/evolution/
   - Free capital

3. SPAWN replacements:
   - 40% MUTATIONS from top 10%
   - 40% CROSSOVERS from top 2 parents
   - 20% RANDOM EXPLORERS (fresh random params within bounds)

4. SELF-ADJUST surviving strategies:
   - If losses with high leverage → reduce auto_leverage by 20%
   - If no trades in last hour → increase capital_usage_pct by 10%
   - If all trades hit stop loss → widen stop_loss_atr_mult by 0.5
   - If all trades hit take profit → tighten take_profit_rr by 0.3

5. SAVE full snapshot to data/snapshots/

6. LOG health check
```

---

## 5. Implementation

### New/Modified Rust Files

| File | What |
|------|------|
| `core/engine/src/mutation.rs` | Add crossover(), random_explorer(), execution params |
| `core/engine/src/evolution.rs` | Upgraded cycle: crossover spawns, self-adjustment, penalties |
| `core/engine/src/data_logger.rs` | NEW: JSONL append logger for trades/evolution/snapshots |
| `core/api/src/main.rs` | Aggressive capital deployment, higher leverage |

### Build Order

| Phase | What |
|-------|------|
| 1 | Add execution params to StrategyParams + crossover + random explorer |
| 2 | Aggressive capital deployment (80%+ usage, self-deciding leverage/timeframe) |
| 3 | Data persistence (JSONL logging for trades/evolution/snapshots) |
| 4 | Upgraded evolution cycle (crossover spawns, self-adjustment, penalties) |
