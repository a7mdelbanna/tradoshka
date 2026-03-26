# Phase 1: Polymarket Module — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a complete Polymarket trading system — Rust adapter for the CLOB API, Python strategies (AI prediction, copy trading, market making, arbitrage), ensemble combiner, dry mode integration, and a public performance dashboard.

**Architecture:** Rust adapter handles all Polymarket API communication (REST + WebSocket, EIP-712 auth, HMAC signing). Python strategies receive market events via the existing PyO3 bridge and return Signals. The core engine (Phase 0) handles risk management, order lifecycle, and portfolio tracking. Next.js dashboard displays live performance.

**Tech Stack:** Rust (reqwest, tokio-tungstenite, ethers-core for EIP-712), Python 3.13 (strategies), React + Next.js + Tailwind (dashboard)

**API Reference:** See `polymarket-api-research.md` for complete endpoint documentation.

---

## Sub-Project Decomposition

Phase 1 is too large for a single plan. It is broken into **3 sub-plans**:

1. **Phase 1A: Polymarket Adapter** (this plan) — Rust CLOB client, WebSocket feed, market scanner, dry mode integration
2. **Phase 1B: Polymarket Strategies** (separate plan) — AI predictor, copy trading, market making, arbitrage, ensemble
3. **Phase 1C: Dashboard v1** (separate plan) — Next.js public performance dashboard

This plan covers **Phase 1A** only.

---

## Prerequisites

- Phase 0 complete (core engine, risk, data, API server all working)
- On branch `feature/core-engine` or `dev`

## New Dependencies (Rust — workspace level)

Add to root `Cargo.toml` `[workspace.dependencies]`:

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio-tungstenite = { version = "0.26", features = ["rustls-tls-webpki-roots"] }
hmac = "0.12"
sha2 = "0.10"
base64 = "0.22"
url = "2.5"
```

**Security justification:** reqwest (HTTP client, 9K+ stars, Mozilla-backed), tokio-tungstenite (WebSocket, tokio ecosystem), hmac/sha2 (RustCrypto, audited), base64 (rust-lang-nursery). All well-established, audited crates.

---

## File Structure

```
markets/polymarket/
├── Cargo.toml
└── src/
    ├── lib.rs                    # Module exports
    ├── config.rs                 # API URLs, rate limit constants
    ├── auth.rs                   # HMAC-SHA256 L2 authentication
    ├── client.rs                 # REST client for CLOB + Gamma + Data APIs
    ├── types.rs                  # Polymarket-specific types (Market, Event, OrderBook, etc.)
    ├── websocket.rs              # WebSocket feed (market channel)
    ├── scanner.rs                # Market discovery + filtering
    ├── adapter.rs                # MarketAdapter trait implementation
    └── tests/
        ├── mod.rs
        ├── test_auth.rs          # HMAC signature tests
        ├── test_types.rs         # Deserialization tests
        └── test_scanner.rs       # Market filtering tests
```

---

### Task 1: Polymarket Crate Setup

**Files:**
- Create: `markets/polymarket/Cargo.toml`
- Create: `markets/polymarket/src/lib.rs`
- Create: `markets/polymarket/src/config.rs`
- Modify: `Cargo.toml` (workspace root — add member + deps)

- [ ] **Step 1: Create feature branch**

```bash
git checkout dev
git checkout -b feature/polymarket-adapter
```

- [ ] **Step 2: Add workspace dependencies**

Add to root `Cargo.toml` under `[workspace.dependencies]`:

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio-tungstenite = { version = "0.26", features = ["rustls-tls-webpki-roots"] }
futures-util = "0.3"
hmac = "0.12"
sha2 = "0.10"
base64 = "0.22"
url = "2.5"
```

Add `"markets/polymarket"` to `[workspace] members`.

- [ ] **Step 3: Create markets/polymarket/Cargo.toml**

```toml
[package]
name = "tradoshka-polymarket"
version.workspace = true
edition.workspace = true

[dependencies]
tradoshka-common = { path = "../../core/common" }
tradoshka-data = { path = "../../core/data" }
reqwest = { workspace = true }
tokio-tungstenite = { workspace = true }
futures-util = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
rust_decimal = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
hmac = { workspace = true }
sha2 = { workspace = true }
base64 = { workspace = true }
url = { workspace = true }

[dev-dependencies]
rust_decimal_macros = "1.36"
tokio = { workspace = true, features = ["test-util", "macros"] }
```

