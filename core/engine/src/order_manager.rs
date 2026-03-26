use std::collections::HashMap;
use tradoshka_common::types::*;
use tradoshka_common::error::{TradoshkaError, Result};

pub struct OrderManager {
    orders: HashMap<OrderId, Order>,
}

impl OrderManager {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
        }
    }

    pub fn register(&mut self, order: Order) -> OrderId {
        let id = order.id;
        self.orders.insert(id, order);
        id
    }

    pub fn transition(&mut self, id: OrderId, new_status: OrderStatus) -> Result<()> {
        let order = self.orders.get_mut(&id).ok_or_else(|| {
            TradoshkaError::OrderNotFound(id.to_string())
        })?;

        if !order.status.can_transition_to(&new_status) {
            return Err(TradoshkaError::InvalidTransition {
                from: order.status,
                to: new_status,
            });
        }

        order.status = new_status;
        order.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn record_fill(&mut self, fill: Fill) -> Result<()> {
        let order = self.orders.get_mut(&fill.order_id).ok_or_else(|| {
            TradoshkaError::OrderNotFound(fill.order_id.to_string())
        })?;

        order.filled_quantity += fill.quantity;
        order.updated_at = chrono::Utc::now();

        if order.filled_quantity >= order.quantity {
            let current = order.status;
            let next = OrderStatus::Filled;
            if current.can_transition_to(&next) {
                order.status = next;
            }
        } else {
            let current = order.status;
            let next = OrderStatus::PartiallyFilled;
            if current.can_transition_to(&next) {
                order.status = next;
            }
        }

        Ok(())
    }

    pub fn get(&self, id: OrderId) -> Option<&Order> {
        self.orders.get(&id)
    }

    pub fn get_mut(&mut self, id: OrderId) -> Option<&mut Order> {
        self.orders.get_mut(&id)
    }

    pub fn active_orders(&self) -> Vec<&Order> {
        self.orders
            .values()
            .filter(|o| !o.status.is_terminal())
            .collect()
    }

    pub fn orders_by_strategy(&self, strategy_id: &str) -> Vec<&Order> {
        self.orders
            .values()
            .filter(|o| o.strategy_id == strategy_id)
            .collect()
    }

    pub fn all_orders(&self) -> Vec<&Order> {
        self.orders.values().collect()
    }

    pub fn order_count(&self) -> usize {
        self.orders.len()
    }
}

impl Default for OrderManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_order(symbol: &str, qty: rust_decimal::Decimal, strategy_id: &str) -> Order {
        Order::new(
            symbol.to_string(),
            OrderSide::Buy,
            OrderType::Market,
            qty,
            strategy_id.to_string(),
            Market::Crypto,
        )
    }

    fn make_fill(order: &Order, qty: rust_decimal::Decimal, price: rust_decimal::Decimal) -> Fill {
        Fill {
            order_id: order.id,
            trade_id: Uuid::new_v4(),
            symbol: order.symbol.clone(),
            side: order.side,
            price,
            quantity: qty,
            fee: dec!(0.1),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_register_order() {
        let mut mgr = OrderManager::new();
        let order = make_order("BTC-USD", dec!(1), "strat-1");
        let id = mgr.register(order);
        assert!(mgr.get(id).is_some());
        assert_eq!(mgr.order_count(), 1);
    }

    #[test]
    fn test_valid_transition() {
        let mut mgr = OrderManager::new();
        let order = make_order("BTC-USD", dec!(1), "strat-1");
        let id = mgr.register(order);
        assert!(mgr.transition(id, OrderStatus::Submitted).is_ok());
        assert_eq!(mgr.get(id).unwrap().status, OrderStatus::Submitted);
    }

    #[test]
    fn test_invalid_transition() {
        let mut mgr = OrderManager::new();
        let order = make_order("BTC-USD", dec!(1), "strat-1");
        let id = mgr.register(order);
        let result = mgr.transition(id, OrderStatus::Filled);
        assert!(result.is_err());
        assert_eq!(mgr.get(id).unwrap().status, OrderStatus::Pending);
    }

    #[test]
    fn test_partial_fill() {
        let mut mgr = OrderManager::new();
        let mut order = make_order("BTC-USD", dec!(2), "strat-1");
        order.status = OrderStatus::Submitted;
        let id = mgr.register(order.clone());
        let fill = make_fill(&order, dec!(1), dec!(50000));
        mgr.record_fill(fill).unwrap();
        let o = mgr.get(id).unwrap();
        assert_eq!(o.filled_quantity, dec!(1));
        assert_eq!(o.status, OrderStatus::PartiallyFilled);
    }

    #[test]
    fn test_complete_fill() {
        let mut mgr = OrderManager::new();
        let mut order = make_order("BTC-USD", dec!(1), "strat-1");
        order.status = OrderStatus::Submitted;
        let id = mgr.register(order.clone());
        let fill = make_fill(&order, dec!(1), dec!(50000));
        mgr.record_fill(fill).unwrap();
        let o = mgr.get(id).unwrap();
        assert_eq!(o.filled_quantity, dec!(1));
        assert_eq!(o.status, OrderStatus::Filled);
    }

    #[test]
    fn test_multi_fill_to_complete() {
        let mut mgr = OrderManager::new();
        let mut order = make_order("ETH-USD", dec!(3), "strat-2");
        order.status = OrderStatus::Submitted;
        let id = mgr.register(order.clone());

        let fill1 = make_fill(&order, dec!(1), dec!(2000));
        mgr.record_fill(fill1).unwrap();
        assert_eq!(mgr.get(id).unwrap().status, OrderStatus::PartiallyFilled);

        let fill2 = make_fill(&order, dec!(1), dec!(2000));
        mgr.record_fill(fill2).unwrap();
        assert_eq!(mgr.get(id).unwrap().status, OrderStatus::PartiallyFilled);

        let fill3 = make_fill(&order, dec!(1), dec!(2000));
        mgr.record_fill(fill3).unwrap();
        assert_eq!(mgr.get(id).unwrap().status, OrderStatus::Filled);
        assert_eq!(mgr.get(id).unwrap().filled_quantity, dec!(3));
    }

    #[test]
    fn test_active_orders() {
        let mut mgr = OrderManager::new();
        let order1 = make_order("BTC-USD", dec!(1), "strat-1");
        let mut order2 = make_order("ETH-USD", dec!(1), "strat-1");
        order2.status = OrderStatus::Filled;

        mgr.register(order1);
        mgr.register(order2);

        let active = mgr.active_orders();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].symbol, "BTC-USD");
    }

    #[test]
    fn test_nonexistent_order() {
        let mut mgr = OrderManager::new();
        let fake_id = Uuid::new_v4();
        let result = mgr.transition(fake_id, OrderStatus::Submitted);
        assert!(result.is_err());
    }
}
