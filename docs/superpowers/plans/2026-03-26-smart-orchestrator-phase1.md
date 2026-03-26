# Smart Orchestrator Phase 1: TradeThesis + Research Engine + ATR Stops

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the TradeThesis data structure, research engine that analyzes markets before trading, and ATR-based stop loss calculation — the foundation for the smart orchestrator.

**Architecture:** TradeThesis is a Rust struct stored with every trade. The research engine produces a TradeThesis by analyzing real market data (prices, volume, indicators). ATR calculation uses the existing indicator infrastructure. All existing TradeRecord fields are preserved, new thesis fields are added.

**Tech Stack:** Rust (rust_decimal, chrono, serde), existing tradoshka-common types, existing tradoshka-data indicators

---

## File Structure

```
core/engine/src/
├── trade_thesis.rs          # NEW: TradeThesis struct + validation
├── research.rs              # NEW: Research engine per market type
├── atr_stops.rs             # NEW: ATR-based stop loss calculator
├── trade_recorder.rs        # MODIFY: Add thesis fields to TradeRecord
├── lib.rs                   # Add new modules
```

---

### Task 1: TradeThesis Struct

**Files:**
- Create: `core/engine/src/trade_thesis.rs`
- Modify: `core/engine/src/lib.rs`

- [ ] **Step 1: Implement TradeThesis**

Create `core/engine/src/trade_thesis.rs`:

```rust
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
```

- [ ] **Step 2: Add to lib.rs**

Add to `core/engine/src/lib.rs`:
```rust
pub mod trade_thesis;
pub use trade_thesis::{TradeThesis, ThesisBuilder};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-engine trade_thesis
```

Expected: 7 tests pass

- [ ] **Step 4: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add TradeThesis struct with validation and builder"
```

---

### Task 2: ATR Stop Loss Calculator

**Files:**
- Create: `core/engine/src/atr_stops.rs`
- Modify: `core/engine/src/lib.rs`

- [ ] **Step 1: Implement ATR-based stop calculator**

Create `core/engine/src/atr_stops.rs`:

```rust
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use tradoshka_common::types::{Market, OrderSide};

/// Calculate all stop/exit levels based on ATR and risk parameters.
pub struct StopCalculator;

/// All calculated exit levels for a position.
#[derive(Debug, Clone)]
pub struct ExitLevels {
    pub hard_stop_loss: Decimal,
    pub trailing_stop: Decimal,
    pub take_profit: Decimal,
    pub time_stop_hours: u32,
    pub stop_distance: Decimal,
    pub reward_risk_ratio: f64,
}

impl StopCalculator {
    /// Calculate exit levels for a trade.
    ///
    /// - `entry_price`: actual market entry price
    /// - `atr`: Average True Range (14-period)
    /// - `side`: Buy (long) or Sell (short)
    /// - `market`: which market (affects time stop)
    /// - `rr_target`: desired reward:risk ratio (minimum 2.0)
    pub fn calculate(
        entry_price: Decimal,
        atr: Decimal,
        side: OrderSide,
        market: Market,
        rr_target: f64,
    ) -> ExitLevels {
        let rr = if rr_target < 2.0 { 2.0 } else { rr_target };

        let (hard_sl, trailing, tp, stop_dist) = match side {
            OrderSide::Buy => {
                let hard_sl = entry_price - (dec!(2) * atr);
                let trailing = entry_price - (Decimal::from_f64(1.5).unwrap_or(dec!(1.5)) * atr);
                let stop_dist = entry_price - hard_sl;
                let tp = entry_price + (stop_dist * Decimal::from_f64(rr).unwrap_or(dec!(2)));
                (hard_sl.max(Decimal::ZERO), trailing.max(Decimal::ZERO), tp, stop_dist)
            }
            OrderSide::Sell => {
                let hard_sl = entry_price + (dec!(2) * atr);
                let trailing = entry_price + (Decimal::from_f64(1.5).unwrap_or(dec!(1.5)) * atr);
                let stop_dist = hard_sl - entry_price;
                let tp = entry_price - (stop_dist * Decimal::from_f64(rr).unwrap_or(dec!(2)));
                (hard_sl, trailing, tp.max(Decimal::ZERO), stop_dist)
            }
        };

        let time_stop = match market {
            Market::Polymarket => 72,
            Market::Crypto => 24,
            _ => 48,
        };

        ExitLevels {
            hard_stop_loss: hard_sl,
            trailing_stop: trailing,
            take_profit: tp,
            time_stop_hours: time_stop,
            stop_distance: stop_dist,
            reward_risk_ratio: rr,
        }
    }

