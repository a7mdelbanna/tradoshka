/// Polymarket API configuration and constants.

pub const CLOB_BASE_URL: &str = "https://clob.polymarket.com";
pub const GAMMA_BASE_URL: &str = "https://gamma-api.polymarket.com";
pub const DATA_BASE_URL: &str = "https://data-api.polymarket.com";

pub const WS_MARKET_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
pub const WS_USER_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/user";

pub const CHAIN_ID: u64 = 137;

pub const RATE_LIMIT_GENERAL: u32 = 9_000;
pub const RATE_LIMIT_BOOK: u32 = 1_500;
pub const RATE_LIMIT_BATCH: u32 = 500;
pub const RATE_LIMIT_ORDER_POST: u32 = 3_500;

pub const WS_PING_INTERVAL_SECS: u64 = 10;
pub const MAX_BATCH_ORDERS: usize = 15;

#[derive(Debug, Clone)]
pub struct ApiCredentials {
    pub api_key: String,
    pub api_secret: String,
    pub passphrase: String,
    pub wallet_address: String,
}
