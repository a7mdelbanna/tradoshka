use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakerStatus {
    Clear,
    SizeFiltered,
    SequenceDetected,
    InsufficientDepth,
    DailyLossHalt,
    MarketLimitHit,
    ConsecutiveLossReduction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerCheck {
    pub status: BreakerStatus,
    pub reason: String,
    pub size_multiplier: f64, // 1.0 = full, 0.5 = reduced, 0.0 = blocked
}

pub struct CopyCircuitBreaker {
    // Layer 1: Size filter
    min_whale_trade: f64,
    max_whale_trade: f64,
    // Layer 2: Sequence detection
    recent_trades: Vec<(String, DateTime<Utc>)>, // (wallet, timestamp)
    max_trades_per_30s: usize,
    // Layer 3: Depth
    min_book_depth: f64,
    // Layer 4: Portfolio
    daily_loss: f64,
    daily_loss_limit_pct: f64,
    portfolio_equity: f64,
    consecutive_losses: u32,
    halted_until: Option<DateTime<Utc>>,
    market_cooldowns: std::collections::HashMap<String, DateTime<Utc>>,
}

impl CopyCircuitBreaker {
    pub fn new(portfolio_equity: f64) -> Self {
        Self {
            min_whale_trade: 10.0,
            max_whale_trade: 50000.0,
            recent_trades: Vec::new(),
            max_trades_per_30s: 3,
            min_book_depth: 200.0,
            daily_loss: 0.0,
            daily_loss_limit_pct: 0.05,
            portfolio_equity,
            consecutive_losses: 0,
            halted_until: None,
            market_cooldowns: std::collections::HashMap::new(),
        }
    }

    /// Run all 4 layers of checks.
    pub fn check(
        &mut self,
        whale_trade_size: f64,
        whale_address: &str,
        market_id: &str,
        book_depth: f64,
    ) -> BreakerCheck {
        // Check halt
        if let Some(until) = self.halted_until {
            if Utc::now() < until {
                return BreakerCheck {
                    status: BreakerStatus::DailyLossHalt,
                    reason: format!("Trading halted until {}", until.format("%H:%M")),
                    size_multiplier: 0.0,
                };
            } else {
                self.halted_until = None;
            }
        }

        // Check market cooldown
        if let Some(until) = self.market_cooldowns.get(market_id) {
            if Utc::now() < *until {
                return BreakerCheck {
                    status: BreakerStatus::MarketLimitHit,
                    reason: format!("Market {} in cooldown", market_id),
                    size_multiplier: 0.0,
                };
            }
        }

        // Layer 1: Size filter
        if whale_trade_size < self.min_whale_trade {
            return BreakerCheck {
                status: BreakerStatus::SizeFiltered,
                reason: format!("Whale trade ${:.0} below min ${:.0}", whale_trade_size, self.min_whale_trade),
                size_multiplier: 0.0,
            };
        }
        if whale_trade_size > self.max_whale_trade {
            return BreakerCheck {
                status: BreakerStatus::SizeFiltered,
                reason: format!("Whale trade ${:.0} above max ${:.0} (manipulation risk)", whale_trade_size, self.max_whale_trade),
                size_multiplier: 0.0,
            };
        }

        // Layer 2: Sequence detection
        let now = Utc::now();
        let cutoff = now - Duration::seconds(30);
        self.recent_trades.retain(|(_, ts)| *ts > cutoff);
        let wallet_trades_in_window = self.recent_trades.iter()
            .filter(|(w, _)| w == whale_address)
            .count();
        self.recent_trades.push((whale_address.into(), now));

        if wallet_trades_in_window >= self.max_trades_per_30s {
            return BreakerCheck {
                status: BreakerStatus::SequenceDetected,
                reason: format!("Wallet {} made {}+ trades in 30s (manipulation)", whale_address, self.max_trades_per_30s),
                size_multiplier: 0.0,
            };
        }

        // Layer 3: Book depth
        if book_depth < self.min_book_depth {
            return BreakerCheck {
                status: BreakerStatus::InsufficientDepth,
                reason: format!("Book depth ${:.0} below min ${:.0}", book_depth, self.min_book_depth),
                size_multiplier: 0.0,
            };
        }

        // Layer 4: Consecutive losses
        let size_mult = if self.consecutive_losses >= 3 { 0.5 } else { 1.0 };

        BreakerCheck {
            status: if size_mult < 1.0 { BreakerStatus::ConsecutiveLossReduction } else { BreakerStatus::Clear },
            reason: if size_mult < 1.0 {
                format!("{} consecutive losses — size reduced 50%", self.consecutive_losses)
            } else {
                "All checks passed".into()
            },
            size_multiplier: size_mult,
        }
    }

    /// Record a trade result (win/loss).
    pub fn record_result(&mut self, pnl: f64, market_id: &str) {
        if pnl < 0.0 {
            self.daily_loss += pnl.abs();
            self.consecutive_losses += 1;

            // Check daily loss limit
            let limit = self.portfolio_equity * self.daily_loss_limit_pct;
            if self.daily_loss >= limit {
                self.halted_until = Some(Utc::now() + Duration::hours(6));
            }

            // Check per-market limit (10%)
            let market_limit = self.portfolio_equity * 0.10;
            if pnl.abs() >= market_limit {
                self.market_cooldowns.insert(market_id.into(), Utc::now() + Duration::hours(24));
            }
        } else {
            self.consecutive_losses = 0;
        }
    }

    /// Reset daily counters (call at midnight).
    pub fn reset_daily(&mut self) {
        self.daily_loss = 0.0;
    }

    /// Manually clear the trading halt (operator override or end of halt period).
    pub fn clear_halt(&mut self) {
        self.halted_until = None;
    }

    pub fn is_halted(&self) -> bool {
        self.halted_until.map_or(false, |until| Utc::now() < until)
    }

    pub fn consecutive_losses(&self) -> u32 {
        self.consecutive_losses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_on_normal_trade() {
        let mut breaker = CopyCircuitBreaker::new(10_000.0);
        let result = breaker.check(500.0, "0xwhale", "mkt-001", 500.0);
        assert_eq!(result.status, BreakerStatus::Clear);
        assert_eq!(result.size_multiplier, 1.0);
    }

    #[test]
    fn test_size_filter_too_small() {
        let mut breaker = CopyCircuitBreaker::new(10_000.0);
        // Trade of $5 is below $10 minimum
        let result = breaker.check(5.0, "0xwhale", "mkt-001", 500.0);
        assert_eq!(result.status, BreakerStatus::SizeFiltered);
        assert_eq!(result.size_multiplier, 0.0);
        assert!(result.reason.contains("below min"));
    }

    #[test]
    fn test_size_filter_too_large() {
        let mut breaker = CopyCircuitBreaker::new(10_000.0);
        // Trade of $100,000 is above $50,000 maximum
        let result = breaker.check(100_000.0, "0xwhale", "mkt-001", 500.0);
        assert_eq!(result.status, BreakerStatus::SizeFiltered);
        assert_eq!(result.size_multiplier, 0.0);
        assert!(result.reason.contains("above max"));
    }

    #[test]
    fn test_sequence_detection() {
        let mut breaker = CopyCircuitBreaker::new(10_000.0);
        // Make 3 trades from same wallet (max_trades_per_30s = 3)
        // First 3 should pass (count is 0, 1, 2 when checked before push)
        breaker.check(500.0, "0xspammer", "mkt-001", 500.0);
        breaker.check(500.0, "0xspammer", "mkt-002", 500.0);
        breaker.check(500.0, "0xspammer", "mkt-003", 500.0);
        // 4th trade: wallet_trades_in_window = 3 >= max_trades_per_30s = 3 → blocked
        let result = breaker.check(500.0, "0xspammer", "mkt-004", 500.0);
        assert_eq!(result.status, BreakerStatus::SequenceDetected);
        assert_eq!(result.size_multiplier, 0.0);
    }

    #[test]
    fn test_insufficient_depth() {
        let mut breaker = CopyCircuitBreaker::new(10_000.0);
        // Book depth of $100 is below $200 minimum
        let result = breaker.check(500.0, "0xwhale", "mkt-001", 100.0);
        assert_eq!(result.status, BreakerStatus::InsufficientDepth);
        assert_eq!(result.size_multiplier, 0.0);
        assert!(result.reason.contains("below min"));
    }

    #[test]
    fn test_daily_loss_halt() {
        let mut breaker = CopyCircuitBreaker::new(10_000.0);
        // 5% daily loss limit = $500. Record a $600 loss → triggers halt
        breaker.record_result(-600.0, "mkt-001");
        assert!(breaker.is_halted());

        let result = breaker.check(500.0, "0xwhale", "mkt-002", 500.0);
        assert_eq!(result.status, BreakerStatus::DailyLossHalt);
        assert_eq!(result.size_multiplier, 0.0);
        assert!(result.reason.contains("halted until"));
    }

    #[test]
    fn test_consecutive_loss_reduction() {
        let mut breaker = CopyCircuitBreaker::new(10_000.0);
        // Record 3 consecutive losses (small enough not to trigger daily halt)
        breaker.record_result(-10.0, "mkt-001");
        breaker.record_result(-10.0, "mkt-002");
        breaker.record_result(-10.0, "mkt-003");

        assert_eq!(breaker.consecutive_losses(), 3);

        let result = breaker.check(500.0, "0xwhale", "mkt-004", 500.0);
        assert_eq!(result.status, BreakerStatus::ConsecutiveLossReduction);
        assert_eq!(result.size_multiplier, 0.5);
        assert!(result.reason.contains("consecutive losses"));
    }

    #[test]
    fn test_market_cooldown() {
        let mut breaker = CopyCircuitBreaker::new(10_000.0);
        // Record a loss >= 10% of portfolio ($1000) → market goes into 24h cooldown.
        // The same loss also exceeds the 5% daily limit ($500) and sets a halt;
        // clear_halt() simulates an operator override so we can verify the market
        // cooldown fires independently on the next check for that market.
        breaker.record_result(-1500.0, "mkt-hot");
        breaker.clear_halt();
        breaker.reset_daily();

        let result = breaker.check(500.0, "0xwhale", "mkt-hot", 500.0);
        assert_eq!(result.status, BreakerStatus::MarketLimitHit);
        assert_eq!(result.size_multiplier, 0.0);
        assert!(result.reason.contains("cooldown"));
    }
}
