use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct TokenVolumeState {
    pub volume_history: Vec<(DateTime<Utc>, f64)>,  // (timestamp, 5min_volume)
    pub peak_volume_5m: f64,
    pub current_volume_5m: f64,
    pub holder_count: u32,
    pub prev_holder_count: u32,
    pub holder_growth_rate: f64,  // holders per minute
}

impl TokenVolumeState {
    pub fn update(&mut self, volume_5m: f64, holders: u32) {
        let now = Utc::now();
        self.current_volume_5m = volume_5m;
        if volume_5m > self.peak_volume_5m {
            self.peak_volume_5m = volume_5m;
        }
        self.volume_history.push((now, volume_5m));
        // Keep last 24 entries (2 hours at 5-min intervals)
        if self.volume_history.len() > 24 {
            self.volume_history.remove(0);
        }
        // Holder tracking
        self.prev_holder_count = self.holder_count;
        self.holder_count = holders;
        if self.prev_holder_count > 0 {
            self.holder_growth_rate = (holders as f64 - self.prev_holder_count as f64) / 5.0; // per minute
        }
    }

    /// Volume has dropped below 50% of peak — pump is likely over
    pub fn is_volume_cliff(&self) -> bool {
        self.peak_volume_5m > 0.0 && self.current_volume_5m < self.peak_volume_5m * 0.5
    }

    /// Holders are no longer growing — distribution phase
    pub fn is_holder_stalling(&self) -> bool {
        self.holder_growth_rate < 1.0 && self.holder_count > 10
    }

    /// Volume ratio: current / peak (1.0 = at peak, 0.5 = half of peak)
    pub fn volume_ratio(&self) -> f64 {
        if self.peak_volume_5m > 0.0 {
            self.current_volume_5m / self.peak_volume_5m
        } else { 0.0 }
    }
}

pub struct VolumeTracker {
    tokens: HashMap<String, TokenVolumeState>,
}

impl VolumeTracker {
    pub fn new() -> Self {
        Self { tokens: HashMap::new() }
    }

    pub fn update(&mut self, token_address: &str, volume_5m: f64, holders: u32) {
        let state = self.tokens.entry(token_address.into()).or_default();
        state.update(volume_5m, holders);
    }

    pub fn get(&self, token_address: &str) -> Option<&TokenVolumeState> {
        self.tokens.get(token_address)
    }

    pub fn should_exit(&self, token_address: &str) -> Option<&str> {
        if let Some(state) = self.tokens.get(token_address) {
            if state.is_volume_cliff() { return Some("volume_cliff"); }
            if state.is_holder_stalling() { return Some("holder_stalling"); }
        }
        None
    }

    pub fn tracked_count(&self) -> usize { self.tokens.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_cliff_detection() {
        let mut tracker = VolumeTracker::new();
        // Build up peak volume
        tracker.update("tok1", 10000.0, 50);
        tracker.update("tok1", 12000.0, 60);  // peak is 12000
        // Volume drops below 50%
        tracker.update("tok1", 5000.0, 65);   // 5000 < 12000 * 0.5 = 6000 → cliff
        assert_eq!(tracker.should_exit("tok1"), Some("volume_cliff"));
    }

    #[test]
    fn test_holder_stalling() {
        let mut tracker = VolumeTracker::new();
        // First update sets holder_count but prev is 0 so growth_rate is 0
        tracker.update("tok2", 8000.0, 20);
        // Second update: growth_rate = (20 - 20) / 5 = 0.0 → stalling (< 1.0) with holders > 10
        tracker.update("tok2", 7500.0, 20);
        let state = tracker.get("tok2").unwrap();
        assert!(state.is_holder_stalling());
        // Should trigger holder_stalling exit (volume not a cliff here since 7500 > 8000 * 0.5)
        assert_eq!(tracker.should_exit("tok2"), Some("holder_stalling"));
    }

    #[test]
    fn test_volume_ratio() {
        let mut state = TokenVolumeState::default();
        state.update(10000.0, 30);
        state.update(8000.0, 35);
        // peak is 10000, current is 8000 → ratio = 0.8
        let ratio = state.volume_ratio();
        assert!((ratio - 0.8).abs() < 0.001, "Expected ratio ~0.8, got {}", ratio);
    }

    #[test]
    fn test_normal_no_exit() {
        let mut tracker = VolumeTracker::new();
        tracker.update("tok3", 5000.0, 5);   // holders <= 10, no stall trigger
        tracker.update("tok3", 6000.0, 8);   // volume at peak, no cliff
        // No exit condition
        assert_eq!(tracker.should_exit("tok3"), None);
    }

    #[test]
    fn test_peak_tracking() {
        let mut state = TokenVolumeState::default();
        state.update(1000.0, 10);
        state.update(5000.0, 20);
        state.update(3000.0, 25);
        state.update(8000.0, 30);  // new peak
        state.update(4000.0, 35);
        assert_eq!(state.peak_volume_5m, 8000.0);
        assert_eq!(state.current_volume_5m, 4000.0);
    }
}
