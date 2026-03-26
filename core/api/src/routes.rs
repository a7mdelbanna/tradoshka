use axum::{extract::State, Json};
use serde::Serialize;
use crate::state::SharedState;
use serde_json;
use rust_decimal::prelude::*;

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
// Crypto
// ---------------------------------------------------------------------------

pub async fn get_crypto_status(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let connected = state.crypto.is_some();
    Json(serde_json::json!({
        "market": "crypto",
        "exchange": "binance",
        "connected": connected,
        "status": if connected { "available" } else { "not_configured" }
    }))
}

pub async fn get_crypto_assets(
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    let assets = state.crypto_data.tracked_assets();
    Json(serde_json::json!({
        "count": assets.len(),
        "assets": assets.iter().map(|a| serde_json::json!({
            "symbol": a.symbol,
            "price": a.price.to_string(),
            "volume_24h": a.volume_24h,
        })).collect::<Vec<_>>(),
    }))
}

pub async fn trigger_crypto_scan(
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {
    let mut state = state.write().await;
    let assets = state.crypto_data.scan().await;
    Json(serde_json::json!({
        "scanned": assets.len(),
        "assets": assets.iter().map(|a| serde_json::json!({
            "symbol": a.symbol,
            "price": a.price.to_string(),
        })).collect::<Vec<_>>(),
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

// ---------------------------------------------------------------------------
// Wallet
// ---------------------------------------------------------------------------

pub async fn get_wallet(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let w = &state.wallet;
    Json(serde_json::json!({
        "mode": format!("{:?}", w.mode()),
        "balance": w.balance().to_string(),
        "equity": w.equity().to_string(),
        "unrealized_pnl": w.unrealized_pnl().to_string(),
        "realized_pnl": w.realized_pnl().to_string(),
        "drawdown_pct": w.drawdown_pct().to_string(),
        "total_fees": w.total_fees().to_string(),
        "open_positions": w.open_position_count(),
        "positions": w.positions().values().map(|p| serde_json::json!({
            "token_id": p.token_id,
            "question": p.market_question,
            "outcome": p.outcome,
            "side": format!("{:?}", p.side),
            "shares": p.shares.to_string(),
            "avg_price": p.avg_price.to_string(),
            "current_price": p.current_price.to_string(),
            "unrealized_pnl": p.unrealized_pnl.to_string(),
            "strategy": p.strategy_id,
        })).collect::<Vec<_>>(),
    }))
}

// ---------------------------------------------------------------------------
// Trades
// ---------------------------------------------------------------------------

pub async fn get_trades(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let trades = state.trade_recorder.recent_trades(50);
    Json(serde_json::json!({
        "trades": trades.iter().map(|t| serde_json::json!({
            "id": t.id,
            "timestamp": t.timestamp.to_rfc3339(),
            "symbol": t.symbol,
            "question": t.market_question,
            "direction": t.direction,
            "side": format!("{:?}", t.side),
            "shares": t.shares.to_string(),
            "price": t.price.to_string(),
            "fee": t.fee.to_string(),
            "strategy": t.strategy_id,
            "strength": t.signal_strength,
            "edge": t.edge_vs_market,
            "pnl": t.pnl.map(|p| p.to_string()),
            "closed": t.is_closed,
            "market": format!("{:?}", t.market),
        })).collect::<Vec<_>>(),
        "total": state.trade_recorder.total_trade_count(),
    }))
}

// ---------------------------------------------------------------------------
// Readiness
// ---------------------------------------------------------------------------

pub async fn get_readiness(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let dd = state.wallet.drawdown_pct().to_f64().unwrap_or(0.0);
    // For now, use empty daily returns — will be calculated from trade recorder in Phase 2B
    let report = state.readiness_scorer.evaluate(&state.trade_recorder, dd, &[]);
    Json(serde_json::json!({
        "criteria": report.criteria,
        "passed": report.passed_count,
        "total": report.total_count,
        "is_ready": report.is_ready,
    }))
}

// ---------------------------------------------------------------------------
// Orchestrator
// ---------------------------------------------------------------------------

pub async fn get_orchestrator_status(
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    Json(serde_json::json!({
        "cycle_count": state.orchestrator.cycle_count(),
        "last_cycle_at": state.orchestrator.last_cycle_at().map(|t| t.to_rfc3339()),
        "wallet_mode": format!("{:?}", state.wallet.mode()),
        "tracked_markets": state.market_data.tracked_markets().len(),
        "open_positions": state.wallet.open_position_count(),
    }))
}

pub async fn trigger_cycle(
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {
    let mut state = state.write().await;
    // Collect markets first to end the borrow of market_data before running the cycle
    let markets: Vec<_> = state.market_data.tracked_markets().into_iter().cloned().collect();
    // Split borrows manually using raw pointers to avoid multiple-mutable-borrow error
    let orchestrator: *mut tradoshka_engine::Orchestrator = &mut state.orchestrator;
    let wallet: *mut tradoshka_engine::SimulatedWallet = &mut state.wallet;
    let recorder: *mut tradoshka_engine::TradeRecorder = &mut state.trade_recorder;
    // SAFETY: orchestrator, wallet, and trade_recorder are disjoint fields of AppState.
    let result = unsafe {
        (*orchestrator).run_cycle(&markets, &mut *wallet, &mut *recorder)
    };
    let cycle_count = state.orchestrator.cycle_count();
    Json(serde_json::json!({
        "cycle": cycle_count,
        "markets_evaluated": result.markets_evaluated,
        "signals_generated": result.signals_generated,
        "trades_executed": result.trades_executed,
        "trades": result.trades.iter().map(|t| serde_json::json!({
            "question": t.market_question,
            "direction": t.direction,
            "side": format!("{:?}", t.side),
            "shares": t.shares.to_string(),
            "price": t.price.to_string(),
            "strategy": t.strategy_id,
        })).collect::<Vec<_>>(),
    }))
}

pub async fn trigger_scan(
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {
    let mut state = state.write().await;
    let markets = state.market_data.scan_markets().await;
    Json(serde_json::json!({
        "scanned": markets.len(),
        "markets": markets.iter().map(|m| serde_json::json!({
            "question": m.question,
            "yes_price": m.yes_price.to_string(),
            "no_price": m.no_price.to_string(),
            "volume_24h": m.volume_24h,
        })).collect::<Vec<_>>(),
    }))
}

// ---------------------------------------------------------------------------
// System Status
// ---------------------------------------------------------------------------

pub async fn get_system_status(
    State(state): State<SharedState>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    Json(serde_json::json!({
        "status": "running",
        "mode": format!("{:?}", state.wallet.mode()),
        "cycle_count": state.orchestrator.cycle_count(),
        "last_cycle_at": state.orchestrator.last_cycle_at().map(|t| t.to_rfc3339()),
        "tracked_markets": state.market_data.tracked_markets().len(),
        "open_positions": state.wallet.open_position_count(),
        "total_trades": state.trade_recorder.total_trade_count(),
        "wallet_equity": state.wallet.equity().to_string(),
        "wallet_balance": state.wallet.balance().to_string(),
    }))
}

// ---------------------------------------------------------------------------
// Tracked Markets
// ---------------------------------------------------------------------------

pub async fn get_tracked_markets(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let markets = state.market_data.tracked_markets();
    Json(serde_json::json!({
        "count": markets.len(),
        "markets": markets.iter().map(|m| serde_json::json!({
            "condition_id": m.condition_id,
            "question": m.question,
            "yes_price": m.yes_price.to_string(),
            "no_price": m.no_price.to_string(),
            "volume_24h": m.volume_24h,
            "liquidity": m.liquidity,
        })).collect::<Vec<_>>(),
    }))
}
