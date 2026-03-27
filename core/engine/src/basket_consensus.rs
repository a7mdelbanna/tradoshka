use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::wallet_scorer::ScoredWallet;

/// A topic-based basket of wallets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Basket {
    pub name: String,
    pub niche: String,
    pub wallet_addresses: Vec<String>,
    pub min_consensus_pct: f64,
}

impl Basket {
    pub fn new(name: &str, niche: &str, min_consensus: f64) -> Self {
        Self {
            name: name.into(),
            niche: niche.into(),
            wallet_addresses: Vec::new(),
            min_consensus_pct: min_consensus,
        }
    }

    pub fn add_wallet(&mut self, address: &str) {
        if !self.wallet_addresses.contains(&address.to_string()) {
            self.wallet_addresses.push(address.into());
        }
    }

    pub fn wallet_count(&self) -> usize {
        self.wallet_addresses.len()
    }
}

/// A recorded position taken by a wallet in a market.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletPosition {
    pub wallet_address: String,
    pub market_id: String,
    pub side: String,
    pub price: f64,
    pub size: f64,
    pub timestamp: DateTime<Utc>,
}

/// Result of a consensus check on a market.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub market_id: String,
    pub market_question: String,
    pub consensus_side: String,
    pub consensus_pct: f64,
    pub wallets_agreeing: usize,
    pub wallets_total: usize,
    pub avg_entry_price: f64,
    pub basket_name: String,
    pub triggered: bool,
    pub reason: String,
}

/// Adaptive position sizing based on whale trade size.
#[derive(Debug, Clone)]
pub struct AdaptiveSizer;

impl AdaptiveSizer {
    pub fn multiplier(whale_trade_size: f64) -> f64 {
        if whale_trade_size < 50.0 { 2.0 }
        else if whale_trade_size < 200.0 { 1.0 }
        else { 0.5 }
    }

    pub fn calculate_size(
        whale_avg_size: f64,
        portfolio_equity: f64,
        max_per_market_pct: f64,
    ) -> f64 {
        let multiplied = whale_avg_size * Self::multiplier(whale_avg_size);
        let max_allowed = portfolio_equity * max_per_market_pct;
        multiplied.min(max_allowed).max(1.0)
    }
}

/// The basket consensus engine.
pub struct BasketConsensus {
    baskets: HashMap<String, Basket>,
    positions: Vec<WalletPosition>,
    consensus_threshold: f64,
    price_min: f64,
    price_max: f64,
    max_positions: usize,
    current_positions: usize,
}

impl BasketConsensus {
    pub fn new() -> Self {
        Self {
            baskets: HashMap::new(),
            positions: Vec::new(),
            consensus_threshold: 0.80,
            price_min: 0.20,
            price_max: 0.80,
            max_positions: 20,
            current_positions: 0,
        }
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.consensus_threshold = threshold;
        self
    }

    pub fn with_price_range(mut self, min: f64, max: f64) -> Self {
        self.price_min = min;
        self.price_max = max;
        self
    }

    /// Add a basket.
    pub fn add_basket(&mut self, basket: Basket) {
        self.baskets.insert(basket.name.clone(), basket);
    }

    /// Build default baskets from scored wallets.
    pub fn build_baskets_from_wallets(&mut self, wallets: &[&ScoredWallet]) {
        let niches = ["politics", "sports", "crypto", "general"];
        for niche in &niches {
            let mut basket = Basket::new(
                &format!("{}_basket", niche),
                niche,
                self.consensus_threshold,
            );
            for wallet in wallets {
                if wallet.niche == *niche || *niche == "general" {
                    basket.add_wallet(&wallet.address);
                }
            }
            if basket.wallet_count() >= 3 {
                self.add_basket(basket);
            }
        }
    }

    /// Record a wallet position (from API polling).
    pub fn record_position(&mut self, position: WalletPosition) {
        // Remove old position for same wallet+market
        self.positions.retain(|p| {
            !(p.wallet_address == position.wallet_address && p.market_id == position.market_id)
        });
        self.positions.push(position);

        // Trim old positions (keep last 1000)
        if self.positions.len() > 1000 {
            self.positions = self.positions.split_off(self.positions.len() - 1000);
        }
    }

