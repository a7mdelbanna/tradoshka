use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemeWhale {
    pub address: String,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub total_pnl: f64,
    pub avg_entry_age_mins: f64,
    pub last_active: DateTime<Utc>,
}

impl MemeWhale {
    pub fn win_rate(&self) -> f64 {
        if self.total_trades == 0 { return 0.0; }
        self.winning_trades as f64 / self.total_trades as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhaleTrade {
    pub whale_address: String,
    pub token_address: String,
    pub token_symbol: String,
    pub side: String,
    pub size_usd: f64,
    pub price: f64,
    pub timestamp: DateTime<Utc>,
}

pub struct MemeWhaleTracker {
    whales: HashMap<String, MemeWhale>,
    recent_trades: Vec<WhaleTrade>,
}

impl MemeWhaleTracker {
    pub fn new() -> Self {
        Self {
            whales: HashMap::new(),
            recent_trades: Vec::new(),
        }
    }

    pub fn add_whale(&mut self, whale: MemeWhale) {
        self.whales.insert(whale.address.clone(), whale);
    }

    pub fn record_trade(&mut self, trade: WhaleTrade) {
        self.recent_trades.push(trade);
        if self.recent_trades.len() > 500 {
            self.recent_trades = self.recent_trades.split_off(self.recent_trades.len() - 500);
        }
    }

    pub fn whale_count(&self) -> usize { self.whales.len() }

    pub fn recent_buys(&self, token_address: &str) -> Vec<&WhaleTrade> {
        self.recent_trades.iter()
            .filter(|t| t.token_address == token_address && t.side == "BUY")
            .collect()
    }

    pub fn consensus_count(&self, token_address: &str) -> usize {
        let mut wallets = std::collections::HashSet::new();
        for t in &self.recent_trades {
            if t.token_address == token_address && t.side == "BUY" {
                wallets.insert(t.whale_address.clone());
            }
        }
        wallets.len()
    }

    pub fn get_whale(&self, address: &str) -> Option<&MemeWhale> {
        self.whales.get(address)
    }

    pub fn top_whales(&self, limit: usize) -> Vec<&MemeWhale> {
        let mut sorted: Vec<&MemeWhale> = self.whales.values().collect();
        sorted.sort_by(|a, b| b.total_pnl.partial_cmp(&a.total_pnl).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(limit);
        sorted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whale_tracker() {
        let mut tracker = MemeWhaleTracker::new();
        tracker.add_whale(MemeWhale {
            address: "whale1".into(), total_trades: 50, winning_trades: 35,
            total_pnl: 5000.0, avg_entry_age_mins: 3.0, last_active: Utc::now(),
        });
        assert_eq!(tracker.whale_count(), 1);
        assert!((tracker.get_whale("whale1").unwrap().win_rate() - 0.70).abs() < 0.01);
    }

    #[test]
    fn test_consensus() {
        let mut tracker = MemeWhaleTracker::new();
        tracker.record_trade(WhaleTrade {
            whale_address: "w1".into(), token_address: "tok1".into(),
            token_symbol: "T".into(), side: "BUY".into(), size_usd: 1000.0,
            price: 0.001, timestamp: Utc::now(),
        });
        tracker.record_trade(WhaleTrade {
            whale_address: "w2".into(), token_address: "tok1".into(),
            token_symbol: "T".into(), side: "BUY".into(), size_usd: 2000.0,
            price: 0.001, timestamp: Utc::now(),
        });
        assert_eq!(tracker.consensus_count("tok1"), 2);
        assert_eq!(tracker.consensus_count("tok2"), 0);
    }
}
