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

/// Create the default parameter sets for the initial 60 strategies (20 per market).
pub fn initial_strategies() -> Vec<(String, StrategyParams)> {
    vec![
        // Polymarket (20)
        // Momentum variants
        ("PM-momentum-fast".into(), StrategyParams::new("momentum").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("rsi_threshold", 50.0)),
        ("PM-momentum-slow".into(), StrategyParams::new("momentum").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("rsi_threshold", 50.0)),
        ("PM-momentum-ultra".into(), StrategyParams::new("momentum").with_param("ema_fast", 3.0).with_param("ema_slow", 8.0).with_param("rsi_threshold", 45.0)),
        ("PM-momentum-rsi-tight".into(), StrategyParams::new("momentum").with_param("ema_fast", 7.0).with_param("ema_slow", 15.0).with_param("rsi_threshold", 60.0)),
        // Value variants
        ("PM-value-aggressive".into(), StrategyParams::new("value").with_param("min_edge", 3.0)),
        ("PM-value-conservative".into(), StrategyParams::new("value").with_param("min_edge", 8.0)),
        ("PM-value-moderate".into(), StrategyParams::new("value").with_param("min_edge", 5.0)),
        ("PM-value-ultra".into(), StrategyParams::new("value").with_param("min_edge", 2.0)),
        // Copy trading variants
        ("PM-copy-top-pnl".into(), StrategyParams::new("copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 50.0)),
        ("PM-copy-high-wr".into(), StrategyParams::new("copy").with_param("min_trade_size", 50.0).with_param("min_win_rate", 70.0)),
        ("PM-copy-whales".into(), StrategyParams::new("copy").with_param("min_trade_size", 500.0).with_param("min_win_rate", 40.0)),
        ("PM-copy-consensus".into(), StrategyParams::new("copy").with_param("min_trade_size", 100.0).with_param("min_win_rate", 60.0)),
        // Arbitrage variants
        ("PM-arb-mispricing".into(), StrategyParams::new("mispricing").with_param("min_edge", 3.0).with_param("min_volume", 1000.0)),
        ("PM-arb-tight".into(), StrategyParams::new("mispricing").with_param("min_edge", 2.0).with_param("min_volume", 5000.0)),
        ("PM-arb-wide".into(), StrategyParams::new("mispricing").with_param("min_edge", 5.0).with_param("min_volume", 500.0)),
        ("PM-arb-volume".into(), StrategyParams::new("mispricing").with_param("min_edge", 3.0).with_param("min_volume", 10000.0)),
        // Mean reversion
        ("PM-meanrev-bb20".into(), StrategyParams::new("meanrev").with_param("bb_period", 20.0).with_param("bb_std", 2.0)),
        ("PM-meanrev-bb10".into(), StrategyParams::new("meanrev").with_param("bb_period", 10.0).with_param("bb_std", 1.5)),
        // Market making
        ("PM-mm-tight".into(), StrategyParams::new("market_making").with_param("spread_bps", 10.0).with_param("max_inventory", 500.0)),
        ("PM-mm-wide".into(), StrategyParams::new("market_making").with_param("spread_bps", 30.0).with_param("max_inventory", 1000.0)),

        // Crypto Spot (20)
        // Momentum variants
        ("CS-momentum-fast".into(), StrategyParams::new("momentum").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("rsi_threshold", 50.0)),
        ("CS-momentum-slow".into(), StrategyParams::new("momentum").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("rsi_threshold", 50.0)),
        ("CS-momentum-ultra".into(), StrategyParams::new("momentum").with_param("ema_fast", 3.0).with_param("ema_slow", 8.0).with_param("rsi_threshold", 45.0)),
        ("CS-momentum-conservative".into(), StrategyParams::new("momentum").with_param("ema_fast", 12.0).with_param("ema_slow", 26.0).with_param("rsi_threshold", 55.0)),
        // DCA variants
        ("CS-dca-aggressive".into(), StrategyParams::new("dca").with_param("buy_interval", 6.0).with_param("rsi_oversold", 40.0)),
        ("CS-dca-conservative".into(), StrategyParams::new("dca").with_param("buy_interval", 24.0).with_param("rsi_oversold", 30.0)),
        ("CS-dca-moderate".into(), StrategyParams::new("dca").with_param("buy_interval", 12.0).with_param("rsi_oversold", 35.0)),
        ("CS-dca-ultra".into(), StrategyParams::new("dca").with_param("buy_interval", 3.0).with_param("rsi_oversold", 45.0)),
        // Grid variants
        ("CS-grid-tight".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.5).with_param("grid_count", 10.0)),
        ("CS-grid-wide".into(), StrategyParams::new("grid").with_param("spacing_pct", 2.0).with_param("grid_count", 5.0)),
        ("CS-grid-medium".into(), StrategyParams::new("grid").with_param("spacing_pct", 1.0).with_param("grid_count", 8.0)),
        ("CS-grid-dense".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.3).with_param("grid_count", 15.0)),
        // Mean reversion
        ("CS-meanrev-bollinger".into(), StrategyParams::new("meanrev").with_param("bb_period", 20.0).with_param("bb_std", 2.0)),
        ("CS-meanrev-tight".into(), StrategyParams::new("meanrev").with_param("bb_period", 10.0).with_param("bb_std", 1.5)),
        ("CS-meanrev-wide".into(), StrategyParams::new("meanrev").with_param("bb_period", 30.0).with_param("bb_std", 2.5)),
        // Copy trading
        ("CS-copy-whales".into(), StrategyParams::new("copy").with_param("min_trade_size", 10000.0).with_param("min_win_rate", 50.0)),
        ("CS-copy-smart".into(), StrategyParams::new("copy").with_param("min_trade_size", 5000.0).with_param("min_win_rate", 65.0)),
        // Market making
        ("CS-mm-tight".into(), StrategyParams::new("market_making").with_param("spread_bps", 15.0).with_param("max_inventory", 500.0)),
        ("CS-mm-wide".into(), StrategyParams::new("market_making").with_param("spread_bps", 30.0).with_param("max_inventory", 1000.0)),
        // Arbitrage
        ("CS-arb-cross".into(), StrategyParams::new("arb").with_param("min_edge", 0.5).with_param("min_volume", 10000.0)),

        // Crypto Perps (20)
        // Scalp variants across timeframes
        ("CP-scalp-1m-fast".into(), StrategyParams::new("scalp").with_param("ema_fast", 3.0).with_param("ema_slow", 8.0).with_param("leverage", 10.0)),
        ("CP-scalp-5m".into(), StrategyParams::new("scalp").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("leverage", 10.0)),
        ("CP-scalp-5m-slow".into(), StrategyParams::new("scalp").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("leverage", 10.0)),
        ("CP-scalp-15m".into(), StrategyParams::new("scalp").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("leverage", 10.0)),
        ("CP-scalp-15m-fast".into(), StrategyParams::new("scalp").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("leverage", 10.0)),
        ("CP-scalp-1h".into(), StrategyParams::new("scalp").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("leverage", 5.0)),
        ("CP-scalp-1h-aggressive".into(), StrategyParams::new("scalp").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("leverage", 15.0)),
        ("CP-scalp-4h".into(), StrategyParams::new("scalp").with_param("ema_fast", 12.0).with_param("ema_slow", 26.0).with_param("leverage", 3.0)),
        // Funding arb variants
        ("CP-funding-arb".into(), StrategyParams::new("funding_arb").with_param("min_funding_rate", 0.0003)),
        ("CP-funding-arb-tight".into(), StrategyParams::new("funding_arb").with_param("min_funding_rate", 0.0005)),
        ("CP-funding-arb-wide".into(), StrategyParams::new("funding_arb").with_param("min_funding_rate", 0.0001)),
        // Grid perp variants
        ("CP-grid-perp".into(), StrategyParams::new("grid").with_param("spacing_pct", 1.0).with_param("grid_count", 8.0)),
        ("CP-grid-perp-tight".into(), StrategyParams::new("grid").with_param("spacing_pct", 0.5).with_param("grid_count", 12.0)),
        ("CP-grid-perp-wide".into(), StrategyParams::new("grid").with_param("spacing_pct", 2.0).with_param("grid_count", 5.0)),
        // Mean reversion perp
        ("CP-meanrev-perp".into(), StrategyParams::new("meanrev").with_param("bb_period", 20.0).with_param("bb_std", 2.0)),
        ("CP-meanrev-perp-tight".into(), StrategyParams::new("meanrev").with_param("bb_period", 10.0).with_param("bb_std", 1.5)),
        // Momentum perp
        ("CP-momentum-perp-fast".into(), StrategyParams::new("momentum").with_param("ema_fast", 5.0).with_param("ema_slow", 13.0).with_param("rsi_threshold", 50.0)),
        ("CP-momentum-perp-slow".into(), StrategyParams::new("momentum").with_param("ema_fast", 9.0).with_param("ema_slow", 21.0).with_param("rsi_threshold", 55.0)),
        // Copy trading
        ("CP-copy-leaders".into(), StrategyParams::new("copy").with_param("min_trade_size", 10000.0).with_param("min_win_rate", 60.0)),
        // Market making
        ("CP-mm-perp".into(), StrategyParams::new("market_making").with_param("spread_bps", 10.0).with_param("max_inventory", 200.0)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_strategies_count() {
        assert_eq!(initial_strategies().len(), 60);
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
