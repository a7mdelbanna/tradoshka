# PM Copy Trading Engine Phases 1+2 — Wallet Scorer + Basket Consensus

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the wallet discovery/scoring engine and basket consensus system for Polymarket copy trading — find top traders, score them, group into topic baskets, and trigger trades when 80%+ agree.

**Architecture:** WalletScorer fetches trader data from Polymarket Data API, scores/grades/classifies each wallet. BasketConsensus groups qualified wallets into topic baskets (politics/sports/crypto/general), tracks their positions, and triggers copy signals when consensus threshold is met. Both are Rust modules in the engine crate.

**Tech Stack:** Rust (reqwest for API calls, serde for JSON, chrono for timestamps), existing Polymarket client

---

## File Structure

```
core/engine/src/
├── wallet_scorer.rs        # NEW: Wallet scoring, grading, classification
├── basket_consensus.rs     # NEW: Topic baskets, consensus tracking, triggers
├── lib.rs                  # Add new modules
```

---

### Task 1: Wallet Scorer

**Files:**
- Create: `core/engine/src/wallet_scorer.rs`
- Modify: `core/engine/src/lib.rs`

- [ ] **Step 1: Implement WalletScorer**

Create `core/engine/src/wallet_scorer.rs`:

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Grade assigned to a wallet based on its score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WalletGrade {
    F,
    D,
    C,
    B,
    A,
    APlus,
}

impl WalletGrade {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.85 { Self::APlus }
        else if score >= 0.70 { Self::A }
        else if score >= 0.55 { Self::B }
        else if score >= 0.40 { Self::C }
        else if score >= 0.25 { Self::D }
        else { Self::F }
    }

    pub fn is_copyable(&self) -> bool {
        matches!(self, Self::APlus | Self::A | Self::B)
    }

    pub fn size_multiplier(&self) -> f64 {
        match self {
            Self::APlus | Self::A => 1.0,
            Self::B => 0.5,
            _ => 0.0,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::APlus => "A+",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::F => "F",
        }
    }
}

/// Classification of what kind of trader this wallet is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraderType {
    Informed,
    MarketMaker,
    Bot,
    Noise,
}

impl TraderType {
    pub fn copy_value(&self) -> &str {
        match self {
            Self::Informed => "HIGH",
            Self::Bot => "LOW",
            Self::MarketMaker | Self::Noise => "NONE",
        }
    }
}

/// A scored and classified Polymarket wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredWallet {
    pub address: String,
    pub score: f64,
    pub grade: WalletGrade,
    pub trader_type: TraderType,
    pub win_rate: f64,
    pub roi: f64,
    pub total_trades: u32,
    pub total_pnl: f64,
    pub avg_trade_size: f64,
    pub trades_per_month: f64,
    pub std_dev_returns: f64,
    pub days_since_last_win: f64,
    pub days_active: u32,
    pub niche: String,
    pub last_updated: DateTime<Utc>,
}

impl ScoredWallet {
    pub fn is_qualified(&self) -> bool {
        self.total_trades >= 20
            && self.days_active >= 30
            && self.win_rate > 0.60
            && self.roi > 0.10
            && self.trader_type != TraderType::MarketMaker
            && self.trader_type != TraderType::Noise
            && self.days_since_last_win < 30.0
    }
}

/// Scoring engine for Polymarket wallets.
pub struct WalletScorer {
    wallets: HashMap<String, ScoredWallet>,
    min_trades: u32,
    min_days: u32,
    min_win_rate: f64,
    min_roi: f64,
}

impl WalletScorer {
    pub fn new() -> Self {
        Self {
            wallets: HashMap::new(),
            min_trades: 20,
            min_days: 30,
            min_win_rate: 0.60,
            min_roi: 0.10,
        }
    }

    /// Calculate the composite score for a wallet.
    pub fn calculate_score(
        win_rate: f64,
        roi: f64,
        std_dev: f64,
        mean_return: f64,
        trades_per_month: f64,
        days_since_last_win: f64,
    ) -> f64 {
        // 25% win rate (normalized: 50% = 0, 100% = 1)
        let wr_score = ((win_rate - 0.5) / 0.5).clamp(0.0, 1.0);

        // 25% ROI (normalized: 0% = 0, 50%+ = 1)
        let roi_score = (roi / 0.5).clamp(0.0, 1.0);

        // 20% consistency (low std_dev relative to mean = good)
        let consistency = if mean_return.abs() > 0.001 {
            (1.0 - (std_dev / mean_return.abs())).clamp(0.0, 1.0)
        } else {
            0.5
        };

        // 15% selectivity (fewer trades = higher conviction)
        let selectivity = (1.0 / (trades_per_month / 10.0).max(0.1)).clamp(0.0, 1.0);

        // 15% recency (recent wins weighted more)
        let recency = (-0.03 * days_since_last_win).exp();

        let score = wr_score * 0.25
            + roi_score * 0.25
            + consistency * 0.20
            + selectivity * 0.15
            + recency * 0.15;

        score.clamp(0.0, 1.0)
    }

