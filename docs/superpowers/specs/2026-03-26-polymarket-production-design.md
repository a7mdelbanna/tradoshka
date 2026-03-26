# Polymarket Production-Ready System — Design Spec

**Date:** 2026-03-26
**Status:** Approved
**Goal:** Make Polymarket trading fully production-ready with real market data, dry mode validation, and a clear gate to live trading.

---

## 1. Overview

Build a complete dry-mode trading loop that connects to real Polymarket APIs, runs all 5 strategies against live market data, executes simulated trades in a mockup wallet, and displays everything in a real-time split-screen dashboard. A 6-criteria Production Readiness Score gates the transition from dry mode to real money.

**Phases:**
1. Dry mode with simulated wallet on real data → validate strategies
2. Real Polymarket wallet with micro-trades → confirm real execution
3. Scale up once proven

No other markets (crypto, forex, stocks) until Polymarket is production-proven.

---

## 2. Dry Mode Trading Loop

### Data Flow

```
Real Polymarket APIs (live prices, 24/7)
        │
        ▼
Market Data Service (Rust)
  - REST polling every 5 min (Gamma + CLOB APIs)
  - WebSocket for real-time price updates
  - Event triggers on: >3% price move, whale trade, market resolution
        │
        ▼
Strategy Orchestrator (Python via PyO3)
  - Runs all 5 strategies in parallel per market event
  - AI Predictor, Copy Trading, Market Making, Arbitrage, Ensemble
  - Ensemble produces final weighted signal
        │
        ▼
Risk Manager (Rust)
  - Validates signal against portfolio state
  - Half-Kelly position sizing
  - Circuit breakers (daily 5%, portfolio 15%, hard stop 20%)
        │
        ▼
Execution Engine (Rust)
  - DRY MODE: simulates fill at real market price + slippage (5 bps)
  - LIVE MODE: places real order via Polymarket CLOB API
        │
        ▼
Simulated Wallet (Rust)
  - Tracks balance, positions, P&L, fees, trade history
        │
        ▼
Dashboard v2 (Next.js, WebSocket)
  - Split-screen: portfolio + live trade feed
  - Production Readiness Score
```

### Evaluation Frequency

- **Base interval:** Every 5 minutes — pull prices, run strategies, evaluate signals
- **Immediate triggers:**
  - Price moves >3% on a tracked market
  - Whale trade detected on a tracked wallet
  - Market approaching resolution (end_date < 24h) → reduce/close positions
  - Market resolved → auto-settle positions in wallet

### Market Selection

- Scanner runs every 30 minutes via Gamma API
- Filters: volume >$1K/24h, liquidity >$500, order book enabled, not closed
- Ranks by opportunity score (volume + liquidity + mispricing)
- Tracks top 20 markets simultaneously
- Strategies only receive events for tracked markets

---

## 3. Simulated Wallet

### Starting State

- Balance: $100 USDC
- Positions: empty
- Mode: DRY

### What It Tracks

| Field | Description |
|-------|-------------|
| Balance | Available USDC |
| Positions | Map of token_id → {side, shares, avg_price, current_price, unrealized_pnl} |
| Equity | Balance + sum of all position values |
| Trade History | Every trade: timestamp, strategy, market question, direction, price, size, fees |
| Fees | Polymarket-realistic: `base_rate * min(price, 1-price) * size` |
| Daily P&L | Aggregated daily for heatmap |
| Per-Strategy P&L | Individual strategy performance tracking |

### Fill Simulation

1. Strategy signal → risk manager approval → order created
2. Dry engine reads current real Polymarket price (from API/WebSocket)
3. Simulates fill at `market_price + slippage` (configurable, default 5 bps)
4. Deducts Polymarket-realistic fees
5. Updates wallet balance and positions
6. Records trade in history with full metadata

### Position Resolution

- On `market_resolved` WebSocket event:
  - Winning positions → $1.00/share credited to balance
  - Losing positions → $0.00, removed from portfolio
  - Automatic, no manual intervention

### Mode Switching (Dry → Live)

- Same wallet interface, different execution backend
- Dry: fills simulated locally
- Live: orders sent to Polymarket CLOB API via real Polygon wallet
- Requires manual "Go Live" confirmation — never auto-switches
- Dashboard shows clear mode indicator

---

## 4. Strategy Orchestrator

### Strategy Weights

| Strategy | Default Weight | Auto-adjusts based on |
|----------|---------------|----------------------|
| AI Predictor | 0.35 | Prediction accuracy vs actual outcomes |
| Copy Trading | 0.25 | Win rate of copied wallets |
| Market Making | 0.15 | Spread capture rate |
| Arbitrage | 0.10 | Successful arb captures |
| Ensemble override | 0.15 | Combined confidence |

- Weights auto-adjust weekly based on each strategy's recent accuracy
- Strategies that consistently lose get auto-disabled from ensemble
- Individual strategy readiness tracked separately

### Signal Flow

```
For each tracked market, every 5 min or on trigger:
  1. Create MarketEvent from real Polymarket data
  2. Feed to all 5 strategies in parallel
  3. Each returns Signal (LONG/SHORT/CLOSE/HOLD + strength)
  4. Ensemble Combiner produces weighted final signal
  5. Risk Manager validates against portfolio
  6. If approved → execution engine processes order
  7. Wallet updated → Dashboard updated via WebSocket
```

