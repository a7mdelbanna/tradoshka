# Phase 2A: Market Data Service + Simulated Wallet — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the real-time Polymarket data service that pulls live prices and a simulated wallet that tracks positions/P&L with realistic fees — the foundation for the dry mode trading loop.

**Architecture:** Market Data Service lives in the engine crate, uses the existing Polymarket client (REST + WebSocket) to pull real prices, and emits `MarketEvent` objects. The Simulated Wallet extends the existing `PortfolioTracker` with Polymarket-specific fee simulation, position resolution, and trade recording. Both integrate with the existing API server via `AppState`.

**Tech Stack:** Rust (tokio, rust_decimal, chrono, serde), existing tradoshka-polymarket crate for API calls

---

## Prerequisites

- Phase 1 complete (core engine + Polymarket adapter on `dev` branch)
- `export PATH="$HOME/.cargo/bin:$PATH"` before any cargo command

---

## File Structure

```
core/engine/src/
├── lib.rs                   # Add new modules
├── wallet.rs                # NEW: Simulated wallet with Polymarket fees
├── trade_recorder.rs        # NEW: Persists trade history with metadata
├── readiness.rs             # NEW: Production readiness score calculator
├── market_data.rs           # NEW: Polymarket real-time data service
├── order_manager.rs         # EXISTING
├── portfolio.rs             # EXISTING
└── dry_mode.rs              # EXISTING (minor update for Polymarket fees)
```

---

### Task 1: Trade Recorder

**Files:**
- Create: `core/engine/src/trade_recorder.rs`
- Modify: `core/engine/src/lib.rs`

The trade recorder stores every trade with full metadata. This is needed by the wallet and readiness scorer.

- [ ] **Step 1: Implement TradeRecord and TradeRecorder**

Create `core/engine/src/trade_recorder.rs`:

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tradoshka_common::types::{OrderSide, Market};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub market: Market,
    pub symbol: String,
    pub market_question: String,
    pub direction: String,
    pub side: OrderSide,
    pub shares: Decimal,
    pub price: Decimal,
    pub fee: Decimal,
    pub strategy_id: String,
    pub signal_strength: f64,
    pub edge_vs_market: f64,
    pub pnl: Option<Decimal>,
    pub is_closed: bool,
}

pub struct TradeRecorder {
    trades: Vec<TradeRecord>,
    first_trade_at: Option<DateTime<Utc>>,
}

impl TradeRecorder {
    pub fn new() -> Self {
        Self {
            trades: Vec::new(),
            first_trade_at: None,
        }
    }

    pub fn record(&mut self, trade: TradeRecord) {
        if self.first_trade_at.is_none() {
            self.first_trade_at = Some(trade.timestamp);
        }
        self.trades.push(trade);
    }

    pub fn close_trade(&mut self, symbol: &str, strategy_id: &str, pnl: Decimal) {
        for trade in self.trades.iter_mut().rev() {
            if trade.symbol == symbol && trade.strategy_id == strategy_id && !trade.is_closed {
                trade.pnl = Some(pnl);
                trade.is_closed = true;
                break;
            }
        }
    }

    pub fn all_trades(&self) -> &[TradeRecord] {
        &self.trades
    }

    pub fn closed_trades(&self) -> Vec<&TradeRecord> {
        self.trades.iter().filter(|t| t.is_closed).collect()
    }

    pub fn open_trades(&self) -> Vec<&TradeRecord> {
        self.trades.iter().filter(|t| !t.is_closed).collect()
    }

    pub fn recent_trades(&self, limit: usize) -> Vec<&TradeRecord> {
        self.trades.iter().rev().take(limit).collect()
    }

    pub fn trades_by_strategy(&self, strategy_id: &str) -> Vec<&TradeRecord> {
        self.trades.iter().filter(|t| t.strategy_id == strategy_id).collect()
    }

    pub fn first_trade_at(&self) -> Option<DateTime<Utc>> {
        self.first_trade_at
    }

    pub fn total_trade_count(&self) -> usize {
        self.trades.len()
    }

    pub fn closed_trade_count(&self) -> usize {
        self.trades.iter().filter(|t| t.is_closed).count()
    }

    pub fn winning_trade_count(&self) -> usize {
        self.trades.iter()
            .filter(|t| t.is_closed && t.pnl.map_or(false, |p| p > Decimal::ZERO))
            .count()
    }

    pub fn gross_profit(&self) -> Decimal {
        self.trades.iter()
            .filter_map(|t| t.pnl)
            .filter(|p| *p > Decimal::ZERO)
            .sum()
    }

    pub fn gross_loss(&self) -> Decimal {
        self.trades.iter()
            .filter_map(|t| t.pnl)
            .filter(|p| *p < Decimal::ZERO)
            .sum::<Decimal>()
            .abs()
    }

    pub fn daily_pnl(&self) -> Vec<(String, Decimal)> {
        use std::collections::BTreeMap;
        let mut daily: BTreeMap<String, Decimal> = BTreeMap::new();
        for trade in &self.trades {
            if let Some(pnl) = trade.pnl {
                let date = trade.timestamp.format("%Y-%m-%d").to_string();
                *daily.entry(date).or_insert(Decimal::ZERO) += pnl;
            }
        }
        daily.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_trade(symbol: &str, strategy: &str, pnl: Option<Decimal>, closed: bool) -> TradeRecord {
        TradeRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            market: Market::Polymarket,
            symbol: symbol.into(),
            market_question: "Test?".into(),
            direction: "YES".into(),
            side: OrderSide::Buy,
            shares: dec!(10),
            price: dec!(0.50),
            fee: dec!(0.05),
            strategy_id: strategy.into(),
            signal_strength: 0.8,
            edge_vs_market: 0.05,
            pnl,
            is_closed: closed,
        }
    }

