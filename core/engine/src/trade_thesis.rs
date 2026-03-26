use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

/// Complete trade reasoning, risk parameters, and exit levels.
/// Every trade in every market must carry one of these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeThesis {
    // WHY
    pub reasoning: String,
    pub signals_used: Vec<String>,
    pub signals_agreed: u32,
    pub signals_total: u32,
    pub confidence: f64,

    // ENTRY
    pub entry_price: Decimal,
    pub entry_reason: String,

    // EXITS
    pub hard_stop_loss: Decimal,
    pub trailing_stop: Decimal,
    pub take_profit: Decimal,
    pub time_stop_hours: u32,
    pub thesis_invalidation: String,

    // RISK
    pub risk_per_trade_pct: f64,
    pub risk_amount: Decimal,
    pub reward_risk_ratio: f64,
    pub position_size: Decimal,
    pub max_loss: Decimal,
    pub strategy_tier: String,
}

impl TradeThesis {
    /// Validate that the thesis meets all entry rules.
    /// Returns Ok(()) or Err with the reason it fails.
    pub fn validate(&self) -> Result<(), String> {
        if self.reasoning.is_empty() {
            return Err("No reasoning provided".into());
        }
        if self.signals_agreed < 2 {
            return Err(format!("Only {} signals agreed, need at least 2", self.signals_agreed));
        }
        if self.confidence < 0.60 {
            return Err(format!("Confidence {:.0}% below 60% threshold", self.confidence * 100.0));
        }
        if self.reward_risk_ratio < 2.0 {
            return Err(format!("R:R {:.1} below 2.0 minimum", self.reward_risk_ratio));
        }
        if self.entry_price <= Decimal::ZERO {
            return Err("Entry price must be positive".into());
        }
        if self.hard_stop_loss <= Decimal::ZERO {
            return Err("Stop loss must be set".into());
        }
        if self.take_profit <= Decimal::ZERO {
            return Err("Take profit must be set".into());
        }
        if self.position_size <= Decimal::ZERO {
            return Err("Position size must be positive".into());
        }
        if self.risk_amount <= Decimal::ZERO {
            return Err("Risk amount must be positive".into());
        }
        Ok(())
    }

    /// Create a thesis that will fail validation (for rejected signals).
    pub fn rejected(reason: &str) -> Self {
        Self {
            reasoning: format!("REJECTED: {}", reason),
            signals_used: vec![],
            signals_agreed: 0,
            signals_total: 0,
            confidence: 0.0,
            entry_price: Decimal::ZERO,
            entry_reason: String::new(),
            hard_stop_loss: Decimal::ZERO,
            trailing_stop: Decimal::ZERO,
            take_profit: Decimal::ZERO,
            time_stop_hours: 0,
            thesis_invalidation: String::new(),
            risk_per_trade_pct: 0.0,
            risk_amount: Decimal::ZERO,
            reward_risk_ratio: 0.0,
            position_size: Decimal::ZERO,
            max_loss: Decimal::ZERO,
            strategy_tier: "Unproven".into(),
        }
    }
}

/// Builder for constructing a TradeThesis step by step.
pub struct ThesisBuilder {
    thesis: TradeThesis,
}

impl ThesisBuilder {
    pub fn new() -> Self {
        Self {
            thesis: TradeThesis {
                reasoning: String::new(),
                signals_used: vec![],
                signals_agreed: 0,
                signals_total: 0,
                confidence: 0.0,
                entry_price: Decimal::ZERO,
                entry_reason: String::new(),
                hard_stop_loss: Decimal::ZERO,
                trailing_stop: Decimal::ZERO,
                take_profit: Decimal::ZERO,
                time_stop_hours: 24,
                thesis_invalidation: String::new(),
                risk_per_trade_pct: 1.0,
                risk_amount: Decimal::ZERO,
                reward_risk_ratio: 0.0,
                position_size: Decimal::ZERO,
                max_loss: Decimal::ZERO,
                strategy_tier: "Unproven".into(),
            },
        }
    }

    pub fn reasoning(mut self, r: &str) -> Self { self.thesis.reasoning = r.into(); self }
    pub fn entry_reason(mut self, r: &str) -> Self { self.thesis.entry_reason = r.into(); self }
    pub fn thesis_invalidation(mut self, r: &str) -> Self { self.thesis.thesis_invalidation = r.into(); self }
    pub fn strategy_tier(mut self, t: &str) -> Self { self.thesis.strategy_tier = t.into(); self }

    pub fn add_signal(mut self, name: &str, fired: bool) -> Self {
        self.thesis.signals_used.push(name.into());
        self.thesis.signals_total += 1;
        if fired { self.thesis.signals_agreed += 1; }
        self
    }