    /// Check consensus for a specific market across all baskets.
    pub fn check_consensus(
        &self,
        market_id: &str,
        market_question: &str,
        current_price: f64,
    ) -> Vec<ConsensusResult> {
        let mut results = Vec::new();

        // Price range check
        if current_price < self.price_min || current_price > self.price_max {
            return results;
        }

        // Check if too many open positions
        if self.current_positions >= self.max_positions {
            return results;
        }

        for (basket_name, basket) in &self.baskets {
            // Get positions for this market from basket wallets
            let basket_positions: Vec<&WalletPosition> = self.positions.iter()
                .filter(|p| {
                    p.market_id == market_id
                        && basket.wallet_addresses.contains(&p.wallet_address)
                })
                .collect();

            if basket_positions.is_empty() {
                continue;
            }

            // Count sides
            let yes_count = basket_positions.iter().filter(|p| p.side == "YES").count();
            let no_count = basket_positions.iter().filter(|p| p.side == "NO").count();
            let total = basket.wallet_count();

            if total == 0 { continue; }

            let (consensus_side, agreeing) = if yes_count >= no_count {
                ("YES".to_string(), yes_count)
            } else {
                ("NO".to_string(), no_count)
            };

            let consensus_pct = agreeing as f64 / total as f64;

            // Calculate average entry price of agreeing wallets
            let avg_price = basket_positions.iter()
                .filter(|p| p.side == consensus_side)
                .map(|p| p.price)
                .sum::<f64>() / agreeing.max(1) as f64;

            let triggered = consensus_pct >= basket.min_consensus_pct;
            let reason = if triggered {
                format!("{}/{} wallets agree on {} ({:.0}%)",
                    agreeing, total, consensus_side, consensus_pct * 100.0)
            } else {
                format!("Only {}/{} wallets agree ({:.0}% < {:.0}% threshold)",
                    agreeing, total, consensus_pct * 100.0, basket.min_consensus_pct * 100.0)
            };

            results.push(ConsensusResult {
                market_id: market_id.into(),
                market_question: market_question.into(),
                consensus_side,
                consensus_pct,
                wallets_agreeing: agreeing,
                wallets_total: total,
                avg_entry_price: avg_price,
                basket_name: basket_name.clone(),
                triggered,
                reason,
            });
        }

        results
    }

    /// Get all baskets.
    pub fn baskets(&self) -> &HashMap<String, Basket> {
        &self.baskets
    }

    /// Get basket count.
    pub fn basket_count(&self) -> usize {
        self.baskets.len()
    }

    /// Update current position count.
    pub fn set_current_positions(&mut self, count: usize) {
        self.current_positions = count;
    }

