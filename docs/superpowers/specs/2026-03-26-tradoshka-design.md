# Tradoshka — Multi-Market Auto Trading Platform

**Date:** 2026-03-26
**Status:** Approved
**Author:** Ahmed (a7mdelbanna) + Claude
**Repo:** tradoshka
**Goal:** #1 GitHub trading repo, attract investors based on verified performance numbers

---

## 1. Vision

Tradoshka is a multi-market auto trading platform that combines multi-agent AI simulation, multi-strategy execution, and institutional-grade risk management into a single open-source system. Every feature must be the best-in-market and outperform all existing implementations.

**Target markets (in build order):**
1. Polymarket (prediction markets)
2. Crypto (spot, futures, perps, DeFi)
3. Forex (majors, crosses, exotics, metals)
4. Stocks (equities, ETFs, options)

Each market is a first-class module with 100% attention — not a bolted-on feature.

**Phase 1 deliverable:** Public performance dashboard with verified returns.
**Long-term:** Copy trading platform, SaaS, investor attraction.

---

## 2. Architecture — Modular Monorepo

```
tradoshka/
├── core/                    # Rust workspace
│   ├── engine/              # Order execution, matching, routing
│   ├── risk/                # Risk management, position sizing, circuit breakers
│   ├── data/                # Market data ingestion, normalization, storage
│   ├── common/              # Shared types, traits, interfaces
│   └── api/                 # REST/WebSocket API server (Axum)
├── markets/                 # Market-specific adapters (Rust)
│   ├── polymarket/          # Polymarket CLOB API, USDC settlement
│   ├── crypto/              # Direct exchange APIs (Binance, Bybit, OKX, Coinbase)
│   ├── forex/               # OANDA, IBKR direct API clients
│   └── stocks/              # Alpaca, IBKR direct API clients
├── strategies/              # Python strategy workspace
│   ├── shared/              # Base classes, indicators, ML utils
│   ├── polymarket/          # AI prediction, copy trading, market making, arbitrage
│   ├── crypto/              # Grid, DCA, arbitrage, momentum, market making, copy trading
│   ├── forex/               # Session-aware, macro, carry trade, smart money, correlation
│   └── stocks/              # Momentum, swing, fundamental, options, copy trading (13F/congress)
├── ai/                      # Python AI/ML engine
│   ├── mirofish/            # Multi-agent simulation (reimplemented from scratch)
│   ├── sentiment/           # Multi-source NLP/LLM analysis
│   ├── regime/              # Market regime detection
│   └── learning/            # Self-improving ML pipeline
├── dashboard/               # Next.js + React + Tailwind
│   ├── app/                 # Pages (public performance + private operator)
│   ├── components/          # Charts, cards, tables
│   └── api/                 # tRPC/REST client to Rust backend
└── infra/                   # Docker, CI/CD, deployment configs
```

**Tech stack:**
- **Core engine:** Rust (performance, safety)
- **Strategies & AI:** Python (via PyO3 bridge)
- **Dashboard:** React + Next.js 14+ + Tailwind CSS (styled via ui-ux-pro-max-skill)
- **API layer:** Axum (Rust) → tRPC (Next.js)
- **Data storage:** TimescaleDB (time-series), in-memory ring buffer (real-time)
- **Charting:** Recharts / Lightweight Charts

---

## 3. Core Engine (Rust)

### 3.1 Engine Crate

Order lifecycle management: creation → validation → routing → execution → fill tracking.

Supports: market, limit, stop-loss, trailing stop orders. State machine per order.

### 3.2 Risk Crate

Institutional-grade risk management:
- Per-trade risk: 1-2% maximum of account
- Daily drawdown limit: 5%
- Portfolio drawdown halt: 15%
- Position sizing: Half-Kelly criterion
- Stop losses: ATR-based (adaptive to volatility)
- Correlation checks across strategies
- Circuit breaker: hard stop at 20% drawdown

### 3.3 Data Crate

Unified `MarketEvent` type across all markets. Candle aggregation, tick storage, WebSocket management with automatic reconnect. TimescaleDB for historical data, in-memory ring buffer for real-time.

