# Realistic Wallet & Cost Visibility — Phase 1+2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix SimulatedWallet realism (Kelly compounding, asymmetric exit slippage, gas fees, granular cost tracking) and add cost visibility to the dashboard.

**Architecture:** Extend `SimulatedWallet` with 4 separate cost buckets (trading_fees, entry_slippage, exit_slippage, gas_fees). Add `ExitType` enum to drive asymmetric slippage. Add Kelly-based position sizing for MC strategies using existing `HalfKellySizer`. Expose new fields via API and display in dashboard.

**Tech Stack:** Rust (engine + API), Next.js/React (dashboard), existing `HalfKellySizer` from `core/risk/`

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `core/engine/src/wallet.rs` | Modify | Add exit slippage, gas fees, granular cost tracking fields |
| `core/engine/src/strategy_wallet.rs` | Modify | Add `kelly_position_size()` method, `avg_win_loss_ratio()` helper |
| `core/engine/src/lib.rs` | Modify | Re-export `ExitType` |
| `core/api/src/main.rs` | Modify | Wire Kelly sizing for MC trades, pass `ExitType` to sell |
| `core/api/src/evolution_routes.rs` | Modify | Add cost breakdown fields to strategy wallet API |
| `dashboard/app/(dashboard)/trading/page.tsx` | Modify | Cost summary in dropdown, cost detail card |

---

### Task 1: Add ExitType enum and granular cost tracking to SimulatedWallet

**Files:**
- Modify: `core/engine/src/wallet.rs`

- [ ] **Step 1: Write failing tests for new cost tracking fields**

Add these tests at the bottom of the existing `#[cfg(test)] mod tests` block in `wallet.rs`:

```rust
#[test]
fn test_cost_tracking_separate_buckets() {
    let mut w = SimulatedWallet::new(dec!(100), dec!(100), dec!(0.003)); // 1% slippage, 0.3% fee
    // Buy: should track entry_slippage and trading_fees
    w.buy("tok1", "Test", "Long", dec!(0.001), dec!(10000), "strat1");
    assert!(w.entry_slippage_total() > Decimal::ZERO, "entry slippage should be tracked");
    assert!(w.trading_fees_total() > Decimal::ZERO, "trading fees should be tracked");
    assert_eq!(w.exit_slippage_total(), Decimal::ZERO, "no exit slippage yet");
    assert_eq!(w.gas_fees_total(), Decimal::ZERO, "gas fees not yet implemented here");
}

#[test]
fn test_exit_slippage_applied_on_sell() {
    let mut w = SimulatedWallet::new(dec!(100), dec!(100), dec!(0.003));
    w.buy("tok1", "Test", "Long", dec!(1.00), dec!(10), "strat1");
    let balance_after_buy = w.balance();
    // Sell with stop_loss exit type (2.5% slippage)
    let result = w.sell_with_exit_type("tok1", dec!(1.00), dec!(10), "strat1", ExitType::StopLoss);
    assert!(result.is_some());
    let (fill_price, _, _) = result.unwrap();
    // Fill price should be BELOW market price due to 2.5% slippage
    assert!(fill_price < dec!(1.00), "stop loss exit should have negative slippage");
    assert!(w.exit_slippage_total() > Decimal::ZERO, "exit slippage should be tracked");
}

#[test]
fn test_exit_slippage_asymmetric() {
    // Stop loss = 2.5%, Take profit = 0.75%, Time stop = 1.5%
    let mut w1 = SimulatedWallet::new(dec!(1000), dec!(0), dec!(0.003));
    w1.buy("t1", "Q", "Long", dec!(100.0), dec!(1), "s1");
    let sl_result = w1.sell_with_exit_type("t1", dec!(100.0), dec!(1), "s1", ExitType::StopLoss);
    let sl_fill = sl_result.unwrap().0;

    let mut w2 = SimulatedWallet::new(dec!(1000), dec!(0), dec!(0.003));
    w2.buy("t1", "Q", "Long", dec!(100.0), dec!(1), "s2");
    let tp_result = w2.sell_with_exit_type("t1", dec!(100.0), dec!(1), "s2", ExitType::TakeProfit);
    let tp_fill = tp_result.unwrap().0;

    // Stop loss fill should be worse (lower) than take profit fill
    assert!(sl_fill < tp_fill, "SL fill {} should be worse than TP fill {}", sl_fill, tp_fill);
}

#[test]
fn test_gas_fee_tracking() {
    let mut w = SimulatedWallet::new(dec!(100), dec!(100), dec!(0.003));
    w.add_gas_fee(dec!(0.151));
    w.add_gas_fee(dec!(0.151));
    assert_eq!(w.gas_fees_total(), dec!(0.302));
    // Gas fees should reduce balance
    assert!(w.balance() < dec!(100));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd "C:\Users\ahmed\OneDrive\Documents\$$$$$" && export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p tradoshka-engine -- test_cost_tracking test_exit_slippage test_gas_fee 2>&1`

