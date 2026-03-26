use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub type OrderId = Uuid;
pub type TradeId = Uuid;
pub type StrategyId = String;
pub type Symbol = String;

// ---------------------------------------------------------------------------
// Market
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Market {
    Polymarket,
    Crypto,
    Forex,
    Stocks,
}

// ---------------------------------------------------------------------------
// OrderSide
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

// ---------------------------------------------------------------------------
// OrderType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit { price: Decimal },
    StopLoss { trigger: Decimal },
    TrailingStop { offset_pct: Decimal },
}

// ---------------------------------------------------------------------------
// OrderStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Submitted,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

impl OrderStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Rejected)
    }

    pub fn can_transition_to(&self, next: &OrderStatus) -> bool {
        matches!(
            (self, next),
            (OrderStatus::Pending, OrderStatus::Submitted)
                | (OrderStatus::Pending, OrderStatus::Rejected)
                | (OrderStatus::Submitted, OrderStatus::PartiallyFilled)
                | (OrderStatus::Submitted, OrderStatus::Filled)
                | (OrderStatus::Submitted, OrderStatus::Cancelled)
                | (OrderStatus::Submitted, OrderStatus::Rejected)
                | (OrderStatus::PartiallyFilled, OrderStatus::Filled)
                | (OrderStatus::PartiallyFilled, OrderStatus::Cancelled)
        )
    }
}

// ---------------------------------------------------------------------------
// Order
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub symbol: Symbol,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub strategy_id: StrategyId,
    pub market: Market,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Order {
    pub fn new(
        symbol: Symbol,
        side: OrderSide,
        order_type: OrderType,
        quantity: Decimal,
        strategy_id: StrategyId,
        market: Market,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            symbol,
            side,
            order_type,
            quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::Pending,
            strategy_id,
            market,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn remaining_quantity(&self) -> Decimal {
        self.quantity - self.filled_quantity
    }
}

// ---------------------------------------------------------------------------
// Candle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub symbol: Symbol,
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

// ---------------------------------------------------------------------------
// Trade
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: Symbol,
    pub price: Decimal,
    pub quantity: Decimal,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// OrderBookLevel
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: Decimal,
    pub quantity: Decimal,
}

// ---------------------------------------------------------------------------
// MarketEvent
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketEvent {
    TradeEvent(Trade),
    OrderBookUpdate {
        symbol: Symbol,
        bids: Vec<OrderBookLevel>,
        asks: Vec<OrderBookLevel>,
        timestamp: DateTime<Utc>,
    },
    CandleEvent(Candle),
}

impl MarketEvent {
    pub fn symbol(&self) -> &str {
        match self {
            MarketEvent::TradeEvent(t) => &t.symbol,
            MarketEvent::OrderBookUpdate { symbol, .. } => symbol,
            MarketEvent::CandleEvent(c) => &c.symbol,
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            MarketEvent::TradeEvent(t) => t.timestamp,
            MarketEvent::OrderBookUpdate { timestamp, .. } => *timestamp,
            MarketEvent::CandleEvent(c) => c.timestamp,
        }
    }
}

// ---------------------------------------------------------------------------
// SignalDirection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalDirection {
    Long,
    Short,
    Close,
    Hold,
}

// ---------------------------------------------------------------------------
// Signal
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub strategy_id: StrategyId,
    pub symbol: Symbol,
    pub direction: SignalDirection,
    pub strength: f64,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl Signal {
    pub fn new(
        strategy_id: StrategyId,
        symbol: Symbol,
        direction: SignalDirection,
        strength: f64,
        stop_loss: Option<Decimal>,
        take_profit: Option<Decimal>,
        timestamp: DateTime<Utc>,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            strategy_id,
            symbol,
            direction,
            strength: strength.clamp(0.0, 1.0),
            stop_loss,
            take_profit,
            timestamp,
            metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// RiskDecision
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskDecision {
    Approved { adjusted_quantity: Decimal },
    Rejected { reason: String },
}

// ---------------------------------------------------------------------------
// Position
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: Symbol,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub strategy_id: StrategyId,
    pub market: Market,
    pub opened_at: DateTime<Utc>,
}

impl Position {
    pub fn update_price(&mut self, price: Decimal) {
        self.current_price = price;
        self.unrealized_pnl = match self.side {
            OrderSide::Buy => (price - self.entry_price) * self.quantity,
            OrderSide::Sell => -(price - self.entry_price) * self.quantity,
        };
    }
}

// ---------------------------------------------------------------------------
// Portfolio
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub balance: Decimal,
    pub equity: Decimal,
    pub positions: Vec<Position>,
    pub daily_pnl: Decimal,
    pub peak_equity: Decimal,
    pub current_drawdown_pct: Decimal,
}

