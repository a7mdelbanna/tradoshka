use std::collections::HashMap;

/// Per-asset indicator state for one strategy.
#[derive(Debug, Clone, Default)]
pub struct AssetIndicators {
    pub prices: Vec<f64>,          // Price history (most recent last)
    pub ema_fast: Option<f64>,     // Current fast EMA value
    pub ema_slow: Option<f64>,     // Current slow EMA value
    pub rsi: Option<f64>,          // Current RSI value
    pub atr: Option<f64>,          // Current ATR value
    pub bb_upper: Option<f64>,     // Bollinger upper
    pub bb_middle: Option<f64>,    // Bollinger middle
    pub bb_lower: Option<f64>,     // Bollinger lower
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
}

impl AssetIndicators {
    pub fn update(&mut self, price: f64, high: f64, low: f64, close: f64,
                   ema_fast_period: usize, ema_slow_period: usize,
                   rsi_period: usize, bb_period: usize, bb_std: f64) {
        self.prices.push(price);
        self.highs.push(high);
        self.lows.push(low);
        self.closes.push(close);

        // Keep max 200 data points
        if self.prices.len() > 200 {
            self.prices.remove(0);
            self.highs.remove(0);
            self.lows.remove(0);
            self.closes.remove(0);
        }

        // Compute EMA fast
        self.ema_fast = Self::compute_ema(&self.closes, ema_fast_period);

        // Compute EMA slow
        self.ema_slow = Self::compute_ema(&self.closes, ema_slow_period);

        // Compute RSI
        self.rsi = Self::compute_rsi(&self.closes, rsi_period);

        // Compute ATR
        self.atr = Self::compute_atr(&self.highs, &self.lows, &self.closes, 14);

        // Compute Bollinger Bands
        if let Some((upper, middle, lower)) = Self::compute_bollinger(&self.closes, bb_period, bb_std) {
            self.bb_upper = Some(upper);
            self.bb_middle = Some(middle);
            self.bb_lower = Some(lower);
        }
    }

    fn compute_ema(data: &[f64], period: usize) -> Option<f64> {
        if data.len() < period || period == 0 { return None; }
        let multiplier = 2.0 / (period as f64 + 1.0);
        // Start with SMA of first `period` values
        let sma: f64 = data[..period].iter().sum::<f64>() / period as f64;
        let mut ema = sma;
        for &val in &data[period..] {
            ema = (val - ema) * multiplier + ema;
        }
        Some(ema)
    }

    fn compute_rsi(data: &[f64], period: usize) -> Option<f64> {
        if data.len() < period + 1 || period == 0 { return None; }
        let mut gains = 0.0;
        let mut losses = 0.0;
        let start = data.len() - period - 1;
        for i in (start + 1)..data.len() {
            let change = data[i] - data[i - 1];
            if change > 0.0 { gains += change; }
            else { losses += change.abs(); }
        }
        let avg_gain = gains / period as f64;
        let avg_loss = losses / period as f64;
        if avg_loss == 0.0 { return Some(100.0); }
        let rs = avg_gain / avg_loss;
        Some(100.0 - (100.0 / (1.0 + rs)))
    }

    fn compute_atr(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Option<f64> {
        if highs.len() < period + 1 { return None; }
        let start = highs.len() - period;
        let mut tr_sum = 0.0;
        for i in start..highs.len() {
            let tr = (highs[i] - lows[i])
                .max((highs[i] - closes[i.saturating_sub(1)]).abs())
                .max((lows[i] - closes[i.saturating_sub(1)]).abs());
            tr_sum += tr;
        }
        Some(tr_sum / period as f64)
    }

    fn compute_bollinger(data: &[f64], period: usize, std_mult: f64) -> Option<(f64, f64, f64)> {
        if data.len() < period || period == 0 { return None; }
        let slice = &data[data.len() - period..];
        let mean = slice.iter().sum::<f64>() / period as f64;
        let variance = slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / period as f64;
        let std_dev = variance.sqrt();
        Some((mean + std_mult * std_dev, mean, mean - std_mult * std_dev))
    }

    pub fn has_enough_data(&self, min_period: usize) -> bool {
        self.prices.len() >= min_period
    }

    /// Alias for has_enough_data -- checks if we have at least min price samples.
    pub fn has_data(&self, min: usize) -> bool {
        self.prices.len() >= min
    }
}

/// Indicator engine for one strategy -- tracks all assets it watches.
#[derive(Debug, Clone, Default)]
pub struct StrategyIndicatorEngine {
    pub assets: HashMap<String, AssetIndicators>,
    pub ema_fast_period: usize,
    pub ema_slow_period: usize,
    pub rsi_period: usize,
    pub bb_period: usize,
    pub bb_std: f64,
}

impl StrategyIndicatorEngine {
    pub fn new(ema_fast: usize, ema_slow: usize, rsi_period: usize, bb_period: usize, bb_std: f64) -> Self {
        Self {
            assets: HashMap::new(),
            ema_fast_period: ema_fast,
            ema_slow_period: ema_slow,
            rsi_period,
            bb_period,
            bb_std,
        }
    }

