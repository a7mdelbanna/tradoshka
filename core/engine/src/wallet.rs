use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tradoshka_common::types::OrderSide;

/// Reason for exiting a position — drives asymmetric slippage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitType {
    /// Panic exit; market impact is highest.
    StopLoss,
    /// Passive limit-like exit; market impact is lowest.
    TakeProfit,
    /// Time-based expiry; moderate market impact.
    TimeStop,
    /// Human-initiated exit; moderate market impact.
    Manual,
}

impl ExitType {
    /// Slippage in basis points that should be subtracted from the fill price on exit.
    pub fn slippage_bps(self) -> Decimal {
        match self {
            ExitType::StopLoss   => Decimal::new(250, 0),
            ExitType::TakeProfit => Decimal::new(75,  0),
            ExitType::TimeStop   => Decimal::new(150, 0),
            ExitType::Manual     => Decimal::new(100, 0),
        }
    }
}

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
    fee_rate: Decimal,
    /// Sum of all entry-side slippage costs (buy fills above market price).
    entry_slippage_total: Decimal,
    /// Sum of all exit-side slippage costs (sell fills below market price).
    exit_slippage_total: Decimal,
    /// Sum of all on-chain / network gas fees paid.
    gas_fees_total: Decimal,
}

impl Default for SimulatedWallet {
    fn default() -> Self {
        use rust_decimal_macros::dec;
        Self::new(Decimal::ZERO, dec!(5), dec!(0.001))
    }
}

