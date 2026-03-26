use rust_decimal::Decimal;
use chrono::Utc;
use std::collections::HashMap;
use tradoshka_common::types::*;

pub struct PortfolioTracker {
    balance: Decimal,
    positions: HashMap<String, Position>,
    realized_pnl: Decimal,
    total_fees: Decimal,
    trade_count: u64,
    winning_trades: u64,
}

impl PortfolioTracker {
    pub fn new(initial_balance: Decimal) -> Self {
        Self {
            balance: initial_balance,
            positions: HashMap::new(),
            realized_pnl: Decimal::ZERO,
            total_fees: Decimal::ZERO,
            trade_count: 0,
            winning_trades: 0,
        }
    }

    pub fn process_fill(&mut self, fill: Fill, strategy_id: &str, market: Market) {
        let key = format!("{}:{}", fill.symbol, strategy_id);

        self.total_fees += fill.fee;
        self.balance -= fill.fee;

        if let Some(pos) = self.positions.get_mut(&key) {
            if pos.side == fill.side {
                // Same side — average entry
                let total_qty = pos.quantity + fill.quantity;
                let avg_price = (pos.entry_price * pos.quantity + fill.price * fill.quantity) / total_qty;
                pos.entry_price = avg_price;
                pos.quantity = total_qty;
                pos.update_price(fill.price);
            } else {
                // Opposite side — close or reduce
                let close_qty = fill.quantity.min(pos.quantity);

                let pnl = match pos.side {
                    OrderSide::Buy => (fill.price - pos.entry_price) * close_qty,
                    OrderSide::Sell => (pos.entry_price - fill.price) * close_qty,
                };

                self.realized_pnl += pnl;
                self.balance += pnl;
                self.trade_count += 1;
                if pnl > Decimal::ZERO {
                    self.winning_trades += 1;
                }

                if fill.quantity >= pos.quantity {
                    // Full close
                    self.positions.remove(&key);

                    // If there's excess quantity, open a new position in the opposite direction
                    let excess = fill.quantity - close_qty;
                    if excess > Decimal::ZERO {
                        let new_pos = Position {
                            symbol: fill.symbol.clone(),
                            side: fill.side,
                            quantity: excess,
                            entry_price: fill.price,
                            current_price: fill.price,
                            unrealized_pnl: Decimal::ZERO,
                            realized_pnl: Decimal::ZERO,
                            strategy_id: strategy_id.to_string(),
                            market,
                            opened_at: Utc::now(),
                        };
                        self.positions.insert(key, new_pos);
                    }
                } else {
                    // Partial close
                    pos.quantity -= fill.quantity;
                    pos.update_price(fill.price);
                }
            }
        } else {
            // Open new position
            let pos = Position {
                symbol: fill.symbol.clone(),
                side: fill.side,
                quantity: fill.quantity,
                entry_price: fill.price,
                current_price: fill.price,
                unrealized_pnl: Decimal::ZERO,
                realized_pnl: Decimal::ZERO,
                strategy_id: strategy_id.to_string(),
                market,
                opened_at: Utc::now(),
            };
            self.positions.insert(key, pos);
        }
    }

    pub fn update_price(&mut self, symbol: &str, price: Decimal) {
        for pos in self.positions.values_mut() {
            if pos.symbol == symbol {
                pos.update_price(price);
            }
        }
    }

    pub fn snapshot(&self) -> Portfolio {
        let positions: Vec<Position> = self.positions.values().cloned().collect();
        let mut portfolio = Portfolio {
            balance: self.balance,
            equity: self.balance,
            positions,
            daily_pnl: self.realized_pnl,
            peak_equity: self.balance,
            current_drawdown_pct: Decimal::ZERO,
        };
        portfolio.update_equity();
        portfolio
    }

    pub fn win_rate(&self) -> f64 {
        if self.trade_count == 0 {
            return 0.0;
        }
        self.winning_trades as f64 / self.trade_count as f64
    }

    pub fn realized_pnl(&self) -> Decimal {
        self.realized_pnl
    }

    pub fn total_fees(&self) -> Decimal {
        self.total_fees
    }

