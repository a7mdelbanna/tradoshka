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
        // ── Polymarket (40) ──────────────────────────────────────────────────────
        // Momentum variants (original 4)
        ("PM-momentum-fast".into(), StrategyParams::new("momentum").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("rsi_threshold", 50.0)),
        ("PM-momentum-slow".into(), StrategyParams::new("momentum").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("rsi_threshold", 50.0)),
        ("PM-momentum-ultra".into(), StrategyParams::new("momentum").with_param("ema_fast", 3.0).with_param("ema_slow", 8.0).with_param("rsi_threshold", 45.0)),
        ("PM-momentum-rsi-tight".into(), StrategyParams::new("momentum").with_param("ema_fast", 7.0).with_param("ema_slow", 15.0).with_param("rsi_threshold", 60.0)),
        // Value variants (original 4)
        ("PM-value-aggressive".into(), StrategyParams::new("value").with_param("min_edge", 3.0)),
        ("PM-value-conservative".into(), StrategyParams::new("value").with_param("min_edge", 8.0)),
        ("PM-value-moderate".into(), StrategyParams::new("value").with_param("min_edge", 5.0)),
        ("PM-value-ultra".into(), StrategyParams::new("value").with_param("min_edge", 2.0)),
        // Copy trading variants (original 4)
        ("PM-copy-top-pnl".into(), StrategyParams::new("copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 50.0)),
        ("PM-copy-high-wr".into(), StrategyParams::new("copy").with_param("min_trade_size", 50.0).with_param("min_win_rate", 70.0)),
        ("PM-copy-whales".into(), StrategyParams::new("copy").with_param("min_trade_size", 500.0).with_param("min_win_rate", 40.0)),
        ("PM-copy-consensus".into(), StrategyParams::new("copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 60.0)),
        // Arbitrage variants (original 4)
        ("PM-arb-mispricing".into(), StrategyParams::new("mispricing").with_param("min_edge", 3.0).with_param("min_volume", 1000.0)),
        ("PM-arb-tight".into(), StrategyParams::new("mispricing").with_param("min_edge", 2.0).with_param("min_volume", 5000.0)),
        ("PM-arb-wide".into(), StrategyParams::new("mispricing").with_param("min_edge", 5.0).with_param("min_volume", 500.0)),
        ("PM-arb-volume".into(), StrategyParams::new("mispricing").with_param("min_edge", 3.0).with_param("min_volume", 10000.0)),
        // Mean reversion (original 2)
        ("PM-meanrev-bb20".into(), StrategyParams::new("meanrev").with_param("bb_period", 20.0).with_param("bb_std", 2.0)),
        ("PM-meanrev-bb10".into(), StrategyParams::new("meanrev").with_param("bb_period", 10.0).with_param("bb_std", 1.5)),
        // Market making (original 2)
        ("PM-mm-tight".into(), StrategyParams::new("market_making").with_param("spread_bps", 10.0).with_param("max_inventory", 500.0)),
        ("PM-mm-wide".into(), StrategyParams::new("market_making").with_param("spread_bps", 30.0).with_param("max_inventory", 1000.0)),
        // ── Polymarket additional 20 ──────────────────────────────────────────────
        ("PM-momentum-medium".into(), StrategyParams::new("momentum").with_param("ema_fast", 7.0).with_param("ema_slow", 17.0).with_param("rsi_threshold", 50.0)),
        ("PM-value-micro".into(), StrategyParams::new("value").with_param("min_edge", 1.0)),
        ("PM-value-dynamic".into(), StrategyParams::new("value").with_param("min_edge", 4.0)),
        ("PM-copy-contrarian".into(), StrategyParams::new("copy").with_param("min_trade_size", 200.0).with_param("min_win_rate", 30.0)),
        ("PM-copy-frequent".into(), StrategyParams::new("copy").with_param("min_trade_size", 20.0).with_param("min_win_rate", 55.0)),
        ("PM-arb-micro".into(), StrategyParams::new("mispricing").with_param("min_edge", 1.5).with_param("min_volume", 2000.0)),
        ("PM-arb-aggressive".into(), StrategyParams::new("mispricing").with_param("min_edge", 1.0).with_param("min_volume", 500.0)),
        ("PM-meanrev-bb15".into(), StrategyParams::new("meanrev").with_param("bb_period", 15.0).with_param("bb_std", 1.8)),
        ("PM-meanrev-bb30".into(), StrategyParams::new("meanrev").with_param("bb_period", 30.0).with_param("bb_std", 2.5)),
        ("PM-mm-micro".into(), StrategyParams::new("market_making").with_param("spread_bps", 5.0).with_param("max_inventory", 200.0)),
        ("PM-mm-balanced".into(), StrategyParams::new("market_making").with_param("spread_bps", 20.0).with_param("max_inventory", 750.0)),
        ("PM-grid-pm-tight".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.5).with_param("grid_count", 10.0)),
        ("PM-grid-pm-wide".into(), StrategyParams::new("grid").with_param("spacing_pct", 2.0).with_param("grid_count", 5.0)),
        ("PM-dca-pm".into(), StrategyParams::new("dca").with_param("buy_interval", 10.0).with_param("rsi_oversold", 35.0)),
        ("PM-dca-pm-fast".into(), StrategyParams::new("dca").with_param("buy_interval", 4.0).with_param("rsi_oversold", 42.0)),
        ("PM-scalp-pm-fast".into(), StrategyParams::new("scalp").with_param("ema_fast", 3.0).with_param("ema_slow", 7.0).with_param("leverage", 1.0).with_param("rsi_threshold", 48.0)),
        ("PM-scalp-pm-slow".into(), StrategyParams::new("scalp").with_param("ema_fast", 8.0).with_param("ema_slow", 18.0).with_param("leverage", 1.0).with_param("rsi_threshold", 52.0)),
        ("PM-hybrid-momentum-value".into(), StrategyParams::new("momentum").with_param("ema_fast", 6.0).with_param("ema_slow", 14.0).with_param("rsi_threshold", 50.0).with_param("min_edge", 4.0)),
        ("PM-hybrid-arb-mm".into(), StrategyParams::new("mispricing").with_param("min_edge", 2.0).with_param("min_volume", 1000.0).with_param("spread_bps", 15.0)),
        ("PM-experimental-1".into(), StrategyParams::new("momentum").with_param("ema_fast", 4.0).with_param("ema_slow", 10.0).with_param("rsi_threshold", 55.0)),

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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_strategies_count() {
        assert_eq!(initial_strategies().len(), 120);
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
