use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A meme coin token tracked by the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemeToken {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub chain: String,
    pub price_usd: f64,
    pub market_cap: f64,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub volume_5m: f64,
    pub price_change_5m: f64,
    pub price_change_1h: f64,
    pub price_change_24h: f64,
    pub pair_address: String,
    pub created_at: Option<String>,
    pub safety_score: u32,
    pub first_seen: DateTime<Utc>,
    pub peak_price: f64,
    pub is_trending: bool,
}

impl MemeToken {
    pub fn age_minutes(&self) -> i64 {
        (Utc::now() - self.first_seen).num_minutes()
    }

    pub fn is_new(&self) -> bool {
        self.age_minutes() < 30
    }

    pub fn volume_surge(&self) -> bool {
        self.volume_5m > 0.0 && self.volume_24h > 0.0
            && (self.volume_5m * 288.0) > (self.volume_24h * 2.0) // 5m rate > 2x daily avg
    }

    pub fn update_peak(&mut self) {
        if self.price_usd > self.peak_price {
            self.peak_price = self.price_usd;
        }
    }
}

/// Safety assessment for a token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyReport {
    pub token_address: String,
    pub score: u32,          // 0-100
    pub liquidity_ok: bool,
    pub dev_wallet_ok: bool,
    pub concentration_ok: bool,
    pub mint_revoked: bool,
    pub honeypot_safe: bool,
    pub tax_ok: bool,
    pub issues: Vec<String>,
    pub checked_at: DateTime<Utc>,
}

impl SafetyReport {
    pub fn is_safe_for_conservative(&self) -> bool { self.score >= 80 }
    pub fn is_safe_for_moderate(&self) -> bool { self.score >= 50 }
    pub fn is_safe_for_aggressive(&self) -> bool { self.score >= 20 }
}

