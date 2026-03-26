use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use tradoshka_common::types::{Candle, Symbol};

// ---------------------------------------------------------------------------
// align_to_interval
// ---------------------------------------------------------------------------

/// Aligns a timestamp to the nearest past interval boundary.
///
/// Boundary = `ts - (ts_secs % interval_secs)`.
pub fn align_to_interval(timestamp: DateTime<Utc>, interval_secs: i64) -> DateTime<Utc> {
    let ts_secs = timestamp.timestamp();
    let aligned_secs = ts_secs - (ts_secs % interval_secs);
    DateTime::from_timestamp(aligned_secs, 0).expect("aligned timestamp is always valid")
}

// ---------------------------------------------------------------------------
// CandleBuilder  (private helper)
// ---------------------------------------------------------------------------

struct CandleBuilder {
    symbol: Symbol,
    /// The aligned timestamp for the candle's interval bucket.
    timestamp: DateTime<Utc>,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: Decimal,
}

impl CandleBuilder {
    fn new(symbol: Symbol, timestamp: DateTime<Utc>, price: Decimal, quantity: Decimal) -> Self {
        Self {
            symbol,
            timestamp,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: quantity,
        }
    }

    fn update(&mut self, price: Decimal, quantity: Decimal) {
        if price > self.high {
            self.high = price;
        }
        if price < self.low {
            self.low = price;
        }
        self.close = price;
        self.volume += quantity;
    }

    fn build(self) -> Candle {
        Candle {
            symbol: self.symbol,
            timestamp: self.timestamp,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
        }
    }
}

// ---------------------------------------------------------------------------
// CandleAggregator
// ---------------------------------------------------------------------------

/// Converts individual trades into OHLCV candles at a fixed time interval.
///
/// # Usage
/// ```ignore
/// let mut agg = CandleAggregator::new(Duration::minutes(1));
/// if let Some(completed) = agg.process_trade("BTC-USD", price, qty, ts) {
///     // previous candle closed
/// }
/// if let Some(in_progress) = agg.current_candle() {
///     // snapshot of the open candle
/// }
/// ```
pub struct CandleAggregator {
    interval_secs: i64,
    current: Option<CandleBuilder>,
}

impl CandleAggregator {
    /// Create a new aggregator with the given candle interval.
    ///
    /// # Panics
    /// Panics if `interval` is zero or negative.
    pub fn new(interval: Duration) -> Self {
        let interval_secs = interval.num_seconds();
        assert!(interval_secs > 0, "CandleAggregator interval must be positive");
        Self {
            interval_secs,
            current: None,
        }
    }

    /// Process a single trade tick.
    ///
    /// Returns `Some(candle)` when a candle for the *previous* interval is
    /// completed (i.e. the trade belongs to a newer interval bucket).
    /// Returns `None` when the trade was absorbed into the current candle.
    pub fn process_trade(
        &mut self,
        symbol: Symbol,
        price: Decimal,
        quantity: Decimal,
        timestamp: DateTime<Utc>,
    ) -> Option<Candle> {
        let bucket = align_to_interval(timestamp, self.interval_secs);

        match &mut self.current {
            None => {
                // First trade ever — start a new candle, nothing completed yet.
                self.current = Some(CandleBuilder::new(symbol, bucket, price, quantity));
                None
            }
            Some(builder) if builder.timestamp == bucket => {
                // Same interval — update the in-progress candle.
                builder.update(price, quantity);
                None
            }
            Some(_) => {
                // New interval — complete the previous candle and start fresh.
                let completed = self
                    .current
                    .take()
                    .expect("checked above that current is Some")
                    .build();
                self.current = Some(CandleBuilder::new(symbol, bucket, price, quantity));
                Some(completed)
            }
        }
    }

    /// Returns a snapshot of the currently open (incomplete) candle, or
    /// `None` if no trades have been processed yet.
    pub fn current_candle(&self) -> Option<Candle> {
        self.current.as_ref().map(|b| Candle {
            symbol: b.symbol.clone(),
            timestamp: b.timestamp,
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Convenience: build a UTC timestamp from a Unix second offset.
    fn ts(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    #[test]
    fn test_first_trade_starts_candle() {
        let mut agg = CandleAggregator::new(Duration::seconds(60));

        // First trade must return None (no previous candle to complete).
        let result = agg.process_trade("BTC-USD".to_string(), dec!(100), dec!(1), ts(0));
        assert!(result.is_none());

        // The in-progress candle should have the correct open price.
        let candle = agg.current_candle().expect("candle should exist after first trade");
        assert_eq!(candle.open, dec!(100));
        assert_eq!(candle.symbol, "BTC-USD");
    }

    #[test]
    fn test_trades_within_interval_update_candle() {
        let mut agg = CandleAggregator::new(Duration::seconds(60));

        // Three trades within the same 60-second bucket (ts 0..59).
        agg.process_trade("ETH-USD".to_string(), dec!(200), dec!(1), ts(0));
        agg.process_trade("ETH-USD".to_string(), dec!(250), dec!(2), ts(30));
        agg.process_trade("ETH-USD".to_string(), dec!(180), dec!(3), ts(59));

        let candle = agg.current_candle().unwrap();
        assert_eq!(candle.open, dec!(200));
        assert_eq!(candle.high, dec!(250));
        assert_eq!(candle.low, dec!(180));
        assert_eq!(candle.close, dec!(180));
        assert_eq!(candle.volume, dec!(6)); // 1 + 2 + 3
    }

    #[test]
    fn test_new_interval_completes_candle() {
        let mut agg = CandleAggregator::new(Duration::seconds(60));

        // Build up a candle in the first interval.
        agg.process_trade("SOL-USD".to_string(), dec!(50), dec!(10), ts(0));
        agg.process_trade("SOL-USD".to_string(), dec!(55), dec!(5), ts(30));

        // Trade in the next interval must return the completed previous candle.
        let completed = agg.process_trade("SOL-USD".to_string(), dec!(60), dec!(2), ts(60));
        let completed = completed.expect("should have completed the previous candle");

        assert_eq!(completed.open, dec!(50));
        assert_eq!(completed.high, dec!(55));
        assert_eq!(completed.low, dec!(50));
        assert_eq!(completed.close, dec!(55));
        assert_eq!(completed.volume, dec!(15)); // 10 + 5

        // A new in-progress candle should have started for the new interval.
        let current = agg.current_candle().unwrap();
        assert_eq!(current.open, dec!(60));
        assert_eq!(current.volume, dec!(2));
    }

    #[test]
    fn test_interval_alignment() {
        // 60-second intervals
        assert_eq!(align_to_interval(ts(0), 60), ts(0));
        assert_eq!(align_to_interval(ts(59), 60), ts(0));
        assert_eq!(align_to_interval(ts(60), 60), ts(60));
        assert_eq!(align_to_interval(ts(119), 60), ts(60));
        assert_eq!(align_to_interval(ts(120), 60), ts(120));

        // 300-second (5-minute) intervals
        assert_eq!(align_to_interval(ts(0), 300), ts(0));
        assert_eq!(align_to_interval(ts(299), 300), ts(0));
        assert_eq!(align_to_interval(ts(300), 300), ts(300));
        assert_eq!(align_to_interval(ts(599), 300), ts(300));
    }
}