- [ ] **Step 4: Create config.rs**

```rust
/// Polymarket API configuration and constants.

pub const CLOB_BASE_URL: &str = "https://clob.polymarket.com";
pub const GAMMA_BASE_URL: &str = "https://gamma-api.polymarket.com";
pub const DATA_BASE_URL: &str = "https://data-api.polymarket.com";

pub const WS_MARKET_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
pub const WS_USER_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/user";

pub const CHAIN_ID: u64 = 137; // Polygon

// Rate limits (requests per 10 seconds)
pub const RATE_LIMIT_GENERAL: u32 = 9_000;
pub const RATE_LIMIT_BOOK: u32 = 1_500;
pub const RATE_LIMIT_BATCH: u32 = 500;
pub const RATE_LIMIT_ORDER_POST: u32 = 3_500;

// WebSocket
pub const WS_PING_INTERVAL_SECS: u64 = 10;

// Order constraints
pub const MAX_BATCH_ORDERS: usize = 15;

/// API credentials for L2 authentication.
#[derive(Debug, Clone)]
pub struct ApiCredentials {
    pub api_key: String,
    pub api_secret: String,
    pub passphrase: String,
    pub wallet_address: String,
}
```

- [ ] **Step 5: Create lib.rs with module declarations**

```rust
pub mod config;
pub mod auth;
pub mod types;
pub mod client;
pub mod websocket;
pub mod scanner;
pub mod adapter;
```

- [ ] **Step 6: Create stub files**

Create empty stubs for: `auth.rs`, `types.rs`, `client.rs`, `websocket.rs`, `scanner.rs`, `adapter.rs`

- [ ] **Step 7: Verify compilation**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo check -p tradoshka-polymarket
```

- [ ] **Step 8: Commit**

```bash
git add markets/ Cargo.toml
git commit -m "feat(polymarket): initialize crate with config and dependencies"
```

---

### Task 2: HMAC Authentication (L2)

**Files:**
- Create: `markets/polymarket/src/auth.rs`
- Test: inline `#[cfg(test)]`

- [ ] **Step 1: Implement L2 HMAC-SHA256 signer**

```rust
use base64::{engine::general_purpose::URL_SAFE, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::config::ApiCredentials;

type HmacSha256 = Hmac<Sha256>;

/// Generate L2 authentication headers for Polymarket CLOB API.
///
/// Signature = HMAC-SHA256(secret, timestamp + method + path [+ body])
pub struct L2Auth {
    creds: ApiCredentials,
}

impl L2Auth {
    pub fn new(creds: ApiCredentials) -> Self {
        Self { creds }
    }

    /// Build HMAC signature for a request.
    pub fn sign(&self, method: &str, path: &str, body: Option<&str>, timestamp: i64) -> String {
        let mut message = format!("{}{}{}", timestamp, method, path);
        if let Some(b) = body {
            message.push_str(b);
        }

        let secret_bytes = URL_SAFE
            .decode(&self.creds.api_secret)
            .expect("Invalid base64 API secret");

        let mut mac = HmacSha256::new_from_slice(&secret_bytes)
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());

        URL_SAFE.encode(mac.finalize().into_bytes())
    }

    /// Build all required L2 headers as (key, value) pairs.
    pub fn headers(&self, method: &str, path: &str, body: Option<&str>) -> Vec<(String, String)> {
        let timestamp = chrono::Utc::now().timestamp();
        let signature = self.sign(method, path, body, timestamp);

        vec![
            ("POLY_ADDRESS".into(), self.creds.wallet_address.clone()),
            ("POLY_SIGNATURE".into(), signature),
            ("POLY_TIMESTAMP".into(), timestamp.to_string()),
            ("POLY_API_KEY".into(), self.creds.api_key.clone()),
            ("POLY_PASSPHRASE".into(), self.creds.passphrase.clone()),
        ]
    }

    pub fn wallet_address(&self) -> &str {
        &self.creds.wallet_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_creds() -> ApiCredentials {
        ApiCredentials {
            api_key: "test-key".into(),
            // base64url-encoded "test-secret-key!"
            api_secret: URL_SAFE.encode(b"test-secret-key!"),
            passphrase: "test-pass".into(),
            wallet_address: "0x1234567890abcdef1234567890abcdef12345678".into(),
        }
    }

    #[test]
    fn test_sign_deterministic() {
        let auth = L2Auth::new(test_creds());
        let sig1 = auth.sign("GET", "/book", None, 1700000000);
        let sig2 = auth.sign("GET", "/book", None, 1700000000);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_sign_different_with_body() {
        let auth = L2Auth::new(test_creds());
        let sig_no_body = auth.sign("POST", "/order", None, 1700000000);
        let sig_with_body = auth.sign("POST", "/order", Some(r#"{"order":"test"}"#), 1700000000);
        assert_ne!(sig_no_body, sig_with_body);
    }

    #[test]
    fn test_sign_different_methods() {
        let auth = L2Auth::new(test_creds());
        let sig_get = auth.sign("GET", "/book", None, 1700000000);
        let sig_post = auth.sign("POST", "/book", None, 1700000000);
        assert_ne!(sig_get, sig_post);
    }

    #[test]
    fn test_sign_different_timestamps() {
        let auth = L2Auth::new(test_creds());
        let sig1 = auth.sign("GET", "/book", None, 1700000000);
        let sig2 = auth.sign("GET", "/book", None, 1700000001);
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_headers_contain_all_fields() {
        let auth = L2Auth::new(test_creds());
        let headers = auth.headers("GET", "/book", None);
        assert_eq!(headers.len(), 5);
        let keys: Vec<&str> = headers.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"POLY_ADDRESS"));
        assert!(keys.contains(&"POLY_SIGNATURE"));
        assert!(keys.contains(&"POLY_TIMESTAMP"));
        assert!(keys.contains(&"POLY_API_KEY"));
        assert!(keys.contains(&"POLY_PASSPHRASE"));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p tradoshka-polymarket auth
```

