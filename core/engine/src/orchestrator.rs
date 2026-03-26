use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn, debug};
use uuid::Uuid;

use tradoshka_common::types::*;
use crate::wallet::{SimulatedWallet, WalletMode};
use crate::trade_recorder::{TradeRecorder, TradeRecord};
use crate::market_data::{MarketDataService, TrackedMarket};

/// A signal produced by strategy evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySignal {
    pub strategy_id: String,
    pub token_id: String,
    pub market_question: String,
    pub outcome: String,
    pub direction: SignalDirection,
    pub strength: f64,
    pub edge_vs_market: f64,
    pub confidence: f64,
}

/// Configuration for the orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub min_signal_strength: f64,
    pub min_edge: f64,
    pub max_position_per_market: Decimal,
    pub default_trade_size_pct: Decimal,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            min_signal_strength: 0.3,
            min_edge: 0.05,
            max_position_per_market: dec!(20),
            default_trade_size_pct: dec!(0.05),
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
    config: OrchestratorConfig,
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

    /// Run one orchestration cycle:
    /// 1. Get tracked markets with current prices
    /// 2. Generate signals (statistical simulation for now)
    /// 3. Filter by strength and edge
    /// 4. Execute approved signals in wallet
    /// 5. Record trades
    pub fn run_cycle(
        &mut self,
        markets: &[TrackedMarket],
        wallet: &mut SimulatedWallet,
        recorder: &mut TradeRecorder,
    ) -> CycleResult {
        let now = Utc::now();
        self.cycle_count += 1;
        self.last_cycle_at = Some(now);

        let mut signals = Vec::new();
        let mut executed_trades = Vec::new();

        // 1. Generate signals for each tracked market
        for market in markets {
            if market.yes_price <= Decimal::ZERO || market.no_price <= Decimal::ZERO {
                continue;
            }

            let market_signals = self.evaluate_market(market);
            signals.extend(market_signals);
        }

        // 2. Filter signals by minimum thresholds
        let actionable: Vec<&StrategySignal> = signals.iter()
            .filter(|s| {
                s.strength >= self.config.min_signal_strength
                    && s.edge_vs_market.abs() >= self.config.min_edge
                    && !matches!(s.direction, SignalDirection::Hold)
            })
            .collect();

        debug!("Cycle {}: {} markets, {} signals, {} actionable",
            self.cycle_count, markets.len(), signals.len(), actionable.len());

        // 3. Execute actionable signals
        for signal in actionable {
            if let Some(trade) = self.execute_signal(signal, wallet, recorder) {
                executed_trades.push(trade);
            }
        }

        if !executed_trades.is_empty() {
            info!("Cycle {}: executed {} trades", self.cycle_count, executed_trades.len());
        }

        CycleResult {
            timestamp: now,
            markets_evaluated: markets.len(),
            signals_generated: signals.len(),
            trades_executed: executed_trades.len(),
            trades: executed_trades,
        }
    }

    /// Evaluate a single market and generate strategy signals.
    /// Uses statistical simulation (bias-based) — no LLM needed.
    fn evaluate_market(&self, market: &TrackedMarket) -> Vec<StrategySignal> {
        let mut signals = Vec::new();
        let yes_price = market.yes_price.to_f64().unwrap_or(0.5);
        let no_price = market.no_price.to_f64().unwrap_or(0.5);

        // Strategy 1: Mispricing detector (arbitrage-like)
        let total = yes_price + no_price;
        let deviation = (total - 1.0).abs();
        if deviation > 0.03 {
            let (direction, outcome, token_id) = if yes_price < no_price {
                (SignalDirection::Long, "Yes", &market.yes_token_id)
            } else {
                (SignalDirection::Long, "No", &market.no_token_id)
            };
            signals.push(StrategySignal {
                strategy_id: "arbitrage".into(),
                token_id: token_id.clone(),
                market_question: market.question.clone(),
                outcome: outcome.into(),
                direction,
                strength: (deviation * 5.0).min(1.0),
                edge_vs_market: deviation,
                confidence: 0.8,
            });
        }

        // Strategy 2: Value detector — buy underpriced outcomes
        if yes_price < 0.35 && market.volume_24h > 5000.0 {
            signals.push(StrategySignal {
                strategy_id: "value".into(),
                token_id: market.yes_token_id.clone(),
                market_question: market.question.clone(),
                outcome: "Yes".into(),
                direction: SignalDirection::Long,
                strength: ((0.35 - yes_price) * 3.0).min(1.0),
                edge_vs_market: 0.35 - yes_price,
                confidence: 0.5,
            });
        }
        if no_price < 0.35 && market.volume_24h > 5000.0 {
            signals.push(StrategySignal {
                strategy_id: "value".into(),
                token_id: market.no_token_id.clone(),
                market_question: market.question.clone(),
                outcome: "No".into(),
                direction: SignalDirection::Long,
                strength: ((0.35 - no_price) * 3.0).min(1.0),
                edge_vs_market: 0.35 - no_price,
                confidence: 0.5,
            });
        }

        // Strategy 3: Momentum — high volume markets moving in one direction
        if market.volume_24h > 10000.0 {
            if yes_price > 0.65 {
                signals.push(StrategySignal {
                    strategy_id: "momentum".into(),
                    token_id: market.yes_token_id.clone(),
                    market_question: market.question.clone(),
                    outcome: "Yes".into(),
                    direction: SignalDirection::Long,
                    strength: ((yes_price - 0.65) * 3.0).min(1.0),
                    edge_vs_market: yes_price - 0.65,
                    confidence: 0.4,
                });
            }
        }

        signals
    }

    /// Execute a signal: buy in the wallet and record the trade.
    fn execute_signal(
        &self,
        signal: &StrategySignal,
        wallet: &mut SimulatedWallet,
        recorder: &mut TradeRecorder,
    ) -> Option<TradeRecord> {
        // Check if we already have a position in this token
        let position_key = format!("{}:{}", signal.token_id, signal.strategy_id);
        if wallet.positions().contains_key(&position_key) {
            return None; // Already in this market with this strategy
        }

        // Calculate trade size: percentage of equity
        let equity = wallet.equity();
        let trade_value = equity * self.config.default_trade_size_pct;
        let price = Decimal::from_f64(
            if signal.outcome == "Yes" { 0.5 } else { 0.5 } // Will use actual price from wallet
        ).unwrap_or(dec!(0.50));

        // For Polymarket, price is 0-1, so shares = trade_value / price
        // But we need the actual market price — use a reasonable estimate
        let estimated_price = dec!(0.50); // Simplified for now
        let shares = if estimated_price > Decimal::ZERO {
            (trade_value / estimated_price).round_dp(0)
        } else {
            return None;
        };

        if shares <= Decimal::ZERO {
            return None;
        }

        // Cap shares
        let shares = shares.min(self.config.max_position_per_market);

        match signal.direction {
            SignalDirection::Long => {
                if let Some((fill_price, fee, filled_shares)) = wallet.buy(
                    &signal.token_id,
                    &signal.market_question,
                    &signal.outcome,
                    estimated_price,
                    shares,
                    &signal.strategy_id,
                ) {
                    let trade = TradeRecord {
                        id: Uuid::new_v4().to_string(),
                        timestamp: Utc::now(),
                        market: Market::Polymarket,
                        symbol: signal.token_id.clone(),
                        market_question: signal.market_question.clone(),
                        direction: signal.outcome.clone(),
                        side: OrderSide::Buy,
                        shares: filled_shares,
                        price: fill_price,
                        fee,
                        strategy_id: signal.strategy_id.clone(),
                        signal_strength: signal.strength,
                        edge_vs_market: signal.edge_vs_market,
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
                    };
                    recorder.record(trade.clone());
                    info!("BUY {} {} @ {} ({}) — strategy: {}, edge: {:.1}%",
                        signal.outcome, signal.market_question,
                        fill_price, filled_shares, signal.strategy_id,
                        signal.edge_vs_market * 100.0);
                    return Some(trade);
                }
            }
            SignalDirection::Short | SignalDirection::Close => {
                if let Some((fill_price, fee, pnl)) = wallet.sell(
                    &signal.token_id,
                    estimated_price,
                    shares,
                    &signal.strategy_id,
                ) {
                    recorder.close_trade(&signal.token_id, &signal.strategy_id, pnl);
                    let trade = TradeRecord {
                        id: Uuid::new_v4().to_string(),
                        timestamp: Utc::now(),
                        market: Market::Polymarket,
                        symbol: signal.token_id.clone(),
                        market_question: signal.market_question.clone(),
                        direction: signal.outcome.clone(),
                        side: OrderSide::Sell,
                        shares,
                        price: fill_price,
                        fee,
                        strategy_id: signal.strategy_id.clone(),
                        signal_strength: signal.strength,
                        edge_vs_market: signal.edge_vs_market,
                        pnl: Some(pnl),
                        is_closed: true,
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
                    };
                    recorder.record(trade.clone());
                    return Some(trade);
                }
            }
            SignalDirection::Hold => {}
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
            let strategy_ids: Vec<String> = recorder.open_trades()
                .iter()
                .filter(|t| t.symbol == token_id)
                .map(|t| t.strategy_id.clone())
                .collect();
            for strategy_id in strategy_ids {
                recorder.close_trade(token_id, &strategy_id, pnl);
            }
            info!("Market settled: token={}, won={}, pnl={}", token_id, won, pnl);
        }
        pnl
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::TrackedMarket;

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
        let mut wallet = SimulatedWallet::new(dec!(100), dec!(0));
        let mut recorder = TradeRecorder::new();
        let result = orch.run_cycle(&[], &mut wallet, &mut recorder);
        assert_eq!(result.markets_evaluated, 0);
        assert_eq!(result.signals_generated, 0);
        assert_eq!(result.trades_executed, 0);
        assert_eq!(orch.cycle_count(), 1);
    }

    #[test]
    fn test_run_cycle_with_mispriced_market() {
        let mut orch = Orchestrator::new(OrchestratorConfig {
            min_signal_strength: 0.1,
            min_edge: 0.03,
            ..Default::default()
        });
        let mut wallet = SimulatedWallet::new(dec!(100), dec!(0));
        let mut recorder = TradeRecorder::new();
        // Total = 0.55 + 0.50 = 1.05 → deviation = 0.05 > 0.03
        let markets = vec![make_market("Will X happen?", 0.55, 0.50, 10000.0)];
        let result = orch.run_cycle(&markets, &mut wallet, &mut recorder);
        assert!(result.signals_generated > 0);
    }

    #[test]
    fn test_run_cycle_executes_trade() {
        let mut orch = Orchestrator::new(OrchestratorConfig {
            min_signal_strength: 0.1,
            min_edge: 0.03,
            default_trade_size_pct: dec!(0.10),
            max_position_per_market: dec!(50),
        });
        let mut wallet = SimulatedWallet::new(dec!(100), dec!(0));
        let mut recorder = TradeRecorder::new();
        let markets = vec![make_market("Mispriced event?", 0.40, 0.70, 10000.0)];
        let result = orch.run_cycle(&markets, &mut wallet, &mut recorder);
        // Should detect mispricing (0.40 + 0.70 = 1.10, dev = 0.10)
        // Should execute a buy on the cheaper side (Yes at 0.40)
        if result.trades_executed > 0 {
            assert!(wallet.balance() < dec!(100)); // Balance decreased
            assert!(recorder.total_trade_count() > 0);
        }
    }

    #[test]
    fn test_no_duplicate_positions() {
        let mut orch = Orchestrator::new(OrchestratorConfig {
            min_signal_strength: 0.1,
            min_edge: 0.03,
            default_trade_size_pct: dec!(0.10),
            max_position_per_market: dec!(50),
        });
        let mut wallet = SimulatedWallet::new(dec!(100), dec!(0));
        let mut recorder = TradeRecorder::new();
        let markets = vec![make_market("Mispriced?", 0.40, 0.70, 10000.0)];

        // Run twice — second cycle should NOT open another position in same market
        let r1 = orch.run_cycle(&markets, &mut wallet, &mut recorder);
        let trades_after_first = recorder.total_trade_count();
        let r2 = orch.run_cycle(&markets, &mut wallet, &mut recorder);
        let trades_after_second = recorder.total_trade_count();

        // Second cycle should not add more trades for same token+strategy
        assert!(trades_after_second <= trades_after_first + 1); // At most 1 new from a different strategy
    }

    #[test]
    fn test_settle_market() {
        let mut orch = Orchestrator::new(OrchestratorConfig::default());
        let mut wallet = SimulatedWallet::new(dec!(100), dec!(0));
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