impl SimulatedWallet {
    pub fn new(initial_balance: Decimal, slippage_bps: Decimal, fee_rate: Decimal) -> Self {
        Self {
            balance: initial_balance,
            positions: HashMap::new(),
            mode: WalletMode::Dry,
            slippage_bps,
            peak_equity: initial_balance,
            total_fees: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            fee_rate,
            entry_slippage_total: Decimal::ZERO,
            exit_slippage_total: Decimal::ZERO,
            gas_fees_total: Decimal::ZERO,
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

    // ── Granular cost accessors ────────────────────────────────────────────────

    pub fn entry_slippage_total(&self) -> Decimal {
        self.entry_slippage_total
    }

    pub fn exit_slippage_total(&self) -> Decimal {
        self.exit_slippage_total
    }

    pub fn gas_fees_total(&self) -> Decimal {
        self.gas_fees_total
    }

    /// Alias for `total_fees()` — trading/exchange fees charged on fills.
    pub fn trading_fees_total(&self) -> Decimal {
        self.total_fees
    }

    /// Sum of all four cost buckets: entry slippage + exit slippage + trading fees + gas fees.
    pub fn total_costs(&self) -> Decimal {
        self.entry_slippage_total + self.exit_slippage_total + self.total_fees + self.gas_fees_total
    }

    /// Deduct a gas fee from the balance and record it in `gas_fees_total`.
    pub fn add_gas_fee(&mut self, amount: Decimal) {
        let amount = amount.abs();
        self.balance -= amount;
        self.gas_fees_total += amount;
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

    /// Calculate market-aware fee.
    /// fee = fee_rate * price * shares
    /// fee_rate is set per market:
    ///   - Crypto spot/perps: 0.001 (0.1% Binance taker fee)
    ///   - Polymarket:        0.002 (0.2% simplified)
    ///   - Meme coins:        0.003 (0.3% = Raydium fee + gas proxy)
    pub fn calculate_fee(&self, price: Decimal, shares: Decimal) -> Decimal {
        (self.fee_rate * price * shares).abs()
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
        let slippage_cost = fill_price - market_price; // positive = price moved against us
        let cost = fill_price * shares;
        let fee = self.calculate_fee(fill_price, shares);
        let total_cost = cost + fee;

        if total_cost > self.balance {
            return None;
        }

        self.balance -= total_cost;
        self.total_fees += fee;
        self.entry_slippage_total += (slippage_cost * shares).abs();

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
        let fee = self.calculate_fee(fill_price, sell_shares);
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

    /// Sell/close a position with an explicit exit type for asymmetric slippage.
    /// Returns (fill_price, fee, pnl) or None if no position.
    /// The exit-type slippage is applied on top of (replacing) the generic sell slippage.
    pub fn sell_with_exit_type(
        &mut self,
        token_id: &str,
        market_price: Decimal,
        shares: Decimal,
        strategy_id: &str,
        exit_type: ExitType,
    ) -> Option<(Decimal, Decimal, Decimal)> {
        let key = format!("{}:{}", token_id, strategy_id);
        let pos = self.positions.get(&key)?;

        let sell_shares = shares.min(pos.shares);

        // Apply exit-type-specific slippage (deducted from market price)
        let exit_slippage = exit_type.slippage_bps() / Decimal::new(10000, 0);
        let fill_price = market_price * (Decimal::ONE - exit_slippage);

        let slippage_cost = (market_price - fill_price) * sell_shares; // always positive

        let revenue = fill_price * sell_shares;
        let fee = self.calculate_fee(fill_price, sell_shares);
        let pnl = (fill_price - pos.avg_price) * sell_shares - fee;

        self.balance += revenue - fee;
        self.total_fees += fee;
        self.realized_pnl += pnl;
        self.exit_slippage_total += slippage_cost.abs();

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
        let w = SimulatedWallet::new(dec!(100), dec!(5), dec!(0.001));
        assert_eq!(w.balance(), dec!(100));
        assert_eq!(w.equity(), dec!(100));
        assert_eq!(w.mode(), WalletMode::Dry);
        assert_eq!(w.open_position_count(), 0);
    }

    #[test]
    fn test_calculate_fee() {
        // fee_rate=0.002 (Polymarket), price=0.50, shares=100
        // fee = 0.002 * 0.50 * 100 = 0.10
        let w = SimulatedWallet::new(dec!(1000), dec!(0), dec!(0.002));
        let fee = w.calculate_fee(dec!(0.50), dec!(100));
        assert_eq!(fee, dec!(0.10));

        // fee_rate=0.001 (crypto), price=600, shares=0.033
        // fee = 0.001 * 600 * 0.033 = 0.0198
        let w2 = SimulatedWallet::new(dec!(1000), dec!(0), dec!(0.001));
        let fee2 = w2.calculate_fee(dec!(600), dec!(0.033));
        assert!(fee2 > dec!(0)); // must be positive
    }

    #[test]
    fn test_buy_reduces_balance() {
        // fee_rate=0.002, price=0.50, shares=10
        // cost = 0.50 * 10 = 5.00, fee = 0.002 * 0.50 * 10 = 0.01
        let mut w = SimulatedWallet::new(dec!(100), dec!(0), dec!(0.002)); // no slippage
        let result = w.buy("tok1", "Will X?", "Yes", dec!(0.50), dec!(10), "ai");
        assert!(result.is_some());
        let (price, fee, shares) = result.unwrap();
        assert_eq!(price, dec!(0.50));
        assert_eq!(shares, dec!(10));
        assert_eq!(fee, dec!(0.01));
        assert_eq!(w.balance(), dec!(100) - dec!(5.00) - dec!(0.01));
        assert_eq!(w.open_position_count(), 1);
    }

    #[test]
    fn test_buy_insufficient_balance() {
        let mut w = SimulatedWallet::new(dec!(1), dec!(0), dec!(0.001));
        let result = w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(100), "ai");
        assert!(result.is_none());
    }

    #[test]
    fn test_sell_closes_position_with_profit() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0), dec!(0.001));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let result = w.sell("tok1", dec!(0.70), dec!(10), "ai");
        assert!(result.is_some());
        let (_, _, pnl) = result.unwrap();
        assert!(pnl > Decimal::ZERO); // bought at 0.50, sold at 0.70
        assert_eq!(w.open_position_count(), 0);
    }