    #[test]
    fn test_record_and_retrieve() {
        let mut recorder = TradeRecorder::new();
        recorder.record(make_trade("tok1", "ai", None, false));
        recorder.record(make_trade("tok2", "copy", Some(dec!(5)), true));
        assert_eq!(recorder.total_trade_count(), 2);
        assert_eq!(recorder.closed_trade_count(), 1);
        assert_eq!(recorder.open_trades().len(), 1);
    }

    #[test]
    fn test_close_trade() {
        let mut recorder = TradeRecorder::new();
        recorder.record(make_trade("tok1", "ai", None, false));
        recorder.close_trade("tok1", "ai", dec!(12.50));
        assert_eq!(recorder.closed_trade_count(), 1);
        assert_eq!(recorder.winning_trade_count(), 1);
    }

    #[test]
    fn test_gross_profit_loss() {
        let mut recorder = TradeRecorder::new();
        recorder.record(make_trade("t1", "ai", Some(dec!(10)), true));
        recorder.record(make_trade("t2", "ai", Some(dec!(-4)), true));
        recorder.record(make_trade("t3", "ai", Some(dec!(6)), true));
        assert_eq!(recorder.gross_profit(), dec!(16));
        assert_eq!(recorder.gross_loss(), dec!(4));
    }

    #[test]
    fn test_win_rate() {
        let mut recorder = TradeRecorder::new();
        recorder.record(make_trade("t1", "ai", Some(dec!(10)), true));
        recorder.record(make_trade("t2", "ai", Some(dec!(-4)), true));
        recorder.record(make_trade("t3", "ai", Some(dec!(6)), true));
        let closed = recorder.closed_trade_count();
        let wins = recorder.winning_trade_count();
        let win_rate = wins as f64 / closed as f64;
        assert!((win_rate - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_trades_by_strategy() {
        let mut recorder = TradeRecorder::new();
        recorder.record(make_trade("t1", "ai", None, false));
        recorder.record(make_trade("t2", "copy", None, false));
        recorder.record(make_trade("t3", "ai", None, false));
        assert_eq!(recorder.trades_by_strategy("ai").len(), 2);
        assert_eq!(recorder.trades_by_strategy("copy").len(), 1);
    }

    #[test]
    fn test_first_trade_at() {
        let mut recorder = TradeRecorder::new();
        assert!(recorder.first_trade_at().is_none());
        recorder.record(make_trade("t1", "ai", None, false));
        assert!(recorder.first_trade_at().is_some());
    }
}
```

- [ ] **Step 2: Add uuid to engine Cargo.toml dev-dependencies if not already there**

Check `core/engine/Cargo.toml` — `uuid` should already be in dependencies from Phase 0.

- [ ] **Step 3: Add module to lib.rs**

Add to `core/engine/src/lib.rs`:
```rust
pub mod trade_recorder;
pub use trade_recorder::{TradeRecorder, TradeRecord};
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p tradoshka-engine trade_recorder
```

Expected: 6 tests pass

- [ ] **Step 5: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add trade recorder with full metadata tracking"
```

---

### Task 2: Simulated Wallet

**Files:**
- Create: `core/engine/src/wallet.rs`
- Modify: `core/engine/src/lib.rs`

A Polymarket-specific wallet that handles realistic fees, position tracking, and settlement.

- [ ] **Step 1: Implement SimulatedWallet**

Create `core/engine/src/wallet.rs`:

```rust
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tradoshka_common::types::OrderSide;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletPosition {
    pub token_id: String,
    pub market_question: String,
    pub outcome: String,
    pub side: OrderSide,
    pub shares: Decimal,
    pub avg_price: Decimal,
    pub current_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub strategy_id: String,
    pub opened_at: DateTime<Utc>,
}

impl WalletPosition {
    pub fn update_price(&mut self, price: Decimal) {
        self.current_price = price;
        self.unrealized_pnl = match self.side {
            OrderSide::Buy => (price - self.avg_price) * self.shares,
            OrderSide::Sell => (self.avg_price - price) * self.shares,
        };
    }

    pub fn market_value(&self) -> Decimal {
        self.shares * self.current_price
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalletMode {
    Dry,
    Live,
}

pub struct SimulatedWallet {
    balance: Decimal,
    positions: HashMap<String, WalletPosition>,
    mode: WalletMode,
    slippage_bps: Decimal,
    peak_equity: Decimal,
    total_fees: Decimal,
    realized_pnl: Decimal,
}

impl SimulatedWallet {
    pub fn new(initial_balance: Decimal, slippage_bps: Decimal) -> Self {
        Self {
            balance: initial_balance,
            positions: HashMap::new(),
            mode: WalletMode::Dry,
            slippage_bps,
            peak_equity: initial_balance,
            total_fees: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
        }
    }

    pub fn mode(&self) -> WalletMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: WalletMode) {
        self.mode = mode;
    }

    pub fn balance(&self) -> Decimal {
        self.balance
    }

    pub fn equity(&self) -> Decimal {
        let position_value: Decimal = self.positions.values().map(|p| p.market_value()).sum();
        self.balance + position_value
    }

    pub fn unrealized_pnl(&self) -> Decimal {
        self.positions.values().map(|p| p.unrealized_pnl).sum()
    }

    pub fn realized_pnl(&self) -> Decimal {
        self.realized_pnl
    }

    pub fn total_fees(&self) -> Decimal {
        self.total_fees
    }

    pub fn drawdown_pct(&self) -> Decimal {
        if self.peak_equity > Decimal::ZERO {
            (self.peak_equity - self.equity()) / self.peak_equity
        } else {
            Decimal::ZERO
        }
    }

    pub fn positions(&self) -> &HashMap<String, WalletPosition> {
        &self.positions
    }

    pub fn open_position_count(&self) -> usize {
        self.positions.len()
    }

    /// Calculate Polymarket-realistic fee.
    /// fee = base_rate * min(price, 1 - price) * shares
    /// Base rate is approximately 2% (0.02)
    pub fn calculate_fee(price: Decimal, shares: Decimal) -> Decimal {
        let base_rate = Decimal::new(2, 2); // 0.02
        let one = Decimal::ONE;
        let complement = one - price;
        let min_price = price.min(complement);
        base_rate * min_price * shares
    }

    /// Apply slippage to a price.
    fn apply_slippage(&self, price: Decimal, side: OrderSide) -> Decimal {
        let slippage = self.slippage_bps / Decimal::new(10000, 0);
        match side {
            OrderSide::Buy => price * (Decimal::ONE + slippage),
            OrderSide::Sell => price * (Decimal::ONE - slippage),
        }
    }

    /// Execute a simulated buy.
    /// Returns (fill_price, fee, shares) or None if insufficient balance.
    pub fn buy(
        &mut self,
        token_id: &str,
        market_question: &str,
        outcome: &str,
        market_price: Decimal,
        shares: Decimal,
        strategy_id: &str,
    ) -> Option<(Decimal, Decimal, Decimal)> {
        let fill_price = self.apply_slippage(market_price, OrderSide::Buy);
        let cost = fill_price * shares;
        let fee = Self::calculate_fee(fill_price, shares);
        let total_cost = cost + fee;

        if total_cost > self.balance {
            return None;
        }

        self.balance -= total_cost;
        self.total_fees += fee;

        let key = format!("{}:{}", token_id, strategy_id);
        if let Some(pos) = self.positions.get_mut(&key) {
            // Average into existing position
            let total_shares = pos.shares + shares;
            pos.avg_price = (pos.avg_price * pos.shares + fill_price * shares) / total_shares;
            pos.shares = total_shares;
            pos.update_price(market_price);
        } else {
            self.positions.insert(key, WalletPosition {
                token_id: token_id.into(),
                market_question: market_question.into(),
                outcome: outcome.into(),
                side: OrderSide::Buy,
                shares,
                avg_price: fill_price,
                current_price: market_price,
                unrealized_pnl: Decimal::ZERO,
                strategy_id: strategy_id.into(),
                opened_at: Utc::now(),
            });
        }

        self.update_peak();
        Some((fill_price, fee, shares))
    }

    /// Sell/close a position. Returns (fill_price, fee, pnl) or None if no position.
    pub fn sell(
        &mut self,
        token_id: &str,
        market_price: Decimal,
        shares: Decimal,
        strategy_id: &str,
    ) -> Option<(Decimal, Decimal, Decimal)> {
        let key = format!("{}:{}", token_id, strategy_id);
        let pos = self.positions.get(&key)?;

        let sell_shares = shares.min(pos.shares);
        let fill_price = self.apply_slippage(market_price, OrderSide::Sell);
        let revenue = fill_price * sell_shares;
        let fee = Self::calculate_fee(fill_price, sell_shares);
        let pnl = (fill_price - pos.avg_price) * sell_shares - fee;

        self.balance += revenue - fee;
        self.total_fees += fee;
        self.realized_pnl += pnl;

        // Update or remove position
        let remaining = pos.shares - sell_shares;
        if remaining <= Decimal::ZERO {
            self.positions.remove(&key);
        } else {
            let pos = self.positions.get_mut(&key).unwrap();
            pos.shares = remaining;
            pos.update_price(market_price);
        }

        self.update_peak();
        Some((fill_price, fee, pnl))
    }

    /// Settle a market resolution. Winning positions get $1/share, losers get $0.
    pub fn settle_market(&mut self, token_id: &str, won: bool) -> Decimal {
        let mut total_pnl = Decimal::ZERO;
        let keys_to_remove: Vec<String> = self.positions.keys()
            .filter(|k| k.starts_with(&format!("{}:", token_id)))
            .cloned()
            .collect();

        for key in keys_to_remove {
            if let Some(pos) = self.positions.remove(&key) {
                if won {
                    let payout = pos.shares; // $1 per share
                    let pnl = payout - (pos.avg_price * pos.shares);
                    self.balance += payout;
                    self.realized_pnl += pnl;
                    total_pnl += pnl;
                } else {
                    let pnl = -(pos.avg_price * pos.shares);
                    self.realized_pnl += pnl;
                    total_pnl += pnl;
                }
            }
        }

        self.update_peak();
        total_pnl
    }

    /// Update all position prices from a price map.
    pub fn update_prices(&mut self, prices: &HashMap<String, Decimal>) {
        for (key, pos) in self.positions.iter_mut() {
            if let Some(price) = prices.get(&pos.token_id) {
                pos.update_price(*price);
            }
        }
        self.update_peak();
    }

    fn update_peak(&mut self) {
        let eq = self.equity();
        if eq > self.peak_equity {
            self.peak_equity = eq;
        }
    }

    pub fn peak_equity(&self) -> Decimal {
        self.peak_equity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_new_wallet() {
        let w = SimulatedWallet::new(dec!(100), dec!(5));
        assert_eq!(w.balance(), dec!(100));
        assert_eq!(w.equity(), dec!(100));
        assert_eq!(w.mode(), WalletMode::Dry);
        assert_eq!(w.open_position_count(), 0);
    }

    #[test]
    fn test_calculate_fee() {
        // fee = 0.02 * min(0.50, 0.50) * 100 = 1.00
        let fee = SimulatedWallet::calculate_fee(dec!(0.50), dec!(100));
        assert_eq!(fee, dec!(1.00));

        // fee = 0.02 * min(0.80, 0.20) * 100 = 0.40
        let fee = SimulatedWallet::calculate_fee(dec!(0.80), dec!(100));
        assert_eq!(fee, dec!(0.40));
    }

    #[test]
    fn test_buy_reduces_balance() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0)); // no slippage
        let result = w.buy("tok1", "Will X?", "Yes", dec!(0.50), dec!(10), "ai");
        assert!(result.is_some());
        let (price, fee, shares) = result.unwrap();
        assert_eq!(price, dec!(0.50));
        assert_eq!(shares, dec!(10));
        // cost = 0.50 * 10 = 5.00, fee = 0.02 * 0.50 * 10 = 0.10
        assert_eq!(fee, dec!(0.10));
        assert_eq!(w.balance(), dec!(100) - dec!(5.00) - dec!(0.10));
        assert_eq!(w.open_position_count(), 1);
    }

    #[test]
    fn test_buy_insufficient_balance() {
        let mut w = SimulatedWallet::new(dec!(1), dec!(0));
        let result = w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(100), "ai");
        assert!(result.is_none());
    }

    #[test]
    fn test_sell_closes_position_with_profit() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let result = w.sell("tok1", dec!(0.70), dec!(10), "ai");
        assert!(result.is_some());
        let (_, _, pnl) = result.unwrap();
        assert!(pnl > Decimal::ZERO); // bought at 0.50, sold at 0.70
        assert_eq!(w.open_position_count(), 0);
    }

    #[test]
    fn test_sell_partial() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        w.sell("tok1", dec!(0.60), dec!(5), "ai");
        assert_eq!(w.open_position_count(), 1);
        let pos = w.positions().values().next().unwrap();
        assert_eq!(pos.shares, dec!(5));
    }

    #[test]
    fn test_settle_market_win() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let balance_after_buy = w.balance();
        let pnl = w.settle_market("tok1", true);
        assert!(pnl > Decimal::ZERO);
        assert_eq!(w.open_position_count(), 0);
        // Should have gotten $1 * 10 shares = $10 back
        assert_eq!(w.balance(), balance_after_buy + dec!(10));
    }

    #[test]
    fn test_settle_market_loss() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let pnl = w.settle_market("tok1", false);
        assert!(pnl < Decimal::ZERO);
        assert_eq!(w.open_position_count(), 0);
    }

    #[test]
    fn test_drawdown() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0));
        assert_eq!(w.drawdown_pct(), Decimal::ZERO);
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(40), "ai");
        // Balance dropped by ~20 (cost) + fees, equity should be ~100 still (position value)
        // Simulate a price drop
        let mut prices = HashMap::new();
        prices.insert("tok1".into(), dec!(0.30));
        w.update_prices(&prices);
        assert!(w.drawdown_pct() > Decimal::ZERO);
    }

    #[test]
    fn test_slippage_applied() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(10)); // 10 bps slippage
        let result = w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let (fill_price, _, _) = result.unwrap();
        assert!(fill_price > dec!(0.50)); // Buy slippage makes price worse
    }
}
```

- [ ] **Step 2: Add to lib.rs**

Add to `core/engine/src/lib.rs`:
```rust
pub mod wallet;
pub use wallet::{SimulatedWallet, WalletPosition, WalletMode};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-engine wallet
```

Expected: 10 tests pass

- [ ] **Step 4: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add simulated wallet with Polymarket fees and settlement"
```

---

### Task 3: Production Readiness Scorer

**Files:**
- Create: `core/engine/src/readiness.rs`
- Modify: `core/engine/src/lib.rs`

- [ ] **Step 1: Implement ReadinessScorer**

Create `core/engine/src/readiness.rs`:

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::trade_recorder::TradeRecorder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessCriterion {
    pub name: String,
    pub threshold: String,
    pub current_value: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    pub criteria: Vec<ReadinessCriterion>,
    pub passed_count: usize,
    pub total_count: usize,
    pub is_ready: bool,
}

pub struct ReadinessScorer {
    pub min_days: i64,
    pub min_trades: usize,
    pub min_win_rate: f64,
    pub min_sharpe: f64,
    pub max_drawdown_pct: f64,
    pub min_profit_factor: f64,
}

impl Default for ReadinessScorer {
    fn default() -> Self {
        Self {
            min_days: 14,
            min_trades: 50,
            min_win_rate: 0.70,
            min_sharpe: 1.0,
            max_drawdown_pct: 0.15,
            min_profit_factor: 1.5,
        }
    }
}

impl ReadinessScorer {
    pub fn evaluate(
        &self,
        recorder: &TradeRecorder,
        current_drawdown_pct: f64,
        daily_returns: &[f64],
    ) -> ReadinessReport {
        let closed = recorder.closed_trade_count();
        let wins = recorder.winning_trade_count();
        let win_rate = if closed > 0 { wins as f64 / closed as f64 } else { 0.0 };

        let days_active = recorder.first_trade_at()
            .map(|first| (Utc::now() - first).num_days())
            .unwrap_or(0);

        let gross_profit = recorder.gross_profit();
        let gross_loss = recorder.gross_loss();
        let profit_factor = if gross_loss > Decimal::ZERO {
            (gross_profit / gross_loss).to_f64().unwrap_or(0.0)
        } else if gross_profit > Decimal::ZERO {
            999.0
        } else {
            0.0
        };

        let sharpe = Self::calculate_sharpe(daily_returns);

        let mut criteria = vec![
            ReadinessCriterion {
                name: "Days active".into(),
                threshold: format!(">= {}", self.min_days),
                current_value: format!("{}", days_active),
                passed: days_active >= self.min_days,
            },
            ReadinessCriterion {
                name: "Total trades".into(),
                threshold: format!(">= {}", self.min_trades),
                current_value: format!("{}", closed),
                passed: closed >= self.min_trades,
            },
            ReadinessCriterion {
                name: "Win rate".into(),
                threshold: format!(">= {:.0}%", self.min_win_rate * 100.0),
                current_value: format!("{:.1}%", win_rate * 100.0),
                passed: win_rate >= self.min_win_rate,
            },
            ReadinessCriterion {
                name: "Sharpe ratio".into(),
                threshold: format!(">= {:.1}", self.min_sharpe),
                current_value: format!("{:.2}", sharpe),
                passed: sharpe >= self.min_sharpe,
            },
            ReadinessCriterion {
                name: "Max drawdown".into(),
                threshold: format!("<= {:.0}%", self.max_drawdown_pct * 100.0),
                current_value: format!("{:.1}%", current_drawdown_pct * 100.0),
                passed: current_drawdown_pct <= self.max_drawdown_pct,
            },
            ReadinessCriterion {
                name: "Profit factor".into(),
                threshold: format!(">= {:.1}", self.min_profit_factor),
                current_value: format!("{:.2}", profit_factor),
                passed: profit_factor >= self.min_profit_factor,
            },
        ];

        let passed_count = criteria.iter().filter(|c| c.passed).count();
        let total_count = criteria.len();

        ReadinessReport {
            criteria,
            passed_count,
            total_count,
            is_ready: passed_count == total_count,
        }
    }

    fn calculate_sharpe(daily_returns: &[f64]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }
        let n = daily_returns.len() as f64;
        let mean = daily_returns.iter().sum::<f64>() / n;
        let variance = daily_returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();
        if std_dev == 0.0 {
            return 0.0;
        }
        // Annualized: mean/std_dev * sqrt(365)
        (mean / std_dev) * (365.0_f64).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade_recorder::{TradeRecorder, TradeRecord};
    use tradoshka_common::types::{OrderSide, Market};
    use rust_decimal_macros::dec;

    fn make_closed_trade(pnl: Decimal) -> TradeRecord {
        TradeRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now() - chrono::Duration::days(20),
            market: Market::Polymarket,
            symbol: "tok".into(),
            market_question: "Q?".into(),
            direction: "YES".into(),
            side: OrderSide::Buy,
            shares: dec!(10),
            price: dec!(0.50),
            fee: dec!(0.05),
            strategy_id: "ai".into(),
            signal_strength: 0.8,
            edge_vs_market: 0.05,
            pnl: Some(pnl),
            is_closed: true,
        }
    }

    #[test]
    fn test_not_ready_with_no_trades() {
        let scorer = ReadinessScorer::default();
        let recorder = TradeRecorder::new();
        let report = scorer.evaluate(&recorder, 0.0, &[]);
        assert!(!report.is_ready);
        assert_eq!(report.passed_count, 2); // drawdown and days might pass vacuously
    }

    #[test]
    fn test_ready_when_all_criteria_met() {
        let scorer = ReadinessScorer {
            min_days: 0, // Override for test
            min_trades: 3,
            min_win_rate: 0.60,
            min_sharpe: 0.5,
            max_drawdown_pct: 0.20,
            min_profit_factor: 1.0,
        };
        let mut recorder = TradeRecorder::new();
        recorder.record(make_closed_trade(dec!(10)));
        recorder.record(make_closed_trade(dec!(8)));
        recorder.record(make_closed_trade(dec!(-3)));

        let daily_returns = vec![0.02, 0.03, -0.01, 0.04, 0.01, 0.02, 0.03];
        let report = scorer.evaluate(&recorder, 0.05, &daily_returns);
        assert!(report.is_ready);
        assert_eq!(report.passed_count, 6);
    }

    #[test]
    fn test_fails_on_low_win_rate() {
        let scorer = ReadinessScorer {
            min_days: 0,
            min_trades: 2,
            min_win_rate: 0.70,
            min_sharpe: 0.0,
            max_drawdown_pct: 1.0,
            min_profit_factor: 0.0,
        };
        let mut recorder = TradeRecorder::new();
        recorder.record(make_closed_trade(dec!(10)));
        recorder.record(make_closed_trade(dec!(-5)));
        // Win rate = 50%, below 70%
        let report = scorer.evaluate(&recorder, 0.0, &[0.01, 0.02]);
        let wr_criterion = report.criteria.iter().find(|c| c.name == "Win rate").unwrap();
        assert!(!wr_criterion.passed);
    }

    #[test]
    fn test_sharpe_calculation() {
        let returns = vec![0.01, 0.02, 0.015, 0.01, 0.02, 0.005, 0.01];
        let sharpe = ReadinessScorer::calculate_sharpe(&returns);
        assert!(sharpe > 0.0); // All positive returns → positive Sharpe
    }
}
```

- [ ] **Step 2: Add to lib.rs**

Add to `core/engine/src/lib.rs`:
```rust
pub mod readiness;
pub use readiness::{ReadinessScorer, ReadinessReport, ReadinessCriterion};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-engine readiness
```

Expected: 4 tests pass

- [ ] **Step 4: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add production readiness scorer with 6 criteria"
```

---

### Task 4: Market Data Service

**Files:**
- Create: `core/engine/src/market_data.rs`
- Modify: `core/engine/src/lib.rs`
- Modify: `core/engine/Cargo.toml`

The service that pulls real Polymarket data and emits events.

- [ ] **Step 1: Add tradoshka-polymarket dependency to engine**

Add to `core/engine/Cargo.toml` under `[dependencies]`:
```toml
tradoshka-polymarket = { path = "../../markets/polymarket" }
```

- [ ] **Step 2: Implement MarketDataService**

Create `core/engine/src/market_data.rs`:

```rust
use rust_decimal::Decimal;
use chrono::Utc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tradoshka_common::types::{MarketEvent, Trade, Symbol};
use tradoshka_polymarket::client::PolymarketClient;
use tradoshka_polymarket::scanner::{MarketScanner, MarketFilter};
use tradoshka_polymarket::types::GammaMarket;
use tracing::{info, warn, error};

/// A tracked market with its current state.
#[derive(Debug, Clone)]
pub struct TrackedMarket {
    pub condition_id: String,
    pub question: String,
    pub yes_token_id: String,
    pub no_token_id: String,
    pub yes_price: Decimal,
    pub no_price: Decimal,
    pub volume_24h: f64,
    pub liquidity: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Configuration for the market data service.
pub struct MarketDataConfig {
    pub poll_interval_secs: u64,
    pub scan_interval_secs: u64,
    pub max_tracked_markets: usize,
    pub price_change_trigger_pct: Decimal,
}

impl Default for MarketDataConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 300,       // 5 minutes
            scan_interval_secs: 1800,      // 30 minutes
            max_tracked_markets: 20,
            price_change_trigger_pct: Decimal::new(3, 2), // 3%
        }
    }
}

/// Service that pulls real Polymarket data and emits MarketEvents.
pub struct MarketDataService {
    client: PolymarketClient,
    scanner: MarketScanner,
    config: MarketDataConfig,
    tracked_markets: HashMap<String, TrackedMarket>,
}

impl MarketDataService {
    pub fn new(config: MarketDataConfig) -> Self {
        Self {
            client: PolymarketClient::new_public(),
            scanner: MarketScanner::new(MarketFilter::default()),
            config,
            tracked_markets: HashMap::new(),
        }
    }