    /// Classify a trader based on their trading patterns.
    pub fn classify_trader(
        avg_trade_size: f64,
        trades_per_month: f64,
        win_rate: f64,
        has_both_sides: bool,
        pnl_to_volume_ratio: f64,
    ) -> TraderType {
        // Market maker: high volume, both sides, low PnL/volume
        if has_both_sides && pnl_to_volume_ratio < 0.02 && trades_per_month > 50.0 {
            return TraderType::MarketMaker;
        }
        // Bot: very regular patterns, high frequency
        if trades_per_month > 100.0 {
            return TraderType::Bot;
        }
        // Informed: large size, low frequency, high win rate
        if avg_trade_size > 500.0 && trades_per_month < 30.0 && win_rate > 0.60 {
            return TraderType::Informed;
        }
        // Noise: everything else
        if win_rate < 0.50 || avg_trade_size < 50.0 {
            return TraderType::Noise;
        }
        TraderType::Informed
    }

    /// Add or update a wallet's score.
    pub fn score_wallet(
        &mut self,
        address: &str,
        win_rate: f64,
        roi: f64,
        total_trades: u32,
        total_pnl: f64,
        avg_trade_size: f64,
        trades_per_month: f64,
        std_dev: f64,
        mean_return: f64,
        days_since_last_win: f64,
        days_active: u32,
        has_both_sides: bool,
        pnl_to_volume: f64,
        niche: &str,
    ) -> &ScoredWallet {
        let score = Self::calculate_score(
            win_rate, roi, std_dev, mean_return, trades_per_month, days_since_last_win,
        );
        let grade = WalletGrade::from_score(score);
        let trader_type = Self::classify_trader(
            avg_trade_size, trades_per_month, win_rate, has_both_sides, pnl_to_volume,
        );

        let wallet = ScoredWallet {
            address: address.into(),
            score,
            grade,
            trader_type,
            win_rate,
            roi,
            total_trades,
            total_pnl,
            avg_trade_size,
            trades_per_month,
            std_dev_returns: std_dev,
            days_since_last_win,
            days_active,
            niche: niche.into(),
            last_updated: Utc::now(),
        };

        self.wallets.insert(address.into(), wallet);
        self.wallets.get(address).unwrap()
    }

    /// Get all qualified wallets (grade A+, A, or B).
    pub fn qualified_wallets(&self) -> Vec<&ScoredWallet> {
        self.wallets.values()
            .filter(|w| w.is_qualified() && w.grade.is_copyable())
            .collect()
    }

    /// Get wallets by niche.
    pub fn wallets_by_niche(&self, niche: &str) -> Vec<&ScoredWallet> {
        self.qualified_wallets().into_iter()
            .filter(|w| w.niche == niche || w.niche == "general")
            .collect()
    }

    /// Get all wallets.
    pub fn all_wallets(&self) -> &HashMap<String, ScoredWallet> {
        &self.wallets
    }

    /// Get a specific wallet.
    pub fn get_wallet(&self, address: &str) -> Option<&ScoredWallet> {
        self.wallets.get(address)
    }

    /// Total wallet count.
    pub fn wallet_count(&self) -> usize {
        self.wallets.len()
    }

