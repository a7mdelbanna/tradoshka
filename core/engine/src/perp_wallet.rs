use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tradoshka_common::types::OrderSide;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpPosition {
    pub symbol: String,
    pub side: OrderSide,
    pub size: Decimal,           // Contract quantity
    pub entry_price: Decimal,
    pub mark_price: Decimal,
    pub leverage: u32,
    pub margin: Decimal,         // Locked margin = notional / leverage
    pub unrealized_pnl: Decimal,
    pub liquidation_price: Decimal,
    pub funding_accumulated: Decimal,
    pub strategy_id: String,
    pub timeframe: String,       // "5m", "15m", "1h", "4h"
    pub opened_at: DateTime<Utc>,
}

impl PerpPosition {
    pub fn notional(&self) -> Decimal {
        self.size * self.mark_price
    }

    pub fn update_mark_price(&mut self, price: Decimal) {
        self.mark_price = price;
        let diff = price - self.entry_price;
        self.unrealized_pnl = match self.side {
            OrderSide::Buy => diff * self.size,
            OrderSide::Sell => -diff * self.size,
        };
        // Update liquidation price (simplified)
        let margin_ratio = self.margin / self.notional();
        self.liquidation_price = match self.side {
            OrderSide::Buy => self.entry_price * (Decimal::ONE - margin_ratio + dec!(0.005)),
            OrderSide::Sell => self.entry_price * (Decimal::ONE + margin_ratio - dec!(0.005)),
        };
    }

    pub fn roe_pct(&self) -> Decimal {
        if self.margin > Decimal::ZERO {
            (self.unrealized_pnl / self.margin) * dec!(100)
        } else {
            Decimal::ZERO
        }
    }
}

pub struct PerpWallet {
    balance: Decimal,
    positions: HashMap<String, PerpPosition>,
    default_leverage: u32,
    realized_pnl: Decimal,
    total_fees: Decimal,
    total_funding: Decimal,
    peak_equity: Decimal,
}

impl PerpWallet {
    pub fn new(initial_balance: Decimal, default_leverage: u32) -> Self {
        Self {
            balance: initial_balance,
            positions: HashMap::new(),
            default_leverage,
            realized_pnl: Decimal::ZERO,
            total_fees: Decimal::ZERO,
            total_funding: Decimal::ZERO,
            peak_equity: initial_balance,
        }
    }

    pub fn balance(&self) -> Decimal { self.balance }
    pub fn default_leverage(&self) -> u32 { self.default_leverage }
    pub fn realized_pnl(&self) -> Decimal { self.realized_pnl }
    pub fn total_fees(&self) -> Decimal { self.total_fees }
    pub fn total_funding(&self) -> Decimal { self.total_funding }
    pub fn positions(&self) -> &HashMap<String, PerpPosition> { &self.positions }
    pub fn open_position_count(&self) -> usize { self.positions.len() }

    pub fn equity(&self) -> Decimal {
        self.balance + self.unrealized_pnl()
    }

    pub fn unrealized_pnl(&self) -> Decimal {
        self.positions.values().map(|p| p.unrealized_pnl).sum()
    }

    pub fn used_margin(&self) -> Decimal {
        self.positions.values().map(|p| p.margin).sum()
    }

    pub fn available_margin(&self) -> Decimal {
        self.balance - self.used_margin()
    }

    pub fn drawdown_pct(&self) -> Decimal {
        if self.peak_equity > Decimal::ZERO {
            (self.peak_equity - self.equity()) / self.peak_equity
        } else {
            Decimal::ZERO
        }
    }

    /// Open a long or short perpetual position.
    pub fn open_position(
        &mut self,
        symbol: &str,
        side: OrderSide,
        price: Decimal,
        size: Decimal,
        leverage: Option<u32>,
        strategy_id: &str,
        timeframe: &str,
    ) -> Option<PerpPosition> {
        let lev = leverage.unwrap_or(self.default_leverage);
        let notional = size * price;
        let margin = notional / Decimal::from(lev);
        let fee = notional * dec!(0.0004); // 0.04% taker fee

        if margin + fee > self.available_margin() {
            return None;
        }

        self.balance -= margin + fee;
        self.total_fees += fee;

        let mut pos = PerpPosition {
            symbol: symbol.into(),
            side,
            size,
            entry_price: price,
            mark_price: price,
            leverage: lev,
            margin,
            unrealized_pnl: Decimal::ZERO,
            liquidation_price: Decimal::ZERO,
            funding_accumulated: Decimal::ZERO,
            strategy_id: strategy_id.into(),
            timeframe: timeframe.into(),
            opened_at: Utc::now(),
        };
        pos.update_mark_price(price);

        let key = format!("{}:{}:{}", symbol, strategy_id, timeframe);
        self.positions.insert(key.clone(), pos.clone());
        self.update_peak();
        Some(pos)
    }

