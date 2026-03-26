# Strategy Evolution Engine Phases 1+2 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the strategy wallet manager (creates/tracks/kills isolated wallets per strategy variant), mutation engine (parameter tweaking), and evolution engine (hourly Darwinian rank/kill/spawn cycle).

**Architecture:** `StrategyWalletManager` holds a `HashMap<String, StrategySlot>` where each slot has its own `SimulatedWallet` + `TradeRecorder` + strategy parameters. The `EvolutionEngine` ranks slots by Sharpe, kills bottom 10%, clones top 10% with mutated parameters. All events logged in an `EvolutionTimeline`.

**Tech Stack:** Rust (rust_decimal, chrono, serde, rand), existing SimulatedWallet + TradeRecorder

---

## File Structure

```
core/engine/src/
├── strategy_wallet.rs       # NEW: StrategySlot + StrategyWalletManager
├── mutation.rs              # NEW: Parameter mutation per strategy type
├── evolution.rs             # NEW: Hourly evolution engine
├── lib.rs                   # Add new modules
```

---

### Task 1: Strategy Parameters + Mutation Engine

**Files:**
- Create: `core/engine/src/mutation.rs`
- Modify: `core/engine/src/lib.rs`

- [ ] **Step 1: Implement StrategyParams and mutation**

Create `core/engine/src/mutation.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strategy parameters that can be mutated during evolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyParams {
    pub strategy_type: String,
    pub params: HashMap<String, f64>,
}

impl StrategyParams {
    pub fn new(strategy_type: &str) -> Self {
        Self {
            strategy_type: strategy_type.into(),
            params: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: &str, value: f64) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    pub fn get(&self, key: &str) -> f64 {
        self.params.get(key).copied().unwrap_or(0.0)
    }

    pub fn set(&mut self, key: &str, value: f64) {
        self.params.insert(key.into(), value);
    }

    /// Get the list of mutable parameter names for this strategy type.
    pub fn mutable_params(&self) -> Vec<String> {
        match self.strategy_type.as_str() {
            "momentum" => vec!["ema_fast".into(), "ema_slow".into(), "rsi_threshold".into()],
            "dca" => vec!["buy_interval".into(), "rsi_oversold".into()],
            "grid" => vec!["spacing_pct".into(), "grid_count".into()],
            "meanrev" => vec!["bb_period".into(), "bb_std".into()],
            "copy" => vec!["min_trade_size".into(), "min_win_rate".into()],
            "arb" | "mispricing" => vec!["min_edge".into(), "min_volume".into()],
            "market_making" => vec!["spread_bps".into(), "max_inventory".into()],
            "scalp" => vec!["ema_fast".into(), "ema_slow".into(), "leverage".into()],
            "funding_arb" => vec!["min_funding_rate".into()],
            "value" => vec!["min_edge".into()],
            _ => self.params.keys().cloned().collect(),
        }
    }
}

/// Mutation result describing what changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationResult {
    pub param_name: String,
    pub old_value: f64,
    pub new_value: f64,
    pub change_pct: f64,
}

/// Mutate one random parameter of a strategy by ±10-30%.
pub fn mutate(params: &StrategyParams, seed: u64) -> (StrategyParams, MutationResult) {
    let mutable = params.mutable_params();
    if mutable.is_empty() {
        return (params.clone(), MutationResult {
            param_name: "none".into(),
            old_value: 0.0,
            new_value: 0.0,
            change_pct: 0.0,
        });
    }

    // Simple deterministic "random" using seed
    let param_idx = (seed as usize) % mutable.len();
    let param_name = &mutable[param_idx];
    let old_value = params.get(param_name);

    // Change direction: even seed = increase, odd = decrease
    let direction = if (seed / 7) % 2 == 0 { 1.0 } else { -1.0 };
    // Change magnitude: 10-30% based on seed
    let magnitude = 0.10 + (((seed % 20) as f64) / 100.0); // 0.10 to 0.29
    let change_pct = direction * magnitude;
    let new_value = old_value * (1.0 + change_pct);

    // Clamp to reasonable bounds
    let new_value = new_value.max(1.0); // Never go below 1

    let mut new_params = params.clone();
    new_params.set(param_name, new_value);

    (new_params, MutationResult {
        param_name: param_name.clone(),
        old_value,
        new_value,
        change_pct: change_pct * 100.0,
    })
}

/// Create the default parameter sets for the initial 20 strategies.
pub fn initial_strategies() -> Vec<(String, StrategyParams)> {
    vec![
        // Polymarket (6)
        ("PM-momentum-fast".into(), StrategyParams::new("momentum").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("rsi_threshold", 50.0)),
        ("PM-momentum-slow".into(), StrategyParams::new("momentum").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("rsi_threshold", 50.0)),
        ("PM-value-aggressive".into(), StrategyParams::new("value").with_param("min_edge", 3.0)),
        ("PM-value-conservative".into(), StrategyParams::new("value").with_param("min_edge", 8.0)),
        ("PM-copy-top-pnl".into(), StrategyParams::new("copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 50.0)),
        ("PM-arb-mispricing".into(), StrategyParams::new("mispricing").with_param("min_edge", 3.0).with_param("min_volume", 1000.0)),

        // Crypto Spot (8)
        ("CS-momentum-fast".into(), StrategyParams::new("momentum").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("rsi_threshold", 50.0)),
        ("CS-momentum-slow".into(), StrategyParams::new("momentum").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("rsi_threshold", 50.0)),
        ("CS-dca-aggressive".into(), StrategyParams::new("dca").with_param("buy_interval", 6.0).with_param("rsi_oversold", 40.0)),
        ("CS-dca-conservative".into(), StrategyParams::new("dca").with_param("buy_interval", 24.0).with_param("rsi_oversold", 30.0)),
        ("CS-grid-tight".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.5).with_param("grid_count", 10.0)),
        ("CS-grid-wide".into(), StrategyParams::new("grid").with_param("spacing_pct", 2.0).with_param("grid_count", 5.0)),
        ("CS-meanrev-bollinger".into(), StrategyParams::new("meanrev").with_param("bb_period", 20.0).with_param("bb_std", 2.0)),
        ("CS-copy-whales".into(), StrategyParams::new("copy").with_param("min_trade_size", 10000.0).with_param("min_win_rate", 50.0)),

        // Crypto Perps (6)
        ("CP-scalp-5m".into(), StrategyParams::new("scalp").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("leverage", 10.0)),
        ("CP-scalp-15m".into(), StrategyParams::new("scalp").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("leverage", 10.0)),
        ("CP-scalp-1h".into(), StrategyParams::new("scalp").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("leverage", 5.0)),
        ("CP-funding-arb".into(), StrategyParams::new("funding_arb").with_param("min_funding_rate", 0.0003)),
        ("CP-grid-perp".into(), StrategyParams::new("grid").with_param("spacing_pct", 1.0).with_param("grid_count", 8.0)),
        ("CP-meanrev-perp".into(), StrategyParams::new("meanrev").with_param("bb_period", 20.0).with_param("bb_std", 2.0)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_strategies_count() {
        assert_eq!(initial_strategies().len(), 20);
    }

    #[test]
    fn test_strategy_params() {
        let p = StrategyParams::new("momentum")
            .with_param("ema_fast", 5.0)
            .with_param("ema_slow", 13.0);
        assert_eq!(p.get("ema_fast"), 5.0);
        assert_eq!(p.get("ema_slow"), 13.0);
        assert_eq!(p.get("nonexistent"), 0.0);
    }

    #[test]
    fn test_mutable_params() {
        let p = StrategyParams::new("momentum");
        let mutable = p.mutable_params();
        assert!(mutable.contains(&"ema_fast".to_string()));
        assert!(mutable.contains(&"ema_slow".to_string()));
    }

    #[test]
    fn test_mutate_changes_one_param() {
        let p = StrategyParams::new("momentum")
            .with_param("ema_fast", 5.0)
            .with_param("ema_slow", 13.0)
            .with_param("rsi_threshold", 50.0);
        let (new_p, result) = mutate(&p, 42);
        assert_ne!(result.old_value, result.new_value);
        assert!(!result.param_name.is_empty());
        // Only one param changed
        let unchanged_count = ["ema_fast", "ema_slow", "rsi_threshold"].iter()
            .filter(|&&k| (new_p.get(k) - p.get(k)).abs() < 0.001)
            .count();
        assert_eq!(unchanged_count, 2); // 2 unchanged, 1 mutated
    }

    #[test]
    fn test_mutate_deterministic() {
        let p = StrategyParams::new("dca").with_param("buy_interval", 12.0).with_param("rsi_oversold", 30.0);
        let (r1, m1) = mutate(&p, 100);
        let (r2, m2) = mutate(&p, 100);
        assert_eq!(m1.param_name, m2.param_name);
        assert_eq!(r1.get(&m1.param_name), r2.get(&m2.param_name));
    }

    #[test]
    fn test_mutate_stays_positive() {
        let p = StrategyParams::new("dca").with_param("buy_interval", 2.0).with_param("rsi_oversold", 5.0);
        for seed in 0..100 {
            let (new_p, _) = mutate(&p, seed);
            for (_, v) in &new_p.params {
                assert!(*v >= 1.0, "Param went below 1.0: {}", v);
            }
        }
    }
}
```

