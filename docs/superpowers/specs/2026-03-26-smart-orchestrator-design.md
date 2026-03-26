# Smart Orchestrator — Trade Reasoning, Risk Management & Position Management

**Date:** 2026-03-26
**Status:** Approved
**Goal:** Replace the dumb orchestrator with a research-first, reasoning-documented, risk-managed trade execution system. Every trade must explain WHY it was made, have stop losses, and show all risk parameters visually.

---

## 1. Trade Signal Architecture

Every trade signal from any strategy in any market must carry a **TradeThesis**:

### TradeThesis Structure

```
TradeThesis {
  // WHY — human-readable reasoning
  reasoning: String,              // 2-3 sentence explanation
  signals_used: Vec<String>,      // ["ema_crossover", "volume_surge", "funding_negative"]
  signals_agreed: u32,            // How many signals confirmed
  signals_total: u32,             // Total signals checked
  confidence: f64,                // 0.0 to 1.0

  // ENTRY
  entry_price: Decimal,           // ACTUAL market price (never estimated)
  entry_reason: String,           // "Breakout above 1h resistance at 69000"

  // EXITS — all 4 types required
  hard_stop_loss: Decimal,        // ATR-based, never moved. Max pain.
  trailing_stop: Decimal,         // 1.5 × ATR below highest price. Moves up only.
  take_profit: Decimal,           // Based on reward:risk ratio (min 2:1)
  time_stop_hours: u32,           // Close if no significant move within this time
  thesis_invalidation: String,    // Condition that kills the trade thesis

  // RISK
  risk_per_trade_pct: f64,        // 1% (unproven) or 2% (proven)
  risk_amount: Decimal,           // Actual dollars at risk
  reward_risk_ratio: f64,         // Must be >= 2.0 to enter
  position_size: Decimal,         // Calculated: risk_amount / (entry - stop_loss)
  max_loss: Decimal,              // Absolute max loss = risk_amount
  strategy_tier: String,          // "Unproven", "Tested", "Proven"
}
```

### Entry Rules

- No trade without a `reasoning` string
- No trade without all 4 exit levels set
- Confidence must be >= 0.60
- At least 2 signals must agree
- Reward:risk ratio must be >= 2.0
- Position size ALWAYS calculated from risk amount ÷ stop distance
- Entry price ALWAYS from live market API (never estimated/hardcoded)

### Applies To All Markets

Same TradeThesis for Polymarket, Crypto Spot, Crypto Perps. Only the specific signals and data sources differ.

---

## 2. Research-First Decision Engine

Before any strategy fires a signal, it runs a mini-research cycle.

### Polymarket Research

1. Pull current Yes/No prices from CLOB API (actual market price)
2. Check volume trend (24h vs 7d average)
3. Check holder concentration (whale activity via Data API)
4. Check time to resolution (days remaining)
5. Check price history (trending direction)
6. Run AI predictor if available (multi-agent probability estimate)
7. Compare estimated probability vs market price → calculate edge
8. Signal only if: edge > 5% AND confidence > 60% AND 2+ signals agree

### Crypto Spot Research

1. Pull current price + 24h stats from Binance
2. Calculate technical indicators (EMA9/21, RSI14, ATR14, Bollinger Bands)
3. Check volume profile (above/below 7d average)
4. Check corresponding perp funding rate (sentiment indicator)
5. Detect market regime (trending/ranging/volatile/crash)
6. Signal only if: indicators align with regime AND confidence > 60% AND 2+ agree

### Crypto Perps Research

Same as spot PLUS:
1. Check funding rate direction and magnitude
2. Check open interest changes
3. Determine optimal timeframe for the signal (5m/15m/1h/4h based on ATR)
4. Calculate leverage based on volatility (low vol = higher leverage ok, max 10x)
5. For funding arb: verify rate is persistently elevated (not a one-off spike)

### Research Output

Every research cycle produces:
- Written analysis (2-3 sentences) stored permanently with the trade
- List of which signals fired and which didn't
- Confidence score (0.0-1.0)
- Calculated entry price (ACTUAL market price)
- All 4 exit levels (calculated from ATR and risk tolerance)
- Position size (calculated from risk amount and stop distance)
- Reward:risk ratio

---

## 3. Position Management (Active Monitoring)

Every 5 minutes, each open position is re-evaluated:

### Check Sequence (in order)

1. **Update prices** — pull latest from market API, update mark price/unrealized PnL/ROE%

2. **Check hard stop loss** — if price <= hard_stop → CLOSE immediately. Log reason.

3. **Check trailing stop** — if new high → move trailing stop up. If price <= trailing → CLOSE, lock profits. Log amount locked.

4. **Check take profit** — if price >= take_profit → CLOSE. Log R:R achieved.

5. **Check time stop** — if position age > time_stop_hours AND |pnl%| < 2% → CLOSE. Log "stale trade, capital freed."

6. **Check thesis invalidation** — re-run the original entry signals. If original condition no longer holds → CLOSE. Log which signal invalidated.

7. **Log everything** — every check result stored in position history. Timestamp, price, SL status, TS level, thesis status.

### Stop Loss Calculation

