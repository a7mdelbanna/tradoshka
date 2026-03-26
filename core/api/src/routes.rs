use axum::{extract::State, Json};
use serde::Serialize;
use crate::state::SharedState;
use serde_json;

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

// ---------------------------------------------------------------------------
// Portfolio
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct PortfolioResponse {
    pub balance: String,
    pub equity: String,
    pub unrealized_pnl: String,
    pub realized_pnl: String,
    pub open_positions: usize,
    pub win_rate: String,
    pub total_fees: String,
}

pub async fn get_portfolio(State(state): State<SharedState>) -> Json<PortfolioResponse> {
    let app = state.read().await;
    let snapshot = app.portfolio.snapshot();
    let unrealized: rust_decimal::Decimal = snapshot
        .positions
        .iter()
        .map(|p| p.unrealized_pnl)
        .sum();

    Json(PortfolioResponse {
        balance: snapshot.balance.to_string(),
        equity: snapshot.equity.to_string(),
        unrealized_pnl: unrealized.to_string(),
        realized_pnl: app.portfolio.realized_pnl().to_string(),
        open_positions: app.portfolio.open_position_count(),
        win_rate: app.portfolio.win_rate().to_string(),
        total_fees: app.portfolio.total_fees().to_string(),
    })
}

// ---------------------------------------------------------------------------
// Positions
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct PositionItem {
    pub symbol: String,
    pub side: String,
    pub quantity: String,
    pub entry_price: String,
    pub current_price: String,
    pub unrealized_pnl: String,
    pub strategy_id: String,
    pub market: String,
}

#[derive(Serialize)]
pub struct PositionsResponse {
    pub positions: Vec<PositionItem>,
}

pub async fn get_positions(State(state): State<SharedState>) -> Json<PositionsResponse> {
    let app = state.read().await;
    let snapshot = app.portfolio.snapshot();

    let positions = snapshot
        .positions
        .into_iter()
        .map(|p| PositionItem {
            symbol: p.symbol,
            side: format!("{:?}", p.side),
            quantity: p.quantity.to_string(),
            entry_price: p.entry_price.to_string(),
            current_price: p.current_price.to_string(),
            unrealized_pnl: p.unrealized_pnl.to_string(),
            strategy_id: p.strategy_id,
            market: format!("{:?}", p.market),
        })
        .collect();

    Json(PositionsResponse { positions })
}

// ---------------------------------------------------------------------------
// Orders
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct OrdersResponse {
    pub active_orders: usize,
    pub total_orders: usize,
}

pub async fn get_orders(State(state): State<SharedState>) -> Json<OrdersResponse> {
    let app = state.read().await;
    Json(OrdersResponse {
        active_orders: app.order_manager.active_orders().len(),
        total_orders: app.order_manager.order_count(),
    })
}

// ---------------------------------------------------------------------------
// Risk
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct RiskResponse {
    pub breaker_state: String,
    pub allows_trading: bool,
}

pub async fn get_risk(State(state): State<SharedState>) -> Json<RiskResponse> {
    let app = state.read().await;
    let breaker_state = app.risk_manager.breaker_state();
    Json(RiskResponse {
        breaker_state: format!("{:?}", breaker_state),
        allows_trading: breaker_state.allows_new_trades(),
    })
}

// ---------------------------------------------------------------------------
// Polymarket
// ---------------------------------------------------------------------------

pub async fn get_polymarket_status(
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    let connected = state.polymarket.is_some();
    Json(serde_json::json!({
        "market": "polymarket",
        "connected": connected,
        "status": if connected { "available" } else { "not_configured" }
    }))
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

pub async fn get_equity_curve() -> Json<Vec<serde_json::Value>> {
    let mut data = Vec::new();
    let now = chrono::Utc::now();
    let mut value = 100.0_f64;
    for i in 0..30 {
        let date = now - chrono::Duration::days(30 - i);
        let seed = ((i * 1103515245 + 12345) & 0x7fffffff) as f64;
        let r = seed / 0x7fffffff as f64;
        value += (r - 0.3) * 5.0;
        value = value.max(50.0);
        data.push(serde_json::json!({
            "time": date.format("%Y-%m-%d").to_string(),
            "value": (value * 100.0).round() / 100.0,
        }));
    }
    Json(data)
}

pub async fn get_daily_pnl() -> Json<Vec<serde_json::Value>> {
    let mut data = Vec::new();
    let now = chrono::Utc::now();
    for i in 0..90 {
        let date = now - chrono::Duration::days(90 - i);
        let seed = ((i * 7 * 1103515245 + 12345) & 0x7fffffff) as f64;
        let r = seed / 0x7fffffff as f64;
        let pnl = (r - 0.4) * 20.0;
        data.push(serde_json::json!({
            "date": date.format("%Y-%m-%d").to_string(),
            "pnl": (pnl * 100.0).round() / 100.0,
        }));
    }
    Json(data)
}

pub async fn get_strategies() -> Json<Vec<serde_json::Value>> {
    Json(vec![
        serde_json::json!({"name": "AI Predictor", "return_pct": 12.5, "trades": 45, "win_rate": 0.64}),
        serde_json::json!({"name": "Copy Trading", "return_pct": 8.3, "trades": 32, "win_rate": 0.59}),
        serde_json::json!({"name": "Market Making", "return_pct": 5.1, "trades": 128, "win_rate": 0.72}),
        serde_json::json!({"name": "Arbitrage", "return_pct": 3.2, "trades": 18, "win_rate": 0.89}),
    ])
}

pub async fn get_stats() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "total_roi": 29.1,
        "sharpe": 1.85,
        "max_drawdown": 8.3,
        "calmar": 3.51,
        "win_rate": 0.67,
        "profit_factor": 2.14,
        "total_trades": 223,
        "recovery_factor": 3.5,
    }))
}
