use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tradoshka_common::types::OrderSide;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletPosition {
    pub token_id: String,
    pub market_question: String,
    pub outcome: String,
    pub side: OrderSide,
    pub shares: Decimal,
    pub avg_price: Decimal,
    pub current_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub strategy_id: String,
    pub opened_at: DateTime<Utc>,
}

impl WalletPosition {
    pub fn update_price(&mut self, price: Decimal) {
        self.current_price = price;
        self.unrealized_pnl = match self.side {
            OrderSide::Buy => (price - self.avg_price) * self.shares,
            OrderSide::Sell => (self.avg_price - price) * self.shares,
        };
    }

    pub fn market_value(&self) -> Decimal {
        self.shares * self.current_price
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalletMode {
    Dry,
    Live,
}

pub struct SimulatedWallet {
    balance: Decimal,
    positions: HashMap<String, WalletPosition>,
    mode: WalletMode,
    slippage_bps: Decimal,
    peak_equity: Decimal,
    total_fees: Decimal,
    realized_pnl: Decimal,
}

impl Default for SimulatedWallet {
    fn default() -> Self {
        Self::new(Decimal::ZERO, Decimal::new(5, 0))
    }
}

impl SimulatedWallet {
    pub fn new(initial_balance: Decimal, slippage_bps: Decimal) -> Self {
        Self {
            balance: initial_balance,
            positions: HashMap::new(),
            mode: WalletMode::Dry,
            slippage_bps,
            peak_equity: initial_balance,
            total_fees: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
        }
    }

    pub fn mode(&self) -> WalletMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: WalletMode) {
        self.mode = mode;
    }

    pub fn balance(&self) -> Decimal {
        self.balance
    }

    pub fn equity(&self) -> Decimal {
        let position_value: Decimal = self.positions.values().map(|p| p.market_value()).sum();
        self.balance + position_value
    }

    pub fn unrealized_pnl(&self) -> Decimal {
        self.positions.values().map(|p| p.unrealized_pnl).sum()
    }

    pub fn realized_pnl(&self) -> Decimal {
        self.realized_pnl
    }

    pub fn total_fees(&self) -> Decimal {
        self.total_fees
    }

    pub fn drawdown_pct(&self) -> Decimal {
        if self.peak_equity > Decimal::ZERO {
            (self.peak_equity - self.equity()) / self.peak_equity
        } else {
            Decimal::ZERO
        }
    }

    pub fn positions(&self) -> &HashMap<String, WalletPosition> {
        &self.positions
    }

    pub fn open_position_count(&self) -> usize {
        self.positions.len()
    }

    /// Calculate Polymarket-realistic fee.
    /// fee = base_rate * min(price, 1 - price) * shares
    /// Base rate is approximately 2% (0.02)
    pub fn calculate_fee(price: Decimal, shares: Decimal) -> Decimal {
        let base_rate = Decimal::new(2, 2); // 0.02
        let one = Decimal::ONE;
        let complement = one - price;
        let min_price = price.min(complement);
        base_rate * min_price * shares
    }

    /// Apply slippage to a price.
    fn apply_slippage(&self, price: Decimal, side: OrderSide) -> Decimal {
        let slippage = self.slippage_bps / Decimal::new(10000, 0);
        match side {
            OrderSide::Buy => price * (Decimal::ONE + slippage),
            OrderSide::Sell => price * (Decimal::ONE - slippage),
        }
    }

    /// Execute a simulated buy.
    /// Returns (fill_price, fee, shares) or None if insufficient balance.
    pub fn buy(
        &mut self,
        token_id: &str,
        market_question: &str,
        outcome: &str,
        market_price: Decimal,
        shares: Decimal,
        strategy_id: &str,
    ) -> Option<(Decimal, Decimal, Decimal)> {
        let fill_price = self.apply_slippage(market_price, OrderSide::Buy);
        let cost = fill_price * shares;
        let fee = Self::calculate_fee(fill_price, shares);
        let total_cost = cost + fee;

        if total_cost > self.balance {
            return None;
        }

        self.balance -= total_cost;
        self.total_fees += fee;

        let key = format!("{}:{}", token_id, strategy_id);

        if let Some(pos) = self.positions.get_mut(&key) {
            // Average into existing position
            let total_shares = pos.shares + shares;
            pos.avg_price = (pos.avg_price * pos.shares + fill_price * shares) / total_shares;
            pos.shares = total_shares;
            pos.update_price(market_price);
        } else {
            self.positions.insert(key, WalletPosition {
                token_id: token_id.into(),
                market_question: market_question.into(),
                outcome: outcome.into(),
                side: OrderSide::Buy,
                shares,
                avg_price: fill_price,
                current_price: market_price,
                unrealized_pnl: Decimal::ZERO,
                strategy_id: strategy_id.into(),
                opened_at: Utc::now(),
            });
        }

        self.update_peak();
        Some((fill_price, fee, shares))
    }

    /// Sell/close a position. Returns (fill_price, fee, pnl) or None if no position.
    pub fn sell(
        &mut self,
        token_id: &str,
        market_price: Decimal,
        shares: Decimal,
        strategy_id: &str,
    ) -> Option<(Decimal, Decimal, Decimal)> {
        let key = format!("{}:{}", token_id, strategy_id);
        let pos = self.positions.get(&key)?;

        let sell_shares = shares.min(pos.shares);
        let fill_price = self.apply_slippage(market_price, OrderSide::Sell);
        let revenue = fill_price * sell_shares;
        let fee = Self::calculate_fee(fill_price, sell_shares);
        let pnl = (fill_price - pos.avg_price) * sell_shares - fee;

        self.balance += revenue - fee;
        self.total_fees += fee;
        self.realized_pnl += pnl;

        // Update or remove position
        let remaining = pos.shares - sell_shares;
        if remaining <= Decimal::ZERO {
            self.positions.remove(&key);
        } else {
            let pos = self.positions.get_mut(&key).unwrap();
            pos.shares = remaining;
            pos.update_price(market_price);
        }

        self.update_peak();
        Some((fill_price, fee, pnl))
    }

    /// Settle a market resolution. Winning positions get $1/share, losers get $0.
    pub fn settle_market(&mut self, token_id: &str, won: bool) -> Decimal {
        let mut total_pnl = Decimal::ZERO;
        let keys_to_remove: Vec<String> = self.positions.keys()
            .filter(|k| k.starts_with(&format!("{}:", token_id)))
            .cloned()
            .collect();

        for key in keys_to_remove {
            if let Some(pos) = self.positions.remove(&key) {
                if won {
                    let payout = pos.shares; // $1 per share
                    let pnl = payout - (pos.avg_price * pos.shares);
                    self.balance += payout;
                    self.realized_pnl += pnl;
                    total_pnl += pnl;
                } else {
                    let pnl = -(pos.avg_price * pos.shares);
                    self.realized_pnl += pnl;
                    total_pnl += pnl;
                }
            }
        }

        self.update_peak();
        total_pnl
    }

    /// Update all position prices from a price map.
    pub fn update_prices(&mut self, prices: &HashMap<String, Decimal>) {
        for (_key, pos) in self.positions.iter_mut() {
            if let Some(price) = prices.get(&pos.token_id) {
                pos.update_price(*price);
            }
        }
        self.update_peak();
    }

    fn update_peak(&mut self) {
        let eq = self.equity();
        if eq > self.peak_equity {
            self.peak_equity = eq;
        }
    }

    pub fn peak_equity(&self) -> Decimal {
        self.peak_equity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_new_wallet() {
        let w = SimulatedWallet::new(dec!(100), dec!(5));
        assert_eq!(w.balance(), dec!(100));
        assert_eq!(w.equity(), dec!(100));
        assert_eq!(w.mode(), WalletMode::Dry);
        assert_eq!(w.open_position_count(), 0);
    }

    #[test]
    fn test_calculate_fee() {
        // fee = 0.02 * min(0.50, 0.50) * 100 = 1.00
        let fee = SimulatedWallet::calculate_fee(dec!(0.50), dec!(100));
        assert_eq!(fee, dec!(1.00));

        // fee = 0.02 * min(0.80, 0.20) * 100 = 0.40
        let fee = SimulatedWallet::calculate_fee(dec!(0.80), dec!(100));
        assert_eq!(fee, dec!(0.40));
    }

    #[test]
    fn test_buy_reduces_balance() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0)); // no slippage
        let result = w.buy("tok1", "Will X?", "Yes", dec!(0.50), dec!(10), "ai");
        assert!(result.is_some());
        let (price, fee, shares) = result.unwrap();
        assert_eq!(price, dec!(0.50));
        assert_eq!(shares, dec!(10));
        // cost = 0.50 * 10 = 5.00, fee = 0.02 * 0.50 * 10 = 0.10
        assert_eq!(fee, dec!(0.10));
        assert_eq!(w.balance(), dec!(100) - dec!(5.00) - dec!(0.10));
        assert_eq!(w.open_position_count(), 1);
    }

    #[test]
    fn test_buy_insufficient_balance() {
        let mut w = SimulatedWallet::new(dec!(1), dec!(0));
        let result = w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(100), "ai");
        assert!(result.is_none());
    }

    #[test]
    fn test_sell_closes_position_with_profit() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let result = w.sell("tok1", dec!(0.70), dec!(10), "ai");
        assert!(result.is_some());
        let (_, _, pnl) = result.unwrap();
        assert!(pnl > Decimal::ZERO); // bought at 0.50, sold at 0.70
        assert_eq!(w.open_position_count(), 0);
    }

    #[test]
    fn test_sell_partial() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        w.sell("tok1", dec!(0.60), dec!(5), "ai");
        assert_eq!(w.open_position_count(), 1);
        let pos = w.positions().values().next().unwrap();
        assert_eq!(pos.shares, dec!(5));
    }

    #[test]
    fn test_settle_market_win() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let balance_after_buy = w.balance();
        let pnl = w.settle_market("tok1", true);
        assert!(pnl > Decimal::ZERO);
        assert_eq!(w.open_position_count(), 0);
        // Should have gotten $1 * 10 shares = $10 back
        assert_eq!(w.balance(), balance_after_buy + dec!(10));
    }

    #[test]
    fn test_settle_market_loss() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let pnl = w.settle_market("tok1", false);
        assert!(pnl < Decimal::ZERO);
        assert_eq!(w.open_position_count(), 0);
    }

    #[test]
    fn test_drawdown() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0));
        assert_eq!(w.drawdown_pct(), Decimal::ZERO);
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(40), "ai");
        // Balance dropped by ~20 (cost) + fees, equity should be ~100 still (position value)
        // Simulate a price drop
        let mut prices = HashMap::new();
        prices.insert("tok1".into(), dec!(0.30));
        w.update_prices(&prices);
        assert!(w.drawdown_pct() > Decimal::ZERO);
    }

    #[test]
    fn test_slippage_applied() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(10)); // 10 bps slippage
        let result = w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let (fill_price, _, _) = result.unwrap();
        assert!(fill_price > dec!(0.50)); // Buy slippage makes price worse
    }
}