/// DexScreener API response types.
#[derive(Debug, Clone, Deserialize)]
pub struct DexScreenerResponse {
    pub pairs: Option<Vec<DexScreenerPair>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DexScreenerPair {
    #[serde(rename = "chainId")]
    pub chain_id: Option<String>,
    #[serde(rename = "dexId")]
    pub dex_id: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "pairAddress")]
    pub pair_address: Option<String>,
    #[serde(rename = "baseToken")]
    pub base_token: Option<DexToken>,
    #[serde(rename = "quoteToken")]
    pub quote_token: Option<DexToken>,
    #[serde(rename = "priceUsd")]
    pub price_usd: Option<String>,
    #[serde(rename = "priceNative")]
    pub price_native: Option<String>,
    pub volume: Option<DexVolume>,
    #[serde(rename = "priceChange")]
    pub price_change: Option<DexPriceChange>,
    pub liquidity: Option<DexLiquidity>,
    pub fdv: Option<f64>,
    #[serde(rename = "pairCreatedAt")]
    pub pair_created_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DexToken {
    pub address: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DexVolume {
    pub m5: Option<f64>,
    pub h1: Option<f64>,
    pub h6: Option<f64>,
    pub h24: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DexPriceChange {
    pub m5: Option<f64>,
    pub h1: Option<f64>,
    pub h6: Option<f64>,
    pub h24: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DexLiquidity {
    pub usd: Option<f64>,
    pub base: Option<f64>,
    pub quote: Option<f64>,
}

impl DexScreenerPair {
    /// Convert a DexScreener pair to our MemeToken type.
    pub fn to_meme_token(&self) -> Option<MemeToken> {
        let base = self.base_token.as_ref()?;
        let price = self.price_usd.as_ref()?.parse::<f64>().ok()?;
        let symbol = base.symbol.as_ref()?.clone();
        let name = base.name.as_ref()?.clone();
        let address = base.address.as_ref()?.clone();
        let pair_addr = self.pair_address.as_ref()?.clone();

        let vol_5m = self.volume.as_ref().and_then(|v| v.m5).unwrap_or(0.0);
        let vol_24h = self.volume.as_ref().and_then(|v| v.h24).unwrap_or(0.0);
        let change_5m = self.price_change.as_ref().and_then(|p| p.m5).unwrap_or(0.0);
        let change_1h = self.price_change.as_ref().and_then(|p| p.h1).unwrap_or(0.0);
        let change_24h = self.price_change.as_ref().and_then(|p| p.h24).unwrap_or(0.0);
        let liq = self.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
        let mcap = self.fdv.unwrap_or(0.0);

        Some(MemeToken {
            address,
            symbol,
            name,
            chain: "solana".into(),
            price_usd: price,
            market_cap: mcap,
            liquidity_usd: liq,
            volume_24h: vol_24h,
            volume_5m: vol_5m,
            price_change_5m: change_5m,
            price_change_1h: change_1h,
            price_change_24h: change_24h,
            pair_address: pair_addr,
            created_at: self.pair_created_at.map(|ts| {
                chrono::DateTime::from_timestamp_millis(ts)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }),
            safety_score: 0,
            first_seen: Utc::now(),
            peak_price: price,
            is_trending: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meme_token_new() {
        let token = MemeToken {
            address: "So1...".into(), symbol: "PEPE".into(), name: "Pepe Sol".into(),
            chain: "solana".into(), price_usd: 0.0001, market_cap: 50000.0,
            liquidity_usd: 10000.0, volume_24h: 25000.0, volume_5m: 500.0,
            price_change_5m: 15.0, price_change_1h: 45.0, price_change_24h: 200.0,
            pair_address: "pair1".into(), created_at: None, safety_score: 75,
            first_seen: Utc::now(), peak_price: 0.0001, is_trending: false,
        };
        assert!(token.is_new());
        assert_eq!(token.chain, "solana");
    }

    #[test]
    fn test_volume_surge() {
        let mut token = MemeToken {
            address: "x".into(), symbol: "T".into(), name: "T".into(),
            chain: "solana".into(), price_usd: 0.01, market_cap: 1000.0,
            liquidity_usd: 5000.0, volume_24h: 10000.0, volume_5m: 200.0,
            price_change_5m: 0.0, price_change_1h: 0.0, price_change_24h: 0.0,
            pair_address: "p".into(), created_at: None, safety_score: 0,
            first_seen: Utc::now(), peak_price: 0.01, is_trending: false,
        };
        // 200 * 288 = 57600 > 10000 * 2 = 20000 → surge
        assert!(token.volume_surge());

        token.volume_5m = 10.0;
        // 10 * 288 = 2880 < 20000 → no surge
        assert!(!token.volume_surge());
    }

    #[test]
    fn test_safety_report_thresholds() {
        let safe = SafetyReport {
            token_address: "x".into(), score: 85, liquidity_ok: true,
            dev_wallet_ok: true, concentration_ok: true, mint_revoked: true,
            honeypot_safe: true, tax_ok: true, issues: vec![],
            checked_at: Utc::now(),
        };
        assert!(safe.is_safe_for_conservative());
        assert!(safe.is_safe_for_moderate());
        assert!(safe.is_safe_for_aggressive());

        let risky = SafetyReport { score: 35, ..safe.clone() };
        assert!(!risky.is_safe_for_conservative());
        assert!(!risky.is_safe_for_moderate());
        assert!(risky.is_safe_for_aggressive());
    }

    #[test]
    fn test_dexscreener_pair_to_meme_token() {
        let pair = DexScreenerPair {
            chain_id: Some("solana".into()),
            dex_id: Some("raydium".into()),
            url: None,
            pair_address: Some("pair123".into()),
            base_token: Some(DexToken {
                address: Some("tok123".into()),
                name: Some("Test Meme".into()),
                symbol: Some("TMEME".into()),
            }),
            quote_token: None,
            price_usd: Some("0.00005".into()),
            price_native: None,
            volume: Some(DexVolume { m5: Some(500.0), h1: Some(3000.0), h6: Some(15000.0), h24: Some(50000.0) }),
            price_change: Some(DexPriceChange { m5: Some(10.0), h1: Some(50.0), h6: Some(120.0), h24: Some(300.0) }),
            liquidity: Some(DexLiquidity { usd: Some(25000.0), base: None, quote: None }),
            fdv: Some(100000.0),
            pair_created_at: Some(1711540800000),
        };
        let token = pair.to_meme_token().unwrap();
        assert_eq!(token.symbol, "TMEME");
        assert_eq!(token.price_usd, 0.00005);
        assert_eq!(token.liquidity_usd, 25000.0);
        assert_eq!(token.volume_24h, 50000.0);
    }

    #[test]
    fn test_peak_price_update() {
        let mut token = MemeToken {
            address: "x".into(), symbol: "T".into(), name: "T".into(),
            chain: "solana".into(), price_usd: 0.01, market_cap: 1000.0,
            liquidity_usd: 5000.0, volume_24h: 10000.0, volume_5m: 100.0,
            price_change_5m: 0.0, price_change_1h: 0.0, price_change_24h: 0.0,
            pair_address: "p".into(), created_at: None, safety_score: 0,
            first_seen: Utc::now(), peak_price: 0.01, is_trending: false,
        };
        token.price_usd = 0.05;
        token.update_peak();
        assert_eq!(token.peak_price, 0.05);

        token.price_usd = 0.03;
        token.update_peak();
        assert_eq!(token.peak_price, 0.05); // Peak doesn't decrease
    }
}
