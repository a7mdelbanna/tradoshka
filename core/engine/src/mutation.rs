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
        let mut params = HashMap::new();
        // Add default execution params based on strategy type
        match strategy_type {
            "scalp" => {
                params.insert("auto_leverage".into(), 10.0);
                params.insert("auto_timeframe".into(), 5.0);
                params.insert("auto_position_count".into(), 5.0);
                params.insert("capital_usage_pct".into(), 90.0);
                params.insert("stop_loss_atr_mult".into(), 2.0);
                params.insert("take_profit_rr".into(), 2.0);
            }
            "momentum" => {
                params.insert("auto_leverage".into(), 7.0);
                params.insert("auto_timeframe".into(), 60.0);
                params.insert("auto_position_count".into(), 8.0);
                params.insert("capital_usage_pct".into(), 80.0);
                params.insert("stop_loss_atr_mult".into(), 2.0);
                params.insert("take_profit_rr".into(), 2.5);
            }
            "dca" => {
                params.insert("auto_leverage".into(), 2.0);
                params.insert("auto_timeframe".into(), 60.0);
                params.insert("auto_position_count".into(), 15.0);
                params.insert("capital_usage_pct".into(), 95.0);
                params.insert("stop_loss_atr_mult".into(), 3.0);
                params.insert("take_profit_rr".into(), 1.5);
            }
            "grid" => {
                params.insert("auto_leverage".into(), 4.0);
                params.insert("auto_timeframe".into(), 15.0);
                params.insert("auto_position_count".into(), 12.0);
                params.insert("capital_usage_pct".into(), 85.0);
                params.insert("stop_loss_atr_mult".into(), 2.5);
                params.insert("take_profit_rr".into(), 2.0);
            }
            "meanrev" => {
                params.insert("auto_leverage".into(), 7.0);
                params.insert("auto_timeframe".into(), 30.0);
                params.insert("auto_position_count".into(), 4.0);
                params.insert("capital_usage_pct".into(), 70.0);
                params.insert("stop_loss_atr_mult".into(), 2.0);
                params.insert("take_profit_rr".into(), 2.0);
            }
            "mispricing" | "arb" | "funding_arb" => {
                params.insert("auto_leverage".into(), 12.0);
                params.insert("auto_timeframe".into(), 5.0);
                params.insert("auto_position_count".into(), 3.0);
                params.insert("capital_usage_pct".into(), 60.0);
                params.insert("stop_loss_atr_mult".into(), 1.5);
                params.insert("take_profit_rr".into(), 3.0);
            }
            "market_making" => {
                params.insert("auto_leverage".into(), 4.0);
                params.insert("auto_timeframe".into(), 15.0);
                params.insert("auto_position_count".into(), 8.0);
                params.insert("capital_usage_pct".into(), 80.0);
                params.insert("stop_loss_atr_mult".into(), 2.0);
                params.insert("take_profit_rr".into(), 1.8);
            }
            "copy" => {
                params.insert("auto_leverage".into(), 5.0);
                params.insert("auto_timeframe".into(), 60.0);
                params.insert("auto_position_count".into(), 5.0);
                params.insert("capital_usage_pct".into(), 75.0);
                params.insert("stop_loss_atr_mult".into(), 2.0);
                params.insert("take_profit_rr".into(), 2.0);
            }
            "pm_copy" => {
                params.insert("auto_position_count".into(), 8.0);
                params.insert("capital_usage_pct".into(), 70.0);
                params.insert("min_trade_size".into(), 100.0);
                params.insert("min_win_rate".into(), 50.0);
                params.insert("price_min".into(), 0.20);
                params.insert("price_max".into(), 0.80);
                params.insert("min_volume".into(), 5000.0);
            }
            "pm_niche" => {
                params.insert("auto_position_count".into(), 6.0);
                params.insert("capital_usage_pct".into(), 75.0);
                params.insert("niche".into(), 0.0);
                params.insert("price_min".into(), 0.20);
                params.insert("price_max".into(), 0.80);
                params.insert("confidence_threshold".into(), 0.55);
            }
            "mc_snipe" => {
                params.insert("auto_leverage".into(), 1.0);
                params.insert("auto_timeframe".into(), 1.0); // 1 minute
                params.insert("auto_position_count".into(), 3.0);
                params.insert("capital_usage_pct".into(), 90.0);
                params.insert("stop_loss_atr_mult".into(), 1.0); // Tight for memes
                params.insert("take_profit_rr".into(), 2.0);
                params.insert("time_limit".into(), 15.0); // minutes
                params.insert("target_mult".into(), 2.0);
                params.insert("min_safety".into(), 30.0);
            }
            "mc_trend" => {
                params.insert("auto_leverage".into(), 1.0);
                params.insert("auto_timeframe".into(), 5.0);
                params.insert("auto_position_count".into(), 5.0);
                params.insert("capital_usage_pct".into(), 80.0);
                params.insert("stop_loss_atr_mult".into(), 1.5);
                params.insert("take_profit_rr".into(), 2.0);
                params.insert("target_mult".into(), 2.0);
                params.insert("min_safety".into(), 50.0);
            }
            "mc_whale" => {
                params.insert("auto_leverage".into(), 1.0);
                params.insert("auto_timeframe".into(), 5.0);
                params.insert("auto_position_count".into(), 4.0);
                params.insert("capital_usage_pct".into(), 75.0);
                params.insert("stop_loss_atr_mult".into(), 1.5);
                params.insert("take_profit_rr".into(), 2.0);
                params.insert("min_safety".into(), 40.0);
            }
            _ => {
                params.insert("auto_leverage".into(), 5.0);
                params.insert("auto_timeframe".into(), 15.0);
                params.insert("auto_position_count".into(), 5.0);
                params.insert("capital_usage_pct".into(), 75.0);
                params.insert("stop_loss_atr_mult".into(), 2.0);
                params.insert("take_profit_rr".into(), 2.0);
            }
        }
        Self { strategy_type: strategy_type.into(), params }
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
        let mut params = match self.strategy_type.as_str() {
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
            "pm_copy" => vec!["min_trade_size".into(), "min_win_rate".into(), "price_min".into(), "price_max".into(), "min_volume".into()],
            "pm_niche" => vec!["price_min".into(), "price_max".into(), "confidence_threshold".into()],
            "mc_snipe" => vec!["time_limit".into(), "target_mult".into(), "min_safety".into(), "min_buyers".into(), "max_age_mins".into(), "max_mcap".into()],
            "mc_trend" => vec!["ema_fast".into(), "ema_slow".into(), "volume_mult".into(), "target_mult".into(), "min_safety".into(), "dip_pct".into(), "breakout_pct".into()],
            "mc_whale" => vec!["min_whale_wr".into(), "min_consensus".into(), "min_safety".into(), "max_delay_secs".into()],
            _ => self.params.keys().cloned().collect(),
        };
        // Add execution params for ALL strategy types
        params.extend(vec![
            "auto_leverage".into(), "auto_timeframe".into(),
            "auto_position_count".into(), "capital_usage_pct".into(),
            "stop_loss_atr_mult".into(), "take_profit_rr".into(),
        ]);
        params
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

