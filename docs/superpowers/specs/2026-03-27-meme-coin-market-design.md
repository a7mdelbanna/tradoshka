# Meme Coin Market — Solana DEX Trading with Early Detection, Trend Riding & Whale Copy

**Date:** 2026-03-27
**Status:** Approved
**Version:** V1
**Goal:** Add meme coins as a new market on Solana DEX (Raydium/Jupiter) with 40 strategy wallets: 15 early detection, 15 trend riding, 10 whale copy. Ladder exits, rug pull protection, and self-improving evolution.

---

## 1. Solana DEX Adapter

### New Crate: `markets/memecoins/`

```
markets/memecoins/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── config.rs              # Jupiter/Raydium API URLs, Solana RPC
    ├── client.rs              # REST client (Jupiter price, Raydium pools)
    ├── token_scanner.rs       # New token detection, trending, volume surges
    ├── types.rs               # MemeToken, PoolInfo, TokenLaunch, SafetyScore
    ├── safety.rs              # Rug pull protection (6-point filter)
    ├── whale_tracker.rs       # Track Solana meme whale wallets
    └── adapter.rs             # MarketAdapter trait implementation
```

### Data Sources (free, no wallet needed for dry mode)

| Source | What | Endpoint |
|--------|------|----------|
| Jupiter Price API | Real-time token prices | `https://price.jup.ag/v4/price` |
| Jupiter Token List | Known tokens | `https://token.jup.ag/all` |
| Raydium API | New pools, volume | `https://api-v3.raydium.io/` |
| DexScreener | Trending, top gainers | `https://api.dexscreener.com/latest/dex/tokens/` |
| Birdeye | Token analytics | `https://public-api.birdeye.so/` |
| Solana RPC | On-chain events | Public RPC endpoints |

### Key Adapter Behaviors

- Scan for NEW tokens every 60 seconds
- Track volume surges (2x+ in 5 minutes = signal)
- Get real-time prices for all tracked tokens
- Maintain a list of trending meme coins (top 50 by volume)

---

## 2. Strategy Groups (40 total, $100 each = $4,000)

### Group A: Early Detection / Snipe (15) — prefix `MC-ED-`

Find new tokens within minutes of launch, buy early, sell the pump.

| Strategy | Params | Target |
|----------|--------|--------|
| MC-ED-pump-instant | buy < 60s of launch | 2x |
| MC-ED-pump-confirmed | wait for 5+ buys first | 3x |
| MC-ED-pool-fresh | new Raydium pools < 5min | 2x |
| MC-ED-pool-volume | new pools when volume > $10K | 2x |
| MC-ED-mcap-micro | tokens under $10K mcap | 5x |
| MC-ED-mcap-small | tokens under $50K mcap | 3x |
| MC-ED-liquidity-lock | only if liquidity locked | 3x |
| MC-ED-dev-clean | only if dev wallet < 5% | 2x |
| MC-ED-social-mention | first social media spike | 2x |
| MC-ED-multi-buy | 10+ unique buyers in 2 min | 2x |
| MC-ED-fast-flip | 5min max hold, > 20% profit | any |
| MC-ED-slow-flip | 30min hold | 3x |
| MC-ED-conservative | strict filters | 2x |
| MC-ED-aggressive | loose filters, high risk | 5x |
| MC-ED-explorer | random params, evolution | varies |

### Group B: Trend Riding (15) — prefix `MC-TR-`

Find tokens already trending, ride the wave.

| Strategy | Params | Target |
|----------|--------|--------|
| MC-TR-volume-surge | 24h volume 5x above avg | 2x |
| MC-TR-volume-mega | volume crosses $1M/24h | 2x |
| MC-TR-price-breakout | price breaks 1h high by 20%+ | 2x |
| MC-TR-momentum-fast | EMA 3/8 on 5m candles | 2x |
| MC-TR-momentum-slow | EMA 9/21 on 15m candles | 2x |
| MC-TR-rsi-bounce | RSI bounces 30→40 | 2x |
| MC-TR-dip-buy | 30%+ dips on high-vol tokens | 2x |
| MC-TR-social-trending | trending on Twitter/Reddit | 2x |
| MC-TR-dexscreener-hot | top gainers from DexScreener | 2x |
| MC-TR-multi-timeframe | confirm on 5m + 15m + 1h | 3x |
| MC-TR-mean-revert | oversold memes (RSI < 20) | 2x |
| MC-TR-grid-volatile | grid trading on volatile memes | spread |
| MC-TR-conservative | only > $100K mcap | 2x |
| MC-TR-aggressive | any trending token | 3x |
| MC-TR-explorer | random params, evolution | varies |

### Group C: Whale Copy (10) — prefix `MC-WC-`

Track known Solana meme coin whales.

