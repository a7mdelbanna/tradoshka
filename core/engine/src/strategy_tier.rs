use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    Unproven,
    Tested,
    Proven,
    Disabled,
}

impl Tier {
    pub fn label(&self) -> &str {
        match self {
            Self::Unproven => "Unproven",
            Self::Tested => "Tested",
            Self::Proven => "Proven",
            Self::Disabled => "Disabled",
        }
    }

    pub fn emoji(&self) -> &str {
        match self {
            Self::Unproven => "🔵",
            Self::Tested => "🟡",
            Self::Proven => "🟢",
            Self::Disabled => "🔴",
        }
    }

    pub fn risk_pct(&self) -> f64 {
        match self {
            Self::Unproven => 1.0,
            Self::Tested => 1.5,
            Self::Proven => 2.0,
            Self::Disabled => 0.0,
        }
    }

    pub fn max_positions(&self) -> usize {
        match self {
            Self::Unproven => 3,
            Self::Tested => 5,
            Self::Proven => 8,
            Self::Disabled => 0,
        }
    }

    pub fn max_exposure_pct(&self) -> f64 {
        match self {
            Self::Unproven => 10.0,
            Self::Tested => 20.0,
            Self::Proven => 30.0,
            Self::Disabled => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyTierTracker {
    pub strategy_id: String,
    pub tier: Tier,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub consecutive_losses: u32,
    pub recent_results: Vec<bool>, // Last 20 trade results (true=win)
}

impl StrategyTierTracker {
    pub fn new(strategy_id: &str) -> Self {
        Self {
            strategy_id: strategy_id.into(),
            tier: Tier::Unproven,
            total_trades: 0,
            winning_trades: 0,
            consecutive_losses: 0,
            recent_results: Vec::new(),
        }
    }

    pub fn win_rate(&self) -> f64 {
        if self.total_trades == 0 { return 0.0; }
        self.winning_trades as f64 / self.total_trades as f64
    }

    pub fn recent_win_rate(&self) -> f64 {
        if self.recent_results.is_empty() { return 0.0; }
        let wins = self.recent_results.iter().filter(|&&w| w).count();
        wins as f64 / self.recent_results.len() as f64
    }

    pub fn record_result(&mut self, won: bool) {
        self.total_trades += 1;
        if won {
            self.winning_trades += 1;
            self.consecutive_losses = 0;
        } else {
            self.consecutive_losses += 1;
        }

        self.recent_results.push(won);
        if self.recent_results.len() > 20 {
            self.recent_results.remove(0);
        }

        self.evaluate_tier();
    }

    fn evaluate_tier(&mut self) {
        // Disabled check: 5 consecutive losses while Unproven
        if self.tier == Tier::Unproven && self.consecutive_losses >= 5 {
            self.tier = Tier::Disabled;
            return;
        }

        // Promotion: Unproven → Tested
        if self.tier == Tier::Unproven && self.total_trades >= 20 && self.win_rate() > 0.50 {
            self.tier = Tier::Tested;
        }

        // Promotion: Tested → Proven
        if self.tier == Tier::Tested && self.total_trades >= 50 && self.win_rate() > 0.70 {
            self.tier = Tier::Proven;
        }

        // Demotion: Proven → Tested
        if self.tier == Tier::Proven && self.recent_results.len() >= 20 && self.recent_win_rate() < 0.60 {
            self.tier = Tier::Tested;
        }

        // Demotion: Tested → Unproven
        if self.tier == Tier::Tested && self.recent_results.len() >= 20 && self.recent_win_rate() < 0.40 {
            self.tier = Tier::Unproven;
        }
    }

    pub fn trades_to_next_tier(&self) -> Option<u32> {
        match self.tier {
            Tier::Unproven => Some(20u32.saturating_sub(self.total_trades)),
            Tier::Tested => Some(50u32.saturating_sub(self.total_trades)),
            Tier::Proven | Tier::Disabled => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker_is_unproven() {
        let t = StrategyTierTracker::new("momentum");
        assert_eq!(t.tier, Tier::Unproven);
        assert_eq!(t.tier.risk_pct(), 1.0);
        assert_eq!(t.tier.max_positions(), 3);
    }

    #[test]
    fn test_promotion_to_tested() {
        let mut t = StrategyTierTracker::new("m");
        for _ in 0..20 {
            t.record_result(true); // 20 wins = 100% WR > 50%
        }
        assert_eq!(t.tier, Tier::Tested);
        assert_eq!(t.tier.risk_pct(), 1.5);
    }

    #[test]
    fn test_promotion_to_proven() {
        let mut t = StrategyTierTracker::new("m");
        for _ in 0..50 {
            t.record_result(true); // 50 wins = 100% WR > 70%
        }
        assert_eq!(t.tier, Tier::Proven);
        assert_eq!(t.tier.risk_pct(), 2.0);
        assert_eq!(t.tier.max_positions(), 8);
    }

    #[test]
    fn test_demotion_proven_to_tested() {
        let mut t = StrategyTierTracker::new("m");
        for _ in 0..50 { t.record_result(true); }
        assert_eq!(t.tier, Tier::Proven);
        // Now lose a lot in recent 20
        for _ in 0..12 { t.record_result(false); }
        // Recent 20: 8 wins out of 20 = 40% < 60%
        assert_eq!(t.tier, Tier::Tested);
    }

    #[test]
    fn test_disable_after_5_consecutive_losses() {
        let mut t = StrategyTierTracker::new("m");
        for _ in 0..5 { t.record_result(false); }
        assert_eq!(t.tier, Tier::Disabled);
        assert_eq!(t.tier.risk_pct(), 0.0);
        assert_eq!(t.tier.max_positions(), 0);
    }

    #[test]
    fn test_win_rate_calculation() {
        let mut t = StrategyTierTracker::new("m");
        t.record_result(true);
        t.record_result(true);
        t.record_result(false);
        assert!((t.win_rate() - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_trades_to_next_tier() {
        let mut t = StrategyTierTracker::new("m");
        assert_eq!(t.trades_to_next_tier(), Some(20));
        for _ in 0..10 { t.record_result(true); }
        assert_eq!(t.trades_to_next_tier(), Some(10));
    }

    #[test]
    fn test_tier_labels_and_emojis() {
        assert_eq!(Tier::Unproven.label(), "Unproven");
        assert_eq!(Tier::Tested.emoji(), "🟡");
        assert_eq!(Tier::Proven.emoji(), "🟢");
        assert_eq!(Tier::Disabled.emoji(), "🔴");
    }
}
