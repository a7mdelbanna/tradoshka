use crate::types::*;

/// Filter criteria for market scanning
pub struct MarketFilter {
    pub min_volume_24h: f64,
    pub min_liquidity: f64,
    pub exclude_closed: bool,
    pub exclude_archived: bool,
    pub require_order_book: bool,
}

impl Default for MarketFilter {
    fn default() -> Self {
        Self {
            min_volume_24h: 1000.0,
            min_liquidity: 500.0,
            exclude_closed: true,
            exclude_archived: true,
            require_order_book: true,
        }
    }
}

/// Result of mispricing detection
pub struct MispricedMarket {
    pub market: GammaMarket,
    pub yes_price: f64,
    pub no_price: f64,
    pub total: f64,
    pub deviation: f64,
}

/// Market ranked by opportunity score
pub struct RankedMarket {
    pub market: GammaMarket,
    pub score: f64,
    pub volume_24h: f64,
    pub liquidity: f64,
}

pub struct MarketScanner {
    filter: MarketFilter,
}

impl MarketScanner {
    pub fn new(filter: MarketFilter) -> Self {
        Self { filter }
    }

    /// Filter a list of markets based on the configured criteria
    pub fn filter_markets(&self, markets: &[GammaMarket]) -> Vec<GammaMarket> {
        markets.iter().filter(|m| {
            if self.filter.exclude_closed && m.closed { return false; }
            if self.filter.require_order_book && m.enable_order_book != Some(true) { return false; }
            if let Some(vol) = m.volume_24hr {
                if vol < self.filter.min_volume_24h { return false; }
            } else {
                return false; // No volume data = skip
            }
            if let Some(liq) = m.liquidity {
                if liq < self.filter.min_liquidity { return false; }
            }
            true
        }).cloned().collect()
    }

    /// Find markets where Yes + No prices deviate significantly from 1.00
    /// This indicates a potential arbitrage or mispricing opportunity
    pub fn find_mispriced(&self, markets: &[GammaMarket], min_deviation: f64) -> Vec<MispricedMarket> {
        markets.iter().filter_map(|m| {
            if m.tokens.len() != 2 { return None; }
            let yes_price = m.tokens.iter()
                .find(|t| t.outcome == "Yes")
                .and_then(|t| t.price)?;
            let no_price = m.tokens.iter()
                .find(|t| t.outcome == "No")
                .and_then(|t| t.price)?;
            let total = yes_price + no_price;
            let deviation = (total - 1.0).abs();
            if deviation >= min_deviation {
                Some(MispricedMarket {
                    market: m.clone(),
                    yes_price,
                    no_price,
                    total,
                    deviation,
                })
            } else {
                None
            }
        }).collect()
    }

    /// Rank markets by trading opportunity (composite score from volume, liquidity)
    pub fn rank_by_opportunity(&self, markets: &[GammaMarket]) -> Vec<RankedMarket> {
        let mut ranked: Vec<RankedMarket> = markets.iter().filter_map(|m| {
            let volume = m.volume_24hr?;
            let liquidity = m.liquidity.unwrap_or(0.0);
            // Score = normalized volume + normalized liquidity
            // Higher is better
            let score = volume.ln().max(0.0) + liquidity.ln().max(0.0);
            Some(RankedMarket {
                market: m.clone(),
                score,
                volume_24h: volume,
                liquidity,
            })
        }).collect();
        ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_market(active: bool, closed: bool, order_book: bool, volume: f64, liquidity: f64, yes_price: f64, no_price: f64) -> GammaMarket {
        GammaMarket {
            id: "test".into(),
            condition_id: "0xcond".into(),
            question_id: None,
            question: "Test?".into(),
            slug: None,
            active,
            closed,
            enable_order_book: Some(order_book),
            neg_risk: None,
            tokens: vec![
                GammaToken { token_id: "yes1".into(), outcome: "Yes".into(), price: Some(yes_price), winner: None },
                GammaToken { token_id: "no1".into(), outcome: "No".into(), price: Some(no_price), winner: None },
            ],
            volume: Some(volume * 10.0),
            volume_24hr: Some(volume),
            liquidity: Some(liquidity),
            end_date_iso: None,
            minimum_order_size: None,
            minimum_tick_size: None,
        }
    }

    #[test]
    fn test_filter_excludes_closed() {
        let scanner = MarketScanner::new(MarketFilter::default());
        let markets = vec![
            make_market(true, false, true, 5000.0, 1000.0, 0.5, 0.5),
            make_market(true, true, true, 5000.0, 1000.0, 0.5, 0.5),
        ];
        let filtered = scanner.filter_markets(&markets);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_excludes_low_volume() {
        let scanner = MarketScanner::new(MarketFilter::default()); // min_volume = 1000
        let markets = vec![
            make_market(true, false, true, 5000.0, 1000.0, 0.5, 0.5),
            make_market(true, false, true, 500.0, 1000.0, 0.5, 0.5),
        ];
        let filtered = scanner.filter_markets(&markets);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_find_mispriced() {
        let scanner = MarketScanner::new(MarketFilter::default());
        let markets = vec![
            make_market(true, false, true, 5000.0, 1000.0, 0.55, 0.50), // total=1.05, dev=0.05
            make_market(true, false, true, 5000.0, 1000.0, 0.50, 0.50), // total=1.00, dev=0.00
        ];
        let mispriced = scanner.find_mispriced(&markets, 0.03);
        assert_eq!(mispriced.len(), 1);
        assert!((mispriced[0].deviation - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_rank_by_opportunity() {
        let scanner = MarketScanner::new(MarketFilter::default());
        let markets = vec![
            make_market(true, false, true, 1000.0, 500.0, 0.5, 0.5),
            make_market(true, false, true, 50000.0, 20000.0, 0.5, 0.5),
        ];
        let ranked = scanner.rank_by_opportunity(&markets);
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].score > ranked[1].score); // Higher volume = higher score
        assert_eq!(ranked[0].volume_24h, 50000.0);
    }
}
