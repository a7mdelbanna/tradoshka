# Realistic Meme Coin Wallet & Live Trading System

**Date:** 2026-03-28
**Status:** Approved
**Goal:** Fix SimulatedWallet realism (compounding, exit slippage, gas), add cost visibility to dashboard, and build unified dry/live execution layer so the same code path drives both simulated and real Solana wallet trading.

**Principle:** Dry mode must produce numbers identical to what a real wallet would produce. When you flip to live mode, the only thing that changes is the transaction signer.

---

## 1. SimulatedWallet Realism Fixes

### 1.1 Position Sizing — Half-Kelly Compounding

- Before each MC trade, calculate position size using the strategy's own historical win rate and avg winner/loser ratio via the existing `HalfKellyPositionSizer` in `core/risk/`
- Input: current equity, strategy's win_rate (from `slot.win_rate()`), avg winner $ / avg loser $ (from `slot.recorder`)
- Output: optimal fraction of equity to risk per trade (Kelly criterion / 2 for safety margin)
- **Minimum 10 closed trades** before Kelly kicks in — until then, use the current fixed formula: `equity * capital_usage_pct / auto_position_count * leverage`
- **Cap at 5% of equity per trade** — Kelly can suggest large sizes with small sample sizes
- **Floor at 0.5% of equity per trade** — prevents Kelly from zeroing out a recovering strategy

### 1.2 Exit Slippage — Asymmetric

Applied in `SimulatedWallet::sell()` by adjusting the fill price before PnL calculation:

| Exit Type | Slippage | Rationale |
|-----------|----------|-----------|
| Stop-loss hit | 2.5% | Selling into a dump, thin order books, panic selling |
| Take-profit hit | 0.75% | Selling into a pump, deep books, buyers stacked |
| Time-stop exit | 1.5% | Neutral conditions, average liquidity |

The exit type is determined by which condition triggered the close in the position monitor.

Slippage is tracked separately as `exit_slippage_total` in SimulatedWallet for dashboard display.

### 1.3 Solana Gas Fee Simulation

Per transaction (entry or exit):
- Base Solana fee: $0.001 (5000 lamports, ~fixed)
- Jito tip estimate: $0.15 (to match live mode costs)
- Total: ~$0.151 per transaction

Tracked as `gas_fees` in SimulatedWallet, separate from trading fees and slippage.

### 1.4 Fee Tracking Fields (SimulatedWallet)

```rust
pub struct SimulatedWallet {
    // existing fields...
    trading_fees: Decimal,      // 0.3% per trade (existing total_fees, renamed for clarity)
    entry_slippage: Decimal,    // 1% on buys (already applied, now tracked)
    exit_slippage: Decimal,     // 0.75-2.5% on sells (new)
    gas_fees: Decimal,          // $0.151 per tx (new)
}
```

All four cost types deducted from balance. `equity()` = balance + unrealized PnL (unchanged).

---

## 2. Unified Dry/Live Execution Layer

### 2.1 TradeExecutor Trait

```rust
#[async_trait]
pub trait TradeExecutor: Send + Sync {
    async fn buy(&self, token_address: &str, amount_sol: Decimal, slippage_bps: u16) -> Result<TradeResult>;
    async fn sell(&self, token_address: &str, token_amount: Decimal, slippage_bps: u16) -> Result<TradeResult>;
    async fn get_sol_balance(&self) -> Result<Decimal>;
    fn mode(&self) -> TradingMode;
}

pub struct TradeResult {
    pub fill_price: Decimal,
    pub filled_amount: Decimal,
    pub fee: Decimal,
    pub slippage_cost: Decimal,
    pub gas_cost: Decimal,
    pub tx_signature: Option<String>,  // None in dry mode, real sig in live
}

pub enum TradingMode { Dry, Live }
```

### 2.2 DryExecutor

- Wraps `SimulatedWallet` (improved with Kelly sizing, asymmetric slippage, gas)
- Gets prices from DexScreener API (same as current)
- Returns `TradeResult` with simulated values, `tx_signature: None`

### 2.3 LiveExecutor

- Loads Solana keypair from `SOLANA_PRIVATE_KEY` env var at startup
- Key format: base58-encoded private key (standard Phantom/Solflare export format)
- Key held in memory only, zeroed on shutdown, never logged or serialized

**Buy transaction flow:**
1. `GET https://quote-api.jup.ag/v6/quote?inputMint=So11111111111111111111111111111111&outputMint={token}&amount={lamports}&slippageBps={bps}`
2. `POST https://quote-api.jup.ag/v6/swap` with quote result + user public key
3. Deserialize returned transaction
4. Sign with loaded keypair
5. Wrap in Jito bundle: `POST https://mainnet.block-engine.jito.wtf/api/v1/bundles` with tip (0.001 SOL)
6. Poll for confirmation via RPC (max 30 seconds, 1-second intervals)
7. Parse transaction logs for actual fill price and amounts
8. Return `TradeResult` with real values and `tx_signature: Some(sig)`

**Sell transaction flow:** Same pipeline, reversed mints (token → SOL).

**Error handling:**
- Transaction fails → retry once after 2 seconds
- Second failure → skip trade, log error, alert dashboard
- Insufficient SOL balance → pause live trading, notify user
- RPC timeout → fall back to backup RPC endpoint (configurable)

