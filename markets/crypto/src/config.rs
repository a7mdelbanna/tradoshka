pub const SPOT_REST_URL: &str = "https://api.binance.com";
pub const FUTURES_REST_URL: &str = "https://fapi.binance.com";
pub const SPOT_WS_URL: &str = "wss://stream.binance.com:9443/ws";
pub const FUTURES_WS_URL: &str = "wss://fstream.binance.com/ws";

pub const SPOT_API_PREFIX: &str = "/api/v3";
pub const FUTURES_API_PREFIX: &str = "/fapi/v1";
pub const FUTURES_API_V2_PREFIX: &str = "/fapi/v2";

// Rate limits
pub const SPOT_WEIGHT_LIMIT_PER_MIN: u32 = 6_000;
pub const FUTURES_WEIGHT_LIMIT_PER_MIN: u32 = 2_400;
pub const ORDER_LIMIT_PER_10S: u32 = 100;
pub const ORDER_LIMIT_PER_DAY: u32 = 200_000;

// Fees
pub const SPOT_TAKER_FEE_BPS: u32 = 10;   // 0.1%
pub const SPOT_MAKER_FEE_BPS: u32 = 10;   // 0.1%
pub const FUTURES_TAKER_FEE_BPS: u32 = 4;  // 0.04%
pub const FUTURES_MAKER_FEE_BPS: u32 = 2;  // 0.02%

#[derive(Debug, Clone)]
pub struct BinanceCredentials {
    pub api_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinanceMarketType {
    Spot,
    UsdtFutures,
}

impl BinanceMarketType {
    pub fn rest_url(&self) -> &str {
        match self {
            Self::Spot => SPOT_REST_URL,
            Self::UsdtFutures => FUTURES_REST_URL,
        }
    }
    pub fn ws_url(&self) -> &str {
        match self {
            Self::Spot => SPOT_WS_URL,
            Self::UsdtFutures => FUTURES_WS_URL,
        }
    }
    pub fn api_prefix(&self) -> &str {
        match self {
            Self::Spot => SPOT_API_PREFIX,
            Self::UsdtFutures => FUTURES_API_PREFIX,
        }
    }
}