Expected: 5 tests pass

- [ ] **Step 3: Commit**

```bash
git add markets/polymarket/src/auth.rs
git commit -m "feat(polymarket): add L2 HMAC-SHA256 authentication"
```

---

### Task 3: Polymarket Types

**Files:**
- Create: `markets/polymarket/src/types.rs`

- [ ] **Step 1: Define all Polymarket-specific types**

```rust
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ── Market Discovery (Gamma API) ──

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GammaEvent {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub active: bool,
    pub closed: bool,
    pub archived: bool,
    pub markets: Vec<GammaMarket>,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub volume: Option<f64>,
    #[serde(rename = "volume24hr")]
    pub volume_24hr: Option<f64>,
    pub liquidity: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GammaMarket {
    pub id: String,
    #[serde(rename = "conditionId")]
    pub condition_id: String,
    #[serde(rename = "questionId")]
    pub question_id: Option<String>,
    pub question: String,
    pub slug: Option<String>,
    pub active: bool,
    pub closed: bool,
    #[serde(rename = "enableOrderBook")]
    pub enable_order_book: Option<bool>,
    #[serde(rename = "negRisk")]
    pub neg_risk: Option<bool>,
    pub tokens: Vec<GammaToken>,
    pub volume: Option<f64>,
    #[serde(rename = "volume24hr")]
    pub volume_24hr: Option<f64>,
    pub liquidity: Option<f64>,
    #[serde(rename = "endDateIso")]
    pub end_date_iso: Option<String>,
    #[serde(rename = "minimumOrderSize")]
    pub minimum_order_size: Option<f64>,
    #[serde(rename = "minimumTickSize")]
    pub minimum_tick_size: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GammaToken {
    pub token_id: String,
    pub outcome: String, // "Yes" or "No"
    pub price: Option<f64>,
    pub winner: Option<bool>,
}

// ── Order Book (CLOB API) ──

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrderBookResponse {
    pub market: String,
    pub asset_id: String,
    pub timestamp: String,
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
    #[serde(rename = "min_order_size")]
    pub min_order_size: Option<String>,
    pub neg_risk: Option<bool>,
    pub tick_size: Option<String>,
    pub last_trade_price: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BookLevel {
    pub price: String,
    pub size: String,
}

impl BookLevel {
    pub fn price_decimal(&self) -> Decimal {
        self.price.parse().unwrap_or(Decimal::ZERO)
    }

    pub fn size_decimal(&self) -> Decimal {
        self.size.parse().unwrap_or(Decimal::ZERO)
    }
}

// ── Price History ──

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PricePoint {
    pub t: i64,
    pub p: String,
}

// ── Trading ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolyOrderType {
    GTC,
    GTD,
    FOK,
    FAK,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolySide {
    BUY,
    SELL,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolyOrderRequest {
    pub token_id: String,
    pub price: Decimal,
    pub size: Decimal,
    pub side: PolySide,
    pub order_type: PolyOrderType,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PolyOrderResponse {
    #[serde(rename = "orderID")]
    pub order_id: Option<String>,
    pub success: Option<bool>,
    #[serde(rename = "errorMsg")]
    pub error_msg: Option<String>,
    pub status: Option<String>,
}

// ── Data API (Copy Trading) ──

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserPosition {
    #[serde(rename = "proxyWallet")]
    pub proxy_wallet: Option<String>,
    pub asset: String,
    #[serde(rename = "conditionId")]
    pub condition_id: String,
    pub size: f64,
    #[serde(rename = "avgPrice")]
    pub avg_price: f64,
    #[serde(rename = "initialValue")]
    pub initial_value: f64,
    #[serde(rename = "currentValue")]
    pub current_value: f64,
    #[serde(rename = "cashPnl")]
    pub cash_pnl: f64,
    #[serde(rename = "percentPnl")]
    pub percent_pnl: f64,
    #[serde(rename = "curPrice")]
    pub cur_price: f64,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserTrade {
    #[serde(rename = "proxyWallet")]
    pub proxy_wallet: Option<String>,
    pub side: String,
    pub asset: String,
    #[serde(rename = "conditionId")]
    pub condition_id: String,
    pub size: f64,
    pub price: f64,
    pub timestamp: String,
    pub title: Option<String>,
    pub outcome: Option<String>,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: Option<String>,
}

// ── WebSocket Events ──

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "event_type")]
pub enum WsMarketEvent {
    #[serde(rename = "book")]
    Book {
        asset_id: String,
        market: String,
        bids: Vec<BookLevel>,
        asks: Vec<BookLevel>,
        timestamp: String,
        hash: Option<String>,
    },
    #[serde(rename = "price_change")]
    PriceChange {
        market: String,
        price_changes: Vec<PriceChangeItem>,
        timestamp: String,
    },
    #[serde(rename = "last_trade_price")]
    LastTradePrice {
        asset_id: String,
        market: String,
        price: String,
        side: String,
        size: String,
        timestamp: String,
    },
    #[serde(rename = "best_bid_ask")]
    BestBidAsk {
        asset_id: String,
        market: String,
        best_bid: String,
        best_ask: String,
        spread: String,
        timestamp: String,
    },
    #[serde(rename = "market_resolved")]
    MarketResolved {
        market: String,
        #[serde(flatten)]
        extra: serde_json::Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriceChangeItem {
    pub asset_id: String,
    pub price: String,
    pub size: Option<String>,
    pub side: Option<String>,
    pub best_bid: Option<String>,
    pub best_ask: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_order_book() {
        let json = r#"{
            "market": "0xabc",
            "asset_id": "12345",
            "timestamp": "1700000000",
            "bids": [{"price": "0.48", "size": "30"}],
            "asks": [{"price": "0.52", "size": "25"}],
            "min_order_size": "5",
            "neg_risk": false,
            "tick_size": "0.01",
            "last_trade_price": "0.50",
            "hash": "0xhash"
        }"#;
        let book: OrderBookResponse = serde_json::from_str(json).unwrap();
        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.asks.len(), 1);
        assert_eq!(book.bids[0].price, "0.48");
    }

    #[test]
    fn test_book_level_decimal_conversion() {
        let level = BookLevel { price: "0.55".into(), size: "100.5".into() };
        assert_eq!(level.price_decimal(), rust_decimal_macros::dec!(0.55));
        assert_eq!(level.size_decimal(), rust_decimal_macros::dec!(100.5));
    }

    #[test]
    fn test_deserialize_gamma_market() {
        let json = r#"{
            "id": "123",
            "conditionId": "0xcond",
            "question": "Will BTC hit 200K?",
            "active": true,
            "closed": false,
            "tokens": [
                {"token_id": "111", "outcome": "Yes", "price": 0.55},
                {"token_id": "222", "outcome": "No", "price": 0.45}
            ]
        }"#;
        let market: GammaMarket = serde_json::from_str(json).unwrap();
        assert_eq!(market.tokens.len(), 2);
        assert_eq!(market.tokens[0].outcome, "Yes");
    }

    #[test]
    fn test_deserialize_ws_book_event() {
        let json = r#"{
            "event_type": "book",
            "asset_id": "65818619",
            "market": "0xbd31dc8a",
            "bids": [{"price": ".48", "size": "30"}],
            "asks": [{"price": ".52", "size": "25"}],
            "timestamp": "123456789000",
            "hash": "0x0"
        }"#;
        let event: WsMarketEvent = serde_json::from_str(json).unwrap();
        match event {
            WsMarketEvent::Book { asset_id, bids, asks, .. } => {
                assert_eq!(asset_id, "65818619");
                assert_eq!(bids.len(), 1);
                assert_eq!(asks.len(), 1);
            }
            _ => panic!("Expected Book event"),
        }
    }

    #[test]
    fn test_deserialize_ws_last_trade() {
        let json = r#"{
            "event_type": "last_trade_price",
            "asset_id": "114122071",
            "market": "0x6a67b9d8",
            "price": "0.456",
            "side": "BUY",
            "size": "219.2",
            "timestamp": "1750428146322"
        }"#;
        let event: WsMarketEvent = serde_json::from_str(json).unwrap();
        match event {
            WsMarketEvent::LastTradePrice { price, side, .. } => {
                assert_eq!(price, "0.456");
                assert_eq!(side, "BUY");
            }
            _ => panic!("Expected LastTradePrice event"),
        }
    }

    #[test]
    fn test_deserialize_user_trade() {
        let json = r#"{
            "proxyWallet": "0xwallet",
            "side": "BUY",
            "asset": "token123",
            "conditionId": "0xcond",
            "size": 50.0,
            "price": 0.55,
            "timestamp": "2026-03-26T00:00:00Z",
            "title": "Will X happen?",
            "outcome": "Yes",
            "transactionHash": "0xtx"
        }"#;
        let trade: UserTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.side, "BUY");
        assert_eq!(trade.size, 50.0);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p tradoshka-polymarket types
```

