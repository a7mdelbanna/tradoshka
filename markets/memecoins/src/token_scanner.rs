use std::collections::HashMap;
use crate::types::MemeToken;
use crate::client::MemeCoinClient;
use tracing::{info, warn};

pub struct TokenScanner {
    client: MemeCoinClient,
    tracked_tokens: HashMap<String, MemeToken>,
    #[allow(dead_code)]
    max_tracked: usize,
}

impl TokenScanner {
    pub fn new() -> Self {
        Self {
            client: MemeCoinClient::new(),
            tracked_tokens: HashMap::new(),
            max_tracked: 50,
        }
    }

    /// Scan for trending Solana meme tokens.
    pub async fn scan_trending(&mut self) -> Vec<MemeToken> {
        match self.client.get_trending_solana().await {
            Ok(tokens) => {
                for token in &tokens {
                    self.tracked_tokens.entry(token.address.clone())
                        .or_insert(token.clone());
                }
                info!("Meme scan: {} tokens found, {} tracked", tokens.len(), self.tracked_tokens.len());
                tokens
            }
            Err(e) => {
                warn!("Meme scan failed: {}", e);
                Vec::new()
            }
        }
    }

    /// Get all currently tracked tokens.
    pub fn tracked_tokens(&self) -> Vec<&MemeToken> {
        self.tracked_tokens.values().collect()
    }

    /// Get a specific tracked token.
    pub fn get_token(&self, address: &str) -> Option<&MemeToken> {
        self.tracked_tokens.get(address)
    }

    /// Update price for a tracked token.
    pub fn update_price(&mut self, address: &str, price: f64) {
        if let Some(token) = self.tracked_tokens.get_mut(address) {
            token.price_usd = price;
            token.update_peak();
        }
    }

    /// Get tokens with volume surge.
    pub fn surging_tokens(&self) -> Vec<&MemeToken> {
        self.tracked_tokens.values().filter(|t| t.volume_surge()).collect()
    }

    /// Get new tokens (< 30 min old).
    pub fn new_tokens(&self) -> Vec<&MemeToken> {
        self.tracked_tokens.values().filter(|t| t.is_new()).collect()
    }

    pub fn tracked_count(&self) -> usize {
        self.tracked_tokens.len()
    }
}
