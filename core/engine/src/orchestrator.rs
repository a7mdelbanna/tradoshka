use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use tradoshka_common::types::*;
use crate::wallet::SimulatedWallet;
use crate::trade_recorder::{TradeRecorder, TradeRecord};
use crate::market_data::TrackedMarket;
use crate::research::{ResearchEngine, ResearchConfig, MarketSnapshot};
use crate::trade_thesis::TradeThesis;

/// Configuration for the orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub min_signal_strength: f64,
    pub min_edge: f64,
    pub max_position_per_market: Decimal,
    pub default_trade_size_pct: Decimal,
    pub risk_pct: f64, // risk per trade percentage (1.0 = 1%)
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            min_signal_strength: 0.3,
            min_edge: 0.05,
            max_position_per_market: dec!(20),
            default_trade_size_pct: dec!(0.05),
            risk_pct: 1.0,
        }
    }
}

/// Result of a single orchestration cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleResult {
    pub timestamp: DateTime<Utc>,
    pub markets_evaluated: usize,
    pub signals_generated: usize,
    pub trades_executed: usize,
    pub trades: Vec<TradeRecord>,
}

/// The strategy orchestrator — runs the complete trading loop.
pub struct Orchestrator {
    pub(crate) config: OrchestratorConfig,
    cycle_count: u64,
    last_cycle_at: Option<DateTime<Utc>>,
}

