use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use tradoshka_common::types::Portfolio;

pub struct HalfKellySizer {
    max_risk_pct: Decimal,
    max_position_pct: Decimal,
}

impl HalfKellySizer {
    pub fn new(max_risk_pct: Decimal, max_position_pct: Decimal) -> Self {
        Self {
            max_risk_pct,
            max_position_pct,
        }
    }

    /// Calculate position size in quote currency using Half-Kelly criterion.
    ///
    /// Kelly% = (W * R - L) / R, then divide by 2
    /// where W = win_rate, L = 1 - win_rate, R = avg_win_loss_ratio
    pub fn calculate(
        &self,
        win_rate: f64,
        avg_win_loss_ratio: f64,
        portfolio: &Portfolio,
        entry_price: Decimal,
        stop_price: Decimal,
    ) -> Decimal {
        let equity = portfolio.equity;

        // Guard: equity or entry_price <= 0
        if equity <= Decimal::ZERO || entry_price <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        // Guard: invalid inputs
        if avg_win_loss_ratio <= 0.0 || !(0.0..=1.0).contains(&win_rate) {
            return Decimal::ZERO;
        }

        let loss_rate = 1.0 - win_rate;
        let kelly_f64 = (win_rate * avg_win_loss_ratio - loss_rate) / avg_win_loss_ratio;

        // If Kelly is negative → return zero
        if kelly_f64 <= 0.0 {
            return Decimal::ZERO;
        }

        let half_kelly = kelly_f64 / 2.0;

        let half_kelly_dec = match Decimal::from_f64(half_kelly) {
            Some(d) => d,
            None => return Decimal::ZERO,
        };

        // Risk amount = min(half_kelly * equity, max_risk_pct * equity)
        let kelly_risk = half_kelly_dec * equity;
        let max_risk = self.max_risk_pct * equity;
        let risk_amount = kelly_risk.min(max_risk);

        // Stop distance = |entry_price - stop_price|
        let stop_distance = (entry_price - stop_price).abs();

        let position_size = if stop_distance <= Decimal::ZERO {
            // Fall back to risk_amount when stop distance is zero
            risk_amount
        } else {
            // Position = risk_amount / stop_distance * entry_price
            risk_amount / stop_distance * entry_price
        };

        // Cap at max_position_pct * equity
        let max_position = self.max_position_pct * equity;
        position_size.min(max_position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_portfolio(equity: Decimal) -> Portfolio {
        let mut p = Portfolio::new(equity);
        p.equity = equity;
        p
    }

    #[test]
    fn test_positive_kelly_returns_position() {
        let sizer = HalfKellySizer::new(dec!(0.02), dec!(0.10));
        let portfolio = make_portfolio(dec!(10000));
        let size = sizer.calculate(0.6, 2.0, &portfolio, dec!(100), dec!(95));
        assert!(size > Decimal::ZERO, "Expected positive position size, got {}", size);
    }

    #[test]
    fn test_negative_kelly_returns_zero() {
        // win_rate = 0 → Kelly = (0 * R - 1) / R = -1/R < 0
        let sizer = HalfKellySizer::new(dec!(0.02), dec!(0.10));
        let portfolio = make_portfolio(dec!(10000));
        let size = sizer.calculate(0.0, 2.0, &portfolio, dec!(100), dec!(95));
        assert_eq!(size, Decimal::ZERO);
    }

    #[test]
    fn test_respects_max_risk_pct() {
        // Use a high win rate and ratio to ensure half-kelly > max_risk_pct
        // win=0.9, ratio=5 → kelly = (0.9*5 - 0.1)/5 = (4.5-0.1)/5 = 0.88, half=0.44
        // half_kelly * equity = 0.44 * 10000 = 4400 > max_risk = 0.02 * 10000 = 200
        let sizer = HalfKellySizer::new(dec!(0.02), dec!(0.10));
        let portfolio = make_portfolio(dec!(10000));
        // entry=100, stop=95 → stop_dist=5
        // risk_amount = 200 (capped by max_risk_pct)
        // position = 200 / 5 * 100 = 4000
        // max_position = 0.10 * 10000 = 1000
        // so it will be capped at 1000 (max_position_pct)
        // Use a narrow stop to push position into max_risk territory
        // entry=1000, stop=999 → stop_dist=1
        // position = 200 / 1 * 1000 = 200_000 → capped at max_position = 1000
        // To test max_risk specifically: set max_position very high
        let sizer2 = HalfKellySizer::new(dec!(0.02), dec!(1.0)); // 100% max position
        // entry=100, stop=95 → stop_dist=5
        // risk = 200 (max_risk)
        // position = 200 / 5 * 100 = 4000
        // max_position = 10000 * 1.0 = 10000 → not capped
        // So risk used = position / entry_price * stop_dist = 4000 / 100 * 5 = 200 ✓
        let size = sizer2.calculate(0.9, 5.0, &portfolio, dec!(100), dec!(95));
        // Verify that position implies risk == 200 = 2% of 10000
        let stop_dist = dec!(5);
        let implied_risk = size / dec!(100) * stop_dist;
        assert!(
            (implied_risk - dec!(200)).abs() < dec!(0.001),
            "Implied risk {} should be ~200",
            implied_risk
        );
        let _ = sizer; // silence unused warning
    }

    #[test]
    fn test_respects_max_position_pct() {
        // Force position > 10% of equity by using wide ratio and narrow stop
        let sizer = HalfKellySizer::new(dec!(0.50), dec!(0.10)); // 50% risk, 10% max position
        let portfolio = make_portfolio(dec!(10000));
        // win=0.9, ratio=5: half_kelly=0.44, risk = min(0.44*10000, 0.50*10000) = 4400
        // entry=1000, stop=999 → stop_dist=1
        // position = 4400 / 1 * 1000 = 4_400_000 → capped at 10% * 10000 = 1000
        let size = sizer.calculate(0.9, 5.0, &portfolio, dec!(1000), dec!(999));
        let max_pos = dec!(0.10) * dec!(10000);
        assert_eq!(size, max_pos, "Expected size to be capped at max_position_pct");
    }

    #[test]
    fn test_zero_equity_returns_zero() {
        let sizer = HalfKellySizer::new(dec!(0.02), dec!(0.10));
        let portfolio = make_portfolio(Decimal::ZERO);
        let size = sizer.calculate(0.6, 2.0, &portfolio, dec!(100), dec!(95));
        assert_eq!(size, Decimal::ZERO);
    }

    #[test]
    fn test_zero_stop_distance() {
        // entry == stop → stop_distance = 0 → falls back to risk_amount
        let sizer = HalfKellySizer::new(dec!(0.02), dec!(0.10));
        let portfolio = make_portfolio(dec!(10000));
        // win=0.6, ratio=2: kelly=(0.6*2-0.4)/2=(1.2-0.4)/2=0.8/2=0.4, half=0.2
        // half_kelly_risk = 0.2 * 10000 = 2000 > max_risk = 0.02 * 10000 = 200
        // risk_amount = 200
        // stop_distance = 0 → position = risk_amount = 200
        // max_position = 0.10 * 10000 = 1000 → 200 < 1000, so result = 200
        let size = sizer.calculate(0.6, 2.0, &portfolio, dec!(100), dec!(100));
        assert!(size > Decimal::ZERO, "Expected > 0 when stop == entry, got {}", size);
    }
}