Expected: 6 tests pass

- [ ] **Step 3: Commit**

```bash
git add markets/polymarket/src/types.rs
git commit -m "feat(polymarket): add all Polymarket types with deserialization tests"
```

---

### Task 4: REST Client (CLOB + Gamma + Data APIs)

**Files:**
- Create: `markets/polymarket/src/client.rs`

- [ ] **Step 1: Implement the Polymarket REST client**

The client wraps all three APIs (CLOB, Gamma, Data) with proper L2 auth for trading endpoints. Key methods:

**Public (no auth):**
- `health() -> Result<()>` — CLOB health check
- `get_order_book(token_id) -> Result<OrderBookResponse>`
- `get_price(token_id, side) -> Result<Decimal>`
- `get_midpoint(token_id) -> Result<Decimal>`
- `get_spread(token_id) -> Result<Decimal>`
- `get_price_history(token_id, interval) -> Result<Vec<PricePoint>>`
- `get_markets(cursor) -> Result<Vec<GammaMarket>>` — from Gamma API
- `get_events(active, limit) -> Result<Vec<GammaEvent>>` — from Gamma API
- `get_user_positions(address) -> Result<Vec<UserPosition>>` — from Data API
- `get_user_trades(address, limit) -> Result<Vec<UserTrade>>` — from Data API
- `get_holders(condition_id) -> Result<serde_json::Value>` — from Data API