    /// Scan for new tradeable markets and update the tracked list.
    pub async fn scan_markets(&mut self) -> Vec<TrackedMarket> {
        match self.client.get_gamma_markets(true, 100, 0).await {
            Ok(markets) => {
                let filtered = self.scanner.filter_markets(&markets);
                let ranked = self.scanner.rank_by_opportunity(&filtered);
                let top = ranked.into_iter()
                    .take(self.config.max_tracked_markets)
                    .collect::<Vec<_>>();

                for rm in &top {
                    let market = &rm.market;
                    if market.tokens.len() >= 2 {
                        let yes_token = market.tokens.iter()
                            .find(|t| t.outcome == "Yes")
                            .map(|t| t.token_id.clone())
                            .unwrap_or_default();
                        let no_token = market.tokens.iter()
                            .find(|t| t.outcome == "No")
                            .map(|t| t.token_id.clone())
                            .unwrap_or_default();
                        let yes_price = market.tokens.iter()
                            .find(|t| t.outcome == "Yes")
                            .and_then(|t| t.price)
                            .map(|p| Decimal::from_f64_retain(p).unwrap_or(Decimal::ZERO))
                            .unwrap_or(Decimal::ZERO);
                        let no_price = market.tokens.iter()
                            .find(|t| t.outcome == "No")
                            .and_then(|t| t.price)
                            .map(|p| Decimal::from_f64_retain(p).unwrap_or(Decimal::ZERO))
                            .unwrap_or(Decimal::ZERO);

                        let tracked = TrackedMarket {
                            condition_id: market.condition_id.clone(),
                            question: market.question.clone(),
                            yes_token_id: yes_token,
                            no_token_id: no_token,
                            yes_price,
                            no_price,
                            volume_24h: market.volume_24hr.unwrap_or(0.0),
                            liquidity: market.liquidity.unwrap_or(0.0),
                            last_updated: Utc::now(),
                        };
                        self.tracked_markets.insert(market.condition_id.clone(), tracked);
                    }
                }

                info!("Scanning complete: tracking {} markets", self.tracked_markets.len());
                self.tracked_markets.values().cloned().collect()
            }
            Err(e) => {
                error!("Market scan failed: {}", e);
                Vec::new()
            }
        }
    }