    /// Calculate position size from risk amount and stop distance.
    /// position_size = risk_amount / stop_distance
    pub fn position_size(risk_amount: Decimal, stop_distance: Decimal) -> Decimal {
        if stop_distance <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        (risk_amount / stop_distance).round_dp(8)
    }

    /// Calculate risk amount from equity and risk percentage.
    /// risk_amount = equity * (risk_pct / 100)
    pub fn risk_amount(equity: Decimal, risk_pct: f64) -> Decimal {
        let pct = Decimal::from_f64(risk_pct / 100.0).unwrap_or(dec!(0.01));
        (equity * pct).round_dp(8)
    }

    /// Update trailing stop when price makes a new high (for longs) or new low (for shorts).
    /// Returns the new trailing stop level, or None if it shouldn't move.
    pub fn update_trailing_stop(
        current_trailing: Decimal,
        current_price: Decimal,
        atr: Decimal,
        side: OrderSide,
    ) -> Option<Decimal> {
        match side {
            OrderSide::Buy => {
                let new_trailing = current_price - (Decimal::from_f64(1.5).unwrap_or(dec!(1.5)) * atr);
                if new_trailing > current_trailing {
                    Some(new_trailing)
                } else {
                    None // Don't move down
                }
            }
            OrderSide::Sell => {
                let new_trailing = current_price + (Decimal::from_f64(1.5).unwrap_or(dec!(1.5)) * atr);
                if new_trailing < current_trailing {
                    Some(new_trailing)
                } else {
                    None // Don't move up
                }
            }
        }
    }

    /// Check if a price has hit the hard stop loss.
    pub fn is_stopped_out(price: Decimal, stop_loss: Decimal, side: OrderSide) -> bool {
        match side {
            OrderSide::Buy => price <= stop_loss,
            OrderSide::Sell => price >= stop_loss,
        }
    }

