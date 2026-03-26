use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Gamma API types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GammaToken {
    #[serde(rename = "token_id")]
    pub token_id: String,
    pub outcome: String,
    pub price: Option<f64>,
    pub winner: Option<bool>,
}

// ---------------------------------------------------------------------------
// CLOB API types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookResponse {
    pub market: String,
    #[serde(rename = "asset_id")]
    pub asset_id: String,
    pub timestamp: String,
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
    #[serde(rename = "min_order_size")]
    pub min_order_size: Option<String>,
    #[serde(rename = "neg_risk")]
    pub neg_risk: Option<bool>,
    #[serde(rename = "tick_size")]
    pub tick_size: Option<String>,
    #[serde(rename = "last_trade_price")]
    pub last_trade_price: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookLevel {
    pub price: String,
    pub size: String,
}

impl BookLevel {
    pub fn price_decimal(&self) -> Decimal {
        Decimal::from_str(&self.price).unwrap_or(Decimal::ZERO)
    }

    pub fn size_decimal(&self) -> Decimal {
        Decimal::from_str(&self.size).unwrap_or(Decimal::ZERO)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub t: i64,
    pub p: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolyOrderType {
    Gtc,
    Gtd,
    Fok,
    Fak,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolySide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolyOrderRequest {
    #[serde(rename = "token_id")]
    pub token_id: String,
    pub price: Decimal,
    pub size: Decimal,
    pub side: PolySide,
    pub order_type: PolyOrderType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolyOrderResponse {
    #[serde(rename = "orderID")]
    pub order_id: Option<String>,
    pub success: Option<bool>,
    #[serde(rename = "errorMsg")]
    pub error_msg: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Data API types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ---------------------------------------------------------------------------
// WebSocket types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceChangeItem {
    #[serde(rename = "asset_id")]
    pub asset_id: String,
    pub price: String,
    pub size: Option<String>,
    pub side: Option<String>,
    pub best_bid: Option<String>,
    pub best_ask: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_order_book() {
        let json = r#"{
            "market": "0xabc",
            "asset_id": "12345",
            "timestamp": "1700000000",
            "bids": [
                {"price": "0.45", "size": "100"},
                {"price": "0.44", "size": "200"}
            ],
            "asks": [
                {"price": "0.55", "size": "150"},
                {"price": "0.56", "size": "300"}
            ],
            "min_order_size": "5",
            "neg_risk": false,
            "tick_size": "0.01",
            "last_trade_price": "0.50",
            "hash": "0xdeadbeef"
        }"#;

        let book: OrderBookResponse = serde_json::from_str(json).expect("deserialize order book");
        assert_eq!(book.market, "0xabc");
        assert_eq!(book.asset_id, "12345");
        assert_eq!(book.bids.len(), 2);
        assert_eq!(book.asks.len(), 2);
        assert_eq!(book.bids[0].price, "0.45");
        assert_eq!(book.asks[0].price, "0.55");
        assert_eq!(book.neg_risk, Some(false));
        assert_eq!(book.hash.as_deref(), Some("0xdeadbeef"));
    }

    #[test]
    fn test_book_level_decimal_conversion() {
        let level = BookLevel {
            price: "0.6789".into(),
            size: "1234.56".into(),
        };
        let price = level.price_decimal();
        let size = level.size_decimal();

        assert_eq!(price, Decimal::from_str("0.6789").unwrap());
        assert_eq!(size, Decimal::from_str("1234.56").unwrap());
        assert!(price > Decimal::ZERO);
        assert!(size > Decimal::ZERO);

        // Invalid string returns ZERO
        let bad = BookLevel {
            price: "not-a-number".into(),
            size: "also-bad".into(),
        };
        assert_eq!(bad.price_decimal(), Decimal::ZERO);
        assert_eq!(bad.size_decimal(), Decimal::ZERO);
    }

    #[test]
    fn test_deserialize_gamma_market() {
        let json = r#"{
            "id": "mkt-001",
            "conditionId": "cond-abc",
            "questionId": "q-xyz",
            "question": "Will X happen?",
            "slug": "will-x-happen",
            "active": true,
            "closed": false,
            "enableOrderBook": true,
            "negRisk": false,
            "tokens": [
                {"token_id": "tok-yes", "outcome": "Yes", "price": 0.6, "winner": null},
                {"token_id": "tok-no",  "outcome": "No",  "price": 0.4, "winner": null}
            ],
            "volume": 50000.0,
            "volume24hr": 1200.5,
            "liquidity": 8000.0,
            "endDateIso": "2025-12-31T00:00:00Z",
            "minimumOrderSize": 1.0,
            "minimumTickSize": 0.01
        }"#;

        let market: GammaMarket = serde_json::from_str(json).expect("deserialize gamma market");
        assert_eq!(market.id, "mkt-001");
        assert_eq!(market.condition_id, "cond-abc");
        assert_eq!(market.question_id.as_deref(), Some("q-xyz"));
        assert_eq!(market.tokens.len(), 2);
        assert_eq!(market.tokens[0].outcome, "Yes");
        assert_eq!(market.tokens[0].price, Some(0.6));
        assert!(market.active);
        assert!(!market.closed);
        assert_eq!(market.neg_risk, Some(false));
        assert_eq!(market.end_date_iso.as_deref(), Some("2025-12-31T00:00:00Z"));
    }

    #[test]
    fn test_deserialize_ws_book_event() {
        let json = r#"{
            "event_type": "book",
            "asset_id": "asset-001",
            "market": "0xmarket",
            "bids": [{"price": "0.5", "size": "50"}],
            "asks": [{"price": "0.6", "size": "40"}],
            "timestamp": "1700000000",
            "hash": "0xhash123"
        }"#;

        let event: WsMarketEvent = serde_json::from_str(json).expect("deserialize ws book event");
        match event {
            WsMarketEvent::Book {
                asset_id,
                market,
                bids,
                asks,
                timestamp,
                hash,
            } => {
                assert_eq!(asset_id, "asset-001");
                assert_eq!(market, "0xmarket");
                assert_eq!(bids.len(), 1);
                assert_eq!(asks.len(), 1);
                assert_eq!(timestamp, "1700000000");
                assert_eq!(hash.as_deref(), Some("0xhash123"));
            }
            other => panic!("expected Book variant, got {:?}", other),
        }
    }

    #[test]
    fn test_deserialize_ws_last_trade() {
        let json = r#"{
            "event_type": "last_trade_price",
            "asset_id": "asset-002",
            "market": "0xmarket2",
            "price": "0.55",
            "side": "BUY",
            "size": "100",
            "timestamp": "1700001000"
        }"#;

        let event: WsMarketEvent =
            serde_json::from_str(json).expect("deserialize ws last trade event");
        match event {
            WsMarketEvent::LastTradePrice {
                asset_id,
                market,
                price,
                side,
                size,
                timestamp,
            } => {
                assert_eq!(asset_id, "asset-002");
                assert_eq!(market, "0xmarket2");
                assert_eq!(price, "0.55");
                assert_eq!(side, "BUY");
                assert_eq!(size, "100");
                assert_eq!(timestamp, "1700001000");
            }
            other => panic!("expected LastTradePrice variant, got {:?}", other),
        }
    }

    #[test]
    fn test_deserialize_user_trade() {
        let json = r#"{
            "proxyWallet": "0xproxy123",
            "side": "BUY",
            "asset": "asset-abc",
            "conditionId": "cond-def",
            "size": 250.0,
            "price": 0.48,
            "timestamp": "2024-01-15T12:00:00Z",
            "title": "Some Market Title",
            "outcome": "Yes",
            "transactionHash": "0xtxhash"
        }"#;

        let trade: UserTrade = serde_json::from_str(json).expect("deserialize user trade");
        assert_eq!(trade.proxy_wallet.as_deref(), Some("0xproxy123"));
        assert_eq!(trade.side, "BUY");
        assert_eq!(trade.asset, "asset-abc");
        assert_eq!(trade.condition_id, "cond-def");
        assert_eq!(trade.size, 250.0);
        assert_eq!(trade.price, 0.48);
        assert_eq!(trade.title.as_deref(), Some("Some Market Title"));
        assert_eq!(trade.outcome.as_deref(), Some("Yes"));
        assert_eq!(
            trade.transaction_hash.as_deref(),
            Some("0xtxhash")
        );
    }
}