    pub fn from_params(params: &crate::mutation::StrategyParams) -> Self {
        Self::new(
            params.get("ema_fast").max(2.0) as usize,
            params.get("ema_slow").max(3.0) as usize,
            14, // RSI always 14
            params.get("bb_period").max(5.0) as usize,
            params.get("bb_std").max(0.5),
        )
    }

    /// Feed a new price tick for an asset. Returns updated indicators.
    pub fn update(&mut self, symbol: &str, price: f64) -> &AssetIndicators {
        let ind = self.assets.entry(symbol.into()).or_default();
        // Use price as OHLC proxy (since we only get spot ticks, not candles)
        let noise = (symbol.len() as f64 * 0.0001).sin() * price * 0.001;
        let high = price + noise.abs();
        let low = price - noise.abs();
        ind.update(price, high, low, price,
            self.ema_fast_period, self.ema_slow_period,
            self.rsi_period, self.bb_period, self.bb_std);
        ind
    }

    pub fn get(&self, symbol: &str) -> Option<&AssetIndicators> {
        self.assets.get(symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_computation() {
        let data = vec![10.0, 11.0, 12.0, 11.5, 12.5, 13.0, 12.0, 13.5, 14.0, 13.0];
        let ema3 = AssetIndicators::compute_ema(&data, 3);
        let ema7 = AssetIndicators::compute_ema(&data, 7);
        assert!(ema3.is_some());
        assert!(ema7.is_some());
        // Faster EMA should be more responsive to recent prices
        assert_ne!(ema3.unwrap(), ema7.unwrap());
    }

    #[test]
    fn test_rsi_overbought() {
        let data: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect(); // Continuously rising
        let rsi = AssetIndicators::compute_rsi(&data, 14);
        assert!(rsi.is_some());
        assert!(rsi.unwrap() > 80.0); // Should be overbought
    }

    #[test]
    fn test_rsi_oversold() {
        let data: Vec<f64> = (0..20).map(|i| 100.0 - i as f64).collect(); // Continuously falling
        let rsi = AssetIndicators::compute_rsi(&data, 14);
        assert!(rsi.is_some());
        assert!(rsi.unwrap() < 20.0); // Should be oversold
    }

    #[test]
    fn test_different_ema_periods_give_different_values() {
        let data: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0).collect();
        let ema5 = AssetIndicators::compute_ema(&data, 5).unwrap();
        let ema21 = AssetIndicators::compute_ema(&data, 21).unwrap();
        assert!((ema5 - ema21).abs() > 0.01, "EMA5={} EMA21={} should differ", ema5, ema21);
    }

    #[test]
    fn test_strategy_engine_builds_from_params() {
        let params = crate::mutation::StrategyParams::new("momentum")
            .with_param("ema_fast", 5.0)
            .with_param("ema_slow", 13.0)
            .with_param("bb_period", 20.0)
            .with_param("bb_std", 2.0);
        let engine = StrategyIndicatorEngine::from_params(&params);
        assert_eq!(engine.ema_fast_period, 5);
        assert_eq!(engine.ema_slow_period, 13);
    }

    #[test]
    fn test_strategy_engine_update() {
        let mut engine = StrategyIndicatorEngine::new(5, 13, 14, 20, 2.0);
        for i in 0..30 {
            engine.update("BTCUSDT", 69000.0 + (i as f64) * 50.0);
        }
        let ind = engine.get("BTCUSDT").unwrap();
        assert!(ind.ema_fast.is_some());
        assert!(ind.ema_slow.is_some());
        assert!(ind.rsi.is_some());
    }

    #[test]
    fn test_two_engines_different_signals() {
        let mut engine_fast = StrategyIndicatorEngine::new(3, 8, 14, 20, 2.0);
        let mut engine_slow = StrategyIndicatorEngine::new(9, 21, 14, 20, 2.0);

        // Feed same prices to both
        let prices: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64 * 0.5).sin() * 10.0).collect();
        for &p in &prices {
            engine_fast.update("TEST", p);
            engine_slow.update("TEST", p);
        }

        let fast = engine_fast.get("TEST").unwrap();
        let slow = engine_slow.get("TEST").unwrap();

        // Different EMA periods MUST produce different values
        assert_ne!(fast.ema_fast.unwrap(), slow.ema_fast.unwrap());
        assert_ne!(fast.ema_slow.unwrap(), slow.ema_slow.unwrap());
    }
}
