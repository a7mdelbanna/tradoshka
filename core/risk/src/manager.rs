use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tradoshka_common::types::*;
use tradoshka_common::traits::RiskManager;
use crate::position_sizer::HalfKellySizer;
use crate::circuit_breaker::{CircuitBreaker, BreakerState};
use crate::drawdown_tracker::DrawdownTracker;
use chrono::Utc;

pub struct RiskConfig {
    pub max_risk_pct: Decimal,
    pub max_position_pct: Decimal,
    pub daily_drawdown_limit: Decimal,
    pub portfolio_drawdown_halt: Decimal,
    pub hard_stop: Decimal,
    pub max_open_positions: usize,
    pub max_exposure_pct: Decimal,
    pub initial_equity: Decimal,
    pub default_win_rate: f64,
    pub default_win_loss_ratio: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_risk_pct: dec!(0.02),
            max_position_pct: dec!(0.10),
            daily_drawdown_limit: dec!(0.05),
            portfolio_drawdown_halt: dec!(0.15),
            hard_stop: dec!(0.20),
            max_open_positions: 10,
            max_exposure_pct: dec!(0.30),
            initial_equity: dec!(10000),
            default_win_rate: 0.55,
            default_win_loss_ratio: 1.5,
        }
    }
}

pub struct TradoshkaRiskManager {
    sizer: HalfKellySizer,
    breaker: CircuitBreaker,
    tracker: DrawdownTracker,
    config: RiskConfig,
}

impl TradoshkaRiskManager {
    pub fn new(config: RiskConfig) -> Self {
        let sizer = HalfKellySizer::new(config.max_risk_pct, config.max_position_pct);
        let breaker = CircuitBreaker::new(
            config.daily_drawdown_limit * Decimal::ONE_HUNDRED,
            config.portfolio_drawdown_halt * Decimal::ONE_HUNDRED,
            config.hard_stop * Decimal::ONE_HUNDRED,
        );
        let tracker = DrawdownTracker::new(config.initial_equity);
        Self {
            sizer,
            breaker,
            tracker,
            config,
        }
    }

    /// Update the current equity, recalculate drawdowns, and re-evaluate the circuit breaker.
    pub fn update_equity(&mut self, equity: Decimal) {
        let (portfolio_drawdown, daily_drawdown) = self.tracker.update(equity, Utc::now());
        self.breaker.evaluate(portfolio_drawdown, daily_drawdown);
    }

    pub fn breaker_state(&self) -> BreakerState {
        self.breaker.state()
    }

    pub fn force_reset_breaker(&mut self) {
        self.breaker.force_reset();
    }
}

impl RiskManager for TradoshkaRiskManager {
    fn validate_order(&self, order: &Order, portfolio: &Portfolio) -> RiskDecision {
        // 1. Circuit breaker check
        if !self.breaker.state().allows_new_trades() {
            return RiskDecision::Rejected {
                reason: format!("Circuit breaker active: {:?}", self.breaker.state()),
            };
        }

        // 2. Max open positions check
        if portfolio.positions.len() >= self.config.max_open_positions {
            return RiskDecision::Rejected {
                reason: format!(
                    "Max open positions reached: {}/{}",
                    portfolio.positions.len(),
                    self.config.max_open_positions
                ),
            };
        }

        // 3. Total exposure check — current exposure + this order
        let current_exposure: Decimal = portfolio
            .positions
            .iter()
            .map(|p| p.quantity * p.current_price)
            .sum();
        let order_price = match &order.order_type {
            OrderType::Limit { price } => *price,
            OrderType::StopLoss { trigger } => *trigger,
            _ => portfolio.equity, // conservative fallback for market orders
        };
        let order_value = order.quantity * order_price;
        let total_exposure = current_exposure + order_value;
        let max_exposure = self.config.max_exposure_pct * portfolio.equity;
        if total_exposure > max_exposure {
            return RiskDecision::Rejected {
                reason: format!(
                    "Total exposure {} exceeds max {}",
                    total_exposure, max_exposure
                ),
            };
        }

        // 4. Sufficient balance check
        if order_value > portfolio.balance {
            return RiskDecision::Rejected {
                reason: format!(
                    "Insufficient balance: order value {} > balance {}",
                    order_value, portfolio.balance
                ),
            };
        }

        RiskDecision::Approved {
            adjusted_quantity: order.quantity,
        }
    }

