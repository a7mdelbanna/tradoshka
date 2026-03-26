use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use tradoshka_common::types::OrderSide;
use crate::atr_stops::StopCalculator;
use crate::wallet::SimulatedWallet;
use crate::trade_recorder::TradeRecorder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionCheck {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub current_price: Decimal,
    pub stop_loss: Decimal,
    pub trailing_stop: Decimal,
    pub take_profit: Decimal,
    pub action: PositionAction,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PositionAction {
    Hold,
    CloseStopLoss,
    CloseTrailingStop,
    CloseTakeProfit,
    CloseTimeStop,
    CloseThesisInvalid,
}

pub struct PositionMonitor;

impl PositionMonitor {
    /// Check a single position against all exit conditions.
    /// Returns the action to take and the reason.
    pub fn check_position(
        current_price: Decimal,
        entry_price: Decimal,
        side: OrderSide,
        stop_loss: Decimal,
        trailing_stop: Decimal,
        take_profit: Decimal,
        time_stop_hours: u32,
        opened_at: DateTime<Utc>,
        unrealized_pnl_pct: f64,
    ) -> (PositionAction, String) {
        // 1. Check hard stop loss
        if StopCalculator::is_stopped_out(current_price, stop_loss, side) {
            return (
                PositionAction::CloseStopLoss,
                format!("Hard stop hit at {}. Entry was {}.", current_price, entry_price),
            );
        }

        // 2. Check trailing stop
        if StopCalculator::is_stopped_out(current_price, trailing_stop, side) {
            return (
                PositionAction::CloseTrailingStop,
                format!("Trailing stop hit at {}. Profits locked.", current_price),
            );
        }

        // 3. Check take profit
        if StopCalculator::is_take_profit_hit(current_price, take_profit, side) {
            return (
                PositionAction::CloseTakeProfit,
                format!("Take profit reached at {}. Target hit.", current_price),
            );
        }

        // 4. Check time stop
        let hours_held = (Utc::now() - opened_at).num_hours() as u32;
        if time_stop_hours > 0 && hours_held >= time_stop_hours && unrealized_pnl_pct.abs() < 2.0 {
            return (
                PositionAction::CloseTimeStop,
                format!("Time stop: held {}h with only {:.1}% move. Capital freed.", hours_held, unrealized_pnl_pct),
            );
        }

        (PositionAction::Hold, "Position healthy. All stops clear.".into())
    }

    /// Run monitoring on all positions in a wallet.
    /// Returns list of positions that should be closed with reasons.
    pub fn monitor_all(
        wallet: &SimulatedWallet,
        recorder: &TradeRecorder,
    ) -> Vec<PositionCheck> {
        let mut checks = Vec::new();
        let now = Utc::now();

        for (_key, pos) in wallet.positions() {
            // Find the corresponding trade record to get stop levels
            let trade = recorder.open_trades().into_iter()
                .find(|t| t.symbol == pos.token_id && t.strategy_id == pos.strategy_id);

            let (sl, ts, tp, time_hours) = if let Some(t) = trade {
                (t.stop_loss, t.trailing_stop, t.take_profit, t.time_stop_hours)
            } else {
                continue; // No trade record — skip
            };

            if sl == Decimal::ZERO && tp == Decimal::ZERO {
                continue; // No stops set — legacy trade, skip
            }

            let pnl_pct = if pos.avg_price > Decimal::ZERO {
                ((pos.current_price - pos.avg_price) / pos.avg_price * Decimal::new(100, 0))
                    .to_f64().unwrap_or(0.0)
            } else {
                0.0
            };

            let (action, reason) = Self::check_position(
                pos.current_price,
                pos.avg_price,
                pos.side,
                sl, ts, tp,
                time_hours,
                pos.opened_at,
                pnl_pct,
            );

            checks.push(PositionCheck {
                symbol: pos.token_id.clone(),
                timestamp: now,
                current_price: pos.current_price,
                stop_loss: sl,
                trailing_stop: ts,
                take_profit: tp,
                action,
                reason,
            });
        }

        checks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_hold_when_all_clear() {
        let (action, _) = PositionMonitor::check_position(
            dec!(100), dec!(95), OrderSide::Buy,
            dec!(90), dec!(92), dec!(110),
            24, Utc::now(), 5.0,
        );
        assert_eq!(action, PositionAction::Hold);
    }

    #[test]
    fn test_stop_loss_triggered() {
        let (action, reason) = PositionMonitor::check_position(
            dec!(89), dec!(100), OrderSide::Buy,
            dec!(90), dec!(92), dec!(110),
            24, Utc::now(), -11.0,
        );
        assert_eq!(action, PositionAction::CloseStopLoss);
        assert!(reason.contains("Hard stop"));
    }

    #[test]
    fn test_trailing_stop_triggered() {
        let (action, _) = PositionMonitor::check_position(
            dec!(91), dec!(95), OrderSide::Buy,
            dec!(85), dec!(92), dec!(110), // trailing at 92, price dropped to 91
            24, Utc::now(), -4.0,
        );
        assert_eq!(action, PositionAction::CloseTrailingStop);
    }

    #[test]
    fn test_take_profit_triggered() {
        let (action, reason) = PositionMonitor::check_position(
            dec!(111), dec!(100), OrderSide::Buy,
            dec!(90), dec!(92), dec!(110),
            24, Utc::now(), 11.0,
        );
        assert_eq!(action, PositionAction::CloseTakeProfit);
        assert!(reason.contains("Take profit"));
    }

    #[test]
    fn test_time_stop_stale_trade() {
        let opened = Utc::now() - chrono::Duration::hours(25);
        let (action, reason) = PositionMonitor::check_position(
            dec!(100.5), dec!(100), OrderSide::Buy,
            dec!(90), dec!(92), dec!(110),
            24, opened, 0.5, // Only 0.5% move in 25 hours
        );
        assert_eq!(action, PositionAction::CloseTimeStop);
        assert!(reason.contains("Time stop"));
    }

    #[test]
    fn test_time_stop_not_triggered_if_profitable() {
        let opened = Utc::now() - chrono::Duration::hours(25);
        let (action, _) = PositionMonitor::check_position(
            dec!(105), dec!(100), OrderSide::Buy,
            dec!(90), dec!(92), dec!(110),
            24, opened, 5.0, // 5% move — not stale
        );
        assert_eq!(action, PositionAction::Hold);
    }

    #[test]
    fn test_short_stop_loss() {
        let (action, _) = PositionMonitor::check_position(
            dec!(111), dec!(100), OrderSide::Sell,
            dec!(110), dec!(108), dec!(80),
            24, Utc::now(), -11.0,
        );
        assert_eq!(action, PositionAction::CloseStopLoss);
    }
}