**Authenticated (L2 HMAC):**
- `place_order(order: PolyOrderRequest) -> Result<PolyOrderResponse>`
- `cancel_order(order_id) -> Result<()>`
- `cancel_all_orders() -> Result<()>`
- `get_open_orders() -> Result<Vec<serde_json::Value>>`
- `get_trade_history() -> Result<Vec<serde_json::Value>>`
- `get_balance() -> Result<serde_json::Value>`

```rust
use reqwest::Client;
use rust_decimal::Decimal;
use crate::config::*;
use crate::auth::L2Auth;
use crate::types::*;
use tradoshka_common::error::{TradoshkaError, Result};

pub struct PolymarketClient {
    http: Client,
    auth: Option<L2Auth>,
}

impl PolymarketClient {
    /// Create a read-only client (public endpoints only)
    pub fn new_public() -> Self {
        Self {
            http: Client::new(),
            auth: None,
        }
    }

    /// Create an authenticated client (trading enabled)
    pub fn new_authenticated(creds: ApiCredentials) -> Self {
        Self {
            http: Client::new(),
            auth: Some(L2Auth::new(creds)),
        }
    }

    // ... implement all methods above
}
```

The client should:
- Parse string prices to `Decimal` where appropriate
- Return `TradoshkaError::AdapterError` on HTTP failures
- Add L2 auth headers to authenticated requests
- Support pagination via cursor parameters

