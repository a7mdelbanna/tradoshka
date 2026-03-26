use axum::{extract::State, Json};
use serde::Serialize;
use crate::state::SharedState;

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
