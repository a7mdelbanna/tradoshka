use tokio::sync::mpsc;
use tokio::time::Duration;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};
use crate::config::*;
use crate::types::*;

pub enum BinanceWsEvent {
    Trade(WsTrade),
    Kline(WsKline),
    BookTicker(WsBookTicker),
}

pub struct BinanceWsFeed {
    market_type: BinanceMarketType,
}

impl BinanceWsFeed {
    pub fn new(market_type: BinanceMarketType) -> Self {
        Self { market_type }
    }

    pub async fn run(&self, streams: Vec<String>, tx: mpsc::Sender<BinanceWsEvent>) {
        let streams_param = streams.join("/");
        let url = format!("{}/{}", self.market_type.ws_url(), streams_param);

        tokio::spawn(async move {
            let mut backoff = 1u64;
            loop {
                match Self::connect_and_stream(&url, &tx).await {
                    Ok(()) => { backoff = 1; }
                    Err(e) => {
                        warn!("Binance WS error: {}, reconnecting in {}s", e, backoff);
                        tokio::time::sleep(Duration::from_secs(backoff)).await;
                        backoff = (backoff * 2).min(60);
                    }
                }
            }
        });
    }

    async fn connect_and_stream(
        url: &str,
        tx: &mpsc::Sender<BinanceWsEvent>,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (ws, _) = connect_async(url).await?;
        let (mut _write, mut read) = ws.split();
        info!("Connected to Binance WebSocket");

        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    // Try parsing as different event types
                    if let Ok(trade) = serde_json::from_str::<WsTrade>(&text) {
                        if trade.event_type == "trade" {
                            let _ = tx.send(BinanceWsEvent::Trade(trade)).await;
                            continue;
                        }
                    }
                    if let Ok(kline) = serde_json::from_str::<WsKline>(&text) {
                        if kline.event_type == "kline" {
                            let _ = tx.send(BinanceWsEvent::Kline(kline)).await;
                            continue;
                        }
                    }
                    if let Ok(book) = serde_json::from_str::<WsBookTicker>(&text) {
                        if !book.symbol.is_empty() {
                            let _ = tx.send(BinanceWsEvent::BookTicker(book)).await;
                        }
                    }
                }
                Message::Ping(_data) => {
                    // Binance requires pong response — handled by tungstenite automatically
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        Ok(())
    }

    pub fn trade_stream(symbol: &str) -> String {
        format!("{}@trade", symbol.to_lowercase())
    }

    pub fn kline_stream(symbol: &str, interval: &str) -> String {
        format!("{}@kline_{}", symbol.to_lowercase(), interval)
    }

    pub fn book_ticker_stream(symbol: &str) -> String {
        format!("{}@bookTicker", symbol.to_lowercase())
    }
}
