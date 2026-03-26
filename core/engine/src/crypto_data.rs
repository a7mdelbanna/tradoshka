use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use chrono::Utc;
use std::collections::HashMap;
use tracing::{info, error};
use tradoshka_crypto::client::BinanceClient;
use tradoshka_crypto::config::BinanceMarketType;

#[derive(Debug, Clone)]
pub struct TrackedCryptoAsset {
    pub symbol: String,        // e.g., "BTCUSDT"
    pub price: Decimal,
    pub volume_24h: f64,
    pub bid: Decimal,
    pub ask: Decimal,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

pub struct CryptoDataService {
    client: BinanceClient,
    tracked: HashMap<String, TrackedCryptoAsset>,
    default_symbols: Vec<String>,
}

impl CryptoDataService {
    pub fn new() -> Self {
        Self {
            client: BinanceClient::new_public(BinanceMarketType::Spot),
            tracked: HashMap::new(),
            default_symbols: vec![
                "BTCUSDT".into(), "ETHUSDT".into(), "SOLUSDT".into(),
                "BNBUSDT".into(), "XRPUSDT".into(), "ADAUSDT".into(),
                "DOGEUSDT".into(), "AVAXUSDT".into(), "DOTUSDT".into(),
                "MATICUSDT".into(),
            ],
        }
    }

    pub async fn scan(&mut self) -> Vec<TrackedCryptoAsset> {
        match self.client.get_all_tickers().await {
            Ok(tickers) => {
                for ticker in &tickers {
                    if self.default_symbols.contains(&ticker.symbol) {
                        let price = ticker.price.parse::<Decimal>().unwrap_or(Decimal::ZERO);
                        self.tracked.insert(ticker.symbol.clone(), TrackedCryptoAsset {
                            symbol: ticker.symbol.clone(),
                            price,
                            volume_24h: 0.0,
                            bid: price,
                            ask: price,
                            last_updated: Utc::now(),
                        });
                    }
                }
                info!("Crypto scan: {} assets tracked", self.tracked.len());
                self.tracked.values().cloned().collect()
            }
            Err(e) => {
                error!("Crypto scan failed: {}", e);
                Vec::new()
            }
        }
    }

    pub async fn poll_prices(&mut self) -> Vec<(String, Decimal)> {
        let mut updates = Vec::new();
        for symbol in &self.default_symbols.clone() {
            match self.client.get_ticker_price(symbol).await {
                Ok(ticker) => {
                    if let Ok(price) = ticker.price.parse::<Decimal>() {
                        if let Some(asset) = self.tracked.get_mut(symbol) {
                            asset.price = price;
                            asset.last_updated = Utc::now();
                        }
                        updates.push((symbol.clone(), price));
                    }
                }
                Err(_) => {}
            }
        }
        updates
    }

    pub fn tracked_assets(&self) -> Vec<&TrackedCryptoAsset> {
        self.tracked.values().collect()
    }

    pub fn current_prices(&self) -> HashMap<String, Decimal> {
        self.tracked.iter().map(|(k, v)| (k.clone(), v.price)).collect()
    }
}

impl Default for CryptoDataService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_service() {
        let svc = CryptoDataService::new();
        assert_eq!(svc.tracked_assets().len(), 0);
        assert_eq!(svc.default_symbols.len(), 10);
    }

    #[test]
    fn test_default_symbols() {
        let svc = CryptoDataService::new();
        assert!(svc.default_symbols.contains(&"BTCUSDT".into()));
        assert!(svc.default_symbols.contains(&"ETHUSDT".into()));
    }
}
