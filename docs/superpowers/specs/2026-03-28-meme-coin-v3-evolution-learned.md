# Meme Coin Engine V3 — Evolution-Learned Redesign

**Date:** 2026-03-28
**Status:** Active
**Supersedes:** V2 (2026-03-27)
**Goal:** Apply lessons from 6+ hours of live Darwinian evolution trading to fix the V2 system. V2's theoretical parameters were catastrophically wrong — all 37 designed strategies lost 85-97% while 3 random explorers generated +4% to +78% profit.

**Based on:** Real evolution data from 5,000+ trades across 40 meme coin strategies, comparing winning explorers vs losing designed strategies.

---

## 1. Critical Findings from Evolution Data

### What V2 Got Wrong (ALL 37 Designed Strategies Lost 85-97%)

| Parameter | V2 Design | Actual Result | Problem |
|-----------|-----------|---------------|---------|
| Hard stop | -50% | Lost ~$85 per strategy | Too wide — by the time meme coin drops 50%, pump is completely dead, all capital gone |
| Take profit | 2x-5x (100-400%) | ~25% win rate but tiny winners | Unrealistic — trend-riding rarely catches 2x, let alone 5x |
| Positions | 5 per strategy | Each loss = 10% of capital | Too concentrated — single bad trade wipes significant equity |
| Leverage | 1x | Winners too small to offset losers | No amplification — small wins can't overcome fee drag |
| Capital/trade | 10% of equity | Bankroll depleted in ~10 losing trades | Too large — no room for the losing streak that meme coins guarantee |

### What Evolution Discovered (3 Random Explorers Made +4% to +78%)

| Parameter | Explorer Value | Why It Works |
|-----------|---------------|-------------|
| Hard stop | 15-20% | Cuts losses FAST — matches meme coin volatility cycle |
| Take profit | 1.08x-1.30x (8-30%) | Takes achievable profits — what the market actually gives |
| Positions | 12-17 | Diversified — single loss is 3-5% of capital, survivable |
| Leverage | 4-6x | Amplifies the small wins into meaningful profit |
| Capital/trade | 3-5% of equity | Small bets survive the 70% loss rate meme coins demand |
| Win rate | ~30% | Doesn't need high WR — large R:R compensates |
| Avg winner | $6.07 | Leverage amplifies the 8-30% price move |
| Avg loser | $3.25 | Tight stop + small position = manageable loss |
| R:R ratio | 1.87:1 | Positive expectancy despite 30% win rate |

### The Core Insight

**Meme coin trading is about FAST SMALL wins, not moonshots.**

The V2 spec was based on sniper strategies (buy at launch, ride to 5x). But our system does trend-riding (buy tokens already trending), which is fundamentally different:
- Snipers enter at $0.000001 and can realistically hit 10x
- Trend riders enter at $0.0005 (after the initial pump) — the 2x target is usually already behind them
- Trend riders should scalp 10-30% on momentum, then move on

---

## 2. V3 Exit System

### Hard Stop (Primary Defense)
- **10-20% from entry** (varies by strategy aggressiveness)
- NOT 50% — that's a rug pull stop, not a trading stop
- Rationale: if momentum dies, get out and redeploy capital

### Quick Take Profit (Primary Exit)
- **8-30% from entry** (1.08x-1.30x target multiplier)
- NOT 2x or 5x — those are sniper targets, not trend-riding targets
- Take what the market gives, don't wait for moonshots

### Volume-Based Exit (Retained from V2)
- Track 5-minute rolling volume for each held token
- When current 5-min volume drops below 50% of peak: EXIT
- This is still valid and important

### Time Stop (Tightened from V2)
- Trend riding: max 15-30 minutes (was 2 hours — way too long)
- Copy trading: max 10-25 minutes (was 30 minutes)
- Dead meme coins are obvious within 10-15 minutes

### Profit Ladder (REMOVED)
The V2 profit ladder (30% at 2x, 30% at 5x, 40% trailing) was designed for sniper plays. For trend-riding:
- Single exit at TP (8-30% gain)
- If volume still strong at TP, evolution will discover which strategies benefit from holding longer

### What V3 Does NOT Do
- No 50% stop losses (too wide for trend-riding)
- No 2x+ take profit targets (unrealistic for trend entries)
- No 5-position concentration (too risky)
- No 1x leverage on all strategies (some need amplification)

---

## 3. V3 Position Sizing

### Many Small Bets
- 12-17 positions per strategy (was 5)
- 3-5% of equity per position (was 10%)
- Capital usage: 55-70% (was 50%)