- [ ] **Step 2: Add to lib.rs**

```rust
pub mod mutation;
pub use mutation::{StrategyParams, MutationResult, mutate, initial_strategies};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-engine mutation
```

Expected: 6 tests pass

- [ ] **Step 4: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add strategy parameter system with mutation for evolution"
```

---

### Task 2: Strategy Wallet Manager

**Files:**
- Create: `core/engine/src/strategy_wallet.rs`
- Modify: `core/engine/src/lib.rs`

- [ ] **Step 1: Implement StrategySlot and StrategyWalletManager**

Create `core/engine/src/strategy_wallet.rs`:

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::wallet::SimulatedWallet;
use crate::trade_recorder::TradeRecorder;
use crate::mutation::StrategyParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotStatus {
    Alive,
    Dead,
}

/// One strategy variant with its own wallet, recorder, and parameters.
#[derive(Serialize, Deserialize)]
pub struct StrategySlot {
    pub name: String,
    pub market: String,
    pub params: StrategyParams,
    pub status: SlotStatus,
    pub created_at: DateTime<Utc>,
    pub killed_at: Option<DateTime<Utc>>,
    pub parent: Option<String>,
    pub generation: u32,
    #[serde(skip)]
    pub wallet: SimulatedWallet,
    #[serde(skip)]
    pub recorder: TradeRecorder,
    pub cause_of_death: Option<String>,
}

impl StrategySlot {
    pub fn new(name: &str, market: &str, params: StrategyParams, initial_balance: Decimal) -> Self {
        Self {
            name: name.into(),
            market: market.into(),
            params,
            status: SlotStatus::Alive,
            created_at: Utc::now(),
            killed_at: None,
            parent: None,
            generation: 1,
            wallet: SimulatedWallet::new(initial_balance, dec!(5)),
            recorder: TradeRecorder::new(),
            cause_of_death: None,
        }
    }

    pub fn with_parent(mut self, parent: &str, gen: u32) -> Self {
        self.parent = Some(parent.into());
        self.generation = gen;
        self
    }

    pub fn is_alive(&self) -> bool {
        self.status == SlotStatus::Alive
    }

    pub fn trade_count(&self) -> usize {
        self.recorder.total_trade_count()
    }

    pub fn closed_trade_count(&self) -> usize {
        self.recorder.closed_trade_count()
    }

    pub fn win_rate(&self) -> f64 {
        let closed = self.recorder.closed_trade_count();
        if closed == 0 { return 0.0; }
        self.recorder.winning_trade_count() as f64 / closed as f64
    }

    pub fn pnl(&self) -> Decimal {
        self.wallet.equity() - dec!(100) // PnL = current equity - initial
    }

    pub fn pnl_pct(&self) -> f64 {
        self.pnl().to_f64().unwrap_or(0.0)
    }

    /// Calculate Sharpe ratio from daily returns. Simplified: use PnL / hours as proxy.
    pub fn sharpe_ratio(&self) -> f64 {
        let hours = (Utc::now() - self.created_at).num_hours().max(1) as f64;
        let pnl = self.pnl_pct();
        let hourly_return = pnl / hours;
        // Simplified Sharpe: hourly_return / estimated_volatility
        // Use PnL magnitude as volatility proxy
        let vol = pnl.abs().max(0.1);
        hourly_return / vol * (24.0_f64 * 365.0).sqrt()
    }

    pub fn age_hours(&self) -> i64 {
        (Utc::now() - self.created_at).num_hours()
    }

    pub fn kill(&mut self, reason: &str) {
        self.status = SlotStatus::Dead;
        self.killed_at = Some(Utc::now());
        self.cause_of_death = Some(reason.into());
    }
}

use rust_decimal::prelude::*;

/// Manages all strategy wallets.
pub struct StrategyWalletManager {
    slots: HashMap<String, StrategySlot>,
    max_alive: usize,
    min_alive: usize,
    initial_balance: Decimal,
}

impl StrategyWalletManager {
    pub fn new(max_alive: usize, min_alive: usize, initial_balance: Decimal) -> Self {
        Self {
            slots: HashMap::new(),
            max_alive,
            min_alive,
            initial_balance,
        }
    }

    pub fn add_slot(&mut self, slot: StrategySlot) {
        self.slots.insert(slot.name.clone(), slot);
    }

    pub fn get(&self, name: &str) -> Option<&StrategySlot> {
        self.slots.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut StrategySlot> {
        self.slots.get_mut(name)
    }

    pub fn alive_slots(&self) -> Vec<&StrategySlot> {
        self.slots.values().filter(|s| s.is_alive()).collect()
    }

    pub fn dead_slots(&self) -> Vec<&StrategySlot> {
        self.slots.values().filter(|s| !s.is_alive()).collect()
    }

    pub fn alive_count(&self) -> usize {
        self.slots.values().filter(|s| s.is_alive()).count()
    }

    pub fn dead_count(&self) -> usize {
        self.slots.values().filter(|s| !s.is_alive()).count()
    }

    pub fn total_count(&self) -> usize {
        self.slots.len()
    }

    pub fn can_kill(&self) -> bool {
        self.alive_count() > self.min_alive
    }

    pub fn can_spawn(&self) -> bool {
        self.alive_count() < self.max_alive
    }

    /// Rank alive strategies by Sharpe ratio (descending).
    /// Only includes strategies with >= min_trades.
    pub fn rank_by_sharpe(&self, min_trades: usize) -> Vec<(&str, f64)> {
        let mut ranked: Vec<(&str, f64)> = self.slots.values()
            .filter(|s| s.is_alive() && s.trade_count() >= min_trades)
            .map(|s| (s.name.as_str(), s.sharpe_ratio()))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    pub fn all_slots(&self) -> &HashMap<String, StrategySlot> {
        &self.slots
    }

    pub fn all_slots_mut(&mut self) -> &mut HashMap<String, StrategySlot> {
        &mut self.slots
    }

    /// Initialize with the default 20 strategies.
    pub fn initialize_defaults(&mut self) {
        for (name, params) in crate::mutation::initial_strategies() {
            let market = if name.starts_with("PM-") {
                "polymarket"
            } else if name.starts_with("CS-") {
                "crypto_spot"
            } else {
                "crypto_perps"
            };
            let slot = StrategySlot::new(&name, market, params, self.initial_balance);
            self.add_slot(slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_slot() {
        let params = StrategyParams::new("momentum").with_param("ema_fast", 5.0);
        let slot = StrategySlot::new("CS-momentum-fast", "crypto_spot", params, dec!(100));
        assert!(slot.is_alive());
        assert_eq!(slot.trade_count(), 0);
        assert_eq!(slot.wallet.balance(), dec!(100));
    }

    #[test]
    fn test_kill_slot() {
        let params = StrategyParams::new("dca");
        let mut slot = StrategySlot::new("test", "crypto", params, dec!(100));
        slot.kill("Bottom 10% by Sharpe");
        assert!(!slot.is_alive());
        assert!(slot.cause_of_death.is_some());
        assert!(slot.killed_at.is_some());
    }

    #[test]
    fn test_manager_initialize() {
        let mut mgr = StrategyWalletManager::new(30, 10, dec!(100));
        mgr.initialize_defaults();
        assert_eq!(mgr.alive_count(), 20);
        assert_eq!(mgr.dead_count(), 0);
    }

    #[test]
    fn test_manager_rank() {
        let mut mgr = StrategyWalletManager::new(30, 10, dec!(100));
        mgr.initialize_defaults();
        let ranked = mgr.rank_by_sharpe(0); // Include all (even 0 trades)
        assert_eq!(ranked.len(), 20);
    }

    #[test]
    fn test_can_kill_respects_minimum() {
        let mut mgr = StrategyWalletManager::new(30, 10, dec!(100));
        mgr.initialize_defaults();
        assert!(mgr.can_kill()); // 20 > 10

        // Kill down to 10
        let names: Vec<String> = mgr.alive_slots().iter().take(10).map(|s| s.name.clone()).collect();
        for name in names {
            mgr.get_mut(&name).unwrap().kill("test");
        }
        assert!(!mgr.can_kill()); // 10 == 10, can't kill more
    }

    #[test]
    fn test_can_spawn_respects_maximum() {
        let mut mgr = StrategyWalletManager::new(20, 10, dec!(100));
        mgr.initialize_defaults();
        assert!(!mgr.can_spawn()); // 20 == 20, can't spawn more
    }

    #[test]
    fn test_slot_with_parent() {
        let params = StrategyParams::new("momentum");
        let slot = StrategySlot::new("CS-momentum-fast-v2", "crypto", params, dec!(100))
            .with_parent("CS-momentum-fast", 2);
        assert_eq!(slot.parent, Some("CS-momentum-fast".into()));
        assert_eq!(slot.generation, 2);
    }
}
```

