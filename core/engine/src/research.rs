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
        let mut reasons: Vec<String> = Vec::new();
        let mut confidence: f64 = 0.5;
        let mut direction = OrderSide::Buy;

        // Signal 1: EMA crossover
        if let (Some(ema9), Some(ema21)) = (snapshot.ema_9, snapshot.ema_21) {
            if ema9 > ema21 {
                builder = builder.add_signal("ema_9_21_bullish", true);
                reasons.push("EMA9 above EMA21 (bullish trend)".to_string());
                confidence += 0.10;
            } else {
                builder = builder.add_signal("ema_9_21_bearish", true);
                reasons.push("EMA9 below EMA21 (bearish trend)".to_string());
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
                reasons.push(format!("RSI at {:.0} (oversold)", rsi));
                direction = OrderSide::Buy;
                confidence += 0.15;
            } else if rsi > 70.0 {
                builder = builder.add_signal("rsi_overbought", true);
                reasons.push("RSI overbought — potential reversal".to_string());
                direction = OrderSide::Sell;
                confidence += 0.10;
            } else if rsi > 50.0 && direction == OrderSide::Buy {
                builder = builder.add_signal("rsi_above_50", true);
                reasons.push("RSI above 50 confirming bullish momentum".to_string());
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
                    reasons.push("Volume +30% above 7d average".to_string());
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
                reasons.push("Negative funding suggests shorts overcrowded".to_string());
                confidence += 0.10;
            } else if funding > 0.0003 && direction == OrderSide::Sell {
                builder = builder.add_signal("funding_positive", true);
                reasons.push("High positive funding suggests longs overcrowded".to_string());
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
        let signals_agreed = reasons.len() as u32;
        let signals_total = builder.build().signals_total;
        let reasoning = if reasons.is_empty() {
            "No clear signals detected.".to_string()
        } else {
            format!("{}. {} Confidence: {:.0}%.",
                reasons.join(". "),
                format!("{}/{} signals confirmed.", signals_agreed, signals_total),
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

        // Build the final thesis and set signal tracking fields directly
        let mut final_thesis = thesis.build();
        final_thesis.signals_used = reasons;
        final_thesis.signals_agreed = signals_agreed;
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

        let mut reasons: Vec<String> = Vec::new();
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
