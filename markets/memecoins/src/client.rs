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

    /// Get currently boosted/trending Solana token addresses from DexScreener.
    async fn get_boosted_addresses(&self) -> Result<Vec<String>> {
        let url = format!("{}{}", DEXSCREENER_BASE, DEXSCREENER_TOKEN_BOOSTS);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        let data: Vec<DexScreenerBoost> = resp.json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(data.into_iter()
            .filter(|b| b.chain_id == "solana")
            .map(|b| b.token_address)
            .collect())
    }

    /// Get recently listed Solana token addresses from DexScreener.
    async fn get_latest_profile_addresses(&self) -> Result<Vec<String>> {
        let url = format!("{}{}", DEXSCREENER_BASE, DEXSCREENER_TOKEN_PROFILES);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        let data: Vec<DexScreenerProfile> = resp.json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(data.into_iter()
            .filter(|p| p.chain_id == "solana")
            .map(|p| p.token_address)
            .collect())
    }

    /// Batch-fetch pair data for multiple token addresses.
    /// DexScreener supports comma-separated addresses (up to ~30).
    async fn get_tokens_batch(&self, addresses: &[String]) -> Result<Vec<DexScreenerPair>> {
        if addresses.is_empty() { return Ok(Vec::new()); }
        let batch = addresses.join(",");
        let url = format!("{}{}{}", DEXSCREENER_BASE, DEXSCREENER_SOLANA_TOKENS, batch);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        let data: DexScreenerResponse = resp.json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(data.pairs.unwrap_or_default()
            .into_iter()
            .filter(|p| p.chain_id.as_deref() == Some("solana"))
            .collect())
    }

    /// Get trending Solana meme tokens using multiple discovery channels:
    /// 1. Token boosts (currently promoted/trending)
    /// 2. Token profiles (recently listed)
    /// 3. Keyword search (popular meme terms) as fallback
    pub async fn get_trending_solana(&self) -> Result<Vec<MemeToken>> {
        let mut all_tokens = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Channel 1: Boosted tokens (highest signal — these are actively trending)
        match self.get_boosted_addresses().await {
            Ok(addrs) if !addrs.is_empty() => {
                // Batch fetch up to 30 at a time
                for chunk in addrs.chunks(30) {
                    match self.get_tokens_batch(&chunk.to_vec()).await {
                        Ok(pairs) => {
                            for pair in pairs {
                                if let Some(token) = pair.to_meme_token() {
                                    if seen.insert(token.address.clone()) {
                                        all_tokens.push(token);
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
                tracing::info!("MC boost scan: {} tokens from boosts", all_tokens.len());
            }
            _ => { tracing::debug!("MC boost scan: no boosted tokens"); }
        }

        // Channel 2: Latest profiles (recently created/updated tokens)
        match self.get_latest_profile_addresses().await {
            Ok(addrs) if !addrs.is_empty() => {
                let before = all_tokens.len();
                for chunk in addrs.chunks(30) {
                    match self.get_tokens_batch(&chunk.to_vec()).await {
                        Ok(pairs) => {
                            for pair in pairs {
                                if let Some(token) = pair.to_meme_token() {
                                    if seen.insert(token.address.clone()) {
                                        all_tokens.push(token);
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
                tracing::info!("MC profile scan: {} new tokens from profiles", all_tokens.len() - before);
            }
            _ => { tracing::debug!("MC profile scan: no profile tokens"); }
        }

        // Channel 3: Keyword search fallback (catches established popular meme coins)
        for keyword in &["pump", "bonk", "wif", "pepe", "doge"] {
            match self.search_tokens(keyword).await {
                Ok(pairs) => {
                    for pair in pairs {
                        if let Some(token) = pair.to_meme_token() {
                            if token.liquidity_usd >= 1000.0 && token.volume_24h >= 500.0 {
                                if seen.insert(token.address.clone()) {
                                    all_tokens.push(token);
                                }
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        // Sort by 5-minute volume (most active first — highest signal for meme coins)
        all_tokens.sort_by(|a, b| b.volume_5m.partial_cmp(&a.volume_5m).unwrap_or(std::cmp::Ordering::Equal));

        // Take top 50
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
        assert!(true);
    }
}