Expected: FAIL — `ExitType` not found, methods don't exist.

- [ ] **Step 3: Add ExitType enum and new fields to SimulatedWallet**

At the top of `wallet.rs`, after the existing imports, add:

```rust
/// What triggered a position exit — determines slippage applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExitType {
    StopLoss,    // 2.5% slippage — selling into a dump
    TakeProfit,  // 0.75% slippage — selling into a pump
    TimeStop,    // 1.5% slippage — neutral conditions
    Manual,      // 1.0% slippage — user-initiated
}

impl ExitType {
    /// Exit slippage in basis points.
    pub fn slippage_bps(&self) -> Decimal {
        match self {
            ExitType::StopLoss => dec!(250),    // 2.5%
            ExitType::TakeProfit => dec!(75),   // 0.75%
            ExitType::TimeStop => dec!(150),    // 1.5%
            ExitType::Manual => dec!(100),      // 1.0%
        }
    }
}
```

Add new fields to the `SimulatedWallet` struct (keep all existing fields):

```rust
pub struct SimulatedWallet {
    balance: Decimal,
    positions: HashMap<String, WalletPosition>,
    mode: WalletMode,
    slippage_bps: Decimal,
    peak_equity: Decimal,
    total_fees: Decimal,       // keep for backwards compat
    realized_pnl: Decimal,
    fee_rate: Decimal,
    // NEW: granular cost tracking
    entry_slippage_total: Decimal,
    exit_slippage_total: Decimal,
    gas_fees_total: Decimal,
}
```

In `SimulatedWallet::new()`, initialize the new fields to `Decimal::ZERO`.

Add accessor methods:

```rust
pub fn entry_slippage_total(&self) -> Decimal { self.entry_slippage_total }
pub fn exit_slippage_total(&self) -> Decimal { self.exit_slippage_total }
pub fn gas_fees_total(&self) -> Decimal { self.gas_fees_total }
pub fn trading_fees_total(&self) -> Decimal { self.total_fees }
pub fn total_costs(&self) -> Decimal {
    self.total_fees + self.entry_slippage_total + self.exit_slippage_total + self.gas_fees_total
}

pub fn add_gas_fee(&mut self, amount: Decimal) {
    self.gas_fees_total += amount;
    self.balance -= amount;
}
```

- [ ] **Step 4: Track entry slippage in the existing `buy()` method**

In the `buy()` method, after calculating `slippage_cost`:

```rust
// existing line: let fill_price = price + slippage_cost;
// ADD after it:
self.entry_slippage_total += (slippage_cost * shares).abs();
```

- [ ] **Step 5: Add `sell_with_exit_type()` method**

Add this new method alongside the existing `sell()` method (keep `sell()` for backwards compat):

