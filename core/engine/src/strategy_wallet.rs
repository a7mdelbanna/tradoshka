use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::wallet::SimulatedWallet;
use crate::trade_recorder::TradeRecorder;
use crate::mutation::StrategyParams;
use crate::indicator_state::StrategyIndicatorEngine;

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
    #[serde(skip)]
    pub indicators: StrategyIndicatorEngine,
    pub cause_of_death: Option<String>,
}

impl StrategySlot {
    pub fn new(name: &str, market: &str, params: StrategyParams, initial_balance: Decimal) -> Self {
        let indicators = StrategyIndicatorEngine::from_params(&params);
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
            indicators,
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
        let minutes = (Utc::now() - self.created_at).num_minutes().max(1) as f64;
        let pnl = self.pnl_pct();
        let trade_count = self.trade_count() as f64;
        if trade_count == 0.0 { return 0.0; }
        // Raw PnL annualized then divided by baseline vol. Different EMA periods produce
        // different pnl magnitudes, so Sharpe values diverge even within the first cycle.
        let annualized_pnl = pnl * (525_960.0_f64 / minutes).sqrt();
        let baseline_vol = 0.5_f64 * trade_count.sqrt();
        annualized_pnl / baseline_vol
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
        let mut mgr = StrategyWalletManager::new(80, 15, dec!(100));
        mgr.initialize_defaults();
        assert_eq!(mgr.alive_count(), 60);
        assert_eq!(mgr.dead_count(), 0);
    }

    #[test]
    fn test_manager_rank() {
        let mut mgr = StrategyWalletManager::new(80, 15, dec!(100));
        mgr.initialize_defaults();
        let ranked = mgr.rank_by_sharpe(0); // Include all (even 0 trades)
        assert_eq!(ranked.len(), 60);
    }

    #[test]
    fn test_can_kill_respects_minimum() {
        let mut mgr = StrategyWalletManager::new(80, 15, dec!(100));
        mgr.initialize_defaults();
        assert!(mgr.can_kill()); // 60 > 15

        // Kill down to 15
        let names: Vec<String> = mgr.alive_slots().iter().take(45).map(|s| s.name.clone()).collect();
        for name in names {
            mgr.get_mut(&name).unwrap().kill("test");
        }
        assert!(!mgr.can_kill()); // 15 == 15, can't kill more
    }

    #[test]
    fn test_can_spawn_respects_maximum() {
        let mut mgr = StrategyWalletManager::new(60, 15, dec!(100));
        mgr.initialize_defaults();
        assert!(!mgr.can_spawn()); // 60 == 60, can't spawn more
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
