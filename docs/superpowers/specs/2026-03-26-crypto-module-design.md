# Crypto Module — Design Spec

**Date:** 2026-03-26
**Status:** Approved
**Goal:** Build a complete crypto trading module with Binance adapter, 9 strategy categories, dry mode with real data, and the same readiness scoring system as Polymarket.

---

## 1. Overview

Start with Binance (largest exchange, best API docs), build our own adapter from scratch (no CCXT). Support both spot and USDT-M futures. Run all strategies against real market data in dry mode. Same 6-criteria production readiness gate as Polymarket.

**Exchange priority:** Binance first. Bybit, OKX, Coinbase adapters added later using the same trait interface.

---

## 2. Binance Adapter (Rust)

### Architecture

```
markets/crypto/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── config.rs            # API URLs, rate limits, constants
    ├── auth.rs              # HMAC-SHA256 signing
    ├── client.rs            # REST client (spot + futures)
    ├── types.rs             # Binance-specific types
    ├── websocket.rs         # WebSocket streams (trades, klines, book, user data)
    ├── rate_limiter.rs      # Weight + order count tracking
    ├── adapter.rs           # MarketAdapter trait implementation
    └── funding.rs           # Funding rate monitor (for arb strategy)
```

### Key API Details

| API | Base URL | Auth |
|-----|----------|------|
| Spot REST | `https://api.binance.com/api/v3/` | HMAC-SHA256 query string |
| Futures REST | `https://fapi.binance.com/fapi/v1/` | HMAC-SHA256 query string |
| Spot WS | `wss://stream.binance.com:9443/ws/` | Listen key for user data |
| Futures WS | `wss://fstream.binance.com/ws/` | Listen key for user data |

### Auth: HMAC-SHA256

- Build query string with all params + `timestamp`
- Sign the ENTIRE query string with secret key
- Append `&signature=<hex>` to query string
- Send `X-MBX-APIKEY` header

### Rate Limits

- 6,000 weight/minute for spot
- Track via `X-MBX-USED-WEIGHT-1M` response header
- Order limit: 100 per 10 seconds
- Pre-flight check before each request

### Supported Order Types

**Spot:** LIMIT, MARKET, STOP_LOSS_LIMIT, TAKE_PROFIT_LIMIT, LIMIT_MAKER
**Futures:** LIMIT, MARKET, STOP, STOP_MARKET, TAKE_PROFIT, TAKE_PROFIT_MARKET, TRAILING_STOP_MARKET

---

## 3. Crypto Strategies (Python)

### 9 Strategy Categories

| # | Strategy | Description | Key Indicators |
|---|----------|-------------|----------------|
| 1 | **Grid Trading** | Buy/sell at fixed intervals around a center price. Dynamic grid adjusts with ATR. | ATR, Bollinger Bands |
| 2 | **DCA** | Dollar-cost average into positions. Smart DCA buys dips harder. | RSI, EMA |
| 3 | **Momentum** | Follow trends on breakouts with volume confirmation. | EMA cross, ADX, RSI divergence |
| 4 | **Mean Reversion** | Buy oversold, sell overbought. Pairs trading on correlated assets. | Bollinger Bands, RSI, Z-score |
| 5 | **Arbitrage** | Cross-exchange price differences, funding rate arb (long spot + short perp). | Spread monitoring, funding rates |
| 6 | **Market Making** | Provide liquidity with bid/ask quotes, manage inventory. | Spread, inventory skew |
| 7 | **Copy Trading** | Follow Binance lead traders, track whale wallets. | Leaderboard API |
| 8 | **AI/Sentiment** | News sentiment analysis, social media signals, regime detection. | LLM analysis |
| 9 | **Ensemble** | Weighted signal fusion. Regime-adaptive allocation. | All of the above |

### Regime-Adaptive Allocation

The ensemble adjusts weights based on detected market regime:

| Regime | Primary Strategies | Position Sizing |
|--------|-------------------|----------------|
| TRENDING | Momentum, DCA | Full |
| RANGING | Grid, Mean Reversion, Market Making | Full |
| VOLATILE | Arbitrage | Reduced |
| CRASH | Circuit breaker, hedge, small DCA | Minimal |

---

## 4. Simulated Wallet (Crypto)

Extends the same wallet design as Polymarket:

- Starting balance: $100 USDT
- Realistic Binance fees: 0.1% taker, 0.075% maker (with BNB discount path for later)
- Slippage simulation: configurable bps
- Supports both spot positions (hold asset) and futures positions (PnL in USDT)
- Funding rate simulation for perpetual futures
- Position tracking with entry price, current price, unrealized PnL

---

## 5. Dashboard Integration

- Add crypto tab to trading dashboard
- Same split-screen layout: portfolio + trade feed
- Same readiness scoring (6 criteria, 70% win rate)
- Market selector: switch between Polymarket / Crypto views
- Strategy health per crypto strategy

---

## 6. Build Order

| Phase | What | Tasks |
|-------|------|-------|
| 3A | Binance adapter (Rust) | Config, auth, types, REST client, WebSocket, rate limiter, MarketAdapter |
| 3B | Crypto strategies (Python) | Grid, DCA, momentum, mean reversion, arb, MM, copy, AI, ensemble |
| 3C | Crypto dry mode + dashboard | Wallet, orchestrator integration, dashboard tab, readiness |

Start with 3A.

---

## 7. Readiness Criteria (Same as Polymarket)

| # | Criterion | Threshold |
|---|-----------|-----------|
| 1 | Days active | >= 14 |
| 2 | Total trades | >= 50 |
| 3 | Win rate | >= 70% |
| 4 | Sharpe ratio | >= 1.0 |
| 5 | Max drawdown | <= 15% |
| 6 | Profit factor | >= 1.5 |