- **Hard stop loss:** Entry price - (2 × ATR14). For Polymarket: max loss = 80% of position value (since binary outcomes can go to 0).
- **Trailing stop:** Starts at entry - (1.5 × ATR14). Only moves UP when price makes new highs. Never moves down.
- **Take profit:** Entry price + (stop_distance × reward_risk_ratio). Default R:R = 2.0, minimum 2.0.
- **Time stop:** 24 hours for crypto spot/perps. 72 hours for Polymarket (longer resolution times).

---

## 4. Strategy Trust Tiers

Strategies earn trust through performance. Trust determines capital allocation.

### Tier Definitions

| Tier | Label | Criteria | Risk/Trade | Max Positions | Max Exposure |
|------|-------|----------|-----------|---------------|-------------|
| 1 | 🔵 Unproven | < 20 trades | 1% equity | 3 | 10% equity |
| 2 | 🟡 Tested | 20-50 trades, WR > 50% | 1.5% equity | 5 | 20% equity |
| 3 | 🟢 Proven | 50+ trades, WR > 70%, Sharpe > 1.0 | 2% equity | 8 | 30% equity |

### Promotion Rules

- Unproven → Tested: complete 20 trades with win rate > 50%
- Tested → Proven: complete 50 trades AND pass readiness (70% WR, Sharpe > 1.0, PF > 1.5)

### Demotion Rules

- Proven → Tested: win rate drops below 60% over last 20 trades
- Tested → Unproven: win rate drops below 40% over last 20 trades
- Unproven → Disabled: 5 consecutive losses → auto-disable, flag for review

### Dashboard Display

Each strategy shows its tier badge (🔵/🟡/🟢), current win rate, trade count, and how close it is to promotion/demotion.

---

## 5. Rust Implementation

### New/Modified Files

| File | What |
|------|------|
| `core/engine/src/trade_thesis.rs` | NEW: TradeThesis struct with all fields |
| `core/engine/src/research.rs` | NEW: Research engine per market type |
| `core/engine/src/position_monitor.rs` | NEW: Active position monitoring with all stop types |
| `core/engine/src/strategy_tier.rs` | NEW: Tier tracking, promotion, demotion |
| `core/engine/src/orchestrator.rs` | REWRITE: Replace dumb logic with research-first pipeline |
| `core/engine/src/trade_recorder.rs` | MODIFY: Store TradeThesis with each trade |
| `core/api/src/routes.rs` | MODIFY: Return thesis, stops, tier in API responses |
| `core/api/src/main.rs` | MODIFY: Add position monitoring to trading loop |

### Modified Trade Record

```rust
pub struct TradeRecord {
    // ... existing fields ...
    pub thesis: TradeThesis,        // NEW: full trade reasoning
    pub stop_loss: Decimal,         // NEW: hard stop level
    pub trailing_stop: Decimal,     // NEW: current trailing stop
    pub take_profit: Decimal,       // NEW: target price
    pub time_stop_hours: u32,       // NEW: max hold time
    pub thesis_invalidation: String,// NEW: condition to exit
    pub risk_amount: Decimal,       // NEW: dollars at risk
    pub reward_risk_ratio: f64,     // NEW: R:R ratio
    pub strategy_tier: String,      // NEW: tier at time of trade
    pub position_log: Vec<String>,  // NEW: monitoring log entries
    pub close_reason: Option<String>,// NEW: why position was closed
}
```

---

## 6. Dashboard Display

### Trade Feed — Always Visible Per Trade

```
┌─────────────────────────────────────────────────────────────┐
│ BUY  Long  momentum  🟡Tested           Crypto   5:14 PM   │
│                                                              │
│ BTCUSDT  10x  [1h]                              ROE +5.3%   │
│ Entry: $69,110  →  Mark: $69,500  Size: 0.000029            │
│                                                              │
│ "EMA9/21 golden cross on 1h. Volume +35% above 7d avg.      │
│  Funding rate negative suggesting shorts overcrowded.        │
│  3/3 signals confirmed. Confidence: 82%"                     │
│                                                              │
│ 🔴 SL: $67,850 (-1.8%)  🟡 Trail: $68,900 (↑)              │
│ 🟢 TP: $71,500 (+3.5%)  ⏱ Time: 18h left                   │
│ 💰 Risk: $1.00 (1%)  R:R 2.1x  Status: ✅ Thesis valid      │
└─────────────────────────────────────────────────────────────┘
```

### Position Card — In Portfolio Panel

Same info as above but in the left panel, each position shows:
- Symbol, side, leverage, timeframe
- Entry → mark price with PnL
- All 4 stop levels with visual indicators
- Reasoning summary
- Thesis status (valid/invalidated)
- Risk amount and R:R

### Strategy Card — Shows Tier

```
┌──────────────────────────────────────────┐
│ 🟡 Momentum (Tested)         +3.2%  🟡  │
│ 25 trades  62% win  Tier: 2/3           │
│ ████████░░ 5 more trades to Proven      │
│ Risk: 1.5%/trade  Positions: 3/5        │
└──────────────────────────────────────────┘
```

---

## 7. Build Order

| Phase | What |
|-------|------|
| 1 | TradeThesis struct + Research engine + ATR-based stop calculation |
| 2 | Rewrite orchestrator with research-first pipeline |
| 3 | Position monitor (active stop checking every cycle) |
| 4 | Strategy tier system (promotion/demotion) |
| 5 | Dashboard update (show reasoning, stops, tiers inline) |
| 6 | Integration test (full cycle with real data, verify stops trigger) |
