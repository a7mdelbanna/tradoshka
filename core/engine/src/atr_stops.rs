use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use tradoshka_common::types::{Market, OrderSide};

/// Calculate all stop/exit levels based on ATR and risk parameters.
pub struct StopCalculator;

/// All calculated exit levels for a position.
#[derive(Debug, Clone)]
pub struct ExitLevels {
    pub hard_stop_loss: Decimal,
    pub trailing_stop: Decimal,
    pub take_profit: Decimal,
    pub time_stop_hours: u32,
    pub stop_distance: Decimal,
    pub reward_risk_ratio: f64,
}

impl StopCalculator {
    /// Calculate exit levels for a trade.
    ///
    /// - `entry_price`: actual market entry price
    /// - `atr`: Average True Range (14-period)
    /// - `side`: Buy (long) or Sell (short)
    /// - `market`: which market (affects time stop)
    /// - `rr_target`: desired reward:risk ratio (minimum 2.0)
    pub fn calculate(
        entry_price: Decimal,
        atr: Decimal,
        side: OrderSide,
        market: Market,
        rr_target: f64,
    ) -> ExitLevels {
        let rr = if rr_target < 2.0 { 2.0 } else { rr_target };

        let (hard_sl, trailing, tp, stop_dist) = match side {
            OrderSide::Buy => {
                let hard_sl = entry_price - (dec!(2) * atr);
                let trailing = entry_price - (Decimal::from_f64(1.5).unwrap_or(dec!(1.5)) * atr);
                let stop_dist = entry_price - hard_sl;
                let tp = entry_price + (stop_dist * Decimal::from_f64(rr).unwrap_or(dec!(2)));
                (hard_sl.max(Decimal::ZERO), trailing.max(Decimal::ZERO), tp, stop_dist)
            }
            OrderSide::Sell => {
                let hard_sl = entry_price + (dec!(2) * atr);
                let trailing = entry_price + (Decimal::from_f64(1.5).unwrap_or(dec!(1.5)) * atr);
                let stop_dist = hard_sl - entry_price;
                let tp = entry_price - (stop_dist * Decimal::from_f64(rr).unwrap_or(dec!(2)));
                (hard_sl, trailing, tp.max(Decimal::ZERO), stop_dist)
            }
        };

        let time_stop = match market {
            Market::Polymarket => 72,
            Market::Crypto => 24,
            _ => 48,
        };

        ExitLevels {
            hard_stop_loss: hard_sl,
            trailing_stop: trailing,
            take_profit: tp,
            time_stop_hours: time_stop,
            stop_distance: stop_dist,
            reward_risk_ratio: rr,
        }
    }

    /// Calculate position size from risk amount and stop distance.
    /// position_size = risk_amount / stop_distance
    pub fn position_size(risk_amount: Decimal, stop_distance: Decimal) -> Decimal {
        if stop_distance <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        (risk_amount / stop_distance).round_dp(8)
    }

    /// Calculate risk amount from equity and risk percentage.
    /// risk_amount = equity * (risk_pct / 100)
    pub fn risk_amount(equity: Decimal, risk_pct: f64) -> Decimal {
        let pct = Decimal::from_f64(risk_pct / 100.0).unwrap_or(dec!(0.01));
        (equity * pct).round_dp(8)
    }

    /// Update trailing stop when price makes a new high (for longs) or new low (for shorts).
    /// Returns the new trailing stop level, or None if it shouldn't move.
    pub fn update_trailing_stop(
        current_trailing: Decimal,
        current_price: Decimal,
        atr: Decimal,
        side: OrderSide,
    ) -> Option<Decimal> {
        match side {
            OrderSide::Buy => {
                let new_trailing = current_price - (Decimal::from_f64(1.5).unwrap_or(dec!(1.5)) * atr);
                if new_trailing > current_trailing {
                    Some(new_trailing)
                } else {
                    None // Don't move down
                }
            }
            OrderSide::Sell => {
                let new_trailing = current_price + (Decimal::from_f64(1.5).unwrap_or(dec!(1.5)) * atr);
                if new_trailing < current_trailing {
                    Some(new_trailing)
                } else {
                    None // Don't move up
                }
            }
        }
    }

