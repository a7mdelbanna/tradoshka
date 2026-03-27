# Meme Coin Engine V2 — Research-Based Redesign

**Date:** 2026-03-27
**Status:** Approved
**Goal:** Replace the V1 guessing-based meme coin engine with a research-backed system using volume-based exits, Claude AI analysis, dev wallet monitoring, and meme-coin-specific rules.

**Based on:** Deep research of pump.fun mechanics, real sniper strategies, on-chain statistics (97-99% of tokens never graduate, profitable traders win 30-45% but 3-10x on winners), and infrastructure requirements.

---

## 1. Exit System (The Core — Replaces ALL Generic Stop Losses)

### Volume-Based Exit (Primary)
- Track 5-minute rolling volume for each held token
- Record peak 5-min volume since entry
- When current 5-min volume drops below 50% of peak → EXIT
- Rationale: volume cliff = pump is over, smart money leaving

### Dev Wallet Exit (Instant)
- Monitor dev/deployer wallet for any token we hold
- If dev sells ANY tokens → EXIT entire position immediately
- If dev transfers tokens to a new wallet → EXIT (preparing to dump)
- No exceptions, no waiting

### Holder Velocity Exit
- Track unique holder count every cycle
- Record holder growth rate (holders per minute)
- If growth stalls (< 1 new holder per minute) or reverses → EXIT
- Growing holders = buying pressure, shrinking = distribution

### Profit Ladder (Mechanical)
```
Sell 30% at 2x — recover initial + lock profit, position is now "free"
Sell 30% at 5x — take major profit
Hold 40% with 50% trailing stop from peak — moonshot runner
```

### Time-Based (Last Resort)
- Trend riding: max 2 hours
- Copy trading: max 30 minutes
- Sell at market regardless of PnL when time expires

### Hard Stop (Emergency Only)
- -50% from entry → EXIT (real rug, not normal volatility)
- NOT -15% (meme coins swing 30%+ normally — tight stops kill valid trades)

