use rust_decimal::Decimal;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    Normal,
    DailyPause,
    PortfolioHalt,
    HardStop,
}

impl BreakerState {
    /// Only Normal allows new trades.
    pub fn allows_new_trades(&self) -> bool {
        matches!(self, BreakerState::Normal)
    }

    /// Only HardStop requires closing all positions.
    pub fn requires_close_all(&self) -> bool {
        matches!(self, BreakerState::HardStop)
    }
}

pub struct CircuitBreaker {
    daily_limit: Decimal,
    portfolio_halt: Decimal,
    hard_stop: Decimal,
    state: BreakerState,
}

impl CircuitBreaker {
    pub fn new(daily_limit: Decimal, portfolio_halt: Decimal, hard_stop: Decimal) -> Self {
        Self {
            daily_limit,
            portfolio_halt,
            hard_stop,
            state: BreakerState::Normal,
        }
    }

    /// Evaluate drawdown levels and update state. Returns the new state.
    pub fn evaluate(
        &mut self,
        portfolio_drawdown: Decimal,
        daily_drawdown: Decimal,
    ) -> BreakerState {
        // Escalate only — never auto-downgrade unless drawdown has recovered
        let new_state = if portfolio_drawdown >= self.hard_stop {
            warn!(
                portfolio_drawdown = %portfolio_drawdown,
                threshold = %self.hard_stop,
                "HARD STOP triggered — portfolio drawdown exceeded hard stop threshold"
            );
            BreakerState::HardStop
        } else if portfolio_drawdown >= self.portfolio_halt {
            warn!(
                portfolio_drawdown = %portfolio_drawdown,
                threshold = %self.portfolio_halt,
                "PORTFOLIO HALT triggered — portfolio drawdown exceeded halt threshold"
            );
            BreakerState::PortfolioHalt
        } else if daily_drawdown >= self.daily_limit {
            warn!(
                daily_drawdown = %daily_drawdown,
                threshold = %self.daily_limit,
                "DAILY PAUSE triggered — daily drawdown exceeded daily limit"
            );
            BreakerState::DailyPause
        } else {
            // Drawdown recovered — return to Normal (unless HardStop, which never auto-recovers)
            if self.state == BreakerState::HardStop {
                BreakerState::HardStop
            } else {
                BreakerState::Normal
            }
        };

        self.state = new_state;
        new_state
    }

    pub fn state(&self) -> BreakerState {
        self.state
    }

    /// Reset DailyPause or PortfolioHalt back to Normal.
    /// Returns false if state is HardStop (cannot be reset this way).
    pub fn reset(&mut self) -> bool {
        match self.state {
            BreakerState::HardStop => false,
            _ => {
                self.state = BreakerState::Normal;
                true
            }
        }
    }

    /// Force reset even from HardStop.
    pub fn force_reset(&mut self) {
        self.state = BreakerState::Normal;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_cb() -> CircuitBreaker {
        // daily_limit=5%, portfolio_halt=15%, hard_stop=20%
        CircuitBreaker::new(dec!(5), dec!(15), dec!(20))
    }

    #[test]
    fn test_normal_state_allows_trades() {
        let cb = make_cb();
        assert_eq!(cb.state(), BreakerState::Normal);
        assert!(cb.state().allows_new_trades());
        assert!(!cb.state().requires_close_all());
    }

    #[test]
    fn test_daily_pause() {
        let mut cb = make_cb();
        // daily drawdown > 5%
        let state = cb.evaluate(dec!(0), dec!(6));
        assert_eq!(state, BreakerState::DailyPause);
        assert!(!state.allows_new_trades());
        assert!(!state.requires_close_all());
    }

    #[test]
    fn test_portfolio_halt() {
        let mut cb = make_cb();
        // portfolio drawdown > 15%
        let state = cb.evaluate(dec!(16), dec!(0));
        assert_eq!(state, BreakerState::PortfolioHalt);
        assert!(!state.allows_new_trades());
        assert!(!state.requires_close_all());
    }

    #[test]
    fn test_hard_stop() {
        let mut cb = make_cb();
        // portfolio drawdown > 20%
        let state = cb.evaluate(dec!(21), dec!(0));
        assert_eq!(state, BreakerState::HardStop);
        assert!(!state.allows_new_trades());
        assert!(state.requires_close_all());
    }

    #[test]
    fn test_hard_stop_cannot_normal_reset() {
        let mut cb = make_cb();
        cb.evaluate(dec!(25), dec!(0));
        assert_eq!(cb.state(), BreakerState::HardStop);
        let result = cb.reset();
        assert!(!result, "reset() should return false for HardStop");
        assert_eq!(cb.state(), BreakerState::HardStop);
    }

    #[test]
    fn test_force_reset_from_hard_stop() {
        let mut cb = make_cb();
        cb.evaluate(dec!(25), dec!(0));
        assert_eq!(cb.state(), BreakerState::HardStop);
        cb.force_reset();
        assert_eq!(cb.state(), BreakerState::Normal);
    }

    #[test]
    fn test_recovery_to_normal() {
        let mut cb = make_cb();
        // Trigger DailyPause
        cb.evaluate(dec!(0), dec!(6));
        assert_eq!(cb.state(), BreakerState::DailyPause);
        // Drawdown recovers below all thresholds → back to Normal
        let state = cb.evaluate(dec!(0), dec!(1));
        assert_eq!(state, BreakerState::Normal);
        assert!(state.allows_new_trades());
    }
}