    pub fn open_position_count(&self) -> usize {
        self.positions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use uuid::Uuid;
    use chrono::Utc;

    fn make_fill(symbol: &str, side: OrderSide, price: Decimal, qty: Decimal, fee: Decimal) -> Fill {
        Fill {
            order_id: Uuid::new_v4(),
            trade_id: Uuid::new_v4(),
            symbol: symbol.to_string(),
            side,
            price,
            quantity: qty,
            fee,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_open_long_position() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        let fill = make_fill("BTC-USD", OrderSide::Buy, dec!(50000), dec!(0.1), dec!(5));
        tracker.process_fill(fill, "strat-1", Market::Crypto);

        assert_eq!(tracker.open_position_count(), 1);
        let snap = tracker.snapshot();
        assert_eq!(snap.positions.len(), 1);
        assert_eq!(snap.positions[0].entry_price, dec!(50000));
        assert_eq!(snap.positions[0].quantity, dec!(0.1));
    }

    #[test]
    fn test_close_long_with_profit() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        let buy_fill = make_fill("BTC-USD", OrderSide::Buy, dec!(50000), dec!(1), dec!(5));
        tracker.process_fill(buy_fill, "strat-1", Market::Crypto);

        let sell_fill = make_fill("BTC-USD", OrderSide::Sell, dec!(55000), dec!(1), dec!(5));
        tracker.process_fill(sell_fill, "strat-1", Market::Crypto);

        assert_eq!(tracker.open_position_count(), 0);
        assert_eq!(tracker.realized_pnl(), dec!(5000));
        assert!(tracker.win_rate() > 0.0);
    }

    #[test]
    fn test_close_long_with_loss() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        let buy_fill = make_fill("BTC-USD", OrderSide::Buy, dec!(50000), dec!(1), dec!(5));
        tracker.process_fill(buy_fill, "strat-1", Market::Crypto);

        let sell_fill = make_fill("BTC-USD", OrderSide::Sell, dec!(45000), dec!(1), dec!(5));
        tracker.process_fill(sell_fill, "strat-1", Market::Crypto);

        assert_eq!(tracker.open_position_count(), 0);
        assert_eq!(tracker.realized_pnl(), dec!(-5000));
        assert_eq!(tracker.win_rate(), 0.0);
    }

    #[test]
    fn test_partial_close() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        let buy_fill = make_fill("BTC-USD", OrderSide::Buy, dec!(50000), dec!(2), dec!(10));
        tracker.process_fill(buy_fill, "strat-1", Market::Crypto);

        let sell_fill = make_fill("BTC-USD", OrderSide::Sell, dec!(55000), dec!(1), dec!(5));
        tracker.process_fill(sell_fill, "strat-1", Market::Crypto);

        assert_eq!(tracker.open_position_count(), 1);
        let snap = tracker.snapshot();
        assert_eq!(snap.positions[0].quantity, dec!(1));
        assert_eq!(tracker.realized_pnl(), dec!(5000));
    }

    #[test]
    fn test_update_price_changes_unrealized() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        let fill = make_fill("ETH-USD", OrderSide::Buy, dec!(1000), dec!(2), dec!(2));
        tracker.process_fill(fill, "strat-1", Market::Crypto);

        tracker.update_price("ETH-USD", dec!(1200));

        let snap = tracker.snapshot();
        assert_eq!(snap.positions[0].current_price, dec!(1200));
        // unrealized = (1200 - 1000) * 2 = 400
        assert_eq!(snap.positions[0].unrealized_pnl, dec!(400));
    }

    #[test]
    fn test_fees_tracked() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        let fill1 = make_fill("BTC-USD", OrderSide::Buy, dec!(50000), dec!(1), dec!(5));
        let fill2 = make_fill("BTC-USD", OrderSide::Buy, dec!(50000), dec!(1), dec!(5));
        tracker.process_fill(fill1, "strat-1", Market::Crypto);
        tracker.process_fill(fill2, "strat-1", Market::Crypto);
        assert_eq!(tracker.total_fees(), dec!(10));
    }
}
