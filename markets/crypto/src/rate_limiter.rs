use std::time::Instant;
use tracing::warn;

pub struct RateLimiter {
    weight_limit: u32,
    weight_used: u32,
    window_start: Instant,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(weight_limit: u32, window_secs: u64) -> Self {
        Self { weight_limit, weight_used: 0, window_start: Instant::now(), window_secs }
    }

    pub fn check(&mut self, weight: u32) -> bool {
        self.maybe_reset();
        if self.weight_used + weight > self.weight_limit {
            warn!("Rate limit would be exceeded: {}/{}", self.weight_used + weight, self.weight_limit);
            return false;
        }
        true
    }

    pub fn record(&mut self, weight: u32) {
        self.maybe_reset();
        self.weight_used += weight;
    }

    pub fn update_from_header(&mut self, used_weight: u32) {
        self.weight_used = used_weight;
    }

    pub fn remaining(&self) -> u32 {
        self.weight_limit.saturating_sub(self.weight_used)
    }

    pub fn usage_pct(&self) -> f64 {
        self.weight_used as f64 / self.weight_limit as f64 * 100.0
    }

    fn maybe_reset(&mut self) {
        if self.window_start.elapsed().as_secs() >= self.window_secs {
            self.weight_used = 0;
            self.window_start = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_within_limit() {
        let mut rl = RateLimiter::new(100, 60);
        assert!(rl.check(50));
        rl.record(50);
        assert!(rl.check(50));
    }

    #[test]
    fn test_rejects_over_limit() {
        let mut rl = RateLimiter::new(100, 60);
        rl.record(90);
        assert!(!rl.check(20));
    }

    #[test]
    fn test_remaining() {
        let mut rl = RateLimiter::new(100, 60);
        rl.record(30);
        assert_eq!(rl.remaining(), 70);
    }

    #[test]
    fn test_usage_pct() {
        let mut rl = RateLimiter::new(100, 60);
        rl.record(50);
        assert!((rl.usage_pct() - 50.0).abs() < 0.01);
    }
}