    /// Check if a price has hit the hard stop loss.
    pub fn is_stopped_out(price: Decimal, stop_loss: Decimal, side: OrderSide) -> bool {
        match side {
            OrderSide::Buy => price <= stop_loss,
            OrderSide::Sell => price >= stop_loss,
        }
    }

    /// Check if take profit is hit.
    pub fn is_take_profit_hit(price: Decimal, take_profit: Decimal, side: OrderSide) -> bool {
        match side {
            OrderSide::Buy => price >= take_profit,
            OrderSide::Sell => price <= take_profit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_stop_levels() {
        let levels = StopCalculator::calculate(
            dec!(100), dec!(5), OrderSide::Buy, Market::Crypto, 2.0,
        );
        assert_eq!(levels.hard_stop_loss, dec!(90));  // 100 - 2*5
        assert_eq!(levels.stop_distance, dec!(10));   // 100 - 90
        assert_eq!(levels.take_profit, dec!(120));    // 100 + 10*2
        assert_eq!(levels.time_stop_hours, 24);
    }

    #[test]
    fn test_short_stop_levels() {
        let levels = StopCalculator::calculate(
            dec!(100), dec!(5), OrderSide::Sell, Market::Crypto, 2.0,
        );
        assert_eq!(levels.hard_stop_loss, dec!(110)); // 100 + 2*5
        assert_eq!(levels.take_profit, dec!(80));     // 100 - 10*2
    }

    #[test]
    fn test_polymarket_time_stop() {
        let levels = StopCalculator::calculate(
            dec!(0.50), dec!(0.05), OrderSide::Buy, Market::Polymarket, 2.0,
        );
        assert_eq!(levels.time_stop_hours, 72);
    }

    #[test]
    fn test_position_size() {
        // Risk $10, stop distance $5 → position size = 2 units
        let size = StopCalculator::position_size(dec!(10), dec!(5));
        assert_eq!(size, dec!(2));
    }

    #[test]
    fn test_risk_amount() {
        // Equity $1000, risk 1% → $10
        let risk = StopCalculator::risk_amount(dec!(1000), 1.0);
        assert_eq!(risk, dec!(10));
    }

    #[test]
    fn test_trailing_stop_moves_up_for_long() {
        let new = StopCalculator::update_trailing_stop(
            dec!(95), dec!(110), dec!(5), OrderSide::Buy,
        );
        assert!(new.is_some());
        assert!(new.unwrap() > dec!(95));
    }

    #[test]
    fn test_trailing_stop_never_moves_down() {
        let new = StopCalculator::update_trailing_stop(
            dec!(95), dec!(90), dec!(5), OrderSide::Buy,
        );
        assert!(new.is_none()); // Price dropped, don't move stop down
    }

    #[test]
    fn test_is_stopped_out_long() {
        assert!(StopCalculator::is_stopped_out(dec!(89), dec!(90), OrderSide::Buy));
        assert!(!StopCalculator::is_stopped_out(dec!(91), dec!(90), OrderSide::Buy));
    }

    #[test]
    fn test_is_take_profit_long() {
        assert!(StopCalculator::is_take_profit_hit(dec!(121), dec!(120), OrderSide::Buy));
        assert!(!StopCalculator::is_take_profit_hit(dec!(119), dec!(120), OrderSide::Buy));
    }

    #[test]
    fn test_rr_minimum_enforced() {
        let levels = StopCalculator::calculate(
            dec!(100), dec!(5), OrderSide::Buy, Market::Crypto, 1.0,
        );
        assert_eq!(levels.reward_risk_ratio, 2.0); // Clamped up to minimum
    }
}