```rust
/// Sell with exit-type-specific slippage.
pub fn sell_with_exit_type(
    &mut self,
    token_id: &str,
    price: Decimal,
    shares: Decimal,
    strategy_id: &str,
    exit_type: ExitType,
) -> Option<(Decimal, Decimal, Decimal)> {
    let key = format!("{}:{}", token_id, strategy_id);
    let position = self.positions.get(&key)?;
    let sell_shares = shares.min(position.shares);
    if sell_shares <= Decimal::ZERO { return None; }

    // Apply exit-type-specific slippage (always unfavorable for seller)
    let exit_slippage_bps = exit_type.slippage_bps();
    let slippage_factor = Decimal::ONE - (exit_slippage_bps / dec!(10000));
    let fill_price = price * slippage_factor;
    let exit_slippage_cost = (price - fill_price) * sell_shares;
    self.exit_slippage_total += exit_slippage_cost;

    // Calculate PnL from entry
    let avg_entry = position.avg_price;
    let pnl = (fill_price - avg_entry) * sell_shares;

    // Fee on exit
    let fee = (self.fee_rate * fill_price * sell_shares).abs();
    self.total_fees += fee;

    // Update balance
    self.balance += pnl - fee;
    self.realized_pnl += pnl;

    // Remove or reduce position
    if sell_shares >= position.shares {
        self.positions.remove(&key);
    } else {
        if let Some(pos) = self.positions.get_mut(&key) {
            pos.shares -= sell_shares;
        }
    }

    // Update peak equity
    let eq = self.equity();
    if eq > self.peak_equity { self.peak_equity = eq; }

    Some((fill_price, fee, pnl))
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p tradoshka-engine -- test_cost_tracking test_exit_slippage test_gas_fee 2>&1`

Expected: All 4 new tests PASS.

- [ ] **Step 7: Run full test suite**

Run: `cargo test --workspace 2>&1`

Expected: All existing tests still pass.

- [ ] **Step 8: Commit**

```bash
git add core/engine/src/wallet.rs
git commit -m "feat(wallet): add ExitType, asymmetric exit slippage, granular cost tracking"
```

---

### Task 2: Add Kelly-based position sizing to StrategySlot

**Files:**
- Modify: `core/engine/src/strategy_wallet.rs`

- [ ] **Step 1: Write failing test for Kelly position sizing**

Add to the existing test module in `strategy_wallet.rs`:

```rust
#[test]
fn test_kelly_position_size_with_history() {
    let params = StrategyParams::new("mc_trend_v2")
        .with_param("auto_position_count", 15.0)
        .with_param("capital_usage_pct", 60.0)
        .with_param("auto_leverage", 5.0);
    let mut slot = StrategySlot::new("MC-TR-test", "meme_coins", params, dec!(1000));

    // Simulate 20 closed trades: 8 wins, 12 losses
    for i in 0..20 {
        let pnl = if i < 8 { dec!(10) } else { dec!(-5) }; // avg win $10, avg loss $5, R:R = 2.0
        let mut trade = crate::trade_recorder::TradeRecord {
            id: format!("t{}", i),
            timestamp: chrono::Utc::now(),
            market: tradoshka_common::types::Market::Crypto,
            symbol: format!("tok{}", i),
            market_question: "test".into(),
            direction: "Long".into(),
            side: tradoshka_common::types::OrderSide::Buy,
            shares: dec!(100),
            price: dec!(1.0),
            fee: dec!(0.01),
            strategy_id: "MC-TR-test".into(),
            signal_strength: 0.0,
            edge_vs_market: 0.0,
            pnl: Some(pnl),
            is_closed: true,
            thesis_reasoning: String::new(),
            stop_loss: dec!(0.80),
            trailing_stop: dec!(0.90),
            take_profit: dec!(1.20),
            time_stop_hours: 1,
            thesis_invalidation: String::new(),
            risk_amount: dec!(5.0),
            reward_risk_ratio: 2.0,
            strategy_tier: "Unproven".into(),
            close_reason: None,
        };
        slot.recorder.record(trade);
    }

    let size = slot.kelly_position_size(dec!(1.0));
    // With 40% WR, R:R 2.0: Kelly = (0.4*2 - 0.6)/2 = 0.2/2 = 0.1, half = 0.05
    // 5% of $1000 equity = $50 per position
    assert!(size > Decimal::ZERO, "Kelly should produce positive size");
    assert!(size <= dec!(50), "Size should be capped at 5% of equity: got {}", size);
    assert!(size >= dec!(5), "Size should be at least 0.5% of equity: got {}", size);
}

#[test]
fn test_kelly_falls_back_before_10_trades() {
    let params = StrategyParams::new("mc_trend_v2")
        .with_param("auto_position_count", 15.0)
        .with_param("capital_usage_pct", 60.0)
        .with_param("auto_leverage", 5.0);
    let slot = StrategySlot::new("MC-TR-new", "meme_coins", params, dec!(100));

    let size = slot.kelly_position_size(dec!(0.001));
    // No trades yet → fallback to fixed formula
    // $100 * 60% / 15 * 5 = $20
    let expected = dec!(100) * dec!(0.60) / dec!(15) * dec!(5);
    assert!((size - expected).abs() < dec!(1), "Before 10 trades, should use fixed formula. Got {} expected ~{}", size, expected);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p tradoshka-engine -- test_kelly_position 2>&1`

