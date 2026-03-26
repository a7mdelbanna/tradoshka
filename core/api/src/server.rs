use axum::{routing::get, Router};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::routes;
use crate::state::SharedState;
use crate::ws::ws_handler;
use crate::trading_ws;

pub fn create_router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route("/api/portfolio", get(routes::get_portfolio))
        .route("/api/positions", get(routes::get_positions))
        .route("/api/orders", get(routes::get_orders))
        .route("/api/risk", get(routes::get_risk))
        .route("/api/markets/polymarket", get(routes::get_polymarket_status))
        .route("/api/wallet", get(routes::get_wallet))
        .route("/api/trades/live", get(routes::get_trades))
        .route("/api/readiness", get(routes::get_readiness))
        .route("/api/markets/tracked", get(routes::get_tracked_markets))
        .route("/api/equity-curve", get(routes::get_equity_curve))
        .route("/api/pnl/daily", get(routes::get_daily_pnl))
        .route("/api/strategies", get(routes::get_strategies))
        .route("/api/stats", get(routes::get_stats))
        .route("/ws", get(ws_handler))
        .route("/ws/trading", get(trading_ws::trading_ws_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn start(state: SharedState, port: u16) -> anyhow::Result<()> {
    let router = create_router(state);
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Tradoshka API listening on {}", addr);
    axum::serve(listener, router).await?;
    Ok(())
}