### Leverage
- 3-6x leverage per strategy (was 1x)
- Rationale: with 8-30% TP and 30% win rate, leverage amplifies winners from $0.80 to $6.00
- Risk is managed by tight stops and small positions, not by avoiding leverage

### Position Management
- Close on TP hit or SL hit (no partial exits for now)
- Evolution will discover if partial exits improve performance

---

## 4. Token Discovery (Fixed in V2.1)

### DexScreener Endpoints (Priority Order)
1. **`/token-boosts/top/v1`** — Currently trending/promoted tokens (highest signal)
2. **`/token-profiles/latest/v1`** — Recently listed tokens (fresh opportunities)
3. **`/latest/dex/search?q={keyword}`** — Keyword search (fallback for coverage)

### Why Keyword Search Alone Failed
V2 used keyword search ("pepe", "doge", etc.) which returned established $1B tokens that never passed entry criteria. Boosted/profile endpoints return actually trending tokens with realistic mcap ($50K-$5M) and volume.

---

## 5. V3 Strategy Params (40 Strategies)

### Group A: Trend Riding (20) — prefix MC-TR-
All strategies now use:
- `hard_stop_pct`: 10-20% (varies by strategy)
- `target_mult`: 1.08-1.30x (varies by strategy)
- `auto_position_count`: 12-17 (diversified)
- `auto_leverage`: 3-6x (amplify winners)
- `capital_usage_pct`: 55-70% (deploy capital)
- `time_limit_mins`: 15-30 (quick exits)

Differentiation between strategies is in:
- Volume thresholds (1K-10K min_volume_5m)
- Safety requirements (30-80 min_safety)
- Market cap ranges (100K-10M max_mcap)
- Aggressiveness of SL/TP/leverage combos

### Group B: Copy Trading (20) — prefix MC-CT-
Same mechanical improvements applied:
- Tight stops, quick TP, many positions, leverage
- Additional params for copy-specific logic (when whale tracking is available)

---

## 6. Data Backing These Decisions

### Evolution Run #1 (2026-03-27, 6 hours)
```
WINNERS (3 strategies):
  MC-explorer-2:    +77.80%  246 trades  29.9% WR  SL=ATR  TP=R:R  17 pos  6x lev
  MC-explorer-1:    +16.78%  315 trades  29.0% WR  SL=ATR  TP=R:R  18 pos  6x lev
  MC-explorer-2-v2:  +4.84%   82 trades  30.0% WR  SL=ATR  TP=R:R  17 pos  6x lev

LOSERS (37 strategies, sample):
  MC-TR-balanced:   -86.73%  130 trades  24.8% WR  SL=50%  TP=2x    5 pos  1x lev
  MC-CT-aggressive: -96.89%  156 trades  21.9% WR  SL=50%  TP=2x    5 pos  1x lev
  MC-TR-fresh-5m:   -89.98%  151 trades  25.8% WR  SL=50%  TP=5x    5 pos  1x lev
```

### Key Metrics Comparison
```
              | Winners (avg) | Losers (avg)  | Delta
Win Rate      |    29.6%      |    23.8%      | +5.8pp (minor)
Avg Winner $  |    $6.07      |    $0.81      | 7.5x LARGER
Avg Loser $   |   -$3.25      |   -$1.04      | 3.1x larger (but offset by wins)
R:R Ratio     |    1.87:1     |    0.78:1     | Winners have positive expectancy
Positions     |    17         |    5          | 3.4x more diversified
Leverage      |    6.0x       |    1.0x       | 6x amplification on wins
Stop Loss     |    ~16%       |    50%        | 3x tighter
Take Profit   |    ~9%        |    100-400%   | Achievable vs. moonshot
```

---

## 7. Next Steps

### Immediate (V3 — Done)
- [x] Fix all 40 MC strategy params based on evolution data
- [x] Fix DexScreener scanner to use boost/profile endpoints
- [x] Fix market routing (MC- → meme_coins, not crypto_perps)
- [x] Fix safety test assertion

### Short-term
- [ ] Add volume-based exit to position monitor (V2 spec, not yet implemented)
- [ ] Add holder velocity tracking (V2 spec, not yet implemented)
- [ ] Claude AI token scoring integration
- [ ] Track per-strategy win/loss distribution for auto-tuning

### Medium-term
- [ ] Dev wallet monitoring via Solana RPC
- [ ] Real copy trading with wallet scoring
- [ ] A/B test tighter stops (10% vs 15% vs 20%) systematically
- [ ] Implement partial exits when evolution data supports it
