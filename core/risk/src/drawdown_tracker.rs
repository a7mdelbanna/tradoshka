use chrono::{DateTime, Utc, Datelike};
use rust_decimal::Decimal;

pub struct DrawdownTracker {
    peak_equity: Decimal,
    daily_start_equity: Decimal,
    current_day: u32,
}

impl DrawdownTracker {
    pub fn new(initial_equity: Decimal) -> Self {
        Self {
            peak_equity: initial_equity,
            daily_start_equity: initial_equity,
            current_day: 0,
        }
    }

    /// Update the tracker with new equity and timestamp.
    /// Returns (portfolio_drawdown_pct, daily_drawdown_pct).
    pub fn update(&mut self, equity: Decimal, now: DateTime<Utc>) -> (Decimal, Decimal) {
        let day = now.day();

        // Detect new day
        if day != self.current_day {
            self.daily_start_equity = equity;
            self.current_day = day;
        }

        // Update peak
        if equity > self.peak_equity {
            self.peak_equity = equity;
        }

        // Portfolio drawdown from peak
        let portfolio_drawdown = if self.peak_equity > Decimal::ZERO {
            (self.peak_equity - equity) / self.peak_equity * Decimal::ONE_HUNDRED
        } else {
            Decimal::ZERO
        };

        // Daily drawdown from daily start
        let daily_drawdown = if self.daily_start_equity > Decimal::ZERO {
            (self.daily_start_equity - equity) / self.daily_start_equity * Decimal::ONE_HUNDRED
        } else {
            Decimal::ZERO
        };

        (portfolio_drawdown, daily_drawdown)
    }

    pub fn peak_equity(&self) -> Decimal {
        self.peak_equity
    }

    pub fn reset_daily(&mut self, equity: Decimal) {
        self.daily_start_equity = equity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use chrono::TimeZone;

    fn ts(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, day, 12, 0, 0).unwrap()
    }

    #[test]
    fn test_no_drawdown_at_peak() {
        let mut tracker = DrawdownTracker::new(dec!(10000));
        let (port_dd, daily_dd) = tracker.update(dec!(10000), ts(1));
        assert_eq!(port_dd, Decimal::ZERO);
        assert_eq!(daily_dd, Decimal::ZERO);
    }

    #[test]
    fn test_portfolio_drawdown() {
        let mut tracker = DrawdownTracker::new(dec!(10000));
        // Rise to peak 12000
        tracker.update(dec!(12000), ts(1));
        assert_eq!(tracker.peak_equity(), dec!(12000));

        // Drop to 10800 → portfolio drawdown = (12000 - 10800) / 12000 * 100 = 10%
        let (port_dd, _daily_dd) = tracker.update(dec!(10800), ts(1));
        assert_eq!(port_dd, dec!(10));
    }

    #[test]
    fn test_peak_updates_on_new_high() {
        let mut tracker = DrawdownTracker::new(dec!(10000));
        tracker.update(dec!(10000), ts(1));
        assert_eq!(tracker.peak_equity(), dec!(10000));

        tracker.update(dec!(11000), ts(1));
        assert_eq!(tracker.peak_equity(), dec!(11000));

        tracker.update(dec!(12500), ts(2));
        assert_eq!(tracker.peak_equity(), dec!(12500));

        // Drawdown should be zero at the new peak
        let (port_dd, _) = tracker.update(dec!(12500), ts(2));
        assert_eq!(port_dd, Decimal::ZERO);
    }
}