### What We Do NOT Do
- No ATR-based stops (meme coins don't follow ATR)
- No generic -15% stop losses (kills trades on normal volatility)
- No 1-hour time stops for early trades (too long for dead tokens)

---

## 2. Trend Riding Engine

### Entry Criteria (ALL must be true)
1. Token graduated to Raydium (top 1-3% — survived bonding curve)
2. 5-minute volume > $5,000 (real buying activity)
3. Buy/sell ratio > 1.5 in last 5 min (more buyers than sellers)
4. Unique holders growing (new wallets acquiring)
5. Price UP from Raydium launch price (not already dumping)
6. Market cap < $500K (room to run)
7. NOT a copycat (Claude checks originality)
8. Safety check passes (mint revoked, no freeze, no honeypot)

### Claude AI Second-Pass Analysis (30-60 seconds)

For each token that passes automated filters, Claude analyzes:

```
PROMPT: "Analyze this Solana meme token for trading potential:
  Name: {name}, Symbol: {symbol}
  Market cap: ${mcap}, Liquidity: ${liq}
  5min volume: ${vol_5m}, 1h volume: ${vol_1h}
  Price change 5m: {change_5m}%, 1h: {change_1h}%
  Holders: {holders}, growth rate: {rate}/min
  Token age: {age} minutes

  Score 0-100 on:
  1. Narrative freshness (is this concept original or a copycat?)
  2. Community signals (does volume/holders suggest real interest?)
  3. Risk assessment (red flags?)
  4. Timing (is the pump still early or already peaking?)

  Return JSON: {score: N, reasoning: '...', verdict: 'BUY/SKIP/AVOID'}"
```

### Decision Matrix

| Claude Score | Action | Position Size |
|-------------|--------|---------------|
| 80-100 | STRONG BUY | $20 (full) |
| 60-79 | BUY | $10 (half) |
| 40-59 | SKIP | $0 |
| 0-39 | AVOID | $0 |

### Capital Allocation
- 50% of MC wallet ($50) for trend riding
- Max 5 open trend positions at once
- $10-20 per trade

---

## 3. Copy Trading Engine (Meme Coin Specific)

### Wallet Scoring (Meme Coin Criteria)
- 50+ meme coin trades in last 30 days
- Win rate 35%+ (30-45% is good for meme coins)
- Average winner: 3x+ (this matters more than win rate)
- Average loser: -50% max (shows they cut losses)
- NOT a bot pattern (no regular intervals)
- NOT front-running other copy traders

### Copy Decision Logic

When tracked wallet buys:
1. Token < 30 minutes old? (fresh enough)
2. Our price < 3x from wallet's entry? (not too late)
3. Volume still growing? (pump active)
4. No dev sells detected? (safe)
5. Claude score > 50? (AI verified)

If 4/5 pass → COPY at $10-20

When tracked wallet SELLS → We sell within 30 seconds. No exceptions.

### Anti-Manipulation
- 5+ trades in 1 minute from one wallet → flag suspicious
- Multiple tracked wallets buy same token in 10 seconds → wait 2 min (coordinated)
- Wallet's last 5 signals ALL lost → reduce allocation 50% for 10 signals

### Capital Allocation
- 50% of MC wallet ($50) for copy trading
- Max 5 open copy positions at once
- $10-20 per trade

---

## 4. Strategy Definitions (40 total, $100 each)

### Group A: Trend Riding (20) — prefix `MC-TR-`

| Strategy | Key Filter | Target |
|----------|-----------|--------|
| MC-TR-vol-surge-5x | Volume 5x above avg | 2x |
| MC-TR-vol-surge-10x | Volume 10x above avg | 3x |
| MC-TR-fresh-grad-1h | Graduated < 1 hour | 2x |
| MC-TR-fresh-grad-5m | Graduated < 5 min | 5x |
| MC-TR-holder-rocket | 100+ new holders in 5 min | 3x |
| MC-TR-holder-steady | 20+ holders/min sustained | 2x |
| MC-TR-narrative-hot | Claude: trending topic match | 3x |
| MC-TR-narrative-original | Claude: first-of-kind concept | 5x |
| MC-TR-buy-pressure | Buy/sell ratio > 3.0 | 2x |
| MC-TR-buy-moderate | Buy/sell ratio > 1.5 | 2x |
| MC-TR-mcap-micro | MCap $10K-$50K | 5x |
| MC-TR-mcap-small | MCap $50K-$200K | 3x |
| MC-TR-mcap-mid | MCap $200K-$1M | 2x |
| MC-TR-dip-recovery | Dropped 30%+ then recovering | 2x |
| MC-TR-momentum-5m | 5min candle breakout | 2x |
| MC-TR-momentum-15m | 15min trend confirmation | 2x |
| MC-TR-high-safety | Safety > 80 only | 2x |
| MC-TR-balanced | All filters moderate | 2x |
| MC-TR-aggressive | Loose filters, high target | 5x |
| MC-TR-explorer | Random params, evolution | varies |

### Group B: Copy Trading (20) — prefix `MC-CT-`

| Strategy | Key Filter |
|----------|-----------|
| MC-CT-top-pnl-10 | Copy top 10 PnL wallets |
| MC-CT-top-pnl-5 | Copy top 5 only (selective) |
| MC-CT-high-wr | Copy wallets > 40% WR |
| MC-CT-high-multiplier | Copy wallets with avg 5x+ wins |
| MC-CT-early-buyer | Copy wallets that enter < 2min of launch |
| MC-CT-whale-50k | Copy > $50K portfolio wallets |
| MC-CT-whale-10k | Copy $10-30K smart money |
| MC-CT-consensus-2 | 2+ tracked wallets buy same token |
| MC-CT-consensus-3 | 3+ tracked wallets (high conviction) |
| MC-CT-fast-follow | Copy within 15 seconds |
| MC-CT-delayed-safe | Wait 60 seconds (avoid front-run) |
| MC-CT-sell-mirror | Mirror sells instantly |
| MC-CT-fresh-only | Only copy if token < 10min old |
| MC-CT-volume-confirm | Only copy if volume > $10K |
| MC-CT-ai-verified | Only copy if Claude score > 70 |
| MC-CT-conservative | Strict filters all |
| MC-CT-balanced | Moderate filters |
| MC-CT-aggressive | Loose filters, more trades |
| MC-CT-contrarian | Fade sells (buy dips whales create) |
| MC-CT-explorer | Random params, evolution |

---

## 5. Data Collection (For AI Learning)

Every token encountered:
```json
{
  "token_address": "...",
  "symbol": "PEPE2",
  "chain": "solana",
  "source": "dexscreener",
  "first_seen_at": "2026-03-27T15:00:00Z",
  "graduated": true,
  "initial_mcap": 69000,
  "peak_mcap": 450000,
  "time_to_peak_mins": 22,
  "initial_volume_5m": 8500,
  "peak_volume_5m": 45000,
  "volume_at_our_entry": 12000,
  "volume_at_our_exit": 6000,
  "holders_at_entry": 150,
  "holders_at_exit": 280,
  "holder_growth_rate": 8.5,
  "dev_wallet_sold": false,
  "safety_score": 75,
  "claude_score": 82,
  "claude_reasoning": "Original cat+politics hybrid concept, strong first-hour volume...",
  "our_entry_price": 0.00000012,
  "our_exit_prices": [0.00000024, 0.00000060, 0.00000045],
  "our_pnl_pct": 283,
  "exit_reason": "profit_ladder_stage_2",
  "strategy": "MC-TR-narrative-original",
  "hold_duration_mins": 18
}
```

---

## 6. Implementation Changes

### Replace in existing code:
- Remove generic -15% stop loss → volume-based exit
- Remove generic time stops → meme-specific time limits
- Remove `quick_check` optimistic safety → realistic defaults
- Remove random DexScreener keyword search → graduated token detection
- Replace fixed $5 positions → $10-20 based on Claude score

### New modules needed:
- `volume_tracker.rs` — track 5-min rolling volume per token, detect cliffs
- `holder_tracker.rs` — track unique holder count velocity
- Update `safety.rs` — add bundle detection, dev sell monitoring
- Update `token_scanner.rs` — focus on graduated tokens, not random search
- Add Claude API integration for real-time token analysis

### Build Order:
1. Volume-based exit system (replace all MC stop losses)
2. Trend riding with real entry criteria
3. Copy trading with wallet scoring
4. Claude AI integration for scoring
5. Update 40 strategy definitions
6. Data collection for AI learning