### 3.4 Common Crate

Key traits that define the contract between all components:

```rust
trait MarketAdapter {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe(&self, symbols: &[Symbol]) -> Result<Stream<MarketEvent>>;
    async fn place_order(&self, order: Order) -> Result<OrderId>;
    async fn cancel_order(&self, id: OrderId) -> Result<()>;
    async fn get_positions(&self) -> Result<Vec<Position>>;
    async fn get_balances(&self) -> Result<Balances>;
}

trait RiskManager {
    fn validate_order(&self, order: &Order, portfolio: &Portfolio) -> Result<RiskDecision>;
    fn check_circuit_breakers(&self, portfolio: &Portfolio) -> bool;
    fn calculate_position_size(&self, signal: &Signal, portfolio: &Portfolio) -> Decimal;
}
```

### 3.5 API Crate

Built on Axum. REST + WebSocket server exposing: strategy performance, live positions, P&L, risk metrics, order history. WebSocket for real-time dashboard updates.

**Communication with Python:** PyO3 bridge. Rust calls Python strategy functions, Python returns `Signal` objects that Rust validates through risk management before executing.

**Performance target:** 10M+ candles/sec backtesting throughput (beating NautilusTrader's 5M).

---

## 4. Market Modules

### 4.1 Polymarket (Phase 1)

**Adapter (Rust):**
- Direct HTTP/WS client to Polymarket CLOB API (no external SDK)
- Order book management, spread tracking
- USDC settlement on Polygon, outcome resolution
- New market detection, volume/liquidity filtering

**Strategies (Python):**

| Strategy | Description |
|----------|-------------|
| AI Predictor | MiroFish-inspired multi-agent simulation — spawn 1000+ agents, run Monte Carlo simulations, estimate event probabilities |
| Copy Trading | Track top Polymarket wallets on-chain (Polygon), extract trading signals from whale movements, filter by win rate/ROI |
| Market Making | Provide liquidity on both sides, inventory risk management, fair value pricing from AI + copy signals |
| Arbitrage | Cross-platform (Polymarket vs Kalshi/Manifold), related market mispricing |
| Ensemble | Combine all strategy signals with weighted voting |

**AI Prediction Flow:**
1. New event detected on Polymarket
2. World Builder creates simulation environment with real data (news, on-chain, macro)
3. Agent Factory spawns 1000+ autonomous agents with distinct personalities/biases
4. Monte Carlo simulation runs 1000+ iterations
5. Scorer converts outcomes to probability estimates
6. If divergence from market price > threshold → trading signal
7. Ensemble Combiner weights signal with copy trading + market making + arbitrage signals
8. Risk manager validates → Rust engine executes

### 4.2 Crypto (Phase 2)

**Adapter (Rust — our own, no CCXT):**
- Direct API clients: Binance (spot + futures), Bybit (V5), OKX, Coinbase
- L2/L3 order book aggregation across exchanges
- Funding rate tracking for perpetuals
- DEX interaction: EVM chains (Uniswap) + Solana (Raydium/Jupiter)

**Strategies (Python):**

| Category | Strategies |
|----------|-----------|
| Grid | Dynamic (ATR-based), infinity (trending), geometric |
| DCA | Smart DCA (buy dips harder), reverse DCA (scale out on rallies) |
| Momentum | Breakout, trend follow (EMA/ADX), RSI divergence |
| Mean Reversion | Bollinger bounce, pairs trading (BTC/ETH ratio) |
| Arbitrage | Cross-exchange, triangular, funding rate, DEX-CEX |
| Market Making | Spread trading, inventory hedging |
| Copy Trading | Binance lead traders, Bybit masters, on-chain whale tracking |
| AI | Sentiment (Twitter/Reddit), regime detection, multi-agent prediction |
| Ensemble | Meta-strategy: regime-adaptive capital allocation |

**Regime-Adaptive Allocation:**
- TRENDING → momentum, trend follow, DCA
- RANGING → grid, mean reversion, market making
- VOLATILE → arbitrage, reduce positions
- CRASH → circuit breaker, hedge, small DCA

### 4.3 Forex (Phase 3)

**Adapter (Rust — our own):**
- Direct API clients: OANDA v20, Interactive Brokers TWS
- Session clock: Sydney → Tokyo → London → New York
- Economic calendar integration (NFP, CPI, FOMC, ECB)
- Real-time spread monitoring, swap/rollover tracking

**Strategies (Python):**

| Category | Strategies |
|----------|-----------|
| Session | London breakout, Asian range, NY momentum, dead zone filter |
| Macro | News trading, interest rate differential, carry trade, calendar filter |
| Technical | Supply/demand zones, smart money concepts (ICT), harmonic patterns, multi-timeframe |
| Mean Reversion | Range trading, overextension snap-back |
| Correlation | Pair divergence, basket trading (USD strength), hedging |
| AI | Central bank NLP (FOMC/ECB), COT report analysis, regime detection |
| Ensemble | Session-aware meta-strategy |

**Session-Aware Allocation:**
- ASIAN (00:00-08:00 GMT) → Range strategies, carry trade, low sizing
- LONDON (08:00-16:00 GMT) → Breakout, trend following, full sizing
- LONDON/NY OVERLAP (13:00-16:00 GMT) → Maximum aggression, momentum
- NY (13:00-21:00 GMT) → News trading, trend continuation
- DEAD ZONE (21:00-00:00 GMT) → No new trades, manage only

**Economic Calendar Overlay:**
- High impact in < 30min → flatten or hedge
- Post-news first 5min → no trading (whipsaw)
- Post-news 5-60min → trend following on new direction

### 4.4 Stocks (Phase 4)

**Adapter (Rust — our own):**
- Direct API clients: Alpaca (commission-free), Interactive Brokers TWS
- Market hours management: pre-market, regular, after-hours, holidays
- Real-time stock screening (volume, gap, momentum)
- Fundamentals: earnings dates, P/E, revenue, SEC filings
- Options chain parsing, Greeks calculation (Black-Scholes, binomial)
- Sector/industry classification, rotation tracking

**Strategies (Python):**

| Category | Strategies |
|----------|-----------|
| Momentum | Gap trading, opening range breakout, relative strength, VWAP |
| Swing | Earnings play, sector rotation, breakout, pullback to EMA |
| Mean Reversion | Oversold bounce, pairs trading (KO/PEP, V/MA), ETF arbitrage |
| Fundamental | Value screener, earnings surprise drift, insider tracker, multi-factor model |
| Options | Wheel, iron condor, straddle, spreads, Greeks engine |
| AI | SEC filing NLP, earnings call transcript analysis, news impact classification |
| Copy Trading | Congress tracker, 13F hedge fund following, insider cluster following |
| Ensemble | Market-hours-aware, regime-adaptive allocation |

**Trading Day Engine:**
- PRE-MARKET (04:00-09:30 ET) → Gap scanning, earnings reaction, overnight news AI
- OPENING (09:30-10:00 ET) → Opening range breakout, VWAP establishment
- MID-DAY (10:00-14:00 ET) → Mean reversion, pairs trading, lower sizing
- POWER HOUR (15:00-16:00 ET) → Institutional flow, trend continuation
- AFTER-HOURS (16:00-20:00 ET) → Earnings reaction trades only

---

## 5. AI/ML Engine

### 5.1 MiroFish-Inspired Multi-Agent Simulation (Reimplemented from Scratch)

**Core components:**
- `world.py` — Simulation world containing agents, environment, rules
- `agent.py` — Autonomous agent with persona, memory, decision logic
- `memory.py` — Short-term + long-term agent recall
- `personality.py` — Biases, risk tolerance, knowledge profiles
- `interaction.py` — Agent-to-agent communication, opinion formation

**Builders:**
- `world_builder.py` — Constructs simulation from real-world data
- `agent_factory.py` — Generates diverse agent populations (institutional, retail, analysts, contrarians)
- `scenario_injector.py` — Injects "what if" variables mid-simulation

**Runners:**
- `monte_carlo.py` — 1000+ simulation iterations
- `parallel.py` — Distribute across CPU cores
- `convergence.py` — Detect when simulations have converged

**Scorers:**
- `probability.py` — Simulation outcomes → probability estimates
- `confidence.py` — Agreement across iterations
- `calibration.py` — Track accuracy, auto-correct biases over time

### 5.2 Multi-Source Sentiment Analysis

**Sources:** News (Reuters, Bloomberg, AP), Social (Twitter/X, Reddit, StockTwits), On-chain (whale movements, exchange flows), Congress (trades, lobby filings), SEC (10-K, 10-Q, 8-K, 13-F, Form 4), Central Banks (FOMC, ECB, BOJ)

**Processors:**
- LLM Analyzer — Claude/local LLM for deep text analysis
- Entity Extractor — Companies, people, events from text
- Impact Scorer — How much will this move the market?
- Contradiction Detector — Conflicting signals across sources

### 5.3 Market Regime Detection

**5 detectors running in parallel:**
1. Volatility (VIX, ATR)
2. Trend (ADX, MA slope, momentum)
3. Correlation (cross-asset, risk-on/risk-off)
4. Liquidity (volume profile, spread changes)
5. Macro (yield curve, DXY, commodities)

**Ensemble classifier** combines all 5 → regime label.
**Transition detector** identifies regime shifts EARLY, before full confirmation.

### 5.4 Self-Improving ML Pipeline

- Auto feature generation from market data
- Model zoo: XGBoost, LSTM, Transformer, Reinforcement Learning
- Auto-retrain weekly with new data
- Walk-forward validation (no look-ahead bias)
- Feature importance tracking and pruning
- Drift detector flags performance degradation

### 5.5 Signal Fusion Engine (Orchestrator)

Combines all AI outputs with weighted scoring:
- MiroFish simulation: weight 0.35
- Sentiment analysis: weight 0.25
- Regime detection: weight 0.20
- ML models: weight 0.20

Weights auto-adjust based on historical accuracy per source.

**Polymarket as calibration:** Our predictions vs Polymarket odds creates a verifiable track record. If we consistently beat the market → we have alpha. If not → auto-correction.

---

## 6. Dashboard & Performance Analytics

**Tech:** Next.js 14+ (App Router), React, Tailwind CSS (ui-ux-pro-max-skill), Recharts / Lightweight Charts, tRPC, Zustand

### 6.1 Public Pages (Investor-Facing)

| Page | Purpose |
|------|---------|
| Landing | Hero with live stats, social proof, value proposition |
| Performance | Main equity curve, drawdown, P&L calendar (GitHub-style heatmap), strategy breakdown |
| Strategies | Individual strategy performance, sortable by return/sharpe/drawdown |
| Leaderboard | Strategy ranking across all markets |

### 6.2 Private Dashboard (Operator)

| Page | Purpose |
|------|---------|
| Overview | Real-time P&L, open positions, active strategies |
| Markets (x4) | Per-market deep view: positions, strategies, order book |
| Strategies | Enable/disable, configure parameters, backtest |
| Risk | Risk dashboard, circuit breaker status, exposure heatmap |
| Analytics | Deep analytics, correlation matrices, heatmaps |
| Dry Mode | Paper trading monitor, dry vs live comparison |
| Settings | API keys, alerts, preferences |

### 6.3 Key Metrics

Sharpe Ratio, Sortino Ratio, Max Drawdown, Calmar Ratio, Win Rate, Profit Factor, Recovery Factor, Average R:R, Consecutive Losses, Correlation Matrix

### 6.4 Verification System (Trust Layer)

- **On-chain proof** (Polymarket/DeFi) — every trade has a Polygon transaction hash
- **Broker API audit trail** (Forex/Stocks) — trade history from broker APIs, timestamped
- **Exchange API verification** (Crypto) — history from exchange, cross-referenced
- **Third-party audit hook** — export-ready format, GIPS compliance

---

## 7. Development Automation System

### 7.1 CLAUDE.md Rules System

```
tradoshka/
├── CLAUDE.md                    # Master rules
├── core/CLAUDE.md               # Rust-specific rules
├── strategies/CLAUDE.md         # Python strategy rules
├── dashboard/CLAUDE.md          # Next.js/React rules
├── markets/polymarket/CLAUDE.md # Polymarket-specific rules
└── docs/
    ├── MISTAKES.md              # Auto-documented mistakes log
    ├── RESEARCH.md              # Competitive research findings
    ├── BENCHMARKS.md            # Performance benchmarks vs competitors
    └── GOALS.md                 # Goal tracking & estimates
```

### 7.2 Hook System

| Trigger | Action |
|---------|--------|
| After every commit | Update CHANGELOG.md, relevant CLAUDE.md files |
| After every compact | Regenerate all MD files with latest context |
| Before feature start | Spawn research agents to analyze competing implementations |
| After tests pass | Run benchmarks against baseline |
| After PR merge | Update GOALS.md with progress estimates |
| On mistake detection | Append to MISTAKES.md with root cause + prevention rule |
| Weekly (scheduled) | Deep research: scan top 20 trading repos for new approaches |
| On feature completion | Auto-evaluate against goals, recommend rewrite if below bar |

### 7.3 Branch Strategy

```
main                          # Stable, always deployable
├── dev                       # Integration branch
│   ├── feature/core-engine
│   ├── feature/polymarket-adapter
│   ├── feature/risk-engine
│   ├── feature/copy-trading
│   ├── feature/mirofish-ai
│   ├── feature/dashboard-v1
│   └── feature/dry-mode
```

Every feature on its own branch. PR-based workflow with automated review.

---

## 8. Security Policy

**CRITICAL: Never install external repos or packages directly.**

| Source | What we do | What we DON'T do |
|--------|-----------|-------------------|
| MiroFish | Read source, reimplement concepts from scratch | Fork, install, import |
| Exchange APIs | Write our own Rust HTTP/WS clients from official docs | Install third-party wrappers |
| ML/AI | Use established libs (PyTorch, scikit-learn) with pinned versions | Import unknown models |
| Any trading lib | Study the algorithms, rewrite ourselves | pip/npm install |

All dependencies must be:
- Explicitly justified and security-reviewed
- Pinned to exact versions
- Audited before any update
- Documented with: why needed, what was audited, who approved

---

## 9. Competitive Positioning

| Feature | Freqtrade | NautilusTrader | QuantConnect | **Tradoshka** |
|---------|-----------|----------------|--------------|---------------|
| Language | Python | Rust+Python | C#+Python | **Rust+Python** |
| Backtest speed | ~10K/sec | ~5M/sec | Cloud | **10M+/sec** |
| Markets | Crypto | Multi-asset | Multi-asset | **Multi-asset, each first-class** |
| AI/ML | FreqAI | None | Alpha Streams | **Multi-agent sim + LLM + sentiment + regime** |
| Copy trading | None | None | None | **Cross-platform built-in** |
| Prediction markets | None | None | None | **First-class Polymarket** |
| Risk engine | Basic | Good | Good | **Institutional-grade** |
| Self-improving | No | No | No | **Auto-research, auto-benchmark** |
| Verification | None | None | None | **On-chain + broker audit trail** |

---

## 10. Build Order

| Phase | Module | Focus |
|-------|--------|-------|
| 0 | Core engine + infrastructure | Rust workspace, risk engine, data layer, API, CLAUDE.md system, hooks |
| 1 | Polymarket | Adapter + AI predictor + copy trading + market making + dry mode + dashboard v1 |
| 2 | Crypto | Exchange adapters + all strategies + copy trading + dashboard integration |
| 3 | Forex | Broker adapters + session engine + all strategies + dashboard integration |
| 4 | Stocks | Broker adapters + options engine + all strategies + dashboard integration |
| 5 | Polish | Performance optimization, comprehensive testing, documentation, public launch |

Each phase produces a working, deployable system with verified results before moving to the next.