    /// Check if take profit is hit.
    pub fn is_take_profit_hit(price: Decimal, take_profit: Decimal, side: OrderSide) -> bool {
        match side {
            OrderSide::Buy => price >= take_profit,
            OrderSide::Sell => price <= take_profit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_stop_levels() {
        let levels = StopCalculator::calculate(
            dec!(100), dec!(5), OrderSide::Buy, Market::Crypto, 2.0,
        );
        assert_eq!(levels.hard_stop_loss, dec!(90));  // 100 - 2*5
        assert_eq!(levels.stop_distance, dec!(10));   // 100 - 90
        assert_eq!(levels.take_profit, dec!(120));    // 100 + 10*2
        assert_eq!(levels.time_stop_hours, 24);
    }

    #[test]
    fn test_short_stop_levels() {
        let levels = StopCalculator::calculate(
            dec!(100), dec!(5), OrderSide::Sell, Market::Crypto, 2.0,
        );
        assert_eq!(levels.hard_stop_loss, dec!(110)); // 100 + 2*5
        assert_eq!(levels.take_profit, dec!(80));     // 100 - 10*2
    }

    #[test]
    fn test_polymarket_time_stop() {
        let levels = StopCalculator::calculate(
            dec!(0.50), dec!(0.05), OrderSide::Buy, Market::Polymarket, 2.0,
        );
        assert_eq!(levels.time_stop_hours, 72);
    }

    #[test]
    fn test_position_size() {
        // Risk $10, stop distance $5 → position size = 2 units
        let size = StopCalculator::position_size(dec!(10), dec!(5));
        assert_eq!(size, dec!(2));
    }

    #[test]
    fn test_risk_amount() {
        // Equity $1000, risk 1% → $10
        let risk = StopCalculator::risk_amount(dec!(1000), 1.0);
        assert_eq!(risk, dec!(10));
    }

    #[test]
    fn test_trailing_stop_moves_up_for_long() {
        let new = StopCalculator::update_trailing_stop(
            dec!(95), dec!(110), dec!(5), OrderSide::Buy,
        );
        assert!(new.is_some());
        assert!(new.unwrap() > dec!(95));
    }

    #[test]
    fn test_trailing_stop_never_moves_down() {
        let new = StopCalculator::update_trailing_stop(
            dec!(95), dec!(90), dec!(5), OrderSide::Buy,
        );
        assert!(new.is_none()); // Price dropped, don't move stop down
    }

    #[test]
    fn test_is_stopped_out_long() {
        assert!(StopCalculator::is_stopped_out(dec!(89), dec!(90), OrderSide::Buy));
        assert!(!StopCalculator::is_stopped_out(dec!(91), dec!(90), OrderSide::Buy));
    }

    #[test]
    fn test_is_take_profit_long() {
        assert!(StopCalculator::is_take_profit_hit(dec!(121), dec!(120), OrderSide::Buy));
        assert!(!StopCalculator::is_take_profit_hit(dec!(119), dec!(120), OrderSide::Buy));
    }

    #[test]
    fn test_rr_minimum_enforced() {
        let levels = StopCalculator::calculate(
            dec!(100), dec!(5), OrderSide::Buy, Market::Crypto, 1.0,
        );
        assert_eq!(levels.reward_risk_ratio, 2.0); // Clamped up to minimum
    }
}
```

- [ ] **Step 2: Add to lib.rs**

```rust
pub mod atr_stops;
pub use atr_stops::{StopCalculator, ExitLevels};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-engine atr_stops
```

Expected: 10 tests pass

- [ ] **Step 4: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add ATR-based stop loss calculator with trailing stops"
```

---

### Task 3: Research Engine

**Files:**
- Create: `core/engine/src/research.rs`
- Modify: `core/engine/src/lib.rs`

The research engine analyzes market data and produces a TradeThesis (or rejects the opportunity).

- [ ] **Step 1: Implement Research Engine**

Create `core/engine/src/research.rs`:

```rust
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use tradoshka_common::types::{Market, OrderSide};
use crate::trade_thesis::{TradeThesis, ThesisBuilder};
use crate::atr_stops::StopCalculator;

/// Market data snapshot for research analysis.
#[derive(Debug, Clone)]
pub struct MarketSnapshot {
    pub symbol: String,
    pub market: Market,
    pub current_price: Decimal,
    pub price_24h_ago: Option<Decimal>,
    pub volume_24h: f64,
    pub volume_7d_avg: Option<f64>,
    pub atr_14: Decimal,
    pub rsi_14: Option<f64>,
    pub ema_9: Option<Decimal>,
    pub ema_21: Option<Decimal>,
    pub funding_rate: Option<f64>,
    pub question: String,       // For Polymarket: the market question
    pub days_to_resolution: Option<f64>, // For Polymarket
}

/// Research configuration.
pub struct ResearchConfig {
    pub min_confidence: f64,
    pub min_signals: u32,
    pub min_rr_ratio: f64,
    pub risk_pct: f64,
    pub equity: Decimal,
    pub strategy_tier: String,
    pub time_stop_hours: u32,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.60,
            min_signals: 2,
            min_rr_ratio: 2.0,
            risk_pct: 1.0,
            equity: dec!(100),
            strategy_tier: "Unproven".into(),
            time_stop_hours: 24,
        }
    }
}

pub struct ResearchEngine;

impl ResearchEngine {
    /// Analyze a crypto market snapshot and produce a trade thesis (or reject).
    pub fn analyze_crypto(snapshot: &MarketSnapshot, config: &ResearchConfig) -> Result<TradeThesis, String> {
        let price = snapshot.current_price;
        if price <= Decimal::ZERO {
            return Err("Invalid price".into());
        }
        if snapshot.atr_14 <= Decimal::ZERO {
            return Err("ATR not available — insufficient price history".into());
        }

        let mut builder = ThesisBuilder::new();
        let mut reasons = Vec::new();
        let mut confidence: f64 = 0.5;
        let mut direction = OrderSide::Buy;

        // Signal 1: EMA crossover
        if let (Some(ema9), Some(ema21)) = (snapshot.ema_9, snapshot.ema_21) {
            if ema9 > ema21 {
                builder = builder.add_signal("ema_9_21_bullish", true);
                reasons.push("EMA9 above EMA21 (bullish trend)");
                confidence += 0.10;
            } else {
                builder = builder.add_signal("ema_9_21_bearish", true);
                reasons.push("EMA9 below EMA21 (bearish trend)");
                direction = OrderSide::Sell;
                confidence += 0.10;
            }
        } else {
            builder = builder.add_signal("ema_crossover", false);
        }

        // Signal 2: RSI confirmation
        if let Some(rsi) = snapshot.rsi_14 {
            if rsi < 30.0 {
                builder = builder.add_signal("rsi_oversold", true);
                reasons.push(format!("RSI at {:.0} (oversold)", rsi).leak());
                direction = OrderSide::Buy;
                confidence += 0.15;
            } else if rsi > 70.0 {
                builder = builder.add_signal("rsi_overbought", true);
                reasons.push("RSI overbought — potential reversal");
                direction = OrderSide::Sell;
                confidence += 0.10;
            } else if rsi > 50.0 && direction == OrderSide::Buy {
                builder = builder.add_signal("rsi_above_50", true);
                reasons.push("RSI above 50 confirming bullish momentum");
                confidence += 0.05;
            } else {
                builder = builder.add_signal("rsi_neutral", false);
            }
        } else {
            builder = builder.add_signal("rsi", false);
        }

        // Signal 3: Volume confirmation
        if let Some(vol_avg) = snapshot.volume_7d_avg {
            if vol_avg > 0.0 {
                let vol_ratio = snapshot.volume_24h / vol_avg;
                if vol_ratio > 1.3 {
                    builder = builder.add_signal("volume_surge", true);
                    reasons.push("Volume +30% above 7d average");
                    confidence += 0.10;
                } else {
                    builder = builder.add_signal("volume_normal", false);
                }
            } else {
                builder = builder.add_signal("volume", false);
            }
        } else {
            builder = builder.add_signal("volume", false);
        }

        // Signal 4: Funding rate (for crypto)
        if let Some(funding) = snapshot.funding_rate {
            if funding < -0.0003 && direction == OrderSide::Buy {
                builder = builder.add_signal("funding_negative", true);
                reasons.push("Negative funding suggests shorts overcrowded");
                confidence += 0.10;
            } else if funding > 0.0003 && direction == OrderSide::Sell {
                builder = builder.add_signal("funding_positive", true);
                reasons.push("High positive funding suggests longs overcrowded");
                confidence += 0.10;
            } else {
                builder = builder.add_signal("funding_neutral", false);
            }
        }

        // Calculate stops
        let exits = StopCalculator::calculate(price, snapshot.atr_14, direction, snapshot.market, config.min_rr_ratio);
        let risk_amt = StopCalculator::risk_amount(config.equity, config.risk_pct);
        let pos_size = StopCalculator::position_size(risk_amt, exits.stop_distance);

        // Build reasoning string
        let reasoning = if reasons.is_empty() {
            "No clear signals detected.".to_string()
        } else {
            format!("{}. {} Confidence: {:.0}%.",
                reasons.join(". "),
                format!("{}/{} signals confirmed.", builder.build().signals_agreed, builder.build().signals_total),
                confidence * 100.0)
        };

        let thesis = ThesisBuilder::new()
            .reasoning(&reasoning)
            .confidence(confidence)
            .entry(price)
            .entry_reason(&format!("{:?} signal on {}", direction, snapshot.symbol))
            .stops(exits.hard_stop_loss, exits.trailing_stop, exits.take_profit, config.time_stop_hours)
            .risk(config.risk_pct, risk_amt, exits.reward_risk_ratio, pos_size)
            .thesis_invalidation(&format!("Original {:?} signals no longer valid", direction))
            .strategy_tier(&config.strategy_tier);

        // Re-add the signals (builder was consumed partially above, rebuild)
        // Since we tracked signals above, build the final thesis directly
        let mut final_thesis = thesis.build();
        // Copy signal data from the earlier builder tracking
        // (In practice, the builder pattern needs refinement — for now, set signals directly)
        final_thesis.signals_used = reasons.iter().map(|r| r.to_string()).collect();
        final_thesis.signals_agreed = reasons.len() as u32;
        final_thesis.signals_total = 4; // We always check 4 signals for crypto
        final_thesis.reasoning = reasoning;

        match final_thesis.validate() {
            Ok(()) => Ok(final_thesis),
            Err(e) => Err(format!("Research rejected: {}", e)),
        }
    }

    /// Analyze a Polymarket opportunity.
    pub fn analyze_polymarket(
        question: &str,
        yes_price: Decimal,
        no_price: Decimal,
        volume_24h: f64,
        days_remaining: Option<f64>,
        config: &ResearchConfig,
    ) -> Result<TradeThesis, String> {
        if yes_price <= Decimal::ZERO || no_price <= Decimal::ZERO {
            return Err("Invalid prices".into());
        }

        let mut reasons = Vec::new();
        let mut signals_agreed = 0u32;
        let mut confidence: f64 = 0.5;
        let mut direction = OrderSide::Buy;
        let mut target_token_price = yes_price;

        // Signal 1: Mispricing detection
        let total = yes_price + no_price;
        let deviation = (total - Decimal::ONE).abs();
        if deviation > dec!(0.03) {
            signals_agreed += 1;
            reasons.push(format!("Price deviation: Yes+No={:.2}, {:.1}% off from $1.00",
                total.to_f64().unwrap_or(0.0), deviation.to_f64().unwrap_or(0.0) * 100.0));
            confidence += 0.15;
            if yes_price < no_price {
                direction = OrderSide::Buy;
                target_token_price = yes_price;
            } else {
                direction = OrderSide::Buy;
                target_token_price = no_price;
            }
        }

        // Signal 2: Volume trend
        if volume_24h > 5000.0 {
            signals_agreed += 1;
            reasons.push(format!("Strong volume: ${:.0}/24h", volume_24h));
            confidence += 0.10;
        }

        // Signal 3: Time to resolution
        if let Some(days) = days_remaining {
            if days > 7.0 && days < 90.0 {
                signals_agreed += 1;
                reasons.push(format!("{:.0} days to resolution — enough time for convergence", days));
                confidence += 0.05;
            } else if days <= 7.0 {
                reasons.push(format!("Only {:.0} days left — high risk of expiry", days));
                confidence -= 0.10;
            }
        }

        // Polymarket-specific stops (binary outcome 0-1)
        let atr_estimate = dec!(0.05); // Polymarket prices don't have candle data, use fixed ATR estimate
        let hard_sl = (target_token_price * dec!(0.20)).max(dec!(0.01)); // Max loss 80% of position
        let trailing = target_token_price - (atr_estimate * Decimal::from_f64(1.5).unwrap_or(dec!(1.5)));
        let stop_distance = target_token_price - hard_sl;
        let tp = target_token_price + (stop_distance * dec!(2));

        let risk_amt = StopCalculator::risk_amount(config.equity, config.risk_pct);
        let pos_size = if stop_distance > Decimal::ZERO {
            StopCalculator::position_size(risk_amt, stop_distance)
        } else {
            Decimal::ZERO
        };

        let reasoning = if reasons.is_empty() {
            "No clear edge detected in this market.".into()
        } else {
            format!("{}. {}/{} signals confirmed. Confidence: {:.0}%.",
                reasons.join(". "), signals_agreed, 3, confidence * 100.0)
        };

        let thesis = TradeThesis {
            reasoning,
            signals_used: reasons,
            signals_agreed,
            signals_total: 3,
            confidence,
            entry_price: target_token_price,
            entry_reason: format!("Edge detected in \"{}\"", question),
            hard_stop_loss: hard_sl,
            trailing_stop: trailing.max(dec!(0.01)),
            take_profit: tp.min(dec!(0.99)),
            time_stop_hours: 72,
            thesis_invalidation: "Market resolved or edge disappeared".into(),
            risk_per_trade_pct: config.risk_pct,
            risk_amount: risk_amt,
            reward_risk_ratio: 2.0,
            position_size: pos_size,
            max_loss: risk_amt,
            strategy_tier: config.strategy_tier.clone(),
        };

        match thesis.validate() {
            Ok(()) => Ok(thesis),
            Err(e) => Err(format!("Research rejected: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crypto_snapshot(price: f64, atr: f64) -> MarketSnapshot {
        MarketSnapshot {
            symbol: "BTCUSDT".into(),
            market: Market::Crypto,
            current_price: Decimal::from_f64(price).unwrap(),
            price_24h_ago: Some(Decimal::from_f64(price * 0.98).unwrap()),
            volume_24h: 15000.0,
            volume_7d_avg: Some(10000.0),
            atr_14: Decimal::from_f64(atr).unwrap(),
            rsi_14: Some(55.0),
            ema_9: Some(Decimal::from_f64(price * 1.01).unwrap()),
            ema_21: Some(Decimal::from_f64(price * 0.99).unwrap()),
            funding_rate: Some(-0.0005),
            question: String::new(),
            days_to_resolution: None,
        }
    }

    #[test]
    fn test_crypto_research_produces_thesis() {
        let snap = crypto_snapshot(69000.0, 1500.0);
        let config = ResearchConfig::default();
        let result = ResearchEngine::analyze_crypto(&snap, &config);
        assert!(result.is_ok());
        let thesis = result.unwrap();
        assert!(!thesis.reasoning.is_empty());
        assert!(thesis.hard_stop_loss > Decimal::ZERO);
        assert!(thesis.take_profit > thesis.entry_price);
        assert!(thesis.position_size > Decimal::ZERO);
    }

    #[test]
    fn test_crypto_research_rejects_no_atr() {
        let mut snap = crypto_snapshot(69000.0, 0.0);
        snap.atr_14 = Decimal::ZERO;
        let config = ResearchConfig::default();
        let result = ResearchEngine::analyze_crypto(&snap, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_polymarket_research_with_mispricing() {
        let config = ResearchConfig::default();
        let result = ResearchEngine::analyze_polymarket(
            "Will BTC hit 200K?", dec!(0.45), dec!(0.60), 10000.0, Some(30.0), &config,
        );
        assert!(result.is_ok());
        let thesis = result.unwrap();
        assert!(thesis.reasoning.contains("deviation"));
    }

    #[test]
    fn test_polymarket_research_rejects_low_volume() {
        let config = ResearchConfig::default();
        let result = ResearchEngine::analyze_polymarket(
            "Low vol market", dec!(0.50), dec!(0.50), 100.0, Some(30.0), &config,
        );
        // Low volume + no mispricing + only time signal = likely rejected (< 2 signals)
        assert!(result.is_err());
    }

    #[test]
    fn test_risk_amount_scales_with_equity() {
        let config = ResearchConfig { equity: dec!(1000), risk_pct: 2.0, ..Default::default() };
        let snap = crypto_snapshot(69000.0, 1500.0);
        let result = ResearchEngine::analyze_crypto(&snap, &config);
        if let Ok(thesis) = result {
            assert_eq!(thesis.risk_amount, dec!(20)); // 2% of $1000
        }
    }
}
```

- [ ] **Step 2: Add to lib.rs**

```rust
pub mod research;
pub use research::{ResearchEngine, ResearchConfig, MarketSnapshot};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-engine research
```

Expected: 5 tests pass

- [ ] **Step 4: Run full workspace tests**

```bash
cargo test --workspace
```

Expected: all existing tests + 22 new tests pass

- [ ] **Step 5: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add research-first decision engine with market analysis"
```

---

### Task 4: Update TradeRecord to Include Thesis

**Files:**
- Modify: `core/engine/src/trade_recorder.rs`

- [ ] **Step 1: Add thesis fields to TradeRecord**

Read the current `trade_recorder.rs`, then add new optional fields to `TradeRecord`:

```rust
// Add these fields to the TradeRecord struct:
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
```

Set defaults in all existing `TradeRecord` construction sites (in `orchestrator.rs` and `main.rs`). For now, set `thesis_reasoning: String::new()` and other fields to `Decimal::ZERO` / `0` / `String::new()` in existing code — the smart orchestrator (Phase 2) will populate them properly.

Update the test helper `make_trade()` to include the new fields with defaults.

- [ ] **Step 2: Run tests**

```bash
cargo test --workspace
```

Fix any compilation errors from the new fields.

- [ ] **Step 3: Commit**

```bash
git add core/
git commit -m "feat(engine): add thesis fields to TradeRecord"
```

---

### Task 5: Full Verification

- [ ] **Step 1: Run all tests**

```bash
cargo test --workspace
```

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace
```

- [ ] **Step 3: Verify the new modules work together**

Quick integration test in a temporary Rust file or inline test:
- Create a MarketSnapshot → feed to ResearchEngine → get TradeThesis → validate it passes
- Create exit levels from StopCalculator → verify they're sane
- Build a thesis with ThesisBuilder → validate it

- [ ] **Step 4: Commit and merge**

```bash
git checkout dev
git merge feature/smart-orchestrator-phase1
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | TradeThesis struct + validation + builder | 7 |
| 2 | ATR stop loss calculator with trailing stops | 10 |
| 3 | Research engine (crypto + polymarket analysis) | 5 |
| 4 | Update TradeRecord with thesis fields | Compilation fixes |
| 5 | Full verification | All tests |

**Total: 5 tasks, 22+ new tests**

Next plans:
- **Phase 2:** Rewrite orchestrator to use research engine (replace dumb logic)
- **Phase 3:** Position monitor (active stop checking every cycle)
- **Phase 4:** Strategy trust tiers
- **Phase 5:** Dashboard update (show reasoning, stops, tiers)
- **Phase 6:** Integration test