Expected: FAIL — `kelly_position_size` method doesn't exist.

- [ ] **Step 3: Implement `avg_win_loss_ratio()` and `kelly_position_size()` on StrategySlot**

Add these methods to the `impl StrategySlot` block in `strategy_wallet.rs`:

```rust
/// Average win/loss ratio from closed trades. Returns 0.0 if insufficient data.
pub fn avg_win_loss_ratio(&self) -> f64 {
    let trades = self.recorder.all_trades();
    let wins: Vec<f64> = trades.iter()
        .filter(|t| t.is_closed && t.pnl.map_or(false, |p| p > Decimal::ZERO))
        .filter_map(|t| t.pnl?.to_f64())
        .collect();
    let losses: Vec<f64> = trades.iter()
        .filter(|t| t.is_closed && t.pnl.map_or(false, |p| p < Decimal::ZERO))
        .filter_map(|t| t.pnl?.to_f64().map(|v| v.abs()))
        .collect();
    if wins.is_empty() || losses.is_empty() { return 0.0; }
    let avg_win = wins.iter().sum::<f64>() / wins.len() as f64;
    let avg_loss = losses.iter().sum::<f64>() / losses.len() as f64;
    if avg_loss <= 0.0 { return 0.0; }
    avg_win / avg_loss
}

/// Calculate position size in USD using Half-Kelly criterion.
/// Falls back to fixed formula if fewer than 10 closed trades.
/// Caps at 5% of equity, floors at 0.5% of equity.
pub fn kelly_position_size(&self, token_price: Decimal) -> Decimal {
    let equity = self.wallet.equity();
    if equity <= Decimal::ZERO || token_price <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    let closed = self.recorder.closed_trade_count();

    // Fallback: fixed formula before enough data for Kelly
    if closed < 10 {
        let cap_pct = Decimal::from_f64(self.params.get("capital_usage_pct") / 100.0)
            .unwrap_or(dec!(0.60));
        let pos_count = Decimal::from_f64(self.params.get("auto_position_count").max(1.0))
            .unwrap_or(dec!(15));
        let leverage = Decimal::from_f64(self.params.get("auto_leverage").max(1.0))
            .unwrap_or(dec!(5));
        return equity * cap_pct / pos_count * leverage;
    }

    // Kelly calculation
    let wr = self.win_rate();
    let wl_ratio = self.avg_win_loss_ratio();
    if wl_ratio <= 0.0 || wr <= 0.0 {
        return equity * dec!(0.005); // Floor: 0.5% of equity
    }

    let loss_rate = 1.0 - wr;
    let kelly = (wr * wl_ratio - loss_rate) / wl_ratio;
    if kelly <= 0.0 {
        return equity * dec!(0.005); // Negative Kelly → minimum size
    }
    let half_kelly = kelly / 2.0;
    let hk_dec = Decimal::from_f64(half_kelly).unwrap_or(dec!(0.01));

    let position_usd = (equity * hk_dec)
        .max(equity * dec!(0.005))   // floor: 0.5% of equity
        .min(equity * dec!(0.05));   // cap: 5% of equity

    position_usd
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p tradoshka-engine -- test_kelly 2>&1`

