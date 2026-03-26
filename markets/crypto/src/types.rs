use serde::{Deserialize, Serialize};

// ── Market Data ──

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceTicker {
    pub symbol: String,
    pub price: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceBookTicker {
    pub symbol: String,
    #[serde(rename = "bidPrice")]
    pub bid_price: String,
    #[serde(rename = "bidQty")]
    pub bid_qty: String,
    #[serde(rename = "askPrice")]
    pub ask_price: String,
    #[serde(rename = "askQty")]
    pub ask_qty: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceDepth {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<[String; 2]>,  // [price, qty]
    pub asks: Vec<[String; 2]>,
}

// Kline comes as an array, not an object
#[derive(Debug, Clone)]
pub struct BinanceKline {
    pub open_time: i64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub close_time: i64,
    pub quote_volume: String,
    pub num_trades: u64,
}

// ── Trading ──

#[derive(Debug, Clone, Copy, Serialize)]
pub enum BinanceOrderSide {
    BUY,
    SELL,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum BinanceOrderType {
    LIMIT,
    MARKET,
    STOP_LOSS_LIMIT,
    TAKE_PROFIT_LIMIT,
    TRAILING_STOP_MARKET,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum TimeInForce {
    GTC,
    IOC,
    FOK,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceOrderResponse {
    pub symbol: String,
    #[serde(rename = "orderId")]
    pub order_id: u64,
    #[serde(rename = "clientOrderId")]
    pub client_order_id: Option<String>,
    pub status: String,
    pub price: Option<String>,
    #[serde(rename = "origQty")]
    pub orig_qty: Option<String>,
    #[serde(rename = "executedQty")]
    pub executed_qty: Option<String>,
    #[serde(rename = "type")]
    pub order_type: Option<String>,
    pub side: Option<String>,
    pub fills: Option<Vec<BinanceFill>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceFill {
    pub price: String,
    pub qty: String,
    pub commission: String,
    #[serde(rename = "commissionAsset")]
    pub commission_asset: String,
}

// ── Account ──

#[derive(Debug, Clone, Deserialize)]
pub struct SpotBalance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpotAccount {
    pub balances: Vec<SpotBalance>,
}

// ── Futures ──

#[derive(Debug, Clone, Deserialize)]
pub struct FuturesPosition {
    pub symbol: String,
    #[serde(rename = "positionAmt")]
    pub position_amt: String,
    #[serde(rename = "entryPrice")]
    pub entry_price: String,
    #[serde(rename = "markPrice")]
    pub mark_price: Option<String>,
    #[serde(rename = "unRealizedProfit")]
    pub unrealized_profit: String,
    pub leverage: String,
    #[serde(rename = "marginType")]
    pub margin_type: Option<String>,
    #[serde(rename = "liquidationPrice")]
    pub liquidation_price: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FundingRateInfo {
    pub symbol: String,
    #[serde(rename = "markPrice")]
    pub mark_price: String,
    #[serde(rename = "lastFundingRate")]
    pub last_funding_rate: String,
    #[serde(rename = "nextFundingTime")]
    pub next_funding_time: i64,
}

// ── WebSocket Events ──

#[derive(Debug, Clone, Deserialize)]
pub struct WsTrade {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "T")]
    pub trade_time: i64,
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WsKline {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "k")]
    pub kline: WsKlineData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WsKlineData {
    #[serde(rename = "t")]
    pub open_time: i64,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "v")]
    pub volume: String,
    #[serde(rename = "x")]
    pub is_closed: bool,
    #[serde(rename = "i")]
    pub interval: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WsBookTicker {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub bid_price: String,
    #[serde(rename = "B")]
    pub bid_qty: String,
    #[serde(rename = "a")]
    pub ask_price: String,
    #[serde(rename = "A")]
    pub ask_qty: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_ticker() {
        let json = r#"{"symbol":"BTCUSDT","price":"42000.50"}"#;
        let ticker: BinanceTicker = serde_json::from_str(json).unwrap();
        assert_eq!(ticker.symbol, "BTCUSDT");
        assert_eq!(ticker.price, "42000.50");
    }

    #[test]
    fn test_deserialize_depth() {
        let json = r#"{
            "lastUpdateId": 123456789,
            "bids": [["42000.00", "1.500"], ["41999.00", "2.000"]],
            "asks": [["42001.00", "0.500"], ["42002.00", "1.000"]]
        }"#;
        let depth: BinanceDepth = serde_json::from_str(json).unwrap();
        assert_eq!(depth.last_update_id, 123456789);
        assert_eq!(depth.bids.len(), 2);
        assert_eq!(depth.asks.len(), 2);
        assert_eq!(depth.bids[0][0], "42000.00");
        assert_eq!(depth.asks[0][1], "0.500");
    }

    #[test]
    fn test_deserialize_order_response() {
        let json = r#"{
            "symbol": "BTCUSDT",
            "orderId": 987654321,
            "clientOrderId": "my-order-123",
            "status": "FILLED",
            "price": "42000.00",
            "origQty": "0.001",
            "executedQty": "0.001",
            "type": "LIMIT",
            "side": "BUY",
            "fills": [
                {
                    "price": "42000.00",
                    "qty": "0.001",
                    "commission": "0.00000042",
                    "commissionAsset": "BTC"
                }
            ]
        }"#;
        let order: BinanceOrderResponse = serde_json::from_str(json).unwrap();
        assert_eq!(order.symbol, "BTCUSDT");
        assert_eq!(order.order_id, 987654321);
        assert_eq!(order.status, "FILLED");
        assert_eq!(order.client_order_id, Some("my-order-123".into()));
        let fills = order.fills.unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].commission_asset, "BTC");
    }

    #[test]
    fn test_deserialize_ws_trade() {
        let json = r#"{
            "e": "trade",
            "s": "BTCUSDT",
            "p": "42000.00",
            "q": "0.001",
            "T": 1700000000000,
            "m": false
        }"#;
        let trade: WsTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.event_type, "trade");
        assert_eq!(trade.symbol, "BTCUSDT");
        assert_eq!(trade.price, "42000.00");
        assert_eq!(trade.quantity, "0.001");
        assert_eq!(trade.trade_time, 1700000000000);
        assert!(!trade.is_buyer_maker);
    }
}
