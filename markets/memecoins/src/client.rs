use reqwest::Client;
use crate::config::*;
use crate::types::*;
use tradoshka_common::error::{TradoshkaError, Result};

pub struct MemeCoinClient {
    http: Client,
}

impl MemeCoinClient {
    pub fn new() -> Self {
        Self { http: Client::new() }
    }

    /// Search for Solana tokens by keyword.
    pub async fn search_tokens(&self, query: &str) -> Result<Vec<DexScreenerPair>> {
        let url = format!("{}{}?q={}", DEXSCREENER_BASE, DEXSCREENER_SEARCH, query);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        let data: DexScreenerResponse = resp.json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(data.pairs.unwrap_or_default()
            .into_iter()
            .filter(|p| p.chain_id.as_deref() == Some("solana"))
            .collect())
    }

    /// Get token info by address from DexScreener.
    pub async fn get_token(&self, address: &str) -> Result<Vec<DexScreenerPair>> {
        let url = format!("{}{}{}", DEXSCREENER_BASE, DEXSCREENER_SOLANA_TOKENS, address);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        let data: DexScreenerResponse = resp.json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(data.pairs.unwrap_or_default())
    }

    /// Get pair info by pair address.
    pub async fn get_pair(&self, pair_address: &str) -> Result<Vec<DexScreenerPair>> {
        let url = format!("{}{}{}", DEXSCREENER_BASE, DEXSCREENER_SOLANA_PAIRS, pair_address);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        let data: DexScreenerResponse = resp.json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(data.pairs.unwrap_or_default())
    }

    /// Get trending/top Solana meme tokens from DexScreener.
    /// Uses search with common meme terms to find active tokens.
    pub async fn get_trending_solana(&self) -> Result<Vec<MemeToken>> {
        let mut all_tokens = Vec::new();

        // Search for popular meme keywords on Solana
        for keyword in &["pepe", "doge", "cat", "moon", "pump", "sol", "bonk", "wif"] {
            match self.search_tokens(keyword).await {
                Ok(pairs) => {
                    for pair in pairs {
                        if let Some(token) = pair.to_meme_token() {
                            if token.liquidity_usd >= 1000.0 && token.volume_24h >= 500.0 {
                                all_tokens.push(token);
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        // Deduplicate by address
        all_tokens.sort_by(|a, b| b.volume_24h.partial_cmp(&a.volume_24h).unwrap_or(std::cmp::Ordering::Equal));
        let mut seen = std::collections::HashSet::new();
        all_tokens.retain(|t| seen.insert(t.address.clone()));

        // Take top 50 by volume
        all_tokens.truncate(50);

        Ok(all_tokens)
    }

    /// Get price for a specific token.
    pub async fn get_price(&self, token_address: &str) -> Result<f64> {
        let pairs = self.get_token(token_address).await?;
        pairs.first()
            .and_then(|p| p.price_usd.as_ref())
            .and_then(|p| p.parse::<f64>().ok())
            .ok_or_else(|| TradoshkaError::AdapterError("Price not found".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = MemeCoinClient::new();
        // Just verify it creates without panic
        assert!(true);
    }
}