| Strategy | Params |
|----------|--------|
| MC-WC-top-pnl | copy top 10 by PnL |
| MC-WC-high-wr | copy > 60% win rate wallets |
| MC-WC-early-buyer | copy wallets that buy within 5min of launch |
| MC-WC-whale-large | copy > $50K portfolio wallets |
| MC-WC-whale-small | copy $5-20K smart money |
| MC-WC-consensus | only when 3+ whales buy same token |
| MC-WC-contrarian | fade whale sells (buy the dip) |
| MC-WC-fast-follow | copy within 30s |
| MC-WC-delayed | wait 2min after whale (avoid front-run) |
| MC-WC-explorer | random copy params |

---

## 3. Ladder Exit System

Every meme coin position uses a 3-stage exit:

```
ENTRY: Full position at once

EXIT LADDER:
  Stage 1: Sell 33% at 2x (recover initial investment)
  Stage 2: Sell 33% at 5x (take profit)
  Stage 3: Hold 33% with 50% trailing stop (moonshot runner)

HARD STOP: -30% from entry (cut losses immediately)

TIME STOPS (per group):
  Group A (Early Detection): 15 minutes
  Group B (Trend Riding): 1 hour
  Group C (Whale Copy): 30 minutes
```

If time stop hits and position is profitable → sell at market (lock gains).
If time stop hits and position is losing → sell at market (cut loss).

---

## 4. Rug Pull Protection (6-Point Safety Filter)

Every trade must pass ALL checks before buying:

| # | Check | Requirement | Bypass |
|---|-------|-------------|--------|
| 1 | Liquidity | Pool has >= $5K liquidity | None |
| 2 | Dev Wallet | Deployer holds < 10% of supply | None |
| 3 | Whale Concentration | No single wallet > 15% | Group A aggressive only |
| 4 | Mint Authority | Revoked (can't print more) | Group A accepts active |
| 5 | Honeypot | Can actually sell the token | None |
| 6 | Tax/Fee | Buy + sell tax < 10% each | None |

**Blacklist:** Known rug deployer addresses get instant-rejected. Tokens that lose 95%+ in < 10 min → flag deployer permanently.

**Safety score (0-100):** Each token gets a composite safety score. Strategies use this to filter:
- Conservative strategies: only buy safety > 80
- Moderate strategies: safety > 50
- Aggressive strategies: safety > 20

---

## 5. Self-Improving Loop

### Cycle Frequencies

| Interval | Action |
|----------|--------|
| 60 seconds | Scan for new tokens (Group A) |
| 5 minutes | Check trending tokens (Group B), whale activity (Group C), update prices, check exits |
| 30 minutes | Recalculate trending list, update whale scores, log data |
| 1 hour | Evolution cycle: per-group rank/kill/spawn |

### Evolution (Per Group)

- Kill bottom 10% within each group (min 5 alive per group)
- Spawn: 40% mutation, 40% crossover, 20% random explorer
- Self-adjust: winners get wider params, losers get tightened

### Data Persistence

Every token encountered saves to `data/memecoins/`:

```json
{
  "token": "PEPE2SOL",
  "chain": "solana",
  "first_seen": "2026-03-27T15:00:00Z",
  "initial_mcap": 8500,
  "peak_mcap": 450000,
  "current_mcap": 12000,
  "time_to_peak_mins": 22,
  "liquidity_at_launch": 5200,
  "dev_wallet_pct": 3.2,
  "mint_revoked": true,
  "rug_pulled": false,
  "safety_score": 85,
  "our_trades": [
    {
      "strategy": "MC-ED-pump-confirmed",
      "entry_price": 0.00000012,
      "exit_prices": [0.00000024, 0.00000060, 0.00000045],
      "pnl": "+283%",
      "hold_duration_mins": 18
    }
  ]
}
```

---

## 6. Dashboard Integration

### Sidebar
Add "Meme Coins" under MARKETS (with fire emoji or rocket icon)

### Trading Page
New market tab: `[Polymarket] [Crypto] [Meme Coins]`
With sub-tabs: `[Early Detection] [Trend Riding] [Whale Copy]`

### Evolution Page
New filter: `[All] [Polymarket] [Crypto Spot] [Crypto Perps] [Meme Coins]`
With sub-filters: `[All MC] [Early Detection] [Trend Riding] [Whale Copy]`

---

## 7. Build Order

| Phase | What |
|-------|------|
| 1 | Solana DEX adapter (Jupiter/DexScreener client, token scanner, types) |
| 2 | Safety filter (rug protection, honeypot check, blacklist) |
| 3 | 40 strategy definitions + ladder exit system |
| 4 | Wire into trading loop + evolution engine |
| 5 | Dashboard integration (sidebar, trading tab, evolution filter) |
| 6 | Integration test (scan real tokens, dry mode trades) |