    /// Qualified wallet count.
    pub fn qualified_count(&self) -> usize {
        self.qualified_wallets().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grade_from_score() {
        assert_eq!(WalletGrade::from_score(0.90), WalletGrade::APlus);
        assert_eq!(WalletGrade::from_score(0.75), WalletGrade::A);
        assert_eq!(WalletGrade::from_score(0.60), WalletGrade::B);
        assert_eq!(WalletGrade::from_score(0.45), WalletGrade::C);
        assert_eq!(WalletGrade::from_score(0.30), WalletGrade::D);
        assert_eq!(WalletGrade::from_score(0.10), WalletGrade::F);
    }

    #[test]
    fn test_grade_copyable() {
        assert!(WalletGrade::APlus.is_copyable());
        assert!(WalletGrade::A.is_copyable());
        assert!(WalletGrade::B.is_copyable());
        assert!(!WalletGrade::C.is_copyable());
        assert!(!WalletGrade::D.is_copyable());
        assert!(!WalletGrade::F.is_copyable());
    }

    #[test]
    fn test_calculate_score_high_performer() {
        let score = WalletScorer::calculate_score(
            0.80,  // 80% win rate
            0.30,  // 30% ROI
            0.05,  // low std dev
            0.10,  // positive mean return
            15.0,  // 15 trades/month (selective)
            2.0,   // won 2 days ago (recent)
        );
        assert!(score > 0.70, "High performer should score > 0.70, got {}", score);
    }

    #[test]
    fn test_calculate_score_low_performer() {
        let score = WalletScorer::calculate_score(
            0.40,  // 40% win rate (below average)
            -0.10, // -10% ROI (losing)
            0.30,  // high volatility
            -0.05, // negative mean
            100.0, // way too many trades
            60.0,  // hasn't won in 60 days
        );
        assert!(score < 0.30, "Low performer should score < 0.30, got {}", score);
    }

    #[test]
    fn test_classify_informed() {
        let t = WalletScorer::classify_trader(1000.0, 10.0, 0.75, false, 0.10);
        assert_eq!(t, TraderType::Informed);
    }

    #[test]
    fn test_classify_market_maker() {
        let t = WalletScorer::classify_trader(500.0, 60.0, 0.55, true, 0.01);
        assert_eq!(t, TraderType::MarketMaker);
    }

    #[test]
    fn test_classify_bot() {
        let t = WalletScorer::classify_trader(100.0, 150.0, 0.60, false, 0.05);
        assert_eq!(t, TraderType::Bot);
    }

    #[test]
    fn test_classify_noise() {
        let t = WalletScorer::classify_trader(20.0, 30.0, 0.40, false, 0.03);
        assert_eq!(t, TraderType::Noise);
    }

    #[test]
    fn test_score_and_retrieve_wallet() {
        let mut scorer = WalletScorer::new();
        scorer.score_wallet(
            "0xabc123", 0.75, 0.25, 50, 5000.0, 500.0, 15.0,
            0.05, 0.10, 3.0, 90, false, 0.08, "politics",
        );
        assert_eq!(scorer.wallet_count(), 1);
        let w = scorer.get_wallet("0xabc123").unwrap();
        assert!(w.grade.is_copyable());
        assert_eq!(w.trader_type, TraderType::Informed);
    }

    #[test]
    fn test_qualified_wallets_filter() {
        let mut scorer = WalletScorer::new();
        // Good wallet
        scorer.score_wallet(
            "0xgood", 0.75, 0.25, 50, 5000.0, 500.0, 15.0,
            0.05, 0.10, 3.0, 90, false, 0.08, "sports",
        );
        // Bad wallet (low WR)
        scorer.score_wallet(
            "0xbad", 0.40, -0.10, 50, -500.0, 100.0, 80.0,
            0.30, -0.05, 60.0, 90, false, 0.01, "general",
        );
        assert_eq!(scorer.qualified_count(), 1);
    }

    #[test]
    fn test_wallets_by_niche() {
        let mut scorer = WalletScorer::new();
        scorer.score_wallet(
            "0xpol", 0.80, 0.30, 30, 3000.0, 500.0, 10.0,
            0.04, 0.12, 1.0, 60, false, 0.10, "politics",
        );
        scorer.score_wallet(
            "0xsport", 0.70, 0.20, 25, 2000.0, 400.0, 12.0,
            0.06, 0.08, 5.0, 45, false, 0.07, "sports",
        );
        let politics = scorer.wallets_by_niche("politics");
        assert_eq!(politics.len(), 1);
        assert_eq!(politics[0].address, "0xpol");
    }
}
```

- [ ] **Step 2: Add to lib.rs**

```rust
pub mod wallet_scorer;
pub use wallet_scorer::{WalletScorer, ScoredWallet, WalletGrade, TraderType};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-engine wallet_scorer
```

Expected: 10 tests pass

- [ ] **Step 4: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add wallet scorer with grading, classification, and qualification"
```

---

### Task 2: Basket Consensus Engine

**Files:**
- Create: `core/engine/src/basket_consensus.rs`
- Modify: `core/engine/src/lib.rs`

- [ ] **Step 1: Implement BasketConsensus**

Create `core/engine/src/basket_consensus.rs`:

```rust
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
```

- [ ] **Step 2: Add to lib.rs**

```rust
pub mod basket_consensus;
pub use basket_consensus::{BasketConsensus, Basket, ConsensusResult, AdaptiveSizer, WalletPosition as CopyWalletPosition};
```

Note: rename to `CopyWalletPosition` to avoid conflict with the existing `WalletPosition` in wallet.rs.

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-engine basket_consensus
```

Expected: 10 tests pass

- [ ] **Step 4: Run full workspace tests**

```bash
cargo test --workspace
```

Expected: all existing + 20 new tests pass

- [ ] **Step 5: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add basket consensus engine with topic baskets and adaptive sizing"
```

---

### Task 3: Full Verification

- [ ] **Step 1: Run all tests**

```bash
cargo test --workspace
```

- [ ] **Step 2: Merge**

```bash
git checkout dev
git merge feature/pm-copy-trading-engine
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | Wallet Scorer (scoring, grading, classification) | 10 |
| 2 | Basket Consensus (topic baskets, consensus trigger, adaptive sizing) | 10 |
| 3 | Full verification | All |

**Total: 3 tasks, 20 new tests**

Next plans:
- **Phase 3:** Copy execution with AI verification
- **Phase 4:** Circuit breakers (4 layers)
- **Phase 5:** Wire into PM-CT-* strategy wallets
- **Phase 6:** Dashboard: copy trading monitoring view
