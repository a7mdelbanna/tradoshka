use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};
use crate::config::*;
use crate::types::WsMarketEvent;
use tradoshka_common::error::Result;

pub struct PolymarketWsFeed {
    token_ids: Vec<String>,
}

impl PolymarketWsFeed {
    pub fn new(token_ids: Vec<String>) -> Self {
        Self { token_ids }
    }

    /// Spawn a background task that connects to the WebSocket, subscribes to the
    /// given token IDs, and streams parsed WsMarketEvent messages to `tx`.
    /// Auto-reconnects on disconnect with exponential backoff.
    pub async fn run(&self, tx: mpsc::Sender<WsMarketEvent>) -> Result<()> {
        let token_ids = self.token_ids.clone();
        tokio::spawn(async move {
            let mut backoff_secs = 1u64;
            loop {
                match Self::connect_and_stream(&token_ids, &tx).await {
                    Ok(()) => {
                        info!("WebSocket closed normally");
                        backoff_secs = 1;
                    }
                    Err(e) => {
                        warn!("WebSocket error: {}, reconnecting in {}s", e, backoff_secs);
                        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(60);
                    }
                }
            }
        });
        Ok(())
    }

    async fn connect_and_stream(
        token_ids: &[String],
        tx: &mpsc::Sender<WsMarketEvent>,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (ws_stream, _) = connect_async(WS_MARKET_URL).await?;
        let (mut write, mut read) = ws_stream.split();

        info!("Connected to Polymarket WebSocket, subscribing to {} tokens", token_ids.len());

        // Subscribe
        let subscribe_msg = serde_json::json!({
            "assets_ids": token_ids,
            "type": "market",
            "custom_feature_enabled": true
        });
        write.send(Message::Text(subscribe_msg.to_string().into())).await?;

        // PING timer — must send every 10 seconds
        let mut ping_interval = interval(Duration::from_secs(WS_PING_INTERVAL_SECS));

        loop {
            tokio::select! {
                _ = ping_interval.tick() => {
                    write.send(Message::Ping(vec![].into())).await?;
                }
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            // Try to parse as WsMarketEvent
                            match serde_json::from_str::<WsMarketEvent>(&text) {
                                Ok(event) => {
                                    if tx.send(event).await.is_err() {
                                        info!("Event receiver dropped, stopping WS feed");
                                        return Ok(());
                                    }
                                }
                                Err(e) => {
                                    // Not all messages are events (could be PONG, ack, etc.)
                                    tracing::trace!("Non-event WS message: {}", e);
                                }
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {} // Expected response to our PING
                        Some(Ok(Message::Close(_))) => {
                            info!("Server sent close frame");
                            return Ok(());
                        }
                        Some(Err(e)) => return Err(e.into()),
                        None => return Ok(()), // Stream ended
                        _ => {} // Binary, Frame, etc.
                    }
                }
            }
        }
    }

    /// Create a subscribe message for dynamically adding tokens without reconnecting
    pub fn subscribe_message(token_ids: &[String]) -> String {
        serde_json::json!({
            "assets_ids": token_ids,
            "operation": "subscribe",
            "custom_feature_enabled": true
        }).to_string()
    }

    /// Create an unsubscribe message for removing tokens without reconnecting
    pub fn unsubscribe_message(token_ids: &[String]) -> String {
        serde_json::json!({
            "assets_ids": token_ids,
            "operation": "unsubscribe"
        }).to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stores_token_ids() {
        let ids = vec!["tok-a".to_string(), "tok-b".to_string()];
        let feed = PolymarketWsFeed::new(ids.clone());
        assert_eq!(feed.token_ids, ids);
    }

    #[test]
    fn test_subscribe_message_shape() {
        let ids = vec!["id1".to_string(), "id2".to_string()];
        let msg = PolymarketWsFeed::subscribe_message(&ids);
        let v: serde_json::Value = serde_json::from_str(&msg).expect("valid JSON");
        assert_eq!(v["operation"], "subscribe");
        assert_eq!(v["custom_feature_enabled"], true);
        let assets = v["assets_ids"].as_array().unwrap();
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0], "id1");
        assert_eq!(assets[1], "id2");
    }

    #[test]
    fn test_unsubscribe_message_shape() {
        let ids = vec!["id3".to_string()];
        let msg = PolymarketWsFeed::unsubscribe_message(&ids);
        let v: serde_json::Value = serde_json::from_str(&msg).expect("valid JSON");
        assert_eq!(v["operation"], "unsubscribe");
        let assets = v["assets_ids"].as_array().unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0], "id3");
        // no custom_feature_enabled key
        assert!(v.get("custom_feature_enabled").is_none());
    }

    #[test]
    fn test_subscribe_message_empty_ids() {
        let ids: Vec<String> = vec![];
        let msg = PolymarketWsFeed::subscribe_message(&ids);
        let v: serde_json::Value = serde_json::from_str(&msg).expect("valid JSON");
        let assets = v["assets_ids"].as_array().unwrap();
        assert!(assets.is_empty());
    }
}
