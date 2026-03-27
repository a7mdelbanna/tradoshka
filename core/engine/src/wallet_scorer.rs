use chrono::{DateTime, Utc};
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

#[allow(dead_code)]
fn _use_fields(s: &WalletScorer) {
    let _ = s.min_trades;
    let _ = s.min_days;
    let _ = s.min_win_rate;
    let _ = s.min_roi;
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
        assert!(score > 0.60, "High performer should score > 0.60, got {}", score);
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