/// Breed two strategies by combining their parameters.
/// Takes all params from parent A, then overlays params unique to parent B.
/// Shared params: randomly pick from either parent (using seed).
pub fn crossover(parent_a: &StrategyParams, parent_b: &StrategyParams, seed: u64) -> StrategyParams {
    let mut child_params = HashMap::new();

    // Start with parent A's params
    for (k, v) in &parent_a.params {
        child_params.insert(k.clone(), *v);
    }

    // For each param in parent B:
    // - If parent A also has it: pick randomly based on seed
    // - If parent A doesn't have it: add it from B
    for (k, v) in &parent_b.params {
        if parent_a.params.contains_key(k) {
            // Both parents have this param — pick based on seed
            let pick_b = (seed.wrapping_mul(k.len() as u64 + 7)) % 2 == 0;
            if pick_b {
                child_params.insert(k.clone(), *v);
            }
        } else {
            child_params.insert(k.clone(), *v);
        }
    }

    StrategyParams {
        strategy_type: "hybrid".into(),
        params: child_params,
    }
}

/// Generate a strategy with completely random parameters within sane bounds.
pub fn random_explorer(seed: u64) -> StrategyParams {
    let mut params = HashMap::new();

    // Deterministic "random" from seed
    let r = |s: u64, min: f64, max: f64| -> f64 {
        let v = ((s.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff) as f64 / 0x7fffffff as f64;
        min + v * (max - min)
    };

    params.insert("ema_fast".into(), r(seed, 2.0, 15.0).round());
    params.insert("ema_slow".into(), r(seed.wrapping_add(1), 5.0, 50.0).round());
    params.insert("rsi_threshold".into(), r(seed.wrapping_add(2), 30.0, 70.0).round());
    params.insert("bb_period".into(), r(seed.wrapping_add(3), 5.0, 40.0).round());
    params.insert("bb_std".into(), (r(seed.wrapping_add(4), 1.0, 3.0) * 10.0).round() / 10.0);
    params.insert("auto_leverage".into(), r(seed.wrapping_add(5), 1.0, 20.0).round());
    params.insert("auto_timeframe".into(), [1.0, 5.0, 15.0, 60.0, 240.0][(seed as usize % 5)]);
    params.insert("auto_position_count".into(), r(seed.wrapping_add(6), 3.0, 20.0).round());
    params.insert("capital_usage_pct".into(), r(seed.wrapping_add(7), 50.0, 95.0).round());
    params.insert("stop_loss_atr_mult".into(), (r(seed.wrapping_add(8), 1.0, 4.0) * 10.0).round() / 10.0);
    params.insert("take_profit_rr".into(), (r(seed.wrapping_add(9), 1.5, 5.0) * 10.0).round() / 10.0);

    StrategyParams {
        strategy_type: "explorer".into(),
        params,
    }
}

/// Create the default parameter sets for the initial 120 strategies (40 per market).
pub fn initial_strategies() -> Vec<(String, StrategyParams)> {
    vec![
        // ── Polymarket Group A: Copy Trading (20) — prefix PM-CT- ────────────────
        ("PM-CT-top-pnl-loose".into(),    StrategyParams::new("pm_copy").with_param("min_trade_size", 50.0).with_param("min_win_rate", 40.0).with_param("price_min", 0.10).with_param("price_max", 0.90).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-top-pnl-strict".into(),   StrategyParams::new("pm_copy").with_param("min_trade_size", 200.0).with_param("min_win_rate", 60.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-whale-large".into(),       StrategyParams::new("pm_copy").with_param("min_trade_size", 500.0).with_param("min_win_rate", 45.0).with_param("price_min", 0.15).with_param("price_max", 0.85).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-whale-small".into(),       StrategyParams::new("pm_copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 50.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-high-wr".into(),           StrategyParams::new("pm_copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 70.0).with_param("price_min", 0.25).with_param("price_max", 0.75).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-consensus-3".into(),       StrategyParams::new("pm_copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 50.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-consensus-5".into(),       StrategyParams::new("pm_copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 50.0).with_param("price_min", 0.15).with_param("price_max", 0.85).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-contrarian".into(),        StrategyParams::new("pm_copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 30.0).with_param("price_min", 0.10).with_param("price_max", 0.40).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-momentum-follow".into(),   StrategyParams::new("pm_copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 50.0).with_param("price_min", 0.30).with_param("price_max", 0.70).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-value-deep".into(),        StrategyParams::new("pm_copy").with_param("min_trade_size", 50.0).with_param("min_win_rate", 45.0).with_param("price_min", 0.05).with_param("price_max", 0.40).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-safe-bets".into(),         StrategyParams::new("pm_copy").with_param("min_trade_size", 200.0).with_param("min_win_rate", 65.0).with_param("price_min", 0.60).with_param("price_max", 0.90).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-balanced".into(),          StrategyParams::new("pm_copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 55.0).with_param("price_min", 0.25).with_param("price_max", 0.75).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-aggressive".into(),        StrategyParams::new("pm_copy").with_param("min_trade_size", 50.0).with_param("min_win_rate", 40.0).with_param("price_min", 0.10).with_param("price_max", 0.50).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-conservative".into(),      StrategyParams::new("pm_copy").with_param("min_trade_size", 200.0).with_param("min_win_rate", 60.0).with_param("price_min", 0.35).with_param("price_max", 0.65).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-new-markets".into(),       StrategyParams::new("pm_copy").with_param("min_trade_size", 50.0).with_param("min_win_rate", 45.0).with_param("price_min", 0.30).with_param("price_max", 0.70).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-high-volume".into(),       StrategyParams::new("pm_copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 50.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("min_volume", 10000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-low-volume".into(),        StrategyParams::new("pm_copy").with_param("min_trade_size", 50.0).with_param("min_win_rate", 45.0).with_param("price_min", 0.15).with_param("price_max", 0.85).with_param("min_volume", 1000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-spread-hunter".into(),     StrategyParams::new("pm_copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 50.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-quick-flip".into(),        StrategyParams::new("pm_copy").with_param("min_trade_size", 50.0).with_param("min_win_rate", 45.0).with_param("price_min", 0.30).with_param("price_max", 0.70).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),
        ("PM-CT-patient".into(),           StrategyParams::new("pm_copy").with_param("min_trade_size", 150.0).with_param("min_win_rate", 55.0).with_param("price_min", 0.25).with_param("price_max", 0.75).with_param("min_volume", 5000.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 70.0)),

        // ── Polymarket Group B: AI/Analytics Niche Prediction (20) — prefix PM-AI- ─
        // niche encoding: 1=politics, 2=sports, 3=crypto, 4=weather, 5=economics, 6=tech, 7=entertainment, 8=science, 9=legal, 0=general
        ("PM-AI-politics-us".into(),      StrategyParams::new("pm_niche").with_param("niche", 1.0).with_param("price_min", 0.15).with_param("price_max", 0.85).with_param("confidence_threshold", 0.60).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-politics-global".into(),  StrategyParams::new("pm_niche").with_param("niche", 1.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("confidence_threshold", 0.55).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-sports-nba".into(),       StrategyParams::new("pm_niche").with_param("niche", 2.0).with_param("price_min", 0.15).with_param("price_max", 0.85).with_param("confidence_threshold", 0.50).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-sports-soccer".into(),    StrategyParams::new("pm_niche").with_param("niche", 2.0).with_param("price_min", 0.10).with_param("price_max", 0.90).with_param("confidence_threshold", 0.45).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-sports-general".into(),   StrategyParams::new("pm_niche").with_param("niche", 2.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("confidence_threshold", 0.55).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-crypto-price".into(),     StrategyParams::new("pm_niche").with_param("niche", 3.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("confidence_threshold", 0.60).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-crypto-events".into(),    StrategyParams::new("pm_niche").with_param("niche", 3.0).with_param("price_min", 0.15).with_param("price_max", 0.85).with_param("confidence_threshold", 0.50).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-weather".into(),          StrategyParams::new("pm_niche").with_param("niche", 4.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("confidence_threshold", 0.55).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-economics".into(),        StrategyParams::new("pm_niche").with_param("niche", 5.0).with_param("price_min", 0.25).with_param("price_max", 0.75).with_param("confidence_threshold", 0.60).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-tech".into(),             StrategyParams::new("pm_niche").with_param("niche", 6.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("confidence_threshold", 0.55).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-entertainment".into(),    StrategyParams::new("pm_niche").with_param("niche", 7.0).with_param("price_min", 0.15).with_param("price_max", 0.85).with_param("confidence_threshold", 0.45).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-science".into(),          StrategyParams::new("pm_niche").with_param("niche", 8.0).with_param("price_min", 0.25).with_param("price_max", 0.75).with_param("confidence_threshold", 0.60).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-legal".into(),            StrategyParams::new("pm_niche").with_param("niche", 9.0).with_param("price_min", 0.20).with_param("price_max", 0.80).with_param("confidence_threshold", 0.55).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-general-broad".into(),    StrategyParams::new("pm_niche").with_param("niche", 0.0).with_param("price_min", 0.10).with_param("price_max", 0.90).with_param("confidence_threshold", 0.40).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-general-focused".into(),  StrategyParams::new("pm_niche").with_param("niche", 0.0).with_param("price_min", 0.30).with_param("price_max", 0.70).with_param("confidence_threshold", 0.65).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-contrarian".into(),       StrategyParams::new("pm_niche").with_param("niche", 0.0).with_param("price_min", 0.10).with_param("price_max", 0.40).with_param("confidence_threshold", 0.50).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-high-conviction".into(),  StrategyParams::new("pm_niche").with_param("niche", 0.0).with_param("price_min", 0.25).with_param("price_max", 0.75).with_param("confidence_threshold", 0.75).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-diversified".into(),      StrategyParams::new("pm_niche").with_param("niche", 0.0).with_param("price_min", 0.15).with_param("price_max", 0.85).with_param("confidence_threshold", 0.50).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-momentum".into(),         StrategyParams::new("pm_niche").with_param("niche", 0.0).with_param("price_min", 0.30).with_param("price_max", 0.70).with_param("confidence_threshold", 0.55).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),
        ("PM-AI-mean-revert".into(),      StrategyParams::new("pm_niche").with_param("niche", 0.0).with_param("price_min", 0.15).with_param("price_max", 0.45).with_param("confidence_threshold", 0.50).with_param("auto_position_count", 6.0).with_param("capital_usage_pct", 75.0)),

        // ── Crypto Spot (40) ─────────────────────────────────────────────────────
        // Momentum variants (original 4)
        ("CS-momentum-fast".into(), StrategyParams::new("momentum").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("rsi_threshold", 50.0)),
        ("CS-momentum-slow".into(), StrategyParams::new("momentum").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("rsi_threshold", 50.0)),
        ("CS-momentum-ultra".into(), StrategyParams::new("momentum").with_param("ema_fast", 3.0).with_param("ema_slow", 8.0).with_param("rsi_threshold", 45.0)),
        ("CS-momentum-conservative".into(), StrategyParams::new("momentum").with_param("ema_fast", 12.0).with_param("ema_slow", 26.0).with_param("rsi_threshold", 55.0)),
        // DCA variants (original 4)
        ("CS-dca-aggressive".into(), StrategyParams::new("dca").with_param("buy_interval", 6.0).with_param("rsi_oversold", 40.0)),
        ("CS-dca-conservative".into(), StrategyParams::new("dca").with_param("buy_interval", 24.0).with_param("rsi_oversold", 30.0)),
        ("CS-dca-moderate".into(), StrategyParams::new("dca").with_param("buy_interval", 12.0).with_param("rsi_oversold", 35.0)),
        ("CS-dca-ultra".into(), StrategyParams::new("dca").with_param("buy_interval", 3.0).with_param("rsi_oversold", 45.0)),
        // Grid variants (original 4)
        ("CS-grid-tight".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.5).with_param("grid_count", 10.0)),
        ("CS-grid-wide".into(), StrategyParams::new("grid").with_param("spacing_pct", 2.0).with_param("grid_count", 5.0)),
        ("CS-grid-medium".into(), StrategyParams::new("grid").with_param("spacing_pct", 1.0).with_param("grid_count", 8.0)),
        ("CS-grid-dense".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.3).with_param("grid_count", 15.0)),
        // Mean reversion (original 3)
        ("CS-meanrev-bollinger".into(), StrategyParams::new("meanrev").with_param("bb_period", 20.0).with_param("bb_std", 2.0)),
        ("CS-meanrev-tight".into(), StrategyParams::new("meanrev").with_param("bb_period", 10.0).with_param("bb_std", 1.5)),
        ("CS-meanrev-wide".into(), StrategyParams::new("meanrev").with_param("bb_period", 30.0).with_param("bb_std", 2.5)),
        // Copy trading (original 2)
        ("CS-copy-whales".into(), StrategyParams::new("copy").with_param("min_trade_size", 10000.0).with_param("min_win_rate", 50.0)),
        ("CS-copy-smart".into(), StrategyParams::new("copy").with_param("min_trade_size", 5000.0).with_param("min_win_rate", 65.0)),
        // Market making (original 2)
        ("CS-mm-tight".into(), StrategyParams::new("market_making").with_param("spread_bps", 15.0).with_param("max_inventory", 500.0)),
        ("CS-mm-wide".into(), StrategyParams::new("market_making").with_param("spread_bps", 30.0).with_param("max_inventory", 1000.0)),
        // Arbitrage (original 1)
        ("CS-arb-cross".into(), StrategyParams::new("arb").with_param("min_edge", 0.5).with_param("min_volume", 10000.0)),
        // ── Crypto Spot additional 20 ────────────────────────────────────────────
        ("CS-momentum-medium".into(), StrategyParams::new("momentum").with_param("ema_fast", 7.0).with_param("ema_slow", 17.0).with_param("rsi_threshold", 52.0)),
        ("CS-momentum-aggressive".into(), StrategyParams::new("momentum").with_param("ema_fast", 3.0).with_param("ema_slow", 7.0).with_param("rsi_threshold", 45.0)),
        ("CS-dca-micro".into(), StrategyParams::new("dca").with_param("buy_interval", 2.0).with_param("rsi_oversold", 48.0)),
        ("CS-dca-smart".into(), StrategyParams::new("dca").with_param("buy_interval", 8.0).with_param("rsi_oversold", 32.0)),
        ("CS-grid-micro".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.2).with_param("grid_count", 20.0)),
        ("CS-grid-adaptive".into(), StrategyParams::new("grid").with_param("spacing_pct", 1.5).with_param("grid_count", 6.0)),
        ("CS-meanrev-aggressive".into(), StrategyParams::new("meanrev").with_param("bb_period", 8.0).with_param("bb_std", 1.2)),
        ("CS-meanrev-conservative".into(), StrategyParams::new("meanrev").with_param("bb_period", 40.0).with_param("bb_std", 3.0)),
        ("CS-copy-micro".into(), StrategyParams::new("copy").with_param("min_trade_size", 1000.0).with_param("min_win_rate", 55.0)),
        ("CS-copy-elite".into(), StrategyParams::new("copy").with_param("min_trade_size", 50000.0).with_param("min_win_rate", 75.0)),
        ("CS-mm-aggressive".into(), StrategyParams::new("market_making").with_param("spread_bps", 8.0).with_param("max_inventory", 300.0)),
        ("CS-mm-conservative".into(), StrategyParams::new("market_making").with_param("spread_bps", 40.0).with_param("max_inventory", 2000.0)),
        ("CS-arb-tight".into(), StrategyParams::new("arb").with_param("min_edge", 0.2).with_param("min_volume", 50000.0)),
        ("CS-arb-micro".into(), StrategyParams::new("arb").with_param("min_edge", 1.0).with_param("min_volume", 5000.0)),
        ("CS-scalp-spot-1m".into(), StrategyParams::new("scalp").with_param("ema_fast", 3.0).with_param("ema_slow", 7.0).with_param("leverage", 1.0)),
        ("CS-scalp-spot-5m".into(), StrategyParams::new("scalp").with_param("ema_fast", 5.0).with_param("ema_slow", 12.0).with_param("leverage", 1.0)),
        ("CS-hybrid-dca-meanrev".into(), StrategyParams::new("dca").with_param("buy_interval", 6.0).with_param("rsi_oversold", 35.0).with_param("bb_period", 15.0)),
        ("CS-hybrid-momentum-grid".into(), StrategyParams::new("momentum").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("rsi_threshold", 50.0).with_param("spacing_pct", 0.8)),
        ("CS-experimental-1".into(), StrategyParams::new("momentum").with_param("ema_fast", 4.0).with_param("ema_slow", 9.0).with_param("rsi_threshold", 58.0)),
        ("CS-experimental-2".into(), StrategyParams::new("momentum").with_param("ema_fast", 11.0).with_param("ema_slow", 25.0).with_param("rsi_threshold", 48.0)),

        // ── Crypto Perps (40) ────────────────────────────────────────────────────
        // Scalp variants across timeframes (original 8)
        ("CP-scalp-1m-fast".into(), StrategyParams::new("scalp").with_param("ema_fast", 3.0).with_param("ema_slow", 8.0).with_param("leverage", 10.0)),
        ("CP-scalp-5m".into(), StrategyParams::new("scalp").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("leverage", 10.0)),
        ("CP-scalp-5m-slow".into(), StrategyParams::new("scalp").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("leverage", 10.0)),
        ("CP-scalp-15m".into(), StrategyParams::new("scalp").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("leverage", 10.0)),
        ("CP-scalp-15m-fast".into(), StrategyParams::new("scalp").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("leverage", 10.0)),
        ("CP-scalp-1h".into(), StrategyParams::new("scalp").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("leverage", 5.0)),
        ("CP-scalp-1h-aggressive".into(), StrategyParams::new("scalp").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("leverage", 15.0)),
        ("CP-scalp-4h".into(), StrategyParams::new("scalp").with_param("ema_fast", 12.0).with_param("ema_slow", 26.0).with_param("leverage", 3.0)),
        // Funding arb variants (original 3)
        ("CP-funding-arb".into(), StrategyParams::new("funding_arb").with_param("min_funding_rate", 0.0003)),
        ("CP-funding-arb-tight".into(), StrategyParams::new("funding_arb").with_param("min_funding_rate", 0.0005)),
        ("CP-funding-arb-wide".into(), StrategyParams::new("funding_arb").with_param("min_funding_rate", 0.0001)),
        // Grid perp variants (original 3)
        ("CP-grid-perp".into(), StrategyParams::new("grid").with_param("spacing_pct", 1.0).with_param("grid_count", 8.0)),
        ("CP-grid-perp-tight".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.5).with_param("grid_count", 12.0)),
        ("CP-grid-perp-wide".into(), StrategyParams::new("grid").with_param("spacing_pct", 2.0).with_param("grid_count", 5.0)),
        // Mean reversion perp (original 2)
        ("CP-meanrev-perp".into(), StrategyParams::new("meanrev").with_param("bb_period", 20.0).with_param("bb_std", 2.0)),
        ("CP-meanrev-perp-tight".into(), StrategyParams::new("meanrev").with_param("bb_period", 10.0).with_param("bb_std", 1.5)),
        // Momentum perp (original 2)
        ("CP-momentum-perp-fast".into(), StrategyParams::new("momentum").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("rsi_threshold", 50.0)),
        ("CP-momentum-perp-slow".into(), StrategyParams::new("momentum").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("rsi_threshold", 55.0)),
        // Copy trading (original 1)
        ("CP-copy-leaders".into(), StrategyParams::new("copy").with_param("min_trade_size", 10000.0).with_param("min_win_rate", 60.0)),
        // Market making (original 1)
        ("CP-mm-perp".into(), StrategyParams::new("market_making").with_param("spread_bps", 10.0).with_param("max_inventory", 200.0)),
        // ── Crypto Perps additional 20 ───────────────────────────────────────────
        ("CP-scalp-1m-slow".into(), StrategyParams::new("scalp").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("leverage", 10.0)),
        ("CP-scalp-3m".into(), StrategyParams::new("scalp").with_param("ema_fast", 4.0).with_param("ema_slow", 10.0).with_param("leverage", 10.0)),
        ("CP-scalp-5m-aggressive".into(), StrategyParams::new("scalp").with_param("ema_fast", 3.0).with_param("ema_slow", 8.0).with_param("leverage", 15.0)),
        ("CP-scalp-30m".into(), StrategyParams::new("scalp").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("leverage", 7.0)),
        ("CP-scalp-4h-conservative".into(), StrategyParams::new("scalp").with_param("ema_fast", 15.0).with_param("ema_slow", 30.0).with_param("leverage", 2.0)),
        ("CP-funding-arb-aggressive".into(), StrategyParams::new("funding_arb").with_param("min_funding_rate", 0.0001)),
        ("CP-grid-perp-micro".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.3).with_param("grid_count", 15.0)),
        ("CP-grid-perp-aggressive".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.8).with_param("grid_count", 10.0)),
        ("CP-meanrev-perp-aggressive".into(), StrategyParams::new("meanrev").with_param("bb_period", 8.0).with_param("bb_std", 1.2)),
        ("CP-meanrev-perp-wide".into(), StrategyParams::new("meanrev").with_param("bb_period", 30.0).with_param("bb_std", 2.5)),
        ("CP-momentum-perp-medium".into(), StrategyParams::new("momentum").with_param("ema_fast", 7.0).with_param("ema_slow", 15.0).with_param("rsi_threshold", 50.0)),
        ("CP-momentum-perp-aggressive".into(), StrategyParams::new("momentum").with_param("ema_fast", 3.0).with_param("ema_slow", 8.0).with_param("rsi_threshold", 45.0)),
        ("CP-copy-perp-leaders".into(), StrategyParams::new("copy").with_param("min_trade_size", 5000.0).with_param("min_win_rate", 55.0)),
        ("CP-copy-perp-whales".into(), StrategyParams::new("copy").with_param("min_trade_size", 20000.0).with_param("min_win_rate", 50.0)),
        ("CP-mm-perp-tight".into(), StrategyParams::new("market_making").with_param("spread_bps", 5.0).with_param("max_inventory", 100.0)),
        ("CP-mm-perp-wide".into(), StrategyParams::new("market_making").with_param("spread_bps", 20.0).with_param("max_inventory", 500.0)),
        ("CP-hybrid-scalp-funding".into(), StrategyParams::new("scalp").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("leverage", 10.0).with_param("min_funding_rate", 0.0002)),
        ("CP-hybrid-grid-momentum".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.5).with_param("grid_count", 8.0).with_param("ema_fast", 5.0)),
        ("CP-experimental-1".into(), StrategyParams::new("scalp").with_param("ema_fast", 6.0).with_param("ema_slow", 14.0).with_param("leverage", 8.0)),
        ("CP-experimental-2".into(), StrategyParams::new("scalp").with_param("ema_fast", 10.0).with_param("ema_slow", 22.0).with_param("leverage", 4.0)),

        // ═══ Meme Coins — Early Detection (15) ═══
        ("MC-ED-pump-instant".into(),   StrategyParams::new("mc_snipe").with_param("time_limit", 1.0).with_param("target_mult", 2.0).with_param("min_safety", 20.0).with_param("capital_usage_pct", 90.0).with_param("auto_position_count", 3.0)),
        ("MC-ED-pump-confirmed".into(), StrategyParams::new("mc_snipe").with_param("time_limit", 5.0).with_param("target_mult", 3.0).with_param("min_safety", 30.0).with_param("min_buyers", 5.0)),
        ("MC-ED-pool-fresh".into(),     StrategyParams::new("mc_snipe").with_param("max_age_mins", 5.0).with_param("target_mult", 2.0).with_param("min_safety", 25.0)),
        ("MC-ED-pool-volume".into(),    StrategyParams::new("mc_snipe").with_param("min_volume", 10000.0).with_param("target_mult", 2.0).with_param("min_safety", 30.0)),
        ("MC-ED-mcap-micro".into(),     StrategyParams::new("mc_snipe").with_param("max_mcap", 10000.0).with_param("target_mult", 5.0).with_param("min_safety", 20.0)),
        ("MC-ED-mcap-small".into(),     StrategyParams::new("mc_snipe").with_param("max_mcap", 50000.0).with_param("target_mult", 3.0).with_param("min_safety", 30.0)),
        ("MC-ED-liquidity-lock".into(), StrategyParams::new("mc_snipe").with_param("min_safety", 80.0).with_param("target_mult", 3.0)),
        ("MC-ED-dev-clean".into(),      StrategyParams::new("mc_snipe").with_param("max_dev_pct", 5.0).with_param("target_mult", 2.0).with_param("min_safety", 70.0)),
        ("MC-ED-social-mention".into(), StrategyParams::new("mc_snipe").with_param("target_mult", 2.0).with_param("min_safety", 40.0)),
        ("MC-ED-multi-buy".into(),      StrategyParams::new("mc_snipe").with_param("min_buyers", 10.0).with_param("target_mult", 2.0).with_param("min_safety", 30.0)),
        ("MC-ED-fast-flip".into(),      StrategyParams::new("mc_snipe").with_param("time_limit", 5.0).with_param("target_mult", 1.2).with_param("min_safety", 20.0)),
        ("MC-ED-slow-flip".into(),      StrategyParams::new("mc_snipe").with_param("time_limit", 30.0).with_param("target_mult", 3.0).with_param("min_safety", 40.0)),
        ("MC-ED-conservative".into(),   StrategyParams::new("mc_snipe").with_param("min_safety", 80.0).with_param("target_mult", 2.0)),
        ("MC-ED-aggressive".into(),     StrategyParams::new("mc_snipe").with_param("min_safety", 20.0).with_param("target_mult", 5.0)),
        ("MC-ED-explorer".into(),       StrategyParams::new("mc_snipe").with_param("min_safety", 30.0).with_param("target_mult", 2.5)),

        // ═══ Meme Coins — Trend Riding (15) ═══
        ("MC-TR-volume-surge".into(),     StrategyParams::new("mc_trend").with_param("volume_mult", 5.0).with_param("target_mult", 2.0).with_param("min_safety", 50.0)),
        ("MC-TR-volume-mega".into(),      StrategyParams::new("mc_trend").with_param("min_volume", 1000000.0).with_param("target_mult", 2.0).with_param("min_safety", 50.0)),
        ("MC-TR-price-breakout".into(),   StrategyParams::new("mc_trend").with_param("breakout_pct", 20.0).with_param("target_mult", 2.0).with_param("min_safety", 40.0)),
        ("MC-TR-momentum-fast".into(),    StrategyParams::new("mc_trend").with_param("ema_fast", 3.0).with_param("ema_slow", 8.0).with_param("min_safety", 40.0)),
        ("MC-TR-momentum-slow".into(),    StrategyParams::new("mc_trend").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("min_safety", 50.0)),
        ("MC-TR-rsi-bounce".into(),       StrategyParams::new("mc_trend").with_param("rsi_oversold", 30.0).with_param("rsi_entry", 40.0).with_param("min_safety", 50.0)),
        ("MC-TR-dip-buy".into(),          StrategyParams::new("mc_trend").with_param("dip_pct", 30.0).with_param("min_volume", 50000.0).with_param("min_safety", 50.0)),
        ("MC-TR-social-trending".into(),  StrategyParams::new("mc_trend").with_param("target_mult", 2.0).with_param("min_safety", 40.0)),
        ("MC-TR-dexscreener-hot".into(),  StrategyParams::new("mc_trend").with_param("target_mult", 2.0).with_param("min_safety", 40.0)),
        ("MC-TR-multi-timeframe".into(),  StrategyParams::new("mc_trend").with_param("target_mult", 3.0).with_param("min_safety", 60.0)),
        ("MC-TR-mean-revert".into(),      StrategyParams::new("mc_trend").with_param("rsi_oversold", 20.0).with_param("target_mult", 2.0).with_param("min_safety", 50.0)),
        ("MC-TR-grid-volatile".into(),    StrategyParams::new("mc_trend").with_param("spacing_pct", 5.0).with_param("grid_count", 5.0).with_param("min_safety", 50.0)),
        ("MC-TR-conservative".into(),     StrategyParams::new("mc_trend").with_param("min_mcap", 100000.0).with_param("min_safety", 70.0).with_param("target_mult", 2.0)),
        ("MC-TR-aggressive".into(),       StrategyParams::new("mc_trend").with_param("min_safety", 30.0).with_param("target_mult", 3.0)),
        ("MC-TR-explorer".into(),         StrategyParams::new("mc_trend").with_param("min_safety", 40.0).with_param("target_mult", 2.5)),

        // ═══ Meme Coins — Whale Copy (10) ═══
        ("MC-WC-top-pnl".into(),    StrategyParams::new("mc_whale").with_param("min_whale_wr", 50.0).with_param("top_n", 10.0).with_param("min_safety", 40.0)),
        ("MC-WC-high-wr".into(),    StrategyParams::new("mc_whale").with_param("min_whale_wr", 60.0).with_param("min_safety", 50.0)),
        ("MC-WC-early-buyer".into(), StrategyParams::new("mc_whale").with_param("max_entry_age", 5.0).with_param("min_safety", 30.0)),
        ("MC-WC-whale-large".into(), StrategyParams::new("mc_whale").with_param("min_portfolio", 50000.0).with_param("min_safety", 50.0)),
        ("MC-WC-whale-small".into(), StrategyParams::new("mc_whale").with_param("min_portfolio", 5000.0).with_param("max_portfolio", 20000.0).with_param("min_safety", 40.0)),
        ("MC-WC-consensus".into(),  StrategyParams::new("mc_whale").with_param("min_consensus", 3.0).with_param("min_safety", 50.0)),
        ("MC-WC-contrarian".into(), StrategyParams::new("mc_whale").with_param("min_safety", 30.0)),
        ("MC-WC-fast-follow".into(), StrategyParams::new("mc_whale").with_param("max_delay_secs", 30.0).with_param("min_safety", 30.0)),
        ("MC-WC-delayed".into(),    StrategyParams::new("mc_whale").with_param("min_delay_secs", 120.0).with_param("min_safety", 50.0)),
        ("MC-WC-explorer".into(),   StrategyParams::new("mc_whale").with_param("min_safety", 35.0)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_strategies_count() {
        assert_eq!(initial_strategies().len(), 160);
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
        // Execution params also included
        assert!(mutable.contains(&"auto_leverage".to_string()));
        assert!(mutable.contains(&"capital_usage_pct".to_string()));
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
        // Exactly one param changed (could be strategy or execution param)
        let strategy_params = ["ema_fast", "ema_slow", "rsi_threshold"];
        let changed_strategy = strategy_params.iter()
            .filter(|&&k| (new_p.get(k) - p.get(k)).abs() >= 0.001)
            .count();
        // Either exactly one strategy param changed, or the mutated param is an execution param
        let is_execution_param = ["auto_leverage", "auto_timeframe", "auto_position_count",
                                   "capital_usage_pct", "stop_loss_atr_mult", "take_profit_rr"]
            .contains(&result.param_name.as_str());
        assert!(changed_strategy == 1 || is_execution_param,
            "Expected exactly 1 strategy param changed OR an execution param mutated, got {} strategy changes (mutated: {})",
            changed_strategy, result.param_name);
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

    #[test]
    fn test_crossover_combines_params() {
        let a = StrategyParams::new("momentum").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0);
        let b = StrategyParams::new("meanrev").with_param("bb_period", 20.0).with_param("bb_std", 2.0);
        let child = crossover(&a, &b, 42);
        // Child should have params from both parents
        assert!(child.params.contains_key("ema_fast") || child.params.contains_key("bb_period"));
        assert_eq!(child.strategy_type, "hybrid");
    }

    #[test]
    fn test_crossover_shared_params_pick_one() {
        let a = StrategyParams::new("momentum").with_param("ema_fast", 5.0);
        let b = StrategyParams::new("momentum").with_param("ema_fast", 9.0);
        let child = crossover(&a, &b, 42);
        let val = child.get("ema_fast");
        assert!(val == 5.0 || val == 9.0); // Must be from one parent
    }

    #[test]
    fn test_random_explorer_within_bounds() {
        for seed in 0..50 {
            let p = random_explorer(seed);
            assert!(p.get("ema_fast") >= 2.0 && p.get("ema_fast") <= 15.0);
            assert!(p.get("auto_leverage") >= 1.0 && p.get("auto_leverage") <= 20.0);
            assert!(p.get("capital_usage_pct") >= 50.0 && p.get("capital_usage_pct") <= 95.0);
        }
    }

    #[test]
    fn test_random_explorer_deterministic() {
        let a = random_explorer(42);
        let b = random_explorer(42);
        assert_eq!(a.get("ema_fast"), b.get("ema_fast"));
    }

    #[test]
    fn test_execution_params_auto_populated() {
        let p = StrategyParams::new("scalp");
        assert!(p.get("auto_leverage") > 0.0);
        assert!(p.get("capital_usage_pct") > 0.0);
        assert!(p.get("auto_timeframe") > 0.0);
    }
}
