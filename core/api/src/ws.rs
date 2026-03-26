use axum::extract::{
    ws::{Message, WebSocket, WebSocketUpgrade},
    State,
};
use axum::response::IntoResponse;
use serde::Serialize;
use tokio::time::{interval, Duration};

use crate::state::SharedState;

// ---------------------------------------------------------------------------
// Portfolio snapshot for WebSocket
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct WsPortfolioSnapshot {
    pub balance: String,
    pub equity: String,
    pub unrealized_pnl: String,
    pub realized_pnl: String,
    pub open_positions: usize,
    pub win_rate: String,
    pub total_fees: String,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: SharedState) {
    let mut ticker = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let snapshot = {
                    let app = state.read().await;
                    let portfolio_snapshot = app.portfolio.snapshot();
                    let unrealized: rust_decimal::Decimal = portfolio_snapshot
                        .positions
                        .iter()
                        .map(|p| p.unrealized_pnl)
                        .sum();

                    WsPortfolioSnapshot {
                        balance: portfolio_snapshot.balance.to_string(),
                        equity: portfolio_snapshot.equity.to_string(),
                        unrealized_pnl: unrealized.to_string(),
                        realized_pnl: app.portfolio.realized_pnl().to_string(),
                        open_positions: app.portfolio.open_position_count(),
                        win_rate: app.portfolio.win_rate().to_string(),
                        total_fees: app.portfolio.total_fees().to_string(),
                    }
                };

                let json = match serde_json::to_string(&snapshot) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::error!("Failed to serialize portfolio snapshot: {}", e);
                        break;
                    }
                };

                if socket.send(Message::Text(json.into())).await.is_err() {
                    // Client disconnected
                    break;
                }
            }

            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Ignore pong responses
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types
                    }
                    Some(Err(e)) => {
                        tracing::warn!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
        }
    }
}
