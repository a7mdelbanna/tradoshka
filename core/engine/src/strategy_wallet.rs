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
        // Market-aware fee rate and slippage
        let (fee_rate, slippage) = if market.contains("poly") || name.starts_with("PM-") {
            (dec!(0.002), dec!(5))    // 0.2%, 5 bps slippage (Polymarket)
        } else if market.contains("perps") || name.starts_with("CP-") {
            (dec!(0.0004), dec!(5))   // 0.04% (futures), 5 bps slippage
        } else if market.contains("meme") || name.starts_with("MC-") {
            (dec!(0.003), dec!(100))  // 0.3% (Raydium + gas proxy), 100 bps (1%) slippage
        } else {
            (dec!(0.001), dec!(5))    // 0.1% (spot crypto default), 5 bps slippage
        };
        Self {
            name: name.into(),
            market: market.into(),
            params,
            status: SlotStatus::Alive,
            created_at: Utc::now(),
            killed_at: None,
            parent: None,
            generation: 1,
            wallet: SimulatedWallet::new(initial_balance, slippage, fee_rate),
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

    /// Average win / average loss ratio from closed trades.
    /// Returns 0.0 if there are no wins or no losses.
    pub fn avg_win_loss_ratio(&self) -> f64 {
        let trades = self.recorder.all_trades();

        let wins: Vec<f64> = trades
            .iter()
            .filter(|t| t.is_closed && t.pnl.map_or(false, |p| p > Decimal::ZERO))
            .filter_map(|t| t.pnl)
            .map(|p| p.to_f64().unwrap_or(0.0).abs())
            .collect();

        let losses: Vec<f64> = trades
            .iter()
            .filter(|t| t.is_closed && t.pnl.map_or(false, |p| p < Decimal::ZERO))
            .filter_map(|t| t.pnl)
            .map(|p| p.to_f64().unwrap_or(0.0).abs())
            .collect();

        if wins.is_empty() || losses.is_empty() {
            return 0.0;
        }

        let avg_win = wins.iter().sum::<f64>() / wins.len() as f64;
        let avg_loss = losses.iter().sum::<f64>() / losses.len() as f64;

        avg_win / avg_loss
    }

    /// Kelly-based position size in USD.
    /// Falls back to a fixed formula until 10+ closed trades are available.
    pub fn kelly_position_size(&self, token_price: Decimal) -> Decimal {
        let equity = self.wallet.equity();
        if equity <= Decimal::ZERO || token_price <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        let closed_count = self.recorder.closed_trade_count();

        if closed_count < 10 {
            // Fallback: fixed formula
            let capital_usage_pct = Decimal::try_from(self.params.get("capital_usage_pct")).unwrap_or(dec!(60));
            let auto_position_count = Decimal::try_from(self.params.get("auto_position_count")).unwrap_or(dec!(15));
            let auto_leverage = Decimal::try_from(self.params.get("auto_leverage")).unwrap_or(dec!(1));

            if auto_position_count <= Decimal::ZERO {
                return Decimal::ZERO;
            }
            return equity * (capital_usage_pct / dec!(100)) / auto_position_count * auto_leverage;
        }

        // Half-Kelly calculation
        let wr = self.win_rate();
        let wl_ratio = self.avg_win_loss_ratio();

        if wl_ratio <= 0.0 {
            return (equity * dec!(5) / dec!(1000)).max(equity * dec!(5) / dec!(1000));
        }

        let kelly = (wr * wl_ratio - (1.0 - wr)) / wl_ratio;

        if kelly <= 0.0 {
            // Floor: 0.5% of equity
            return equity * dec!(5) / dec!(1000);
        }

        let half_kelly = kelly / 2.0;
        let half_kelly_dec = Decimal::try_from(half_kelly).unwrap_or(dec!(0));
        let position_usd = equity * half_kelly_dec;

        // Clamp: min 0.5%, max 5% of equity
        let min_size = equity * dec!(5) / dec!(1000);
        let max_size = equity * dec!(5) / dec!(100);

        position_usd.max(min_size).min(max_size)
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

    /// Public wrapper for computing fitness from a slot reference (used in evolution.rs).
    pub fn fitness_score_for(slot: &StrategySlot) -> f64 {
        Self::fitness_score(slot)
    }

    /// Calculate a composite fitness score for evolution ranking.
    /// Not just Sharpe — considers multiple dimensions.
    fn fitness_score(slot: &StrategySlot) -> f64 {
        let sharpe = slot.sharpe_ratio();
        let pnl = slot.pnl_pct();
        let trades = slot.trade_count() as f64;
        let win_rate = slot.win_rate();

        // Composite score (weighted):
        // 40% Sharpe ratio (risk-adjusted returns)
        // 25% Raw PnL (absolute performance)
        // 20% Win rate (consistency)
        // 15% Trade count (activity — inactive strategies are penalized)
        let sharpe_score = sharpe.clamp(-100.0, 100.0) / 100.0; // Normalize to -1 to 1
        let pnl_score = (pnl / 10.0).clamp(-1.0, 1.0);          // $10 = max score
        let wr_score = (win_rate * 2.0 - 1.0).clamp(-1.0, 1.0);  // 50% WR = 0, 100% = 1
        let activity_score = (trades / 20.0).clamp(0.0, 1.0);     // 20+ trades = max

        sharpe_score * 0.40 + pnl_score * 0.25 + wr_score * 0.20 + activity_score * 0.15
    }

    /// Rank alive strategies by composite fitness score (descending).
    /// Only includes strategies with >= min_trades.
    pub fn rank_by_fitness(&self, min_trades: usize) -> Vec<(String, f64)> {
        let mut ranked: Vec<(String, f64)> = self.slots.values()
            .filter(|s| s.is_alive() && s.trade_count() >= min_trades)
            .map(|s| (s.name.clone(), Self::fitness_score(s)))
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
            } else if name.starts_with("MC-") {
                "meme_coins"
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
        let mut mgr = StrategyWalletManager::new(200, 20, dec!(100));
        mgr.initialize_defaults();
        assert_eq!(mgr.alive_count(), 160);
        assert_eq!(mgr.dead_count(), 0);
    }

    #[test]
    fn test_manager_rank() {
        let mut mgr = StrategyWalletManager::new(200, 20, dec!(100));
        mgr.initialize_defaults();
        let ranked = mgr.rank_by_sharpe(0); // Include all (even 0 trades)
        assert_eq!(ranked.len(), 160);
    }

    #[test]
    fn test_can_kill_respects_minimum() {
        let mut mgr = StrategyWalletManager::new(200, 20, dec!(100));
        mgr.initialize_defaults();
        assert!(mgr.can_kill()); // 160 > 20

        // Kill down to 20
        let names: Vec<String> = mgr.alive_slots().iter().take(140).map(|s| s.name.clone()).collect();
        for name in names {
            mgr.get_mut(&name).unwrap().kill("test");
        }
        assert!(!mgr.can_kill()); // 20 == 20, can't kill more
    }

    #[test]
    fn test_can_spawn_respects_maximum() {
        let mut mgr = StrategyWalletManager::new(160, 20, dec!(100));
        mgr.initialize_defaults();
        assert!(!mgr.can_spawn()); // 160 == 160, can't spawn more
    }

    #[test]
    fn test_slot_with_parent() {
        let params = StrategyParams::new("momentum");
        let slot = StrategySlot::new("CS-momentum-fast-v2", "crypto", params, dec!(100))
            .with_parent("CS-momentum-fast", 2);
        assert_eq!(slot.parent, Some("CS-momentum-fast".into()));
        assert_eq!(slot.generation, 2);
    }

    #[test]
    fn test_kelly_position_size_with_history() {
        let params = StrategyParams::new("mc_trend_v2")
            .with_param("auto_position_count", 15.0)
            .with_param("capital_usage_pct", 60.0)
            .with_param("auto_leverage", 5.0);
        let mut slot = StrategySlot::new("MC-TR-test", "meme_coins", params, dec!(1000));

        for i in 0..20u32 {
            let pnl = if i < 8 { dec!(10) } else { dec!(-5) };
            let trade = crate::trade_recorder::TradeRecord {
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
        let expected = dec!(100) * dec!(0.60) / dec!(15) * dec!(5);
        assert!((size - expected).abs() < dec!(1), "Before 10 trades, should use fixed formula. Got {} expected ~{}", size, expected);
    }
}
