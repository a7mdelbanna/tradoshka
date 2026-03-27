use serde::{Deserialize, Serialize};
use crate::basket_consensus::{ConsensusResult, AdaptiveSizer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AIVerdict {
    Confirms,   // Edge > 10% — full size
    Neutral,    // Edge 3-10% — half size
    Disagrees,  // Edge < 3% — skip
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyDecision {
    pub market_id: String,
    pub market_question: String,
    pub side: String,
    pub consensus_pct: f64,
    pub ai_probability: f64,
    pub market_price: f64,
    pub ai_edge: f64,
    pub ai_verdict: AIVerdict,
    pub position_size: f64,
    pub should_execute: bool,
    pub reasoning: String,
}

pub struct CopyEngine {
    edge_confirm_threshold: f64,   // 0.10 = 10%
    edge_neutral_threshold: f64,   // 0.03 = 3%
    max_per_market_pct: f64,       // 0.10 = 10% of portfolio
}

impl CopyEngine {
    pub fn new() -> Self {
        Self {
            edge_confirm_threshold: 0.10,
            edge_neutral_threshold: 0.03,
            max_per_market_pct: 0.10,
        }
    }

    /// Evaluate a consensus signal with AI verification.
    /// ai_probability: our AI's estimate of the outcome probability (0.0-1.0)
    pub fn evaluate(
        &self,
        consensus: &ConsensusResult,
        ai_probability: f64,
        portfolio_equity: f64,
    ) -> CopyDecision {
        if !consensus.triggered {
            return CopyDecision {
                market_id: consensus.market_id.clone(),
                market_question: consensus.market_question.clone(),
                side: consensus.consensus_side.clone(),
                consensus_pct: consensus.consensus_pct,
                ai_probability,
                market_price: consensus.avg_entry_price,
                ai_edge: 0.0,
                ai_verdict: AIVerdict::Disagrees,
                position_size: 0.0,
                should_execute: false,
                reasoning: "Consensus not triggered".into(),
            };
        }

        // Calculate edge: difference between AI probability and market price
        let market_price = consensus.avg_entry_price;
        let ai_edge = if consensus.consensus_side == "YES" {
            ai_probability - market_price
        } else {
            (1.0 - ai_probability) - (1.0 - market_price)
        };

        // Determine AI verdict
        let ai_verdict = if ai_edge > self.edge_confirm_threshold {
            AIVerdict::Confirms
        } else if ai_edge > self.edge_neutral_threshold {
            AIVerdict::Neutral
        } else {
            AIVerdict::Disagrees
        };

        // Calculate position size
        let size_multiplier = match ai_verdict {
            AIVerdict::Confirms => 1.0,
            AIVerdict::Neutral => 0.5,
            AIVerdict::Disagrees => 0.0,
        };

        let whale_size = consensus.avg_entry_price * 100.0; // Estimate
        let adaptive_size = AdaptiveSizer::calculate_size(
            whale_size, portfolio_equity, self.max_per_market_pct,
        );
        let position_size = adaptive_size * size_multiplier;

        let should_execute = ai_verdict != AIVerdict::Disagrees && position_size > 0.0;

        let reasoning = format!(
            "Basket '{}': {}/{} wallets agree on {} ({:.0}%). AI probability: {:.0}%, market: {:.0}%, edge: {:.1}%. Verdict: {:?}. Size: ${:.2}",
            consensus.basket_name, consensus.wallets_agreeing, consensus.wallets_total,
            consensus.consensus_side, consensus.consensus_pct * 100.0,
            ai_probability * 100.0, market_price * 100.0, ai_edge * 100.0,
            ai_verdict, position_size,
        );

        CopyDecision {
            market_id: consensus.market_id.clone(),
            market_question: consensus.market_question.clone(),
            side: consensus.consensus_side.clone(),
            consensus_pct: consensus.consensus_pct,
            ai_probability,
            market_price,
            ai_edge,
            ai_verdict,
            position_size,
            should_execute,
            reasoning,
        }
    }
}

impl Default for CopyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_triggered_consensus(side: &str, market_price: f64) -> ConsensusResult {
        ConsensusResult {
            market_id: "mkt-001".into(),
            market_question: "Will BTC hit $100K by June?".into(),
            consensus_side: side.into(),
            consensus_pct: 0.85,
            wallets_agreeing: 9,
            wallets_total: 11,
            avg_entry_price: market_price,
            basket_name: "crypto_basket".into(),
            triggered: true,
            reason: "9/11 wallets agree on YES (85%)".into(),
        }
    }

    fn make_untriggered_consensus() -> ConsensusResult {
        ConsensusResult {
            market_id: "mkt-002".into(),
            market_question: "Will ETH hit $10K?".into(),
            consensus_side: "YES".into(),
            consensus_pct: 0.50,
            wallets_agreeing: 5,
            wallets_total: 10,
            avg_entry_price: 0.40,
            basket_name: "crypto_basket".into(),
            triggered: false,
            reason: "Only 5/10 wallets agree (50% < 80% threshold)".into(),
        }
    }

    #[test]
    fn test_confirms_with_large_edge() {
        // AI says 65%, market at 40% → edge = +25% → CONFIRMS (> 10%)
        let engine = CopyEngine::new();
        let consensus = make_triggered_consensus("YES", 0.40);
        let decision = engine.evaluate(&consensus, 0.65, 10_000.0);

        assert_eq!(decision.ai_verdict, AIVerdict::Confirms);
        assert!(decision.should_execute);
        assert!(decision.ai_edge > 0.10);
    }

    #[test]
    fn test_neutral_with_small_edge() {
        // AI says 47%, market at 40% → edge = +7% → NEUTRAL (3-10%)
        let engine = CopyEngine::new();
        let consensus = make_triggered_consensus("YES", 0.40);
        let decision = engine.evaluate(&consensus, 0.47, 10_000.0);

        assert_eq!(decision.ai_verdict, AIVerdict::Neutral);
        assert!(decision.should_execute);
        assert!(decision.ai_edge > 0.03);
        assert!(decision.ai_edge <= 0.10);
    }

    #[test]
    fn test_disagrees_with_no_edge() {
        // AI says 41%, market at 40% → edge = +1% → DISAGREES (< 3%)
        let engine = CopyEngine::new();
        let consensus = make_triggered_consensus("YES", 0.40);
        let decision = engine.evaluate(&consensus, 0.41, 10_000.0);

        assert_eq!(decision.ai_verdict, AIVerdict::Disagrees);
        assert!(!decision.should_execute);
        assert!(decision.ai_edge < 0.03);
    }

    #[test]
    fn test_no_execute_when_consensus_not_triggered() {
        let engine = CopyEngine::new();
        let consensus = make_untriggered_consensus();
        let decision = engine.evaluate(&consensus, 0.70, 10_000.0);

        assert!(!decision.should_execute);
        assert_eq!(decision.position_size, 0.0);
        assert!(decision.reasoning.contains("Consensus not triggered"));
    }

    #[test]
    fn test_full_size_on_confirm() {
        // CONFIRMS should give full size (1.0x multiplier)
        let engine = CopyEngine::new();
        let consensus = make_triggered_consensus("YES", 0.35);
        // AI says 55%, market at 35% → edge = +20% → CONFIRMS
        let decision = engine.evaluate(&consensus, 0.55, 10_000.0);

        assert_eq!(decision.ai_verdict, AIVerdict::Confirms);
        // whale_size = 0.35 * 100 = 35.0, multiplier = 2.0 → 70.0
        // max_allowed = 10_000 * 0.10 = 1000.0
        // adaptive_size = 70.0 (within cap)
        // position_size = 70.0 * 1.0 = 70.0
        assert!((decision.position_size - 70.0).abs() < 1.0);
    }

    #[test]
    fn test_half_size_on_neutral() {
        // NEUTRAL should give half size (0.5x multiplier)
        let engine = CopyEngine::new();
        let consensus = make_triggered_consensus("YES", 0.35);
        // AI says 42%, market at 35% → edge = +7% → NEUTRAL
        let decision = engine.evaluate(&consensus, 0.42, 10_000.0);

        assert_eq!(decision.ai_verdict, AIVerdict::Neutral);
        // adaptive_size = 70.0, position_size = 70.0 * 0.5 = 35.0
        assert!((decision.position_size - 35.0).abs() < 1.0);
    }
}