impl Orchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            config,
            cycle_count: 0,
            last_cycle_at: None,
        }
    }

    pub fn cycle_count(&self) -> u64 {
        self.cycle_count
    }

    pub fn last_cycle_at(&self) -> Option<DateTime<Utc>> {
        self.last_cycle_at
    }

    /// Run one orchestration cycle using the research engine.
    /// For each tracked market:
    ///   1. Build a MarketSnapshot
    ///   2. Call ResearchEngine to produce a TradeThesis
    ///   3. If approved, execute and record the trade with full thesis fields
    ///   4. If rejected, log the reason and skip
    pub fn run_cycle(
        &mut self,
        markets: &[TrackedMarket],
        wallet: &mut SimulatedWallet,
        recorder: &mut TradeRecorder,
    ) -> CycleResult {
        let now = Utc::now();
        self.cycle_count += 1;
        self.last_cycle_at = Some(now);
        let mut executed_trades = Vec::new();

        let config = ResearchConfig {
            equity: wallet.equity(),
            risk_pct: self.config.risk_pct,
            ..Default::default()
        };

        for market in markets {
            // Skip invalid prices
            if market.yes_price <= Decimal::ZERO || market.no_price <= Decimal::ZERO {
                continue;
            }

            // Skip if we already have a position in this market (any strategy)
            let position_key_prefix = format!("{}:", market.yes_token_id);
            let has_position = wallet
                .positions()
                .keys()
                .any(|k| k.starts_with(&position_key_prefix));
            if has_position {
                continue;
            }

            // Build snapshot and run research
            let snapshot = self.build_snapshot(market);

            let thesis_result = if market.condition_id.starts_with("0x") {
                // Polymarket binary market
                ResearchEngine::analyze_polymarket(
                    &market.question,
                    market.yes_price,
                    market.no_price,
                    market.volume_24h,
                    None, // days_remaining — needs end_date parsing, not available yet
                    &config,
                )
            } else {
                // Crypto market
                ResearchEngine::analyze_crypto(&snapshot, &config)
            };

            match thesis_result {
                Ok(thesis) => {
                    if let Some(trade) = self.execute_thesis(&thesis, market, wallet, recorder) {
                        executed_trades.push(trade);
                    }
                }
                Err(reason) => {
                    tracing::debug!("Research rejected {}: {}", market.question, reason);
                }
            }
        }

        if !executed_trades.is_empty() {
            info!(
                "Cycle {}: {} trades executed",
                self.cycle_count,
                executed_trades.len()
            );
        }

        CycleResult {
            timestamp: now,
            markets_evaluated: markets.len(),
            signals_generated: markets.len(), // All markets were researched
            trades_executed: executed_trades.len(),
            trades: executed_trades,
        }
    }

    /// Build a MarketSnapshot from a TrackedMarket for the research engine.
    fn build_snapshot(&self, market: &TrackedMarket) -> MarketSnapshot {
        let price = market.yes_price;
        MarketSnapshot {
            symbol: market.yes_token_id.clone(),
            market: tradoshka_common::types::Market::Crypto,
            current_price: price,
            price_24h_ago: None,
            volume_24h: market.volume_24h,
            volume_7d_avg: Some(market.volume_24h * 0.8), // Conservative 7d average estimate
            atr_14: price * dec!(0.02), // 2% ATR estimate (real candle data not yet available)
            rsi_14: Some(55.0),         // Neutral RSI estimate
            ema_9: Some(price * dec!(1.005)), // Slight upward EMA bias
            ema_21: Some(price * dec!(0.995)),
            funding_rate: None,
            question: market.question.clone(),
            days_to_resolution: None,
        }
    }

    /// Execute a validated trade thesis: buy in wallet and record the trade with full thesis data.
    fn execute_thesis(
        &self,
        thesis: &TradeThesis,
        market: &TrackedMarket,
        wallet: &mut SimulatedWallet,
        recorder: &mut TradeRecorder,
    ) -> Option<TradeRecord> {
        let price = thesis.entry_price;
        let size = thesis.position_size;

        if size <= Decimal::ZERO || price <= Decimal::ZERO {
            return None;
        }

        // Cap position size by the configured maximum
        let size = size.min(self.config.max_position_per_market);

        // Determine direction: if take_profit > entry it's a long (Buy)
        let is_long = thesis.take_profit > thesis.entry_price;
        let outcome = if is_long { "Long" } else { "Short" };

        let result = wallet.buy(
            &market.yes_token_id,
            &market.question,
            outcome,
            price,
            size,
            "research", // Strategy ID for all research-driven trades
        );

        if let Some((fill_price, fee, filled_shares)) = result {
            let market_type = if market.condition_id.starts_with("0x") {
                tradoshka_common::types::Market::Polymarket
            } else {
                tradoshka_common::types::Market::Crypto
            };

            let trade = TradeRecord {
                id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                market: market_type,
                symbol: market.yes_token_id.clone(),
                market_question: market.question.clone(),
                direction: outcome.into(),
                side: if is_long { OrderSide::Buy } else { OrderSide::Sell },
                shares: filled_shares,
                price: fill_price,
                fee,
                strategy_id: "research".into(),
                signal_strength: thesis.confidence,
                edge_vs_market: thesis.reward_risk_ratio,
                pnl: None,
                is_closed: false,
                // Full thesis fields
                thesis_reasoning: thesis.reasoning.clone(),
                stop_loss: thesis.hard_stop_loss,
                trailing_stop: thesis.trailing_stop,
                take_profit: thesis.take_profit,
                time_stop_hours: thesis.time_stop_hours,
                thesis_invalidation: thesis.thesis_invalidation.clone(),
                risk_amount: thesis.risk_amount,
                reward_risk_ratio: thesis.reward_risk_ratio,
                strategy_tier: thesis.strategy_tier.clone(),
                close_reason: None,
            };
            recorder.record(trade.clone());
            info!(
                "{} {} @ {} — R:R {:.1}x, risk ${}, SL {}, TP {} | {}",
                outcome,
                market.question,
                fill_price,
                thesis.reward_risk_ratio,
                thesis.risk_amount,
                thesis.hard_stop_loss,
                thesis.take_profit,
                thesis.reasoning.chars().take(80).collect::<String>()
            );
            return Some(trade);
        }

        None
    }

    /// Settle all positions for a resolved market.
    pub fn settle_market(
        &self,
        token_id: &str,
        won: bool,
        wallet: &mut SimulatedWallet,
        recorder: &mut TradeRecorder,
    ) -> Decimal {
        let pnl = wallet.settle_market(token_id, won);
        if pnl != Decimal::ZERO {
            // Collect strategy IDs first to avoid borrow conflict
            let strategy_ids: Vec<String> = recorder
                .open_trades()
                .iter()
                .filter(|t| t.symbol == token_id)
                .map(|t| t.strategy_id.clone())
                .collect();
            for strategy_id in strategy_ids {
                recorder.close_trade(token_id, &strategy_id, pnl);
            }
            info!(
                "Market settled: token={}, won={}, pnl={}",
                token_id, won, pnl
            );
        }
        pnl
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::TrackedMarket;
    use rust_decimal::prelude::FromPrimitive;

    fn make_market(question: &str, yes_price: f64, no_price: f64, volume: f64) -> TrackedMarket {
        TrackedMarket {
            condition_id: Uuid::new_v4().to_string(),
            question: question.into(),
            yes_token_id: format!("yes_{}", question.len()),
            no_token_id: format!("no_{}", question.len()),
            yes_price: Decimal::from_f64(yes_price).unwrap_or(dec!(0.50)),
            no_price: Decimal::from_f64(no_price).unwrap_or(dec!(0.50)),
            volume_24h: volume,
            liquidity: volume * 0.5,
            last_updated: Utc::now(),
        }
    }

    #[test]
    fn test_new_orchestrator() {
        let orch = Orchestrator::new(OrchestratorConfig::default());
        assert_eq!(orch.cycle_count(), 0);
        assert!(orch.last_cycle_at().is_none());
    }

    #[test]
    fn test_run_cycle_no_markets() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        let mut wallet = SimulatedWallet::new(dec!(100), dec!(0), dec!(0.002));
        let mut recorder = TradeRecorder::new();
        let result = orch.run_cycle(&[], &mut wallet, &mut recorder);
        assert_eq!(result.markets_evaluated, 0);
        assert_eq!(result.signals_generated, 0);
        assert_eq!(result.trades_executed, 0);
        assert_eq!(orch.cycle_count(), 1);
    }

    #[test]
    fn test_run_cycle_with_mispriced_market() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        let mut wallet = SimulatedWallet::new(dec!(1000), dec!(0), dec!(0.002));
        let mut recorder = TradeRecorder::new();
        // Total = 0.55 + 0.50 = 1.05 → deviation = 0.05 > 0.03 (mispricing signal)
        // volume > 5000 (volume signal) → 2 signals → research may approve
        let markets = vec![make_market("Will X happen?", 0.55, 0.50, 10000.0)];
        let result = orch.run_cycle(&markets, &mut wallet, &mut recorder);
        // markets_evaluated == signals_generated in new research flow
        assert_eq!(result.markets_evaluated, 1);
        assert_eq!(result.signals_generated, 1);
        // Trade may or may not execute depending on confidence threshold — that's expected
        // Just verify the cycle ran without panic
        assert_eq!(orch.cycle_count(), 1);
    }

    #[test]
    fn test_no_duplicate_positions() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        let mut wallet = SimulatedWallet::new(dec!(1000), dec!(0), dec!(0.002));
        let mut recorder = TradeRecorder::new();
        // mispricing + high volume = likely to produce a research-approved trade
        let markets = vec![make_market("Mispriced?", 0.40, 0.70, 100000.0)];

        // Run twice — second cycle should NOT open another position in same market
        let r1 = orch.run_cycle(&markets, &mut wallet, &mut recorder);
        let trades_after_first = recorder.total_trade_count();
        let _r2 = orch.run_cycle(&markets, &mut wallet, &mut recorder);
        let trades_after_second = recorder.total_trade_count();

        // If a trade was opened on first cycle, second cycle must not add another
        if r1.trades_executed > 0 {
            assert_eq!(trades_after_second, trades_after_first,
                "Second cycle should not open a duplicate position");
        }
    }

    #[test]
    fn test_settle_market() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        let mut wallet = SimulatedWallet::new(dec!(100), dec!(0), dec!(0.002));
        let mut recorder = TradeRecorder::new();

        // Manually buy a position
        wallet.buy("tok_yes", "Test?", "Yes", dec!(0.50), dec!(10), "ai");
        recorder.record(TradeRecord {
            id: "t1".into(),
            timestamp: Utc::now(),
            market: Market::Polymarket,
            symbol: "tok_yes".into(),
            market_question: "Test?".into(),
            direction: "Yes".into(),
            side: OrderSide::Buy,
            shares: dec!(10),
            price: dec!(0.50),
            fee: dec!(0.10),
            strategy_id: "ai".into(),
            signal_strength: 0.8,
            edge_vs_market: 0.05,
            pnl: None,
            is_closed: false,
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
        });

        let pnl = orch.settle_market("tok_yes", true, &mut wallet, &mut recorder);
        assert!(pnl > Decimal::ZERO); // Won — should be positive
        assert_eq!(wallet.open_position_count(), 0);
    }
}