    /// Poll current prices for all tracked markets.
    /// Returns list of (token_id, new_price, old_price) for markets that changed significantly.
    pub async fn poll_prices(&mut self) -> Vec<(String, Decimal, Decimal)> {
        let mut significant_changes = Vec::new();

        for (_, market) in self.tracked_markets.iter_mut() {
            // Poll YES token price
            match self.client.get_price(&market.yes_token_id, "BUY").await {
                Ok(price_str) => {
                    if let Ok(new_price) = price_str.parse::<Decimal>() {
                        let old_price = market.yes_price;
                        if old_price > Decimal::ZERO {
                            let change_pct = ((new_price - old_price) / old_price).abs();
                            if change_pct >= self.config.price_change_trigger_pct {
                                significant_changes.push((
                                    market.yes_token_id.clone(),
                                    new_price,
                                    old_price,
                                ));
                            }
                        }
                        market.yes_price = new_price;
                        market.last_updated = Utc::now();
                    }
                }
                Err(e) => {
                    warn!("Price poll failed for {}: {}", market.yes_token_id, e);
                }
            }
        }

        significant_changes
    }

    /// Get all currently tracked markets.
    pub fn tracked_markets(&self) -> Vec<&TrackedMarket> {
        self.tracked_markets.values().collect()
    }