    /// Close a perpetual position.
    pub fn close_position(&mut self, symbol: &str, strategy_id: &str, timeframe: &str, price: Decimal) -> Option<Decimal> {
        let key = format!("{}:{}:{}", symbol, strategy_id, timeframe);
        let pos = self.positions.remove(&key)?;

        let pnl = match pos.side {
            OrderSide::Buy => (price - pos.entry_price) * pos.size,
            OrderSide::Sell => (pos.entry_price - price) * pos.size,
        };
        let fee = pos.size * price * dec!(0.0004);

        self.balance += pos.margin + pnl - fee;
        self.realized_pnl += pnl;
        self.total_fees += fee;
        self.update_peak();
        Some(pnl)
    }

    /// Simulate funding payment (called every 8h).
    pub fn apply_funding(&mut self, symbol: &str, funding_rate: Decimal) {
        for pos in self.positions.values_mut() {
            if pos.symbol == symbol {
                let funding = pos.notional() * funding_rate;
                let payment = match pos.side {
                    OrderSide::Buy => -funding,  // Longs pay when rate positive
                    OrderSide::Sell => funding,   // Shorts receive when rate positive
                };
                pos.funding_accumulated += payment;
                self.total_funding += payment;
                self.balance += payment;
            }
        }
    }

    pub fn update_prices(&mut self, prices: &HashMap<String, Decimal>) {
        for pos in self.positions.values_mut() {
            if let Some(price) = prices.get(&pos.symbol) {
                pos.update_mark_price(*price);
            }
        }
        self.update_peak();
    }

    fn update_peak(&mut self) {
        let eq = self.equity();
        if eq > self.peak_equity { self.peak_equity = eq; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_long() {
        let mut w = PerpWallet::new(dec!(100), 10);
        let pos = w.open_position("BTCUSDT", OrderSide::Buy, dec!(50000), dec!(0.01), None, "momentum", "5m");
        assert!(pos.is_some());
        let p = pos.unwrap();
        assert_eq!(p.leverage, 10);
        assert_eq!(p.margin, dec!(50)); // 500 notional / 10 leverage
        assert!(w.balance() < dec!(100)); // margin + fee deducted
    }

    #[test]
    fn test_open_short() {
        let mut w = PerpWallet::new(dec!(100), 10);
        let pos = w.open_position("ETHUSDT", OrderSide::Sell, dec!(2000), dec!(0.1), None, "scalp", "1m");
        assert!(pos.is_some());
    }

    #[test]
    fn test_close_with_profit() {
        let mut w = PerpWallet::new(dec!(100), 10);
        w.open_position("BTCUSDT", OrderSide::Buy, dec!(50000), dec!(0.01), None, "m", "5m");
        let pnl = w.close_position("BTCUSDT", "m", "5m", dec!(51000));
        assert!(pnl.is_some());
        assert!(pnl.unwrap() > Decimal::ZERO); // 1000 * 0.01 = $10 profit
    }

    #[test]
    fn test_close_with_loss() {
        let mut w = PerpWallet::new(dec!(100), 10);
        w.open_position("BTCUSDT", OrderSide::Buy, dec!(50000), dec!(0.01), None, "m", "5m");
        let pnl = w.close_position("BTCUSDT", "m", "5m", dec!(49000));
        assert!(pnl.unwrap() < Decimal::ZERO);
    }

    #[test]
    fn test_insufficient_margin() {
        let mut w = PerpWallet::new(dec!(10), 10);
        // Trying to open $50000 * 0.1 = $5000 notional, margin = $500 > $10
        let pos = w.open_position("BTCUSDT", OrderSide::Buy, dec!(50000), dec!(0.1), None, "m", "5m");
        assert!(pos.is_none());
    }

    #[test]
    fn test_funding_payment() {
        let mut w = PerpWallet::new(dec!(100), 10);
        w.open_position("BTCUSDT", OrderSide::Sell, dec!(50000), dec!(0.01), None, "arb", "1h");
        let bal_before = w.balance();
        w.apply_funding("BTCUSDT", dec!(0.0001)); // Positive rate = shorts receive
        assert!(w.balance() > bal_before);
    }

    #[test]
    fn test_roe_calculation() {
        let mut w = PerpWallet::new(dec!(1000), 10);
        w.open_position("BTCUSDT", OrderSide::Buy, dec!(50000), dec!(0.01), None, "m", "5m");
        let key = "BTCUSDT:m:5m";
        if let Some(pos) = w.positions.get_mut(key) {
            pos.update_mark_price(dec!(55000));
            // PnL = 5000 * 0.01 = $50, margin = $50, ROE = 100%
            assert!(pos.roe_pct() > dec!(90));
        }
    }
}
