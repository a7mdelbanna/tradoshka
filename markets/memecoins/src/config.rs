/// Meme coin market configuration — Solana DEX APIs.

// DexScreener — free, no auth, best for token discovery
pub const DEXSCREENER_BASE: &str = "https://api.dexscreener.com";
pub const DEXSCREENER_SOLANA_TOKENS: &str = "/latest/dex/tokens/";
pub const DEXSCREENER_SOLANA_PAIRS: &str = "/latest/dex/pairs/solana/";
pub const DEXSCREENER_SEARCH: &str = "/latest/dex/search";

// Jupiter — prices and token list
pub const JUPITER_PRICE: &str = "https://price.jup.ag/v4/price";
pub const JUPITER_TOKENS: &str = "https://token.jup.ag/all";

// Raydium
pub const RAYDIUM_POOLS: &str = "https://api-v3.raydium.io/pools/info/list";

// Solana
pub const SOLANA_RPC: &str = "https://api.mainnet-beta.solana.com";

// Scanning intervals
pub const SCAN_NEW_TOKENS_SECS: u64 = 60;
pub const SCAN_TRENDING_SECS: u64 = 300;
pub const SCAN_WHALES_SECS: u64 = 300;

// Safety thresholds
pub const MIN_LIQUIDITY_USD: f64 = 5000.0;
pub const MAX_DEV_WALLET_PCT: f64 = 10.0;
pub const MAX_WHALE_CONCENTRATION_PCT: f64 = 15.0;
pub const MAX_TAX_PCT: f64 = 10.0;

// Trading
pub const LADDER_EXIT_1_MULT: f64 = 2.0;   // Sell 33% at 2x
pub const LADDER_EXIT_2_MULT: f64 = 5.0;   // Sell 33% at 5x
pub const TRAILING_STOP_PCT: f64 = 0.50;    // 50% trailing stop on remainder
pub const HARD_STOP_PCT: f64 = 0.30;        // -30% hard stop

pub const TIME_STOP_EARLY_DETECTION_MINS: u32 = 15;
pub const TIME_STOP_TREND_RIDING_MINS: u32 = 60;
pub const TIME_STOP_WHALE_COPY_MINS: u32 = 30;