    pub fn confidence(mut self, c: f64) -> Self { self.thesis.confidence = c.clamp(0.0, 1.0); self }

    pub fn entry(mut self, price: Decimal) -> Self { self.thesis.entry_price = price; self }

    pub fn stops(mut self, hard_sl: Decimal, trailing: Decimal, tp: Decimal, time_hours: u32) -> Self {
        self.thesis.hard_stop_loss = hard_sl;
        self.thesis.trailing_stop = trailing;
        self.thesis.take_profit = tp;
        self.thesis.time_stop_hours = time_hours;
        self
    }

    pub fn risk(mut self, pct: f64, amount: Decimal, rr: f64, size: Decimal) -> Self {
        self.thesis.risk_per_trade_pct = pct;
        self.thesis.risk_amount = amount;
        self.thesis.reward_risk_ratio = rr;
        self.thesis.position_size = size;
        self.thesis.max_loss = amount;
        self
    }

    pub fn build(self) -> TradeThesis {
        self.thesis
    }

    /// Build and validate. Returns Err if validation fails.
    pub fn build_validated(self) -> Result<TradeThesis, String> {
        let thesis = self.build();
        thesis.validate()?;
        Ok(thesis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_thesis() {
        let thesis = ThesisBuilder::new()
            .reasoning("EMA crossover + volume surge on 1h. RSI at 58, not overbought.")
            .add_signal("ema_crossover", true)
            .add_signal("volume_surge", true)
            .add_signal("rsi_confirmation", true)
            .confidence(0.82)
            .entry(dec!(69110))
            .entry_reason("Breakout above 1h resistance")
            .stops(dec!(67850), dec!(68400), dec!(71500), 24)
            .risk(1.0, dec!(1.00), 2.1, dec!(0.000029))
            .thesis_invalidation("EMA9 crosses below EMA21 on 1h")
            .build_validated();
        assert!(thesis.is_ok());
    }

    #[test]
    fn test_rejects_no_reasoning() {
        let thesis = ThesisBuilder::new()
            .add_signal("ema", true)
            .add_signal("vol", true)
            .confidence(0.8)
            .entry(dec!(100))
            .stops(dec!(95), dec!(96), dec!(110), 24)
            .risk(1.0, dec!(1), 2.5, dec!(1))
            .build_validated();
        assert!(thesis.is_err());
        assert!(thesis.unwrap_err().contains("No reasoning"));
    }

    #[test]
    fn test_rejects_low_confidence() {
        let thesis = ThesisBuilder::new()
            .reasoning("Some reason")
            .add_signal("a", true)
            .add_signal("b", true)
            .confidence(0.45)
            .entry(dec!(100))
            .stops(dec!(95), dec!(96), dec!(110), 24)
            .risk(1.0, dec!(1), 2.5, dec!(1))
            .build_validated();
        assert!(thesis.is_err());
        assert!(thesis.unwrap_err().contains("60%"));
    }

    #[test]
    fn test_rejects_insufficient_signals() {
        let thesis = ThesisBuilder::new()
            .reasoning("Only one signal")
            .add_signal("a", true)
            .add_signal("b", false)
            .confidence(0.8)
            .entry(dec!(100))
            .stops(dec!(95), dec!(96), dec!(110), 24)
            .risk(1.0, dec!(1), 2.5, dec!(1))
            .build_validated();
        assert!(thesis.is_err());
        assert!(thesis.unwrap_err().contains("1 signals agreed"));
    }

    #[test]
    fn test_rejects_low_rr() {
        let thesis = ThesisBuilder::new()
            .reasoning("Bad risk/reward")
            .add_signal("a", true)
            .add_signal("b", true)
            .confidence(0.8)
            .entry(dec!(100))
            .stops(dec!(95), dec!(96), dec!(110), 24)
            .risk(1.0, dec!(1), 1.5, dec!(1))
            .build_validated();
        assert!(thesis.is_err());
        assert!(thesis.unwrap_err().contains("R:R"));
    }

    #[test]
    fn test_rejected_thesis() {
        let thesis = TradeThesis::rejected("Price data unavailable");
        assert!(thesis.validate().is_err());
        assert!(thesis.reasoning.contains("REJECTED"));
    }

    #[test]
    fn test_builder_clamps_confidence() {
        let thesis = ThesisBuilder::new().confidence(1.5).build();
        assert_eq!(thesis.confidence, 1.0);
        let thesis = ThesisBuilder::new().confidence(-0.5).build();
        assert_eq!(thesis.confidence, 0.0);
    }
}