impl Portfolio {
    pub fn new(initial_balance: Decimal) -> Self {
        Self {
            balance: initial_balance,
            equity: initial_balance,
            positions: Vec::new(),
            daily_pnl: Decimal::ZERO,
            peak_equity: initial_balance,
            current_drawdown_pct: Decimal::ZERO,
        }
    }

    pub fn update_equity(&mut self) {
        let unrealized: Decimal = self.positions.iter().map(|p| p.unrealized_pnl).sum();
        self.equity = self.balance + unrealized;

        if self.equity > self.peak_equity {
            self.peak_equity = self.equity;
        }

        if self.peak_equity > Decimal::ZERO {
            self.current_drawdown_pct =
                (self.peak_equity - self.equity) / self.peak_equity * Decimal::ONE_HUNDRED;
        } else {
            self.current_drawdown_pct = Decimal::ZERO;
        }
    }
}

// ---------------------------------------------------------------------------
// Balances
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balances {
    pub total: Decimal,
    pub available: Decimal,
    pub in_positions: Decimal,
}

// ---------------------------------------------------------------------------
// Fill
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub order_id: OrderId,
    pub trade_id: TradeId,
    pub symbol: Symbol,
    pub side: OrderSide,
    pub price: Decimal,
    pub quantity: Decimal,
    pub fee: Decimal,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_order_status_terminal() {
        assert!(!OrderStatus::Pending.is_terminal());
        assert!(!OrderStatus::Submitted.is_terminal());
        assert!(!OrderStatus::PartiallyFilled.is_terminal());
        assert!(OrderStatus::Filled.is_terminal());
        assert!(OrderStatus::Cancelled.is_terminal());
        assert!(OrderStatus::Rejected.is_terminal());
    }

    #[test]
    fn test_order_status_valid_transitions() {
        assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Submitted));
        assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Rejected));
        assert!(OrderStatus::Submitted.can_transition_to(&OrderStatus::PartiallyFilled));
        assert!(OrderStatus::Submitted.can_transition_to(&OrderStatus::Filled));
        assert!(OrderStatus::Submitted.can_transition_to(&OrderStatus::Cancelled));
        assert!(OrderStatus::Submitted.can_transition_to(&OrderStatus::Rejected));
        assert!(OrderStatus::PartiallyFilled.can_transition_to(&OrderStatus::Filled));
        assert!(OrderStatus::PartiallyFilled.can_transition_to(&OrderStatus::Cancelled));
    }

    #[test]
    fn test_order_status_invalid_transitions() {
        assert!(!OrderStatus::Pending.can_transition_to(&OrderStatus::Filled));
        assert!(!OrderStatus::Pending.can_transition_to(&OrderStatus::PartiallyFilled));
        assert!(!OrderStatus::Filled.can_transition_to(&OrderStatus::Cancelled));
        assert!(!OrderStatus::Cancelled.can_transition_to(&OrderStatus::Submitted));
        assert!(!OrderStatus::Rejected.can_transition_to(&OrderStatus::Pending));
        assert!(!OrderStatus::PartiallyFilled.can_transition_to(&OrderStatus::Submitted));
    }

    #[test]
    fn test_order_new() {
        let order = Order::new(
            "BTC-USD".to_string(),
            OrderSide::Buy,
            OrderType::Market,
            dec!(1.5),
            "strat-1".to_string(),
            Market::Crypto,
        );

        assert_eq!(order.symbol, "BTC-USD");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.quantity, dec!(1.5));
        assert_eq!(order.filled_quantity, Decimal::ZERO);
        assert_eq!(order.status, OrderStatus::Pending);
        assert_eq!(order.remaining_quantity(), dec!(1.5));
    }

    #[test]
    fn test_signal_clamps_strength() {
        let s_high = Signal::new(
            "s".to_string(),
            "X".to_string(),
            SignalDirection::Long,
            1.5,
            None,
            None,
            Utc::now(),
            HashMap::new(),
        );
        assert_eq!(s_high.strength, 1.0);

        let s_low = Signal::new(
            "s".to_string(),
            "X".to_string(),
            SignalDirection::Short,
            -0.3,
            None,
            None,
            Utc::now(),
            HashMap::new(),
        );
        assert_eq!(s_low.strength, 0.0);

        let s_mid = Signal::new(
            "s".to_string(),
            "X".to_string(),
            SignalDirection::Hold,
            0.7,
            None,
            None,
            Utc::now(),
            HashMap::new(),
        );
        assert!((s_mid.strength - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_position_update_price_long() {
        let mut pos = Position {
            symbol: "ETH-USD".to_string(),
            side: OrderSide::Buy,
            quantity: dec!(2),
            entry_price: dec!(1000),
            current_price: dec!(1000),
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            strategy_id: "s".to_string(),
            market: Market::Crypto,
            opened_at: Utc::now(),
        };

        pos.update_price(dec!(1100));
        assert_eq!(pos.current_price, dec!(1100));
        // (1100 - 1000) * 2 = 200
        assert_eq!(pos.unrealized_pnl, dec!(200));
    }

    #[test]
    fn test_position_update_price_short() {
        let mut pos = Position {
            symbol: "ETH-USD".to_string(),
            side: OrderSide::Sell,
            quantity: dec!(2),
            entry_price: dec!(1000),
            current_price: dec!(1000),
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            strategy_id: "s".to_string(),
            market: Market::Crypto,
            opened_at: Utc::now(),
        };

        pos.update_price(dec!(900));
        assert_eq!(pos.current_price, dec!(900));
        // -(900 - 1000) * 2 = 200
        assert_eq!(pos.unrealized_pnl, dec!(200));
    }

    #[test]
    fn test_portfolio_drawdown() {
        let mut portfolio = Portfolio::new(dec!(10000));
        // equity == balance == peak at start
        portfolio.update_equity();
        assert_eq!(portfolio.current_drawdown_pct, Decimal::ZERO);

        // Simulate a position with a loss
        let mut pos = Position {
            symbol: "BTC-USD".to_string(),
            side: OrderSide::Buy,
            quantity: dec!(1),
            entry_price: dec!(10000),
            current_price: dec!(10000),
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            strategy_id: "s".to_string(),
            market: Market::Crypto,
            opened_at: Utc::now(),
        };
        pos.update_price(dec!(9000));
        portfolio.positions.push(pos);
        portfolio.update_equity();

        // equity = 10000 + (-1000) = 9000; peak = 10000; drawdown = 10%
        assert_eq!(portfolio.equity, dec!(9000));
        assert_eq!(portfolio.peak_equity, dec!(10000));
        assert_eq!(portfolio.current_drawdown_pct, dec!(10));
    }

    #[test]
    fn test_market_event_symbol() {
        let trade = Trade {
            symbol: "BTC-USD".to_string(),
            price: dec!(50000),
            quantity: dec!(0.1),
            timestamp: Utc::now(),
        };
        let event = MarketEvent::TradeEvent(trade);
        assert_eq!(event.symbol(), "BTC-USD");

        let candle = Candle {
            symbol: "ETH-USD".to_string(),
            timestamp: Utc::now(),
            open: dec!(1000),
            high: dec!(1100),
            low: dec!(990),
            close: dec!(1050),
            volume: dec!(500),
        };
        let event2 = MarketEvent::CandleEvent(candle);
        assert_eq!(event2.symbol(), "ETH-USD");

        let ts = Utc::now();
        let ob_event = MarketEvent::OrderBookUpdate {
            symbol: "SOL-USD".to_string(),
            bids: vec![],
            asks: vec![],
            timestamp: ts,
        };
        assert_eq!(ob_event.symbol(), "SOL-USD");
        assert_eq!(ob_event.timestamp(), ts);
    }
}