- [ ] **Step 2: Verify compilation**

```bash
cargo check -p tradoshka-polymarket
```

- [ ] **Step 3: Commit**

```bash
git add markets/polymarket/src/client.rs
git commit -m "feat(polymarket): add REST client for CLOB, Gamma, and Data APIs"
```

---

### Task 5: WebSocket Feed

**Files:**
- Create: `markets/polymarket/src/websocket.rs`

- [ ] **Step 1: Implement WebSocket market channel handler**

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc;
use crate::config::*;
use crate::types::WsMarketEvent;

pub struct PolymarketWsFeed {
    subscribed_tokens: Vec<String>,
}

impl PolymarketWsFeed {
    pub fn new() -> Self { ... }

    /// Connect to market WebSocket and stream events to the channel.
    /// Handles: auto-reconnect, PING every 10s, subscribe/unsubscribe.
    pub async fn connect(
        &self,
        token_ids: Vec<String>,
        tx: mpsc::Sender<WsMarketEvent>,
    ) -> Result<()> { ... }
}
```

Key requirements:
- Send PING every 10 seconds (Polymarket requirement)
- Parse incoming JSON into `WsMarketEvent` enum
- Send parsed events through `mpsc::Sender`
- Subscribe with `custom_feature_enabled: true` for BBO and market resolution events
- Auto-reconnect on disconnect with exponential backoff
- Support dynamic subscribe/unsubscribe without reconnecting

- [ ] **Step 2: Verify compilation**

```bash
cargo check -p tradoshka-polymarket
```

- [ ] **Step 3: Commit**

```bash
git add markets/polymarket/src/websocket.rs
git commit -m "feat(polymarket): add WebSocket market channel with auto-reconnect"
```

---

### Task 6: Market Scanner

**Files:**
- Create: `markets/polymarket/src/scanner.rs`

- [ ] **Step 1: Implement market discovery and filtering**

The scanner discovers tradeable markets from the Gamma API and filters them by criteria:

```rust
use crate::client::PolymarketClient;
use crate::types::*;
use rust_decimal::Decimal;

pub struct MarketFilter {
    pub min_volume_24h: f64,       // Minimum 24h volume in USD
    pub min_liquidity: f64,        // Minimum liquidity in USD
    pub exclude_closed: bool,      // Skip closed/resolved markets
    pub exclude_archived: bool,    // Skip archived markets
    pub require_order_book: bool,  // Must have CLOB enabled
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

pub struct MarketScanner {
    client: PolymarketClient,
    filter: MarketFilter,
}

impl MarketScanner {
    pub fn new(client: PolymarketClient, filter: MarketFilter) -> Self { ... }

    /// Scan for all tradeable markets matching the filter.
    pub async fn scan(&self) -> Result<Vec<GammaMarket>> { ... }

    /// Find markets with mispriced outcomes (Yes + No != ~1.00).
    pub fn find_mispriced(&self, markets: &[GammaMarket]) -> Vec<MispricedMarket> { ... }

    /// Rank markets by trading opportunity (volume, spread, mispricing).
    pub fn rank_by_opportunity(&self, markets: &[GammaMarket]) -> Vec<RankedMarket> { ... }
}

pub struct MispricedMarket {
    pub market: GammaMarket,
    pub yes_price: f64,
    pub no_price: f64,
    pub total: f64,          // yes + no, should be ~1.0
    pub deviation: f64,      // |total - 1.0|
}

pub struct RankedMarket {
    pub market: GammaMarket,
    pub score: f64,          // Composite opportunity score
    pub volume_24h: f64,
    pub liquidity: f64,
    pub spread: Option<f64>,
}
```

Tests:
- `test_filter_excludes_closed_markets`
- `test_filter_excludes_low_volume`
- `test_find_mispriced_detects_deviation`
- `test_rank_by_opportunity_ordering`

- [ ] **Step 2: Run tests**

```bash
cargo test -p tradoshka-polymarket scanner
```

- [ ] **Step 3: Commit**

```bash
git add markets/polymarket/src/scanner.rs
git commit -m "feat(polymarket): add market scanner with filtering and ranking"
```

---

### Task 7: MarketAdapter Implementation

**Files:**
- Create: `markets/polymarket/src/adapter.rs`

- [ ] **Step 1: Implement MarketAdapter trait for Polymarket**

This bridges the Polymarket client into the core engine's trait system:

```rust
use async_trait::async_trait;
use tradoshka_common::types::*;
use tradoshka_common::traits::MarketAdapter;
use tradoshka_common::error::Result;
use crate::client::PolymarketClient;
use crate::websocket::PolymarketWsFeed;
use crate::scanner::MarketScanner;

pub struct PolymarketAdapter {
    client: PolymarketClient,
    ws_feed: Option<PolymarketWsFeed>,
    scanner: MarketScanner,
    connected: bool,
}

#[async_trait]
impl MarketAdapter for PolymarketAdapter {
    fn name(&self) -> &str { "polymarket" }
    fn market(&self) -> Market { Market::Polymarket }