Expected: Both Kelly tests PASS.

- [ ] **Step 5: Run full test suite**

Run: `cargo test --workspace 2>&1`

Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add core/engine/src/strategy_wallet.rs
git commit -m "feat(strategy): add Kelly-based position sizing with fallback"
```

---

### Task 3: Export ExitType from engine crate

**Files:**
- Modify: `core/engine/src/lib.rs`

- [ ] **Step 1: Add ExitType to the wallet re-export line**

Change:
```rust
pub use wallet::{SimulatedWallet, WalletPosition, WalletMode};
```
To:
```rust
pub use wallet::{SimulatedWallet, WalletPosition, WalletMode, ExitType};
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p tradoshka-engine 2>&1`

Expected: Compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add core/engine/src/lib.rs
git commit -m "feat: export ExitType from engine crate"
```

---

### Task 4: Wire Kelly sizing and exit slippage into MC trading loop

**Files:**
- Modify: `core/api/src/main.rs`

- [ ] **Step 1: Replace fixed MC position sizing with Kelly**

Find the MC position sizing block (around line 1129-1143 in main.rs). The current code calculates:

```rust
let per_position = if strategy_type == "mc_trend_v2" || strategy_type == "mc_copy_v2" {
    (equity * capital_pct_dec / positions_dec).min(dec!(20))
} else { ... };
```

Replace that entire block with:

```rust
// Kelly-based position sizing (compounds with equity)
let position_usd = slot.kelly_position_size(price);
// Convert USD to shares
let size = if price > Decimal::ZERO {
    (position_usd / price).round_dp(0)
} else { continue };
if size <= Decimal::ZERO { continue; }
```

Remove the old `per_position` variable and the separate `size` calculation below it.

- [ ] **Step 2: Add gas fee on MC buy**

After the `slot.wallet.buy(...)` call succeeds (inside the `if let Some(...)` block), add:

```rust
// Simulate Solana gas + Jito tip
slot.wallet.add_gas_fee(dec!(0.151));
```

- [ ] **Step 3: Replace `sell()` with `sell_with_exit_type()` in MC position monitor**

Find the MC position monitor section (around line 1270-1370). Where positions are closed, there are several exit conditions. Update each:

For **hard stop** exits (the `pnl_pct <= -(hard_stop_pct)` branch):
```rust
// Change: slot.wallet.sell(...)
// To:
slot.wallet.sell_with_exit_type(&token_id, current_price, pos_shares, slot_name, tradoshka_engine::ExitType::StopLoss)
```

For **time stop** exits (the `mins_held >= time_limit_mins` branch):
```rust
slot.wallet.sell_with_exit_type(&token_id, current_price, pos_shares, slot_name, tradoshka_engine::ExitType::TimeStop)
```

For **take profit** exits (the `pnl_pct >= target_pct` branch, if it exists):
```rust
slot.wallet.sell_with_exit_type(&token_id, current_price, pos_shares, slot_name, tradoshka_engine::ExitType::TakeProfit)
```

After each sell call, add gas fee:
```rust
slot.wallet.add_gas_fee(dec!(0.151));
```

- [ ] **Step 4: Build and run tests**

Run: `cargo build --workspace && cargo test --workspace 2>&1`

Expected: Compiles and all tests pass.

- [ ] **Step 5: Commit**

```bash
git add core/api/src/main.rs
git commit -m "feat(mc): wire Kelly sizing + exit slippage + gas fees into trading loop"
```

---

### Task 5: Add cost breakdown fields to evolution API

**Files:**
- Modify: `core/api/src/evolution_routes.rs`

- [ ] **Step 1: Add cost fields to `get_strategy_wallet` response**

In the `get_strategy_wallet` function, find the `Json(serde_json::json!({` block that builds the response. Add these fields after the existing `"total_fees"` field:

```rust
"trading_fees": slot.wallet.trading_fees_total().to_string(),
"entry_slippage": slot.wallet.entry_slippage_total().to_string(),
"exit_slippage": slot.wallet.exit_slippage_total().to_string(),
"gas_fees": slot.wallet.gas_fees_total().to_string(),
"total_costs": slot.wallet.total_costs().to_string(),
"gross_pnl": (slot.wallet.realized_pnl() + slot.wallet.total_costs()).to_string(),
"cost_pct": {
    let gross = (slot.wallet.realized_pnl() + slot.wallet.total_costs()).to_f64().unwrap_or(0.0);
    if gross > 0.0 {
        (slot.wallet.total_costs().to_f64().unwrap_or(0.0) / gross * 100.0 * 100.0).round() / 100.0
    } else { 0.0 }
},
```

- [ ] **Step 2: Add cost summary to leaderboard SlotSnapshot**

In the `get_leaderboard` function, add to the `SlotSnapshot` struct:

```rust
total_costs: f64,
```

And in the mapping:
```rust
total_costs: sl.wallet.total_costs().to_f64().unwrap_or(0.0),
```

And in the JSON output:
```rust
"total_costs": r.total_costs,
```

- [ ] **Step 3: Build and verify**

Run: `cargo build --workspace 2>&1`

Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add core/api/src/evolution_routes.rs
git commit -m "feat(api): add granular cost breakdown to strategy wallet endpoint"
```

---

### Task 6: Dashboard — Cost summary in wallet dropdown

**Files:**
- Modify: `dashboard/app/(dashboard)/trading/page.tsx`

- [ ] **Step 1: Add cost display to strategy dropdown options**

Find the `<select>` dropdown for strategy wallets (the `marketStrategies.map` block). Change the `<option>` content from:

```tsx
{s.name ?? s.id} — ${((s.total_pnl ?? s.pnl ?? 0)).toFixed(2)} ({s.trades ?? 0} trades)
```

To:

```tsx
{s.name ?? s.id} — ${((s.total_pnl ?? s.pnl ?? 0)).toFixed(2)} ({s.trades ?? 0} trades) | Costs: ${(s.total_costs ?? 0).toFixed(2)}
```

- [ ] **Step 2: Add Costs card to strategy detail view**

Find the portfolio section where "UNREALIZED", "REALIZED", and "MAX DRAWDOWN" cards are displayed (after the BALANCE/POSITIONS row). Add a new card after MAX DRAWDOWN:

```tsx
{/* Trading Costs Breakdown */}
{activeWallet && selectedStrategy && (
  <div className="mt-4 p-4 bg-slate-800/50 rounded-xl border border-slate-700/50">
    <h4 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">Trading Costs</h4>
    <div className="grid grid-cols-2 gap-3 text-sm">
      <div>
        <span className="text-slate-500">Trading Fees</span>
        <p className="text-slate-200 font-mono">${parseFloat(activeWallet.trading_fees || '0').toFixed(2)}</p>
      </div>
      <div>
        <span className="text-slate-500">Entry Slippage</span>
        <p className="text-slate-200 font-mono">${parseFloat(activeWallet.entry_slippage || '0').toFixed(2)}</p>
      </div>
      <div>
        <span className="text-slate-500">Exit Slippage</span>
        <p className="text-slate-200 font-mono">${parseFloat(activeWallet.exit_slippage || '0').toFixed(2)}</p>
      </div>
      <div>
        <span className="text-slate-500">Gas Fees</span>
        <p className="text-slate-200 font-mono">${parseFloat(activeWallet.gas_fees || '0').toFixed(2)}</p>
      </div>
    </div>
    <div className="mt-3 pt-3 border-t border-slate-700/50 flex justify-between items-center">
      <div>
        <span className="text-slate-500 text-sm">Total Costs</span>
        <p className="text-white font-semibold font-mono">${parseFloat(activeWallet.total_costs || '0').toFixed(2)}</p>
      </div>
      <div className="text-right">
        <span className="text-slate-500 text-sm">Cost / Gross PnL</span>
        <p className={`font-semibold font-mono ${
          (activeWallet.cost_pct ?? 0) < 20 ? 'text-emerald-400' :
          (activeWallet.cost_pct ?? 0) < 40 ? 'text-yellow-400' : 'text-red-400'
        }`}>
          {(activeWallet.cost_pct ?? 0).toFixed(1)}%
        </p>
      </div>
    </div>
  </div>
)}
```

- [ ] **Step 3: Verify dashboard renders**

Open `http://localhost:3000/trading?market=memecoins`, select a strategy from the dropdown, and verify the costs card appears.