    /// Get a specific tracked market by condition ID.
    pub fn get_market(&self, condition_id: &str) -> Option<&TrackedMarket> {
        self.tracked_markets.get(condition_id)
    }

    /// Get current prices as a map (token_id → price).
    pub fn current_prices(&self) -> HashMap<String, Decimal> {
        let mut prices = HashMap::new();
        for market in self.tracked_markets.values() {
            prices.insert(market.yes_token_id.clone(), market.yes_price);
            prices.insert(market.no_token_id.clone(), market.no_price);
        }
        prices
    }

    /// Convert a tracked market's price update into a MarketEvent.
    pub fn to_market_event(market: &TrackedMarket, token_id: &str, price: Decimal) -> MarketEvent {
        MarketEvent::TradeEvent(Trade {
            symbol: token_id.into(),
            price,
            quantity: Decimal::ONE,
            timestamp: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MarketDataConfig::default();
        assert_eq!(config.poll_interval_secs, 300);
        assert_eq!(config.scan_interval_secs, 1800);
        assert_eq!(config.max_tracked_markets, 20);
    }

    #[test]
    fn test_new_service() {
        let svc = MarketDataService::new(MarketDataConfig::default());
        assert_eq!(svc.tracked_markets().len(), 0);
    }

    #[test]
    fn test_current_prices_empty() {
        let svc = MarketDataService::new(MarketDataConfig::default());
        assert!(svc.current_prices().is_empty());
    }

    #[test]
    fn test_to_market_event() {
        let market = TrackedMarket {
            condition_id: "0x123".into(),
            question: "Test?".into(),
            yes_token_id: "tok_yes".into(),
            no_token_id: "tok_no".into(),
            yes_price: Decimal::new(55, 2),
            no_price: Decimal::new(45, 2),
            volume_24h: 10000.0,
            liquidity: 5000.0,
            last_updated: Utc::now(),
        };
        let event = MarketDataService::to_market_event(&market, "tok_yes", Decimal::new(55, 2));
        assert_eq!(event.symbol(), "tok_yes");
    }
}
```

- [ ] **Step 3: Add to lib.rs**

Add to `core/engine/src/lib.rs`:
```rust
pub mod market_data;
pub use market_data::{MarketDataService, MarketDataConfig, TrackedMarket};
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p tradoshka-engine market_data
```

Expected: 4 tests pass

- [ ] **Step 5: Run full workspace tests**

```bash
cargo test --workspace
```

Expected: all existing tests + 24 new tests pass

- [ ] **Step 6: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add market data service for real Polymarket prices"
```

---

### Task 5: Integrate with API Server

**Files:**
- Modify: `core/api/src/state.rs`
- Modify: `core/api/src/routes.rs`
- Modify: `core/api/src/server.rs`
- Modify: `core/api/Cargo.toml`

- [ ] **Step 1: Update AppState to include new components**

Read current `core/api/src/state.rs`, then add:

```rust
// Add imports
use tradoshka_engine::{SimulatedWallet, TradeRecorder, ReadinessScorer, MarketDataService, MarketDataConfig};

// Update AppState struct to include:
pub struct AppState {
    pub order_manager: OrderManager,
    pub portfolio: PortfolioTracker,
    pub risk_manager: TradoshkaRiskManager,
    pub dry_engine: DryModeEngine,
    pub polymarket: Option<PolymarketAdapter>,
    pub wallet: SimulatedWallet,
    pub trade_recorder: TradeRecorder,
    pub readiness_scorer: ReadinessScorer,
    pub market_data: MarketDataService,
}

// Update new_dry_mode:
impl AppState {
    pub fn new_dry_mode(initial_balance: rust_decimal::Decimal) -> Self {
        Self {
            order_manager: OrderManager::new(),
            portfolio: PortfolioTracker::new(initial_balance),
            risk_manager: TradoshkaRiskManager::new(tradoshka_risk::RiskConfig {
                initial_equity: initial_balance,
                ..Default::default()
            }),
            dry_engine: DryModeEngine::new(dec!(5)),
            polymarket: None,
            wallet: SimulatedWallet::new(initial_balance, dec!(5)),
            trade_recorder: TradeRecorder::new(),
            readiness_scorer: ReadinessScorer::default(),
            market_data: MarketDataService::new(MarketDataConfig::default()),
        }
    }
}
```

- [ ] **Step 2: Add trading API routes**

Add to `core/api/src/routes.rs`:

```rust
pub async fn get_wallet(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let w = &state.wallet;
    Json(serde_json::json!({
        "mode": format!("{:?}", w.mode()),
        "balance": w.balance().to_string(),
        "equity": w.equity().to_string(),
        "unrealized_pnl": w.unrealized_pnl().to_string(),
        "realized_pnl": w.realized_pnl().to_string(),
        "drawdown_pct": w.drawdown_pct().to_string(),
        "total_fees": w.total_fees().to_string(),
        "open_positions": w.open_position_count(),
        "positions": w.positions().values().map(|p| serde_json::json!({
            "token_id": p.token_id,
            "question": p.market_question,
            "outcome": p.outcome,
            "side": format!("{:?}", p.side),
            "shares": p.shares.to_string(),
            "avg_price": p.avg_price.to_string(),
            "current_price": p.current_price.to_string(),
            "unrealized_pnl": p.unrealized_pnl.to_string(),
            "strategy": p.strategy_id,
        })).collect::<Vec<_>>(),
    }))
}

pub async fn get_trades(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let trades = state.trade_recorder.recent_trades(50);
    Json(serde_json::json!({
        "trades": trades.iter().map(|t| serde_json::json!({
            "id": t.id,
            "timestamp": t.timestamp.to_rfc3339(),
            "symbol": t.symbol,
            "question": t.market_question,
            "direction": t.direction,
            "side": format!("{:?}", t.side),
            "shares": t.shares.to_string(),
            "price": t.price.to_string(),
            "fee": t.fee.to_string(),
            "strategy": t.strategy_id,
            "strength": t.signal_strength,
            "edge": t.edge_vs_market,
            "pnl": t.pnl.map(|p| p.to_string()),
            "closed": t.is_closed,
        })).collect::<Vec<_>>(),
        "total": state.trade_recorder.total_trade_count(),
    }))
}

pub async fn get_readiness(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let dd = state.wallet.drawdown_pct().to_f64().unwrap_or(0.0);
    // For now, use empty daily returns — will be calculated from trade recorder in Phase 2B
    let report = state.readiness_scorer.evaluate(&state.trade_recorder, dd, &[]);
    Json(serde_json::json!({
        "criteria": report.criteria,
        "passed": report.passed_count,
        "total": report.total_count,
        "is_ready": report.is_ready,
    }))
}

pub async fn get_tracked_markets(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let markets = state.market_data.tracked_markets();
    Json(serde_json::json!({
        "count": markets.len(),
        "markets": markets.iter().map(|m| serde_json::json!({
            "condition_id": m.condition_id,
            "question": m.question,
            "yes_price": m.yes_price.to_string(),
            "no_price": m.no_price.to_string(),
            "volume_24h": m.volume_24h,
            "liquidity": m.liquidity,
        })).collect::<Vec<_>>(),
    }))
}
```

- [ ] **Step 3: Register routes**

Add to `core/api/src/server.rs` router:

```rust
.route("/api/wallet", get(routes::get_wallet))
.route("/api/trades/live", get(routes::get_trades))
.route("/api/readiness", get(routes::get_readiness))
.route("/api/markets/tracked", get(routes::get_tracked_markets))
```

- [ ] **Step 4: Add rust_decimal prelude to routes.rs**

Add `use rust_decimal::prelude::*;` to the imports in routes.rs (needed for `to_f64()`).

- [ ] **Step 5: Verify**

```bash
cargo build -p tradoshka-api
cargo test --workspace
```

- [ ] **Step 6: Test endpoints**

```bash
cargo run -p tradoshka-api &
sleep 3
curl -s http://localhost:3001/api/wallet | python -m json.tool
curl -s http://localhost:3001/api/readiness | python -m json.tool
curl -s http://localhost:3001/api/trades/live | python -m json.tool
curl -s http://localhost:3001/api/markets/tracked | python -m json.tool
taskkill //F //IM tradoshka.exe 2>/dev/null
```

- [ ] **Step 7: Commit**

```bash
git add core/api/ core/engine/
git commit -m "feat(api): add wallet, trades, readiness, and tracked markets endpoints"
```

---

### Task 6: Full Verification

- [ ] **Step 1: Run all Rust tests**

```bash
cargo test --workspace
```

Expected: all tests pass (existing + ~24 new)

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace
```

- [ ] **Step 3: Run Python tests**

```bash
cd strategies/shared && python -m pytest tests/ -v
cd ../polymarket && python -m pytest tests/ -v
```

- [ ] **Step 4: Verify all new API endpoints**

```bash
cargo run -p tradoshka-api &
sleep 3
echo "=== Wallet ===" && curl -s http://localhost:3001/api/wallet | python -m json.tool
echo "=== Readiness ===" && curl -s http://localhost:3001/api/readiness | python -m json.tool
echo "=== Trades ===" && curl -s http://localhost:3001/api/trades/live | python -m json.tool
echo "=== Tracked ===" && curl -s http://localhost:3001/api/markets/tracked | python -m json.tool
echo "=== Health ===" && curl -s http://localhost:3001/health | python -m json.tool
taskkill //F //IM tradoshka.exe 2>/dev/null
```

- [ ] **Step 5: Merge to dev**

```bash
git checkout dev
git merge feature/phase2a-data-wallet
```

- [ ] **Step 6: Update GOALS.md**

Mark Phase 2A as complete.

```bash
git add docs/GOALS.md
git commit -m "docs: update goals — Phase 2A market data + wallet complete"
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | Trade Recorder | 6 |
| 2 | Simulated Wallet | 10 |
| 3 | Readiness Scorer | 4 |
| 4 | Market Data Service | 4 |
| 5 | API Integration | Compilation + endpoint verification |
| 6 | Full Verification | All tests |

**Total: 6 tasks, 24 new Rust tests**

Next plans:
- **Phase 2B:** Strategy Orchestrator + Trade Recorder integration (wires strategies to the wallet)
- **Phase 2C:** Trading Dashboard v2 (split-screen with live feed + readiness score)
- **Phase 2D:** Integration Testing + Dry Run (end-to-end with real Polymarket data)