    fn check_circuit_breakers(&self, _portfolio: &Portfolio) -> bool {
        self.breaker.state().allows_new_trades()
    }

    fn calculate_position_size(&self, signal: &Signal, portfolio: &Portfolio) -> Decimal {
        // Use entry price from portfolio equity as proxy; stop_loss from signal
        let entry_price = portfolio.equity; // best guess when no explicit entry price
        let stop_price = signal.stop_loss.unwrap_or(entry_price);
        self.sizer.calculate(
            self.config.default_win_rate,
            self.config.default_win_loss_ratio,
            portfolio,
            entry_price,
            stop_price,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_manager(max_positions: usize) -> TradoshkaRiskManager {
        let config = RiskConfig {
            max_open_positions: max_positions,
            ..RiskConfig::default()
        };
        TradoshkaRiskManager::new(config)
    }

    fn make_portfolio(balance: Decimal) -> Portfolio {
        Portfolio::new(balance)
    }

    fn make_order(quantity: Decimal, price: Decimal) -> Order {
        Order::new(
            "BTC-USD".to_string(),
            OrderSide::Buy,
            OrderType::Limit { price },
            quantity,
            "test-strategy".to_string(),
            Market::Crypto,
        )
    }

    #[test]
    fn test_approves_valid_order() {
        let manager = make_manager(10);
        let portfolio = make_portfolio(dec!(10000));
        // quantity=1, price=100 → order_value=100; balance=10000 → OK
        let order = make_order(dec!(1), dec!(100));
        let decision = manager.validate_order(&order, &portfolio);
        assert!(
            matches!(decision, RiskDecision::Approved { .. }),
            "Expected Approved, got {:?}",
            decision
        );
    }

    #[test]
    fn test_rejects_when_max_positions_reached() {
        let manager = make_manager(3);
        let mut portfolio = make_portfolio(dec!(10000));
        // Fill portfolio with 3 positions
        for i in 0..3 {
            portfolio.positions.push(Position {
                symbol: format!("SYM-{}", i),
                side: OrderSide::Buy,
                quantity: dec!(1),
                entry_price: dec!(10),
                current_price: dec!(10),
                unrealized_pnl: Decimal::ZERO,
                realized_pnl: Decimal::ZERO,
                strategy_id: "s".to_string(),
                market: Market::Crypto,
                opened_at: Utc::now(),
            });
        }
        let order = make_order(dec!(1), dec!(100));
        let decision = manager.validate_order(&order, &portfolio);
        assert!(
            matches!(decision, RiskDecision::Rejected { .. }),
            "Expected Rejected due to max positions, got {:?}",
            decision
        );
    }

    #[test]
    fn test_rejects_when_circuit_breaker_tripped() {
        let mut manager = make_manager(10);
        // 25% drawdown → exceeds hard_stop (20%) → HardStop
        manager.update_equity(dec!(7500)); // 25% below initial 10000
        let portfolio = make_portfolio(dec!(7500));
        let order = make_order(dec!(1), dec!(100));
        let decision = manager.validate_order(&order, &portfolio);
        assert!(
            matches!(decision, RiskDecision::Rejected { .. }),
            "Expected Rejected due to circuit breaker, got {:?}",
            decision
        );
        assert!(!manager.breaker_state().allows_new_trades());
    }

    #[test]
    fn test_rejects_insufficient_balance() {
        let manager = make_manager(10);
        // balance=100, order_value = 10 * 50 = 500 → rejected
        let portfolio = make_portfolio(dec!(100));
        let order = make_order(dec!(10), dec!(50));
        let decision = manager.validate_order(&order, &portfolio);
        assert!(
            matches!(decision, RiskDecision::Rejected { .. }),
            "Expected Rejected due to insufficient balance, got {:?}",
            decision
        );
    }
}