- [ ] **Step 2: Add to lib.rs**

```rust
pub mod strategy_wallet;
pub use strategy_wallet::{StrategyWalletManager, StrategySlot, SlotStatus};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-engine strategy_wallet
```

Expected: 7 tests pass

- [ ] **Step 4: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add strategy wallet manager with isolated wallets per variant"
```

---

### Task 3: Evolution Engine

**Files:**
- Create: `core/engine/src/evolution.rs`
- Modify: `core/engine/src/lib.rs`

- [ ] **Step 1: Implement Evolution Engine**

Create `core/engine/src/evolution.rs`:

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use crate::strategy_wallet::{StrategyWalletManager, StrategySlot, SlotStatus};
use crate::mutation::{mutate, StrategyParams};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEvent {
    pub timestamp: DateTime<Utc>,
    pub hour: u64,
    pub action: EvolutionAction,
    pub strategy_name: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvolutionAction {
    Killed,
    Spawned,
    Started,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionReport {
    pub hour: u64,
    pub killed: Vec<String>,
    pub spawned: Vec<String>,
    pub alive_count: usize,
    pub dead_count: usize,
    pub avg_sharpe: f64,
    pub best_strategy: Option<String>,
    pub best_sharpe: f64,
}

pub struct EvolutionEngine {
    pub timeline: Vec<EvolutionEvent>,
    pub hour: u64,
    min_trades_for_ranking: usize,
    kill_pct: f64,
    spawn_pct: f64,
}

impl EvolutionEngine {
    pub fn new() -> Self {
        Self {
            timeline: Vec::new(),
            hour: 0,
            min_trades_for_ranking: 5,
            kill_pct: 0.10,
            spawn_pct: 0.10,
        }
    }

    /// Run one evolution cycle: rank → kill bottom → spawn from top.
    pub fn evolve(&mut self, manager: &mut StrategyWalletManager) -> EvolutionReport {
        self.hour += 1;
        let mut killed = Vec::new();
        let mut spawned = Vec::new();

        // 1. Rank by Sharpe
        let ranked = manager.rank_by_sharpe(self.min_trades_for_ranking);
        let ranked_count = ranked.len();

        if ranked_count >= 3 {
            // 2. Kill bottom 10% (minimum 1)
            let kill_count = ((ranked_count as f64 * self.kill_pct).ceil() as usize).max(1);
            let to_kill: Vec<String> = ranked.iter()
                .rev()
                .take(kill_count)
                .map(|(name, _)| name.to_string())
                .collect();

            for name in &to_kill {
                if !manager.can_kill() { break; }
                if let Some(slot) = manager.get_mut(name) {
                    let reason = format!(
                        "Bottom 10% by Sharpe (hour {}). Sharpe: {:.2}, PnL: ${:.2}, WR: {:.0}%, Trades: {}",
                        self.hour, slot.sharpe_ratio(), slot.pnl_pct(), slot.win_rate() * 100.0, slot.trade_count()
                    );
                    slot.kill(&reason);
                    killed.push(name.clone());
                    self.timeline.push(EvolutionEvent {
                        timestamp: Utc::now(),
                        hour: self.hour,
                        action: EvolutionAction::Killed,
                        strategy_name: name.clone(),
                        details: reason,
                    });
                    info!("EVOLUTION KILL: {} — {}", name, slot.cause_of_death.as_deref().unwrap_or(""));
                }
            }

            // 3. Spawn from top 10%
            let spawn_count = ((ranked_count as f64 * self.spawn_pct).ceil() as usize).max(1);
            let top: Vec<(String, StrategyParams, String, u32)> = ranked.iter()
                .take(spawn_count)
                .filter_map(|(name, _)| {
                    manager.get(name).map(|s| (
                        s.name.clone(),
                        s.params.clone(),
                        s.market.clone(),
                        s.generation,
                    ))
                })
                .collect();

            for (parent_name, parent_params, market, gen) in top {
                if !manager.can_spawn() { break; }
                let seed = (self.hour * 31 + parent_name.len() as u64 * 17) % 1000;
                let (new_params, mutation) = mutate(&parent_params, seed);
                let new_name = format!("{}-v{}", parent_name.split("-v").next().unwrap_or(&parent_name), gen + 1);

                // Don't spawn if name already exists
                if manager.get(&new_name).is_some() {
                    continue;
                }

                let slot = StrategySlot::new(&new_name, &market, new_params, dec!(100))
                    .with_parent(&parent_name, gen + 1);
                manager.add_slot(slot);
                spawned.push(new_name.clone());

                let details = format!(
                    "Spawned from {} (gen {}). Mutated {}: {:.2} → {:.2} ({:+.0}%)",
                    parent_name, gen, mutation.param_name, mutation.old_value, mutation.new_value, mutation.change_pct
                );
                self.timeline.push(EvolutionEvent {
                    timestamp: Utc::now(),
                    hour: self.hour,
                    action: EvolutionAction::Spawned,
                    strategy_name: new_name,
                    details,
                });
            }
        }

        // Build report
        let alive = manager.alive_slots();
        let avg_sharpe = if alive.is_empty() { 0.0 } else {
            alive.iter().map(|s| s.sharpe_ratio()).sum::<f64>() / alive.len() as f64
        };
        let (best_name, best_sharpe) = ranked.first()
            .map(|(n, s)| (Some(n.to_string()), *s))
            .unwrap_or((None, 0.0));

        let report = EvolutionReport {
            hour: self.hour,
            killed: killed.clone(),
            spawned: spawned.clone(),
            alive_count: manager.alive_count(),
            dead_count: manager.dead_count(),
            avg_sharpe,
            best_strategy: best_name,
            best_sharpe,
        };

        info!(
            "EVOLUTION HOUR {}: killed {}, spawned {}, alive {}, dead {}, avg Sharpe {:.2}",
            self.hour, killed.len(), spawned.len(), report.alive_count, report.dead_count, avg_sharpe
        );

        report
    }

    pub fn recent_events(&self, limit: usize) -> Vec<&EvolutionEvent> {
        self.timeline.iter().rev().take(limit).collect()
    }

    pub fn events_by_hour(&self, hour: u64) -> Vec<&EvolutionEvent> {
        self.timeline.iter().filter(|e| e.hour == hour).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mutation::StrategyParams;

    fn setup_manager() -> StrategyWalletManager {
        let mut mgr = StrategyWalletManager::new(30, 5, dec!(100));
        mgr.initialize_defaults();
        mgr
    }

    #[test]
    fn test_new_engine() {
        let engine = EvolutionEngine::new();
        assert_eq!(engine.hour, 0);
        assert_eq!(engine.timeline.len(), 0);
    }

    #[test]
    fn test_evolve_no_ranked_strategies() {
        let mut engine = EvolutionEngine::new();
        let mut mgr = setup_manager();
        // No trades → nothing ranked (min_trades = 5) → no kills/spawns
        let report = engine.evolve(&mut mgr);
        assert_eq!(report.hour, 1);
        assert_eq!(report.killed.len(), 0);
        assert_eq!(report.spawned.len(), 0);
        assert_eq!(report.alive_count, 20);
    }

    #[test]
    fn test_evolve_kills_bottom() {
        let mut engine = EvolutionEngine::new();
        engine.min_trades_for_ranking = 0; // Rank even with 0 trades
        let mut mgr = setup_manager();
        let report = engine.evolve(&mut mgr);
        // 20 strategies, bottom 10% = 2 killed, top 10% = 2 spawned
        assert!(report.killed.len() >= 1);
        assert!(report.alive_count < 22); // Some killed, some spawned
    }

    #[test]
    fn test_timeline_records_events() {
        let mut engine = EvolutionEngine::new();
        engine.min_trades_for_ranking = 0;
        let mut mgr = setup_manager();
        engine.evolve(&mut mgr);
        assert!(!engine.timeline.is_empty());
    }

    #[test]
    fn test_evolve_respects_min_alive() {
        let mut engine = EvolutionEngine::new();
        engine.min_trades_for_ranking = 0;
        let mut mgr = StrategyWalletManager::new(30, 18, dec!(100)); // min_alive = 18
        mgr.initialize_defaults(); // 20 alive
        // Can only kill 2 (20 - 18 = 2 buffer)
        let report = engine.evolve(&mut mgr);
        assert!(mgr.alive_count() >= 18);
    }

    #[test]
    fn test_recent_events() {
        let mut engine = EvolutionEngine::new();
        engine.min_trades_for_ranking = 0;
        let mut mgr = setup_manager();
        engine.evolve(&mut mgr);
        engine.evolve(&mut mgr);
        let recent = engine.recent_events(5);
        assert!(!recent.is_empty());
    }
}
```

- [ ] **Step 2: Add to lib.rs**

```rust
pub mod evolution;
pub use evolution::{EvolutionEngine, EvolutionEvent, EvolutionAction, EvolutionReport};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-engine evolution
```

Expected: 6 tests pass

- [ ] **Step 4: Run full workspace tests**

```bash
cargo test --workspace
```

Expected: all existing + 19 new tests pass

- [ ] **Step 5: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add Darwinian evolution engine with hourly rank/kill/spawn"
```

---

### Task 4: Full Verification

- [ ] **Step 1: Run all tests**

```bash
cargo test --workspace
```

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace
```

- [ ] **Step 3: Merge**

```bash
git checkout dev
git merge feature/strategy-evolution-phase1-2
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | Mutation engine (params + mutate + initial_strategies) | 6 |
| 2 | Strategy wallet manager (slots, create/track/kill) | 7 |
| 3 | Evolution engine (hourly rank/kill/spawn) | 6 |
| 4 | Full verification | All |

**Total: 4 tasks, 19 new tests**

Next plans:
- **Phase 3:** Wire into trading loop + API endpoints
- **Phase 4:** Evolution dashboard page (`/evolution`)
- **Phase 5:** Integration test (multi-hour run)
