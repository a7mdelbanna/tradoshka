# Strategy Evolution Engine — A/B Testing with Darwinian Selection

**Date:** 2026-03-26
**Status:** Approved
**Goal:** Each strategy variant gets its own $100 wallet. Every hour, the system ranks all strategies, kills the bottom 10%, and spawns mutated variants from the top 10%. A dashboard shows the leaderboard, evolution timeline, and graveyard.

---

## 1. Strategy Wallet Architecture

Every strategy variant gets an isolated wallet with its own balance, positions, P&L, and trade history. All wallets trade the same market data simultaneously.

### Wallet Naming Convention

`{market_prefix}-{strategy}-{variant}`

| Market | Prefix | Example |
|--------|--------|---------|
| Polymarket | PM | PM-momentum-fast |
| Crypto Spot | CS | CS-dca-conservative |
| Crypto Perps | CP | CP-scalp-5m-v2 |

### Starting Set (~20 variants)

**Polymarket (6):**
- PM-momentum-fast (EMA 5/13)
- PM-momentum-slow (EMA 9/21)
- PM-value-aggressive (edge > 3%)
- PM-value-conservative (edge > 8%)
- PM-copy-top-pnl (top 10 traders by PnL)
- PM-arb-mispricing (yes+no deviation)

**Crypto Spot (8):**
- CS-momentum-fast (EMA 5/13)
- CS-momentum-slow (EMA 9/21)
- CS-dca-aggressive (buy every 6 ticks, RSI < 40)
- CS-dca-conservative (buy every 24 ticks, RSI < 30)
- CS-grid-tight (0.5% spacing)
- CS-grid-wide (2% spacing)
- CS-meanrev-bollinger (BB 20, 2.0 std)
- CS-copy-whales (follow >$10K trades)

**Crypto Perps (6):**
- CP-scalp-5m (5m EMA cross, 10x)
- CP-scalp-15m (15m momentum, 10x)
- CP-scalp-1h (1h trend follow, 5x)
- CP-funding-arb (long spot + short perp on high funding)
- CP-grid-perp (leveraged grid, 5x)
- CP-meanrev-perp (BB bounce on 15m, 10x)

Each starts with $100. Total initial capital: $2,000 across 20 wallets.

---

## 2. Hourly Evolution Engine

Every 60 minutes, the system runs a Darwinian evolution cycle.

### Evolution Cycle Steps

1. **RANK** all alive strategy wallets by Sharpe ratio (requires >= 5 trades to be ranked, otherwise immune from killing)

2. **KILL bottom 10%** of ranked strategies:
   - Close all open positions at market price
   - Record death with stats: strategy name, lifetime hours, total trades, final PnL, win rate, Sharpe, cause of death
   - Move to graveyard (kept permanently for history)
   - Free the remaining capital (returned to capital pool)

3. **SPAWN from top 10%**:
   - Select best performing strategy
   - Clone it with ONE parameter mutated ±10-30%
   - Name: append `-vN` (v2, v3, etc.)
   - Give it $100 from freed capital pool
   - Record birth: parent strategy, which parameter mutated, old → new value

4. **LOG evolution report**:
   - Hour number
   - Strategies killed (with reasons)
   - Strategies spawned (with parents)
   - Current leaderboard top 5
   - Overall stats (alive, dead, avg Sharpe)

5. **PUSH to dashboard** via WebSocket evolution event

### Mutation Rules

Only ONE parameter changes per spawn. Parameter change: random ±10-30%.

| Strategy Type | Mutable Parameters |
|--------------|-------------------|
| Momentum | EMA fast period, EMA slow period, RSI threshold |
| DCA | Buy interval, RSI oversold level |
| Grid | Grid spacing %, grid count, center offset |
| Mean Reversion | BB period, BB std dev multiplier, RSI oversold/overbought |
| Copy Trading | Min trade size filter, min win rate filter, max traders to follow |
| Arbitrage | Min edge threshold, min volume filter |
| Market Making | Spread bps, max inventory |
| Scalp (Perps) | EMA periods, leverage, timeframe |
| Funding Arb | Min funding rate threshold |

### Protection Rules

- Strategies with < 5 trades are immune from killing (too early to judge)
- Maximum 30 alive strategies at any time (prevent resource explosion)
- Minimum 10 alive strategies (never kill below this floor)
- A strategy can only be cloned once per evolution cycle
- Dead strategies are never resurrected (but their parameter insights inform new spawns)

---

## 3. Evolution Dashboard

### New Route: `/evolution`

Accessible from the nav bar alongside Performance and Trading.

### Components

**Leaderboard Table:**
- Sortable by: Sharpe ratio, total PnL, win rate, trade count, age (hours)
- Each row shows: rank, name, market, Sharpe, PnL ($), PnL (%), win rate, trades, tier badge, age
- Color-coded: top 10% green glow, bottom 10% red glow, rest neutral
- Click row → expand to show strategy parameters and recent trades

**Evolution Timeline:**
- Scrollable chronological feed of evolution events
- Each entry: timestamp, action (KILLED/SPAWNED/PROMOTED/DEMOTED), strategy name, reason
- Color-coded: red for kills, green for spawns, yellow for tier changes

**Graveyard:**
- Table of all dead strategies
- Columns: name, market, lifetime (hours), trades, final PnL, win rate, Sharpe, cause of death
- Sortable — learn from what doesn't work

**Stats Summary:**
- Total wallets alive / dead
- Total capital deployed / freed
- Average Sharpe of alive strategies
- Best performing generation (v1, v2, v3...)
- Hours since system started

---

## 4. Rust Implementation

### New Files

| File | What |
|------|------|
| `core/engine/src/strategy_wallet.rs` | Strategy wallet manager — creates/tracks/kills wallets per strategy |
| `core/engine/src/evolution.rs` | Hourly evolution engine — rank, kill, spawn, mutate |
| `core/engine/src/mutation.rs` | Parameter mutation logic per strategy type |
| `core/api/src/evolution_routes.rs` | API endpoints for leaderboard, timeline, graveyard |

### Modified Files

| File | Change |
|------|--------|
| `core/api/src/state.rs` | Add strategy wallet manager + evolution engine to AppState |
| `core/api/src/main.rs` | Add hourly evolution hook to trading loop |
| `core/api/src/server.rs` | Register evolution routes |

### API Endpoints

| Endpoint | Method | What |
|----------|--------|------|
| `/api/evolution/leaderboard` | GET | All alive strategies ranked by Sharpe |
| `/api/evolution/timeline` | GET | Evolution events (kills, spawns, promotions) |
| `/api/evolution/graveyard` | GET | All dead strategies with stats |
| `/api/evolution/stats` | GET | Summary stats (alive/dead, capital, avg Sharpe) |
| `/api/evolution/trigger` | POST | Manually trigger an evolution cycle |
| `/api/evolution/wallet/:name` | GET | Specific strategy wallet state |

### Dashboard Pages

| Page | Route | What |
|------|-------|------|
| Evolution | `/evolution` | Leaderboard + timeline + graveyard + stats |

---

## 5. Build Order

| Phase | What |
|-------|------|
| 1 | Strategy wallet manager (create/track/kill wallets) + mutation engine |
| 2 | Evolution engine (hourly rank/kill/spawn cycle) |
| 3 | Wire into trading loop + API endpoints |
| 4 | Evolution dashboard page |
| 5 | Integration test (run for multiple hours, verify kills/spawns) |