    /// Get all recent positions.
    pub fn recent_positions(&self) -> &[WalletPosition] {
        &self.positions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_position(wallet: &str, market: &str, side: &str, price: f64) -> WalletPosition {
        WalletPosition {
            wallet_address: wallet.into(),
            market_id: market.into(),
            side: side.into(),
            price,
            size: 100.0,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_basket_creation() {
        let mut basket = Basket::new("politics_basket", "politics", 0.80);
        basket.add_wallet("0x111");
        basket.add_wallet("0x222");
        assert_eq!(basket.wallet_count(), 2);
    }

    #[test]
    fn test_basket_no_duplicates() {
        let mut basket = Basket::new("test", "general", 0.80);
        basket.add_wallet("0x111");
        basket.add_wallet("0x111");
        assert_eq!(basket.wallet_count(), 1);
    }

    #[test]
    fn test_consensus_triggered() {
        let mut engine = BasketConsensus::new().with_threshold(0.60);
        let mut basket = Basket::new("test_basket", "general", 0.60);
        basket.add_wallet("0x1");
        basket.add_wallet("0x2");
        basket.add_wallet("0x3");
        engine.add_basket(basket);

        // 2 out of 3 wallets buy YES = 67% > 60% threshold
        engine.record_position(make_position("0x1", "mkt1", "YES", 0.40));
        engine.record_position(make_position("0x2", "mkt1", "YES", 0.42));

        let results = engine.check_consensus("mkt1", "Will X happen?", 0.41);
        assert_eq!(results.len(), 1);
        assert!(results[0].triggered);
        assert_eq!(results[0].consensus_side, "YES");
    }

    #[test]
    fn test_consensus_not_triggered() {
        let mut engine = BasketConsensus::new().with_threshold(0.80);
        let mut basket = Basket::new("test_basket", "general", 0.80);
        for i in 0..10 {
            basket.add_wallet(&format!("0x{}", i));
        }
        engine.add_basket(basket);

        // Only 3 out of 10 = 30% < 80%
        engine.record_position(make_position("0x0", "mkt1", "YES", 0.40));
        engine.record_position(make_position("0x1", "mkt1", "YES", 0.41));
        engine.record_position(make_position("0x2", "mkt1", "YES", 0.42));

        let results = engine.check_consensus("mkt1", "Test?", 0.41);
        assert!(!results.is_empty());
        assert!(!results[0].triggered);
    }

    #[test]
    fn test_price_range_filter() {
        let mut engine = BasketConsensus::new().with_price_range(0.20, 0.80);
        let mut basket = Basket::new("test", "general", 0.50);
        basket.add_wallet("0x1");
        engine.add_basket(basket);
        engine.record_position(make_position("0x1", "mkt1", "YES", 0.05));

        // Price $0.05 is below min $0.20 — should return empty
        let results = engine.check_consensus("mkt1", "Test?", 0.05);
        assert!(results.is_empty());

        // Price $0.95 is above max $0.80 — should return empty
        let results = engine.check_consensus("mkt1", "Test?", 0.95);
        assert!(results.is_empty());
    }

    #[test]
    fn test_adaptive_sizer_small_trade() {
        assert_eq!(AdaptiveSizer::multiplier(25.0), 2.0);
    }

    #[test]
    fn test_adaptive_sizer_medium_trade() {
        assert_eq!(AdaptiveSizer::multiplier(100.0), 1.0);
    }

    #[test]
    fn test_adaptive_sizer_large_trade() {
        assert_eq!(AdaptiveSizer::multiplier(500.0), 0.5);
    }

    #[test]
    fn test_adaptive_sizer_caps_at_portfolio() {
        let size = AdaptiveSizer::calculate_size(30.0, 100.0, 0.10);
        assert!(size <= 10.0); // 10% of $100 portfolio
    }

    #[test]
    fn test_max_positions_guard() {
        let mut engine = BasketConsensus::new();
        engine.set_current_positions(20); // At max
        let mut basket = Basket::new("test", "general", 0.50);
        basket.add_wallet("0x1");
        engine.add_basket(basket);
        engine.record_position(make_position("0x1", "mkt1", "YES", 0.40));

        let results = engine.check_consensus("mkt1", "Test?", 0.40);
        assert!(results.is_empty()); // Blocked by max positions
    }

    #[test]
    fn test_conflicting_signals() {
        let mut engine = BasketConsensus::new().with_threshold(0.80);
        let mut basket = Basket::new("test", "general", 0.80);
        basket.add_wallet("0x1");
        basket.add_wallet("0x2");
        basket.add_wallet("0x3");
        basket.add_wallet("0x4");
        engine.add_basket(basket);

        // Split: 2 YES, 2 NO = 50% < 80% = no trigger
        engine.record_position(make_position("0x1", "mkt1", "YES", 0.40));
        engine.record_position(make_position("0x2", "mkt1", "YES", 0.41));
        engine.record_position(make_position("0x3", "mkt1", "NO", 0.60));
        engine.record_position(make_position("0x4", "mkt1", "NO", 0.59));

        let results = engine.check_consensus("mkt1", "Test?", 0.50);
        assert!(!results[0].triggered);
    }
}