    #[test]
    fn test_sell_partial() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0), dec!(0.001));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        w.sell("tok1", dec!(0.60), dec!(5), "ai");
        assert_eq!(w.open_position_count(), 1);
        let pos = w.positions().values().next().unwrap();
        assert_eq!(pos.shares, dec!(5));
    }

    #[test]
    fn test_settle_market_win() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0), dec!(0.001));
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
        let mut w = SimulatedWallet::new(dec!(100), dec!(0), dec!(0.001));
        w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let pnl = w.settle_market("tok1", false);
        assert!(pnl < Decimal::ZERO);
        assert_eq!(w.open_position_count(), 0);
    }

    #[test]
    fn test_drawdown() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(0), dec!(0.001));
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
        let mut w = SimulatedWallet::new(dec!(100), dec!(10), dec!(0.001)); // 10 bps slippage
        let result = w.buy("tok1", "Q?", "Yes", dec!(0.50), dec!(10), "ai");
        let (fill_price, _, _) = result.unwrap();
        assert!(fill_price > dec!(0.50)); // Buy slippage makes price worse
    }

    #[test]
    fn test_fee_always_positive_for_high_price() {
        // Regression: old Polymarket formula gave NEGATIVE fee for price > 1
        // e.g. price=$600 → complement=-599 → min(600,-599)=-599 → NEGATIVE fee
        // New formula: fee_rate * price * shares, always positive
        let mut w = SimulatedWallet::new(dec!(10000), dec!(0), dec!(0.001));
        let result = w.buy("btc1", "BTC price", "Up", dec!(600), dec!(1), "cs-strat");
        assert!(result.is_some());
        let (_, fee, _) = result.unwrap();
        assert!(fee > dec!(0), "Fee must be positive for crypto high-price trades");
        // 0.001 * 600 * 1 = 0.6
        assert_eq!(fee, dec!(0.6));
    }

    #[test]
    fn test_cost_tracking_separate_buckets() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(100), dec!(0.003));
        w.buy("tok1", "Test", "Long", dec!(0.001), dec!(10000), "strat1");
        assert!(w.entry_slippage_total() > Decimal::ZERO, "entry slippage should be tracked");
        assert!(w.trading_fees_total() > Decimal::ZERO, "trading fees should be tracked");
        assert_eq!(w.exit_slippage_total(), Decimal::ZERO, "no exit slippage yet");
        assert_eq!(w.gas_fees_total(), Decimal::ZERO, "gas fees not yet implemented here");
    }

    #[test]
    fn test_exit_slippage_applied_on_sell() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(100), dec!(0.003));
        w.buy("tok1", "Test", "Long", dec!(1.00), dec!(10), "strat1");
        let result = w.sell_with_exit_type("tok1", dec!(1.00), dec!(10), "strat1", ExitType::StopLoss);
        assert!(result.is_some());
        let (fill_price, _, _) = result.unwrap();
        assert!(fill_price < dec!(1.00), "stop loss exit should have negative slippage");
        assert!(w.exit_slippage_total() > Decimal::ZERO, "exit slippage should be tracked");
    }

    #[test]
    fn test_exit_slippage_asymmetric() {
        let mut w1 = SimulatedWallet::new(dec!(1000), dec!(0), dec!(0.003));
        w1.buy("t1", "Q", "Long", dec!(100.0), dec!(1), "s1");
        let sl_result = w1.sell_with_exit_type("t1", dec!(100.0), dec!(1), "s1", ExitType::StopLoss);
        let sl_fill = sl_result.unwrap().0;

        let mut w2 = SimulatedWallet::new(dec!(1000), dec!(0), dec!(0.003));
        w2.buy("t1", "Q", "Long", dec!(100.0), dec!(1), "s2");
        let tp_result = w2.sell_with_exit_type("t1", dec!(100.0), dec!(1), "s2", ExitType::TakeProfit);
        let tp_fill = tp_result.unwrap().0;

        assert!(sl_fill < tp_fill, "SL fill {} should be worse than TP fill {}", sl_fill, tp_fill);
    }

    #[test]
    fn test_gas_fee_tracking() {
        let mut w = SimulatedWallet::new(dec!(100), dec!(100), dec!(0.003));
        w.add_gas_fee(dec!(0.151));
        w.add_gas_fee(dec!(0.151));
        assert_eq!(w.gas_fees_total(), dec!(0.302));
        assert!(w.balance() < dec!(100));
    }
}