    async fn connect(&mut self) -> Result<()> {
        // Verify CLOB API is reachable
        self.client.health().await?;
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    async fn place_order(&self, order: &Order) -> Result<OrderId> {
        // Convert core Order → PolyOrderRequest, call client.place_order()
        // Return OrderId from response
        ...
    }

    async fn cancel_order(&self, id: &OrderId) -> Result<()> {
        self.client.cancel_order(&id.to_string()).await
    }

    async fn get_positions(&self) -> Result<Vec<Position>> {
        // Get positions from Data API, convert to core Position type
        ...
    }

    async fn get_balances(&self) -> Result<Balances> {
        // Get balance from CLOB API, convert to core Balances type
        ...
    }
}
```

The adapter translates between Polymarket's data model (token IDs, condition IDs, USDC prices 0-1) and Tradoshka's core types (Symbol, Order, Position).

- [ ] **Step 2: Verify compilation**

```bash
cargo check -p tradoshka-polymarket
```

- [ ] **Step 3: Commit**

```bash
git add markets/polymarket/src/adapter.rs
git commit -m "feat(polymarket): implement MarketAdapter trait for Polymarket"
```

---

### Task 8: Integration with Core Engine

**Files:**
- Modify: `core/api/src/state.rs` — add Polymarket adapter option
- Modify: `core/api/src/routes.rs` — add market scanning endpoint
- Modify: `core/api/Cargo.toml` — add polymarket dependency

- [ ] **Step 1: Add Polymarket to API state**

Update `AppState` to optionally include a PolymarketAdapter. Add a new route `GET /api/markets/polymarket` that returns scanned markets.

- [ ] **Step 2: Add market data route**

```rust
pub async fn get_polymarket_markets(
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {
    // Return active markets from scanner
}
```

- [ ] **Step 3: Verify workspace compiles**

```bash
cargo check --workspace
```

- [ ] **Step 4: Commit**

```bash
git add core/api/ markets/polymarket/
git commit -m "feat: integrate Polymarket adapter with core API server"
```

---

### Task 9: Full Verification

- [ ] **Step 1: Run all Rust tests**

```bash
cargo test --workspace
```

Expected: all existing tests + new Polymarket tests pass

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 3: Verify API server starts with Polymarket**

```bash
cargo run -p tradoshka-api
# In another terminal: curl http://localhost:3001/health
```

- [ ] **Step 4: Merge to dev**

```bash
git checkout dev
git merge feature/polymarket-adapter
```

- [ ] **Step 5: Commit goals update**

```bash
# Update docs/GOALS.md to mark Phase 1A complete
git add docs/GOALS.md
git commit -m "docs: update goals — Phase 1A Polymarket adapter complete"
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | Crate setup + config | Compilation check |
| 2 | HMAC L2 authentication | 5 |
| 3 | Polymarket types | 6 |
| 4 | REST client (CLOB + Gamma + Data) | Compilation check |
| 5 | WebSocket market feed | Compilation check |
| 6 | Market scanner + filtering | 4 |
| 7 | MarketAdapter trait impl | Compilation check |
| 8 | Core engine integration | Compilation check |
| 9 | Full verification | All tests |

**Total: 9 tasks, ~15+ new tests**

Next plans:
- **Phase 1B:** Polymarket Strategies (AI predictor, copy trading, market making, arbitrage, ensemble)
- **Phase 1C:** Dashboard v1 (Next.js public performance page)
