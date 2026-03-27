# Polymarket Copy Trading Engine — Wallet Discovery, Basket Consensus, AI Verification

**Date:** 2026-03-27
**Status:** Approved
**Goal:** Build the best copy trading engine for prediction markets — discover top wallets, require basket consensus (80% agreement), verify with AI before copying, and protect with 4-layer circuit breakers.

---

## 1. Wallet Discovery & Scoring Engine

### Data Sources
- `GET https://data-api.polymarket.com/trades?filterType=CASH&filterAmount=1000&limit=500` — find large trades
- `GET https://data-api.polymarket.com/positions?user={address}` — current holdings
- `GET https://data-api.polymarket.com/activity?user={address}&type=TRADE` — trade history
- `GET https://data-api.polymarket.com/holders?market={conditionId}` — whale detection per market

### Scoring Formula

```
wallet_score = (
    0.25 × normalized_win_rate +          // (win_rate - 0.5) / 0.5
    0.25 × normalized_roi +               // min(roi / 0.5, 1.0)
    0.20 × normalized_consistency +        // 1 - (std_dev / mean_return)
    0.15 × normalized_selectivity +        // 1 / (trades_per_month / 10)
    0.15 × recency_decay                   // exp(-0.03 × days_since_last_win)
)
```

### Wallet Grades

| Grade | Score Range | Copy? |
|-------|-----------|-------|
| A+ | 0.85 - 1.00 | Yes, full size |
| A | 0.70 - 0.85 | Yes, full size |
| B | 0.55 - 0.70 | Yes, half size |
| C | 0.40 - 0.55 | No |
| D | 0.25 - 0.40 | No |
| F | < 0.25 | No |

### Qualification Gates
- Minimum 20 trades across 30+ days
- Win rate > 60%
- ROI > 10% on deployed capital
- Not a market maker (detected by both-side orders)
- Not a bot (regular timing patterns)
- Active within last 30 days

### Trader Classification

| Type | Detection | Copy Value |
|------|-----------|-----------|
| Informed | Large size, low frequency, high WR, niche focus | HIGH |
| Market Maker | High volume, orders on both sides, low PnL/volume | NONE |
| Bot/Algo | Regular patterns, precise sizing | LOW |
| Noise/Retail | Small random bets, many markets, low WR | NONE |

### Refresh Frequency
- Full rescan: every 30 minutes
- Score recalculation: every hour
- Grade changes trigger alerts

---

## 2. Basket Consensus Copy Trading

### Topic Baskets

| Basket | Size | Criteria |
|--------|------|----------|
| Politics | 10-15 wallets | >60% WR on political markets |
| Sports | 10-15 wallets | >60% WR on sports markets |
| Crypto | 10-15 wallets | >60% WR on crypto markets |
| General | 10-15 wallets | Highest overall Sharpe across all markets |

Baskets are auto-built from the scored wallet database. Wallets can appear in multiple baskets if they qualify.

### Consensus Algorithm

```
For each tracked market:
  1. Monitor all basket wallets via 2-second API polling
  2. When wallet trades:
     a. Price range check: $0.20 - $0.80 (skip extremes)
     b. Record: wallet X, side YES/NO, price, timestamp
     c. Check consensus: what % of this market's basket is on same side?
  3. TRIGGER conditions (ALL must be true):
     - 80%+ of basket on same side
     - Market price in $0.20 - $0.80 range
     - Order book depth > $200
     - Not in circuit breaker cooldown
  4. Execute copy trade
```

### Position Sizing (Adaptive Tiered)

| Whale Trade Size | Multiplier | Reasoning |
|-----------------|-----------|-----------|
| $1 - $50 | 2.0x | Small = high conviction |
| $50 - $200 | 1.0x | Standard |
| $200+ | 0.5x | Could be market making |

Caps:
- Max 10% of copy portfolio per market
- Max 20 open copy positions at any time
- Reserve 30% capital for new opportunities

---

## 3. AI-Enhanced Verification

### Decision Matrix

Before every copy trade, run AI verification:

| Whale Consensus | AI Agrees | Action | Size |
|----------------|-----------|--------|------|
| 80%+ same side | AI confirms edge | STRONG BUY | Full size |
| 80%+ same side | AI neutral | BUY | Half size |
| 80%+ same side | AI disagrees | SKIP | No trade |
| < 80% consensus | — | NO TRADE | — |

### AI Probability Estimation

For each market the consensus triggers on:
1. Run MiroFish-style multi-agent simulation (statistical mode, no LLM needed)
2. Get AI probability estimate (0-100%)
3. Compare to market price:
   - AI says 55%, market at 40% → Edge = +15% → CONFIRMS
   - AI says 42%, market at 40% → Edge = +2% → NEUTRAL (too small)
   - AI says 30%, market at 40% → Edge = -10% → DISAGREES

### Edge Thresholds
- Edge > 10%: AI CONFIRMS (strong buy)
- Edge 3-10%: AI NEUTRAL (half size buy)
- Edge < 3%: AI DISAGREES (skip)

---

## 4. Circuit Breakers (4 Layers)

### Layer 1: Size Filter
- Skip whale trades > $50,000 (potential manipulation)
- Skip whale trades < $10 (noise)

### Layer 2: Sequence Detection
- If same wallet makes 3+ trades in 30 seconds → flag as manipulation
- Do NOT copy rapid-fire sequences
- Alert in dashboard timeline

### Layer 3: Order Book Depth
- Before executing, check book depth on our side
- Skip if depth < $200 (too thin, will get slipped)

### Layer 4: Portfolio Trip
- Daily loss limit: 5% of copy trading capital → halt 6 hours
- Per-market limit: 10% of capital → skip that market for 24 hours
- If 3 consecutive copy trades lose → reduce size by 50% for next 5 trades

### Price Band Guard (Hard Rules)
- ONLY buy tokens priced $0.20 - $0.80
- NEVER buy below $0.10 (extreme longshot)
- NEVER buy above $0.90 (tiny upside, large downside)

---

## 5. Data Persistence for AI Learning

### Every copy trade saves:
```json
{
  "timestamp": "2026-03-27T14:30:00Z",
  "type": "copy_trade",
  "market": "Will BTC hit 100K by June?",
  "side": "YES",
  "price": 0.35,
  "size": 15,
  "consensus_pct": 0.85,
  "wallets_agreeing": 9,
  "wallets_total": 11,
  "basket": "crypto",
  "ai_probability": 0.48,
  "ai_edge": 0.13,
  "ai_verdict": "CONFIRMS",
  "whale_avg_entry": 0.33,
  "our_entry": 0.35,
  "slippage": 0.02,
  "circuit_breaker_status": "CLEAR"
}
```

This data trains the AI to learn:
- Which baskets produce the best signals
- What consensus thresholds actually work (maybe 70% is better than 80%)
- How much AI verification adds vs pure copy trading
- Which niche markets are most predictable

---

## 6. Implementation

### New Rust Files

| File | What |
|------|------|
| `core/engine/src/wallet_scorer.rs` | Wallet discovery, scoring, grading, classification |
| `core/engine/src/basket_consensus.rs` | Topic baskets, consensus tracking, trigger logic |
| `core/engine/src/copy_engine.rs` | AI verification, adaptive sizing, execution |
| `core/engine/src/copy_circuit_breaker.rs` | 4-layer protection |

### Integration Points
- Polymarket Data API (wallet discovery, trade monitoring)
- Existing MiroFish AI predictor (probability estimation)
- Existing evolution engine (copy trading strategies compete alongside others)
- Existing data logger (persist all copy trade decisions)

### Build Order

| Phase | What |
|-------|------|
| 1 | Wallet scorer + discovery (fetch real Polymarket trader data) |
| 2 | Basket consensus engine (group wallets, track consensus) |
| 3 | Copy execution with AI verification |
| 4 | Circuit breakers |
| 5 | Wire into PM-CT-* strategy wallets |
| 6 | Dashboard: copy trading monitoring view |
