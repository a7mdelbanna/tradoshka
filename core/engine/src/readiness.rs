use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use crate::trade_recorder::TradeRecorder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessCriterion {
    pub name: String,
    pub threshold: String,
    pub current_value: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    pub criteria: Vec<ReadinessCriterion>,
    pub passed_count: usize,
    pub total_count: usize,
    pub is_ready: bool,
}

pub struct ReadinessScorer {
    pub min_days: i64,
    pub min_trades: usize,
    pub min_win_rate: f64,
    pub min_sharpe: f64,
    pub max_drawdown_pct: f64,
    pub min_profit_factor: f64,
}

impl Default for ReadinessScorer {
    fn default() -> Self {
        Self {
            min_days: 14,
            min_trades: 50,
            min_win_rate: 0.70,
            min_sharpe: 1.0,
            max_drawdown_pct: 0.15,
            min_profit_factor: 1.5,
        }
    }
}

impl ReadinessScorer {
    pub fn evaluate(
        &self,
        recorder: &TradeRecorder,
        current_drawdown_pct: f64,
        daily_returns: &[f64],
    ) -> ReadinessReport {
        let closed = recorder.closed_trade_count();
        let wins = recorder.winning_trade_count();
        let win_rate = if closed > 0 { wins as f64 / closed as f64 } else { 0.0 };

        let days_active = recorder.first_trade_at()
            .map(|first| (Utc::now() - first).num_days())
            .unwrap_or(0);

        let gross_profit = recorder.gross_profit();
        let gross_loss = recorder.gross_loss();
        let profit_factor = if gross_loss > Decimal::ZERO {
            (gross_profit / gross_loss).to_f64().unwrap_or(0.0)
        } else if gross_profit > Decimal::ZERO {
            999.0
        } else {
            0.0
        };

        let sharpe = Self::calculate_sharpe(daily_returns);

        let criteria = vec![
            ReadinessCriterion {
                name: "Days active".into(),
                threshold: format!(">= {}", self.min_days),
                current_value: format!("{}", days_active),
                passed: days_active >= self.min_days,
            },
            ReadinessCriterion {
                name: "Total trades".into(),
                threshold: format!(">= {}", self.min_trades),
                current_value: format!("{}", closed),
                passed: closed >= self.min_trades,
            },
            ReadinessCriterion {
                name: "Win rate".into(),
                threshold: format!(">= {:.0}%", self.min_win_rate * 100.0),
                current_value: format!("{:.1}%", win_rate * 100.0),
                passed: win_rate >= self.min_win_rate,
            },
            ReadinessCriterion {
                name: "Sharpe ratio".into(),
                threshold: format!(">= {:.1}", self.min_sharpe),
                current_value: format!("{:.2}", sharpe),
                passed: sharpe >= self.min_sharpe,
            },
            ReadinessCriterion {
                name: "Max drawdown".into(),
                threshold: format!("<= {:.0}%", self.max_drawdown_pct * 100.0),
                current_value: format!("{:.1}%", current_drawdown_pct * 100.0),
                passed: current_drawdown_pct <= self.max_drawdown_pct,
            },
            ReadinessCriterion {
                name: "Profit factor".into(),
                threshold: format!(">= {:.1}", self.min_profit_factor),
                current_value: format!("{:.2}", profit_factor),
                passed: profit_factor >= self.min_profit_factor,
            },
        ];

        let passed_count = criteria.iter().filter(|c| c.passed).count();
        let total_count = criteria.len();

        ReadinessReport {
            criteria,
            passed_count,
            total_count,
            is_ready: passed_count == total_count,
        }
    }

    pub fn calculate_sharpe(daily_returns: &[f64]) -> f64 {
        if daily_returns.len() < 2 {
            return 0.0;
        }
        let n = daily_returns.len() as f64;
        let mean = daily_returns.iter().sum::<f64>() / n;
        let variance = daily_returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();
        if std_dev == 0.0 {
            return 0.0;
        }
        // Annualized: mean/std_dev * sqrt(365)
        (mean / std_dev) * (365.0_f64).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade_recorder::{TradeRecorder, TradeRecord};
    use tradoshka_common::types::{OrderSide, Market};
    use rust_decimal_macros::dec;

    fn make_closed_trade(pnl: Decimal) -> TradeRecord {
        TradeRecord {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now() - chrono::Duration::days(20),
            market: Market::Polymarket,
            symbol: "tok".into(),
            market_question: "Q?".into(),
            direction: "YES".into(),
            side: OrderSide::Buy,
            shares: dec!(10),
            price: dec!(0.50),
            fee: dec!(0.05),
            strategy_id: "ai".into(),
            signal_strength: 0.8,
            edge_vs_market: 0.05,
            pnl: Some(pnl),
            is_closed: true,
        }
    }

    #[test]
    fn test_not_ready_with_no_trades() {
        let scorer = ReadinessScorer::default();
        let recorder = TradeRecorder::new();
        let report = scorer.evaluate(&recorder, 0.0, &[]);
        assert!(!report.is_ready);
        assert_eq!(report.passed_count, 1); // only drawdown passes vacuously (0.0 <= 0.15)
    }

    #[test]
    fn test_ready_when_all_criteria_met() {
        let scorer = ReadinessScorer {
            min_days: 0, // Override for test
            min_trades: 3,
            min_win_rate: 0.60,
            min_sharpe: 0.5,
            max_drawdown_pct: 0.20,
            min_profit_factor: 1.0,
        };
        let mut recorder = TradeRecorder::new();
        recorder.record(make_closed_trade(dec!(10)));
        recorder.record(make_closed_trade(dec!(8)));
        recorder.record(make_closed_trade(dec!(-3)));

        let daily_returns = vec![0.02, 0.03, -0.01, 0.04, 0.01, 0.02, 0.03];
        let report = scorer.evaluate(&recorder, 0.05, &daily_returns);
        assert!(report.is_ready);
        assert_eq!(report.passed_count, 6);
    }

    #[test]
    fn test_fails_on_low_win_rate() {
        let scorer = ReadinessScorer {
            min_days: 0,
            min_trades: 2,
            min_win_rate: 0.70,
            min_sharpe: 0.0,
            max_drawdown_pct: 1.0,
            min_profit_factor: 0.0,
        };
        let mut recorder = TradeRecorder::new();
        recorder.record(make_closed_trade(dec!(10)));
        recorder.record(make_closed_trade(dec!(-5)));
        // Win rate = 50%, below 70%
        let report = scorer.evaluate(&recorder, 0.0, &[0.01, 0.02]);
        let wr_criterion = report.criteria.iter().find(|c| c.name == "Win rate").unwrap();
        assert!(!wr_criterion.passed);
    }

    #[test]
    fn test_sharpe_calculation() {
        let returns = vec![0.01, 0.02, 0.015, 0.01, 0.02, 0.005, 0.01];
        let sharpe = ReadinessScorer::calculate_sharpe(&returns);
        assert!(sharpe > 0.0); // All positive returns → positive Sharpe
    }
}
