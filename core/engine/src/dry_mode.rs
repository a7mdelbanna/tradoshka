use rust_decimal::Decimal;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use tradoshka_common::types::*;
use tradoshka_common::error::Result;

pub struct DryModeEngine {
    last_prices: HashMap<Symbol, Decimal>,
    slippage_bps: Decimal,
}

impl DryModeEngine {
    pub fn new(slippage_bps: Decimal) -> Self {
        Self {
            last_prices: HashMap::new(),
            slippage_bps,
        }
    }

    pub fn update_price(&mut self, symbol: Symbol, price: Decimal) {
        self.last_prices.insert(symbol, price);
    }

    pub fn try_execute(&self, order: &Order) -> Result<Option<Fill>> {
        let market_price = match self.last_prices.get(&order.symbol) {
            Some(p) => *p,
            None => return Ok(None),
        };

        let fill_price_opt = match &order.order_type {
            OrderType::Market => Some(market_price),

            OrderType::Limit { price } => match order.side {
                OrderSide::Buy => {
                    if market_price <= *price {
                        Some(market_price)
                    } else {
                        None
                    }
                }
                OrderSide::Sell => {
                    if market_price >= *price {
                        Some(market_price)
                    } else {
                        None
                    }
                }
            },

            OrderType::StopLoss { trigger } => match order.side {
                OrderSide::Sell => {
                    if market_price <= *trigger {
                        Some(market_price)
                    } else {
                        None
                    }
                }
                OrderSide::Buy => {
                    if market_price >= *trigger {
                        Some(market_price)
                    } else {
                        None
                    }
                }
            },

            OrderType::TrailingStop { .. } => return Ok(None),
        };

        let raw_price = match fill_price_opt {
            Some(p) => p,
            None => return Ok(None),
        };

        // Apply slippage: buys get worse (higher), sells get worse (lower)
        let slippage_factor = self.slippage_bps / Decimal::new(10000, 0);
        let fill_price = match order.side {
            OrderSide::Buy => raw_price * (Decimal::ONE + slippage_factor),
            OrderSide::Sell => raw_price * (Decimal::ONE - slippage_factor),
        };

        let fill_value = fill_price * order.quantity;
        let fee = fill_value * Decimal::new(1, 3); // 0.001 = 0.1%

        let fill = Fill {
            order_id: order.id,
            trade_id: Uuid::new_v4(),
            symbol: order.symbol.clone(),
            side: order.side,
            price: fill_price,
            quantity: order.quantity,
            fee,
            timestamp: Utc::now(),
        };

        Ok(Some(fill))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn market_order(symbol: &str, side: OrderSide) -> Order {
        Order::new(
            symbol.to_string(),
            side,
            OrderType::Market,
            dec!(1),
            "strat-1".to_string(),
            Market::Crypto,
        )
    }

    fn limit_order(symbol: &str, side: OrderSide, price: Decimal) -> Order {
        Order::new(
            symbol.to_string(),
            side,
            OrderType::Limit { price },
            dec!(1),
            "strat-1".to_string(),
            Market::Crypto,
        )
    }

    fn stoploss_order(symbol: &str, side: OrderSide, trigger: Decimal) -> Order {
        Order::new(
            symbol.to_string(),
            side,
            OrderType::StopLoss { trigger },
            dec!(1),
            "strat-1".to_string(),
            Market::Crypto,
        )
    }

    #[test]
    fn test_market_order_fills_immediately() {
        let mut engine = DryModeEngine::new(dec!(0));
        engine.update_price("BTC-USD".to_string(), dec!(50000));
        let order = market_order("BTC-USD", OrderSide::Buy);
        let result = engine.try_execute(&order).unwrap();
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.quantity, dec!(1));
    }

    #[test]
    fn test_limit_buy_fills_when_price_at_or_below() {
        let mut engine = DryModeEngine::new(dec!(0));
        // Market at limit price — should fill
        engine.update_price("BTC-USD".to_string(), dec!(49000));
        let order = limit_order("BTC-USD", OrderSide::Buy, dec!(50000));
        let result = engine.try_execute(&order).unwrap();
        assert!(result.is_some());

        // Market exactly at limit — should fill
        engine.update_price("BTC-USD".to_string(), dec!(50000));
        let order2 = limit_order("BTC-USD", OrderSide::Buy, dec!(50000));
        let result2 = engine.try_execute(&order2).unwrap();
        assert!(result2.is_some());
    }

    #[test]
    fn test_limit_buy_does_not_fill_above_limit() {
        let mut engine = DryModeEngine::new(dec!(0));
        engine.update_price("BTC-USD".to_string(), dec!(51000));
        let order = limit_order("BTC-USD", OrderSide::Buy, dec!(50000));
        let result = engine.try_execute(&order).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_stop_loss_triggers_below() {
        let mut engine = DryModeEngine::new(dec!(0));
        engine.update_price("BTC-USD".to_string(), dec!(45000));
        let order = stoploss_order("BTC-USD", OrderSide::Sell, dec!(48000));
        let result = engine.try_execute(&order).unwrap();
        assert!(result.is_some());

        // Above trigger — should not fill
        engine.update_price("BTC-USD".to_string(), dec!(50000));
        let order2 = stoploss_order("BTC-USD", OrderSide::Sell, dec!(48000));
        let result2 = engine.try_execute(&order2).unwrap();
        assert!(result2.is_none());
    }

    #[test]
    fn test_slippage_applied() {
        let mut engine = DryModeEngine::new(dec!(10)); // 10 bps = 0.1%
        engine.update_price("BTC-USD".to_string(), dec!(50000));

        // Buy order — price should be higher
        let buy_order = market_order("BTC-USD", OrderSide::Buy);
        let buy_fill = engine.try_execute(&buy_order).unwrap().unwrap();
        assert!(buy_fill.price > dec!(50000));

        // Sell order — price should be lower
        let sell_order = market_order("BTC-USD", OrderSide::Sell);
        let sell_fill = engine.try_execute(&sell_order).unwrap().unwrap();
        assert!(sell_fill.price < dec!(50000));
    }

    #[test]
    fn test_no_price_returns_none() {
        let engine = DryModeEngine::new(dec!(0));
        let order = market_order("BTC-USD", OrderSide::Buy);
        let result = engine.try_execute(&order).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_fee_calculated() {
        let mut engine = DryModeEngine::new(dec!(0));
        engine.update_price("BTC-USD".to_string(), dec!(50000));
        let order = market_order("BTC-USD", OrderSide::Buy);
        let fill = engine.try_execute(&order).unwrap().unwrap();
        // fee = 50000 * 1 * 0.001 = 50
        assert_eq!(fill.fee, dec!(50));
    }
}
