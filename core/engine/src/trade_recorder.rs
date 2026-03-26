use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tradoshka_common::types::{OrderSide, Market};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub market: Market,
    pub symbol: String,
    pub market_question: String,
    pub direction: String,
    pub side: OrderSide,
    pub shares: Decimal,
    pub price: Decimal,
    pub fee: Decimal,
    pub strategy_id: String,
    pub signal_strength: f64,
    pub edge_vs_market: f64,
    pub pnl: Option<Decimal>,
    pub is_closed: bool,
    pub thesis_reasoning: String,
    pub stop_loss: Decimal,
    pub trailing_stop: Decimal,
    pub take_profit: Decimal,
    pub time_stop_hours: u32,
    pub thesis_invalidation: String,
    pub risk_amount: Decimal,
    pub reward_risk_ratio: f64,
    pub strategy_tier: String,
    pub close_reason: Option<String>,
}

pub struct TradeRecorder {
    trades: Vec<TradeRecord>,
    first_trade_at: Option<DateTime<Utc>>,
}

impl Default for TradeRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl TradeRecorder {
    pub fn new() -> Self {
        Self {
            trades: Vec::new(),
            first_trade_at: None,
        }
    }

    pub fn record(&mut self, trade: TradeRecord) {
        if self.first_trade_at.is_none() {
            self.first_trade_at = Some(trade.timestamp);
        }
        self.trades.push(trade);
    }

    pub fn close_trade(&mut self, symbol: &str, strategy_id: &str, pnl: Decimal) {
        for trade in self.trades.iter_mut().rev() {
            if trade.symbol == symbol && trade.strategy_id == strategy_id && !trade.is_closed {
                trade.pnl = Some(pnl);
                trade.is_closed = true;
                break;
            }
        }
    }

    pub fn all_trades(&self) -> &[TradeRecord] {
        &self.trades
    }

    pub fn closed_trades(&self) -> Vec<&TradeRecord> {
        self.trades.iter().filter(|t| t.is_closed).collect()
    }

    pub fn open_trades(&self) -> Vec<&TradeRecord> {
        self.trades.iter().filter(|t| !t.is_closed).collect()
    }

    pub fn recent_trades(&self, limit: usize) -> Vec<&TradeRecord> {
        self.trades.iter().rev().take(limit).collect()
    }

    pub fn trades_by_strategy(&self, strategy_id: &str) -> Vec<&TradeRecord> {
        self.trades.iter().filter(|t| t.strategy_id == strategy_id).collect()
    }

    pub fn first_trade_at(&self) -> Option<DateTime<Utc>> {
        self.first_trade_at
    }

    pub fn total_trade_count(&self) -> usize {
        self.trades.len()
    }

    pub fn closed_trade_count(&self) -> usize {
        self.trades.iter().filter(|t| t.is_closed).count()
    }

    pub fn winning_trade_count(&self) -> usize {
        self.trades.iter()
            .filter(|t| t.is_closed && t.pnl.map_or(false, |p| p > Decimal::ZERO))
            .count()
    }

    pub fn gross_profit(&self) -> Decimal {
        self.trades.iter()
            .filter_map(|t| t.pnl)
            .filter(|p| *p > Decimal::ZERO)
            .sum()
    }

    pub fn gross_loss(&self) -> Decimal {
        self.trades.iter()
            .filter_map(|t| t.pnl)
            .filter(|p| *p < Decimal::ZERO)
            .sum::<Decimal>()
            .abs()
    }

    pub fn daily_pnl(&self) -> Vec<(String, Decimal)> {
        use std::collections::BTreeMap;
        let mut daily: BTreeMap<String, Decimal> = BTreeMap::new();
        for trade in &self.trades {
            if let Some(pnl) = trade.pnl {
                let date = trade.timestamp.format("%Y-%m-%d").to_string();
                *daily.entry(date).or_insert(Decimal::ZERO) += pnl;
            }
        }
        daily.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_trade(symbol: &str, strategy: &str, pnl: Option<Decimal>, closed: bool) -> TradeRecord {
        TradeRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            market: Market::Polymarket,
            symbol: symbol.into(),
            market_question: "Test?".into(),
            direction: "YES".into(),
            side: OrderSide::Buy,
            shares: dec!(10),
            price: dec!(0.50),
            fee: dec!(0.05),
            strategy_id: strategy.into(),
            signal_strength: 0.8,
            edge_vs_market: 0.05,
            pnl,
            is_closed: closed,
            thesis_reasoning: String::new(),
            stop_loss: Decimal::ZERO,
            trailing_stop: Decimal::ZERO,
            take_profit: Decimal::ZERO,
            time_stop_hours: 0,
            thesis_invalidation: String::new(),
            risk_amount: Decimal::ZERO,
            reward_risk_ratio: 0.0,
            strategy_tier: "Unproven".into(),
            close_reason: None,
        }
    }

    #[test]
    fn test_record_and_retrieve() {
        let mut recorder = TradeRecorder::new();
        recorder.record(make_trade("tok1", "ai", None, false));
        recorder.record(make_trade("tok2", "copy", Some(dec!(5)), true));
        assert_eq!(recorder.total_trade_count(), 2);
        assert_eq!(recorder.closed_trade_count(), 1);
        assert_eq!(recorder.open_trades().len(), 1);
    }

    #[test]
    fn test_close_trade() {
        let mut recorder = TradeRecorder::new();
        recorder.record(make_trade("tok1", "ai", None, false));
        recorder.close_trade("tok1", "ai", dec!(12.50));
        assert_eq!(recorder.closed_trade_count(), 1);
        assert_eq!(recorder.winning_trade_count(), 1);
    }

    #[test]
    fn test_gross_profit_loss() {
        let mut recorder = TradeRecorder::new();
        recorder.record(make_trade("t1", "ai", Some(dec!(10)), true));
        recorder.record(make_trade("t2", "ai", Some(dec!(-4)), true));
        recorder.record(make_trade("t3", "ai", Some(dec!(6)), true));
        assert_eq!(recorder.gross_profit(), dec!(16));
        assert_eq!(recorder.gross_loss(), dec!(4));
    }

    #[test]
    fn test_win_rate() {
        let mut recorder = TradeRecorder::new();
        recorder.record(make_trade("t1", "ai", Some(dec!(10)), true));
        recorder.record(make_trade("t2", "ai", Some(dec!(-4)), true));
        recorder.record(make_trade("t3", "ai", Some(dec!(6)), true));
        let closed = recorder.closed_trade_count();
        let wins = recorder.winning_trade_count();
        let win_rate = wins as f64 / closed as f64;
        assert!((win_rate - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_trades_by_strategy() {
        let mut recorder = TradeRecorder::new();
        recorder.record(make_trade("t1", "ai", None, false));
        recorder.record(make_trade("t2", "copy", None, false));
        recorder.record(make_trade("t3", "ai", None, false));
        assert_eq!(recorder.trades_by_strategy("ai").len(), 2);
        assert_eq!(recorder.trades_by_strategy("copy").len(), 1);
    }

    #[test]
    fn test_first_trade_at() {
        let mut recorder = TradeRecorder::new();
        assert!(recorder.first_trade_at().is_none());
        recorder.record(make_trade("t1", "ai", None, false));
        assert!(recorder.first_trade_at().is_some());
    }
}
