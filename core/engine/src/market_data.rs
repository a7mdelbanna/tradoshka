use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use chrono::Utc;
use std::collections::HashMap;
use tradoshka_common::types::{MarketEvent, Trade, Symbol};
use tradoshka_polymarket::client::PolymarketClient;
use tradoshka_polymarket::scanner::{MarketScanner, MarketFilter};
use tradoshka_polymarket::types::GammaMarket;
use tracing::{info, warn, error};

/// A tracked market with its current state.
#[derive(Debug, Clone)]
pub struct TrackedMarket {
    pub condition_id: String,
    pub question: String,
    pub yes_token_id: String,
    pub no_token_id: String,
    pub yes_price: Decimal,
    pub no_price: Decimal,
    pub volume_24h: f64,
    pub liquidity: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Configuration for the market data service.
pub struct MarketDataConfig {
    pub poll_interval_secs: u64,
    pub scan_interval_secs: u64,
    pub max_tracked_markets: usize,
    pub price_change_trigger_pct: Decimal,
}

impl Default for MarketDataConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 300,       // 5 minutes
            scan_interval_secs: 1800,      // 30 minutes
            max_tracked_markets: 20,
            price_change_trigger_pct: Decimal::new(3, 2), // 3%
        }
    }
}

/// Service that pulls real Polymarket data and emits MarketEvents.
pub struct MarketDataService {
    client: PolymarketClient,
    scanner: MarketScanner,
    config: MarketDataConfig,
    tracked_markets: HashMap<String, TrackedMarket>,
}

impl MarketDataService {
    pub fn new(config: MarketDataConfig) -> Self {
        Self {
            client: PolymarketClient::new_public(),
            scanner: MarketScanner::new(MarketFilter::default()),
            config,
            tracked_markets: HashMap::new(),
        }
    }

    /// Scan for new tradeable markets and update the tracked list.
    pub async fn scan_markets(&mut self) -> Vec<TrackedMarket> {
        match self.client.get_gamma_markets(true, 100, 0).await {
            Ok(markets) => {
                let filtered = self.scanner.filter_markets(&markets);
                let ranked = self.scanner.rank_by_opportunity(&filtered);
                let top = ranked.into_iter()
                    .take(self.config.max_tracked_markets)
                    .collect::<Vec<_>>();

                for rm in &top {
                    let market = &rm.market;
                    let token_ids = market.parsed_token_ids();
                    let prices = market.parsed_prices();
                    let outcomes = market.parsed_outcomes();

                    // Determine yes/no token IDs and prices from new API format
                    let (yes_token, no_token, yes_price, no_price) = if token_ids.len() >= 2 && outcomes.len() >= 2 {
                        let yes_idx = outcomes.iter().position(|o| o == "Yes").unwrap_or(0);
                        let no_idx = outcomes.iter().position(|o| o == "No").unwrap_or(1);
                        let yes_tok = token_ids.get(yes_idx).cloned().unwrap_or_default();
                        let no_tok = token_ids.get(no_idx).cloned().unwrap_or_default();
                        let yes_p = prices.get(yes_idx).copied()
                            .and_then(|p| Decimal::from_f64_retain(p))
                            .unwrap_or(Decimal::ZERO);
                        let no_p = prices.get(no_idx).copied()
                            .and_then(|p| Decimal::from_f64_retain(p))
                            .unwrap_or(Decimal::ZERO);
                        (yes_tok, no_tok, yes_p, no_p)
                    } else if market.tokens.len() >= 2 {
                        // Fall back to legacy tokens array
                        let yes_tok = market.tokens.iter()
                            .find(|t| t.outcome == "Yes")
                            .map(|t| t.token_id.clone())
                            .unwrap_or_default();
                        let no_tok = market.tokens.iter()
                            .find(|t| t.outcome == "No")
                            .map(|t| t.token_id.clone())
                            .unwrap_or_default();
                        let yes_p = market.tokens.iter()
                            .find(|t| t.outcome == "Yes")
                            .and_then(|t| t.price)
                            .and_then(|p| Decimal::from_f64_retain(p))
                            .unwrap_or(Decimal::ZERO);
                        let no_p = market.tokens.iter()
                            .find(|t| t.outcome == "No")
                            .and_then(|t| t.price)
                            .and_then(|p| Decimal::from_f64_retain(p))
                            .unwrap_or(Decimal::ZERO);
                        (yes_tok, no_tok, yes_p, no_p)
                    } else {
                        continue; // Not enough token data
                    };

                    if !yes_token.is_empty() {
                        let tracked = TrackedMarket {
                            condition_id: market.condition_id.clone(),
                            question: market.question.clone(),
                            yes_token_id: yes_token,
                            no_token_id: no_token,
                            yes_price,
                            no_price,
                            volume_24h: market.volume_24h_f64(),
                            liquidity: market.liquidity_f64(),
                            last_updated: Utc::now(),
                        };
                        self.tracked_markets.insert(market.condition_id.clone(), tracked);
                    }
                }

                info!("Scanning complete: tracking {} markets", self.tracked_markets.len());
                self.tracked_markets.values().cloned().collect()
            }
            Err(e) => {
                error!("Market scan failed: {}", e);
                Vec::new()
            }
        }
    }

