use axum::{
    extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use serde::Serialize;
use tokio::time::{interval, Duration};
use crate::state::SharedState;

#[derive(Serialize)]
struct TradingUpdate {
    event: String,
    data: serde_json::Value,
}

pub async fn trading_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_trading_socket(socket, state))
}

async fn handle_trading_socket(mut socket: WebSocket, state: SharedState) {
    let mut tick = interval(Duration::from_secs(1));
    let mut last_trade_count = 0usize;
    let mut last_crypto_count = 0usize;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let s = state.read().await;

                // Send aggregate wallet update (legacy)
                let wallet_update = TradingUpdate {
                    event: "wallet_update".into(),
                    data: serde_json::json!({
                        "market": "all",
                        "mode": format!("{:?}", s.wallet.mode()),
                        "balance": s.wallet.balance().to_string(),
                        "equity": s.wallet.equity().to_string(),
                        "unrealized_pnl": s.wallet.unrealized_pnl().to_string(),
                        "realized_pnl": s.wallet.realized_pnl().to_string(),
                        "drawdown_pct": s.wallet.drawdown_pct().to_string(),
                        "open_positions": s.wallet.open_position_count(),
                    }),
                };
                if let Ok(json) = serde_json::to_string(&wallet_update) {
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }

                // Send Polymarket wallet update
                let poly_update = TradingUpdate {
                    event: "wallet_update".into(),
                    data: serde_json::json!({
                        "market": "polymarket",
                        "mode": format!("{:?}", s.polymarket_wallet.mode()),
                        "balance": s.polymarket_wallet.balance().to_string(),
                        "equity": s.polymarket_wallet.equity().to_string(),
                        "unrealized_pnl": s.polymarket_wallet.unrealized_pnl().to_string(),
                        "realized_pnl": s.polymarket_wallet.realized_pnl().to_string(),
                        "drawdown_pct": s.polymarket_wallet.drawdown_pct().to_string(),
                        "open_positions": s.polymarket_wallet.open_position_count(),
                    }),
                };
                if let Ok(json) = serde_json::to_string(&poly_update) {
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }

                // Send Crypto wallet update
                let crypto_update = TradingUpdate {
                    event: "wallet_update".into(),
                    data: serde_json::json!({
                        "market": "crypto",
                        "mode": format!("{:?}", s.crypto_wallet.mode()),
                        "balance": s.crypto_wallet.balance().to_string(),
                        "equity": s.crypto_wallet.equity().to_string(),
                        "unrealized_pnl": s.crypto_wallet.unrealized_pnl().to_string(),
                        "realized_pnl": s.crypto_wallet.realized_pnl().to_string(),
                        "drawdown_pct": s.crypto_wallet.drawdown_pct().to_string(),
                        "open_positions": s.crypto_wallet.open_position_count(),
                    }),
                };
                if let Ok(json) = serde_json::to_string(&crypto_update) {
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }

                // Send new Polymarket trades if any
                let current_count = s.polymarket_recorder.total_trade_count();
                if current_count > last_trade_count {
                    let new_trades = s.polymarket_recorder.recent_trades(current_count - last_trade_count);
                    for trade in new_trades {
                        let trade_event = TradingUpdate {
                            event: "new_trade".into(),
                            data: serde_json::json!({
                                "id": trade.id,
                                "timestamp": trade.timestamp.to_rfc3339(),
                                "market": "polymarket",
                                "question": trade.market_question,
                                "direction": trade.direction,
                                "side": format!("{:?}", trade.side),
                                "shares": trade.shares.to_string(),
                                "price": trade.price.to_string(),
                                "fee": trade.fee.to_string(),
                                "strategy": trade.strategy_id,
                                "strength": trade.signal_strength,
                                "edge": trade.edge_vs_market,
                            }),
                        };
                        if let Ok(json) = serde_json::to_string(&trade_event) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                return;
                            }
                        }
                    }
                    last_trade_count = current_count;
                }

                // Send new Crypto trades if any
                let current_crypto_count = s.crypto_recorder.total_trade_count();
                if current_crypto_count > last_crypto_count {
                    let new_trades = s.crypto_recorder.recent_trades(current_crypto_count - last_crypto_count);
                    for trade in new_trades {
                        let trade_event = TradingUpdate {
                            event: "new_trade".into(),
                            data: serde_json::json!({
                                "id": trade.id,
                                "timestamp": trade.timestamp.to_rfc3339(),
                                "market": "crypto",
                                "symbol": trade.symbol,
                                "question": trade.market_question,
                                "direction": trade.direction,
                                "side": format!("{:?}", trade.side),
                                "shares": trade.shares.to_string(),
                                "price": trade.price.to_string(),
                                "fee": trade.fee.to_string(),
                                "strategy": trade.strategy_id,
                                "strength": trade.signal_strength,
                            }),
                        };
                        if let Ok(json) = serde_json::to_string(&trade_event) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                return;
                            }
                        }
                    }
                    last_crypto_count = current_crypto_count;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() { break; }
                    }
                    _ => {}
                }
            }
        }
    }
}