**Rate limiting:**
- Jupiter: 600 req/min free tier (sufficient for 40 strategies at 5-min cycles)
- Jito: space bundles 500ms apart
- Solana RPC: use paid endpoint (Helius/QuickNode) for live mode reliability

### 2.4 Mode Configuration

```rust
// config.rs
pub struct TradingConfig {
    pub mode: TradingMode,                    // from TRADOSHKA_MODE env (default: "dry")
    pub solana_private_key: Option<String>,    // from SOLANA_PRIVATE_KEY env
    pub solana_rpc_url: String,               // from SOLANA_RPC_URL env (default: public mainnet)
    pub jito_block_engine_url: String,        // from JITO_URL env (default: mainnet.block-engine.jito.wtf)
    pub max_position_usd: Decimal,            // from MAX_POSITION_USD env (default: $100)
    pub max_daily_loss_usd: Decimal,          // from MAX_DAILY_LOSS env (default: $50)
}
```

- `TRADOSHKA_MODE=live` requires `SOLANA_PRIVATE_KEY` to be set — startup fails without it
- Dashboard displays mode badge: "DRY MODE" (orange) or "LIVE MODE" (red, pulsing)
- LiveMode starts **paused** — user must click "Enable Live Trading" on dashboard

---

## 3. Dashboard Cost Visibility

### 3.1 Wallet Picker Dropdown (Summary)

Each strategy entry shows costs inline:

```
MC-TR-aggressive — $4,962 (1244 trades)
  Costs: $136 fees | $89 slippage | $0.19 gas
```

### 3.2 Strategy Detail — Costs Card

New card in the strategy detail view, alongside Unrealized/Realized/Drawdown cards:

```
TRADING COSTS
  Trading Fees     $136.04    (0.3% per trade)
  Entry Slippage   $ 52.18    (1.0% on buys)
  Exit Slippage    $ 89.33    (0.75-2.5% on sells)
  Gas Fees         $  0.19    (Solana tx + Jito tips)
  ─────────────────────────────
  Total Costs      $277.74
  Cost/Gross PnL   5.3%       [GREEN]
```

Color coding:
- Green: costs < 20% of gross profit (healthy)
- Yellow: 20-40% (fee drag significant)
- Red: > 40% (fees eating most profit)

### 3.3 API Endpoint

`GET /api/evolution/wallet/{name}` response adds:

```json
{
    "trading_fees": "136.04",
    "entry_slippage": "52.18",
    "exit_slippage": "89.33",
    "gas_fees": "0.19",
    "total_costs": "277.74",
    "gross_pnl": "5250.73",
    "cost_pct": 5.3
}
```

---

## 4. Safety & Security

### 4.1 Private Key Protection

- Loaded from env var once at startup, stored in memory as `[u8; 64]`
- Never logged (`tracing` filters any field named `*key*` or `*secret*`)
- Never serialized to JSON, database, or config files
- Never sent to any external API (only used for local transaction signing)
- Zeroed from memory on graceful shutdown
- Dashboard displays only public address + SOL balance, never the key

### 4.2 Live Mode Safeguards

| Safeguard | Behavior |
|-----------|----------|
| Max daily loss | If cumulative daily PnL < -$50 (configurable), pause all live trading |
| Max position size | Hard cap at $100 per trade (configurable via `MAX_POSITION_USD`) |
| First trade confirmation | First real transaction requires dashboard click to proceed |
| Ramp-up period | First 10 live trades capped at $5 max regardless of Kelly |
| Kill switch | Dashboard "STOP ALL LIVE TRADING" button — pauses trading, does NOT panic-sell |
| Audit log | Every live tx logged: signature, block, expected vs actual fill, latency |

### 4.3 Dry → Live Transition Path

1. Run dry mode with full realism (Kelly + slippage + gas) for minimum 24 hours
2. Dashboard shows "Ready for Live" when a strategy has 100+ trades and positive PnL
3. User sets env vars: `TRADOSHKA_MODE=live`, `SOLANA_PRIVATE_KEY=<key>`, `SOLANA_RPC_URL=<rpc>`
4. Restart server — starts in paused live state
5. User clicks "Enable Live Trading" on dashboard
6. First 10 trades capped at $5, then ramps to Kelly-sized positions (max $100)

---

## 5. Implementation Order

| Phase | What | Depends On |
|-------|------|-----------|
| 1 | Fix SimulatedWallet: Kelly compounding, exit slippage, gas simulation, fee tracking | Nothing |
| 2 | Update dashboard: cost summary in dropdown, cost detail card, API fields | Phase 1 |
| 3 | TradeExecutor trait + DryExecutor (wraps improved SimulatedWallet) | Phase 1 |
| 4 | LiveExecutor: Jupiter quote/swap, Jito bundles, Solana signing | Phase 3 |
| 5 | Safety layer: daily loss limit, ramp-up, kill switch, audit log | Phase 4 |
| 6 | Dashboard live mode: enable button, wallet display, tx signature links | Phase 5 |

---

## 6. Dependencies (All Audited, Established Libraries)

| Crate | Purpose | Stars/Status |
|-------|---------|-------------|
| `solana-sdk` | Transaction building, keypair, signing | Official Solana |
| `solana-client` | RPC communication | Official Solana |
| `spl-token` | SPL token account operations | Official Solana |
| `bs58` | Base58 encoding/decoding for keys | 100M+ downloads |
| `reqwest` | HTTP client for Jupiter/Jito APIs | Already in project |

No external repos installed. Jupiter and Jito are accessed via their public REST APIs only.