- [ ] **Step 4: Commit**

```bash
git add dashboard/app/(dashboard)/trading/page.tsx
git commit -m "feat(dashboard): add cost breakdown card and summary to strategy wallet"
```

---

### Task 7: Integration test — verify compounding + costs end-to-end

**Files:**
- No new files — uses running server

- [ ] **Step 1: Rebuild and restart server**

```bash
taskkill //F //IM tradoshka.exe 2>/dev/null
sleep 2
cargo build --release
cargo run --release &
sleep 20
```

- [ ] **Step 2: Verify Kelly compounding is working**

After 5 minutes of trading, check that MC position sizes scale with equity:

```bash
curl -s http://localhost:3001/api/evolution/wallet/MC-TR-aggressive | python -c "
import json, sys
data = json.load(sys.stdin)
equity = float(data['equity'])
trades = data.get('trades_list', [])
if trades:
    recent_size = float(trades[0]['shares']) * float(trades[0]['price'])
    print(f'Equity: \${equity:.2f}')
    print(f'Recent position size: \${recent_size:.2f}')
    print(f'Position as % of equity: {recent_size/equity*100:.1f}%')
    print(f'Should be 0.5-5% of equity (Kelly range)')
"
```

Expected: Position size is between 0.5% and 5% of current equity (not fixed at $20).

- [ ] **Step 3: Verify cost fields in API response**

```bash
curl -s http://localhost:3001/api/evolution/wallet/MC-TR-aggressive | python -c "
import json, sys
data = json.load(sys.stdin)
for field in ['trading_fees', 'entry_slippage', 'exit_slippage', 'gas_fees', 'total_costs', 'cost_pct']:
    print(f'{field}: {data.get(field, \"MISSING\")}')
"
```

Expected: All 6 cost fields present with non-zero values.

- [ ] **Step 4: Verify dashboard shows costs**

Open browser to `http://localhost:3000/trading?market=memecoins`, select MC-TR-aggressive, verify the "Trading Costs" card is visible with 4 cost rows + total + percentage.

- [ ] **Step 5: Final commit with all files**

```bash
git add -A
git commit -m "feat: realistic wallet phase 1+2 complete — Kelly compounding, exit slippage, gas fees, cost dashboard"
git push origin dev
```

---

## Spec Coverage Check

| Spec Requirement | Task |
|-----------------|------|
| Half-Kelly compounding | Task 2 (implementation), Task 4 (wiring) |
| Min 10 trades before Kelly | Task 2 (fallback logic) |
| Cap 5% / floor 0.5% | Task 2 (min/max clamps) |
| Asymmetric exit slippage (2.5/0.75/1.5%) | Task 1 (ExitType + sell_with_exit_type) |
| Slippage tracked separately | Task 1 (entry_slippage_total, exit_slippage_total) |
| Gas fee simulation ($0.151/tx) | Task 1 (add_gas_fee), Task 4 (wired to buy/sell) |
| 4 cost buckets (fees/entry_slip/exit_slip/gas) | Task 1 (fields), Task 5 (API) |
| Cost summary in dropdown | Task 6 (dropdown text) |
| Cost detail card in strategy view | Task 6 (costs card) |
| Color-coded cost/PnL ratio | Task 6 (green/yellow/red) |
| API cost fields | Task 5 (6 new fields) |
| Position sizing compounds with equity | Task 2 + Task 4 |