---

## 5. Dashboard v2 — Split-Screen Trading View

### Route: `/trading` (separate from public `/performance`)

### Layout

**Top bar:** Logo, nav links, mode badge (DRY MODE yellow / LIVE MODE green)

**Left panel — Portfolio:**
- Balance, equity, unrealized P&L, drawdown (updates every 1s via WebSocket)
- Open Positions table: market question, direction, shares, entry price, current price, P&L, strategy
- Strategy Performance: per-strategy return with health indicator (green >5%, yellow 0-5%, red <0%)
- Production Readiness Score (see Section 6)

**Right panel — Live Trade Feed:**
- Scrolling real-time log of every trade
- Each entry shows: timestamp, BUY/SELL, YES/NO, market question, price, size
- Expandable details: strategy name, confidence score, edge vs market, reasoning
- Color-coded: green for buys, red for sells, gold for settlements

**Bottom section:**
- Equity Curve (existing chart, updates in real-time)
- P&L Heatmap (existing, updates daily)
- Strategy Breakdown (existing, updates on each trade)

---

## 6. Production Readiness Criteria

### All 6 must pass simultaneously:

| # | Criterion | Threshold | How Measured |
|---|-----------|-----------|-------------|
| 1 | Days active | >= 14 days | Calendar days since first dry trade |
| 2 | Total trades | >= 50 trades | Count of completed (closed + settled) trades |
| 3 | Win rate | >= 70% | Winning trades / total completed trades |
| 4 | Sharpe ratio | >= 1.0 | (Return - risk_free) / std_dev of returns |
| 5 | Max drawdown | <= 15% | Largest peak-to-trough decline |
| 6 | Profit factor | >= 1.5 | Gross profit / gross loss |

### Dashboard Behavior

- Each criterion shows: current value, threshold, pass/fail icon
- Overall score: "4/6 — NOT READY" or "6/6 — PRODUCTION READY"
- When 6/6 reached: prominent green banner + "Go Live" button
- "Go Live" requires manual confirmation — NEVER auto-switches

### Per-Strategy Tracking

- Each strategy tracked individually against same criteria
- Failing strategies auto-disabled from ensemble
- Only strategies passing their own thresholds participate

---

## 7. Rust Components Needed

### New/Modified Crates

| Component | Location | What |
|-----------|----------|------|
| Market Data Service | `core/engine/src/market_data.rs` | Real-time Polymarket data fetching (REST + WS) |
| Strategy Orchestrator | `core/engine/src/orchestrator.rs` | Runs strategies on interval + triggers |
| Simulated Wallet | `core/engine/src/wallet.rs` | Full wallet simulation with realistic fees |
| Trade Recorder | `core/engine/src/trade_recorder.rs` | Persists all trades with metadata |
| Readiness Scorer | `core/engine/src/readiness.rs` | Calculates 6 production readiness criteria |
| Trading API Routes | `core/api/src/trading_routes.rs` | WebSocket + REST for trading dashboard |

### Modified Existing

| Component | Change |
|-----------|--------|
| `core/api/src/state.rs` | Add wallet, orchestrator, trade recorder to AppState |
| `core/api/src/server.rs` | Add `/trading` routes + WebSocket for trade feed |
| `markets/polymarket/src/adapter.rs` | Connect to real APIs, feed events to orchestrator |

---

## 8. Dashboard Components Needed

| Component | Location | What |
|-----------|----------|------|
| TradingPage | `app/(dashboard)/trading/page.tsx` | Split-screen layout |
| PortfolioPanel | `components/trading/PortfolioPanel.tsx` | Left panel: balance, positions |
| TradeFeed | `components/trading/TradeFeed.tsx` | Right panel: scrolling trade log |
| PositionsTable | `components/trading/PositionsTable.tsx` | Open positions with live P&L |
| StrategyHealth | `components/trading/StrategyHealth.tsx` | Per-strategy performance |
| ReadinessScore | `components/trading/ReadinessScore.tsx` | 6-criteria checklist |
| ModeBadge | `components/trading/ModeBadge.tsx` | DRY/LIVE mode indicator |
| TradeDetail | `components/trading/TradeDetail.tsx` | Expandable trade entry |

---

## 9. Build Order

| Phase | What | Depends on |
|-------|------|-----------|
| 2A | Market Data Service + Simulated Wallet | Phase 1 (Polymarket adapter) |
| 2B | Strategy Orchestrator + Trade Recorder | 2A |
| 2C | Trading Dashboard v2 + Readiness Score | 2A, 2B |
| 2D | Integration Testing + Dry Run | 2A, 2B, 2C |
| 2E | Real Wallet Integration (Polygon/USDC) | 2D proven |

Each phase produces a working, testable system.

---

## 10. Future: Command Center (Dashboard v3)

Planned for after Polymarket is production-proven:

- Market scanner showing all active Polymarket events with opportunity scores
- Strategy signals firing in real-time with accept/reject overrides
- Order execution status with fill tracking
- Risk dashboard with circuit breaker status, exposure heatmap
- Multi-market support (crypto, forex, stocks panels)
- All on one screen — full operator command center