    /// Poll current prices for all tracked markets.
    /// Returns list of (token_id, new_price, old_price) for markets that changed significantly.
    pub async fn poll_prices(&mut self) -> Vec<(String, Decimal, Decimal)> {
        let mut significant_changes = Vec::new();

        for (_, market) in self.tracked_markets.iter_mut() {
            // Poll YES token price
            match self.client.get_price(&market.yes_token_id, "BUY").await {
                Ok(price_str) => {
                    if let Ok(new_price) = price_str.parse::<Decimal>() {
                        let old_price = market.yes_price;
                        if old_price > Decimal::ZERO {
                            let change_pct = ((new_price - old_price) / old_price).abs();
                            if change_pct >= self.config.price_change_trigger_pct {
                                significant_changes.push((
                                    market.yes_token_id.clone(),
                                    new_price,
                                    old_price,
                                ));
                            }
                        }
                        market.yes_price = new_price;
                        market.last_updated = Utc::now();
                    }
                }
                Err(e) => {
                    warn!("Price poll failed for {}: {}", market.yes_token_id, e);
                }
            }
        }

        significant_changes
    }

    /// Get all currently tracked markets.
    pub fn tracked_markets(&self) -> Vec<&TrackedMarket> {
        self.tracked_markets.values().collect()
    }

    /// Get a specific tracked market by condition ID.
    pub fn get_market(&self, condition_id: &str) -> Option<&TrackedMarket> {
        self.tracked_markets.get(condition_id)
    }

    /// Get current prices as a map (token_id → price).
    pub fn current_prices(&self) -> HashMap<String, Decimal> {
        let mut prices = HashMap::new();
        for market in self.tracked_markets.values() {
            prices.insert(market.yes_token_id.clone(), market.yes_price);
            prices.insert(market.no_token_id.clone(), market.no_price);
        }
        prices
    }

    /// Convert a tracked market's price update into a MarketEvent.
    pub fn to_market_event(market: &TrackedMarket, token_id: &str, price: Decimal) -> MarketEvent {
        MarketEvent::TradeEvent(Trade {
            symbol: token_id.into(),
            price,
            quantity: Decimal::ONE,
            timestamp: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MarketDataConfig::default();
        assert_eq!(config.poll_interval_secs, 300);
        assert_eq!(config.scan_interval_secs, 1800);
        assert_eq!(config.max_tracked_markets, 20);
    }

    #[test]
    fn test_new_service() {
        let svc = MarketDataService::new(MarketDataConfig::default());
        assert_eq!(svc.tracked_markets().len(), 0);
    }

    #[test]
    fn test_current_prices_empty() {
        let svc = MarketDataService::new(MarketDataConfig::default());
        assert!(svc.current_prices().is_empty());
    }

    #[test]
    fn test_to_market_event() {
        let market = TrackedMarket {
            condition_id: "0x123".into(),
            question: "Test?".into(),
            yes_token_id: "tok_yes".into(),
            no_token_id: "tok_no".into(),
            yes_price: Decimal::new(55, 2),
            no_price: Decimal::new(45, 2),
            volume_24h: 10000.0,
            liquidity: 5000.0,
            last_updated: Utc::now(),
        };
        let event = MarketDataService::to_market_event(&market, "tok_yes", Decimal::new(55, 2));
        assert_eq!(event.symbol(), "tok_yes");
    }
}
