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

pub async fn get_equity_curve(State(state): State<SharedState>) -> Json<Vec<serde_json::Value>> {
    let state = state.read().await;
    let mut data = Vec::new();
    let equity = state.wallet.equity().to_f64().unwrap_or(100.0);
    let now = chrono::Utc::now();

    // Build from trade history — need at least 2 distinct days for the chart
    let trades = state.trade_recorder.all_trades();
    if !trades.is_empty() {
        let first_day = trades[0].timestamp.format("%Y-%m-%d").to_string();
        let today = now.format("%Y-%m-%d").to_string();
        data.push(serde_json::json!({ "time": first_day, "value": 100.0 }));
        if today != first_day {
            data.push(serde_json::json!({ "time": today, "value": equity }));
        }
        // If same day — only 1 point, chart will show "No data yet"
    }
    // No trades → empty array → chart shows "No data yet"

    Json(data)
}

pub async fn get_daily_pnl(State(state): State<SharedState>) -> Json<Vec<serde_json::Value>> {
    let state = state.read().await;
    let daily = state.trade_recorder.daily_pnl();

    let data: Vec<serde_json::Value> = daily.iter().map(|(date, pnl)| {
        serde_json::json!({
            "date": date,
            "pnl": pnl.to_f64().unwrap_or(0.0),
        })
    }).collect();

    Json(data)
}

pub async fn get_strategies(State(state): State<SharedState>) -> Json<Vec<serde_json::Value>> {
    let state = state.read().await;
    let recorder = &state.trade_recorder;

    // Get unique strategy IDs from trade history
    let mut strategy_stats: std::collections::HashMap<String, (f64, u64, u64)> = std::collections::HashMap::new();

    for trade in recorder.all_trades() {
        let entry = strategy_stats.entry(trade.strategy_id.clone()).or_insert((0.0, 0, 0));
        entry.1 += 1; // total trades
        if trade.is_closed {
            if let Some(pnl) = trade.pnl {
                entry.0 += pnl.to_f64().unwrap_or(0.0);
                if pnl > rust_decimal::Decimal::ZERO {
                    entry.2 += 1; // winning trades
                }
            }
        }
    }

    // If no trades yet, return empty
    if strategy_stats.is_empty() {
        return Json(vec![]);
    }

    let mut result: Vec<serde_json::Value> = strategy_stats.iter().map(|(name, (pnl, total, wins))| {
        let initial = 100.0_f64; // Starting equity
        let return_pct = (pnl / initial) * 100.0;
        let win_rate = if *total > 0 { *wins as f64 / *total as f64 } else { 0.0 };
        serde_json::json!({
            "name": name,
            "return_pct": (return_pct * 10.0).round() / 10.0,
            "trades": total,
            "win_rate": (win_rate * 100.0).round() / 100.0,
        })
    }).collect();

    result.sort_by(|a, b| {
        let pa = a["return_pct"].as_f64().unwrap_or(0.0);
        let pb = b["return_pct"].as_f64().unwrap_or(0.0);
        pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
    });

    Json(result)
}

pub async fn get_stats(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let recorder = &state.trade_recorder;
    let wallet = &state.wallet;

    let closed = recorder.closed_trade_count();
    let wins = recorder.winning_trade_count();
    let win_rate = if closed > 0 { wins as f64 / closed as f64 } else { 0.0 };

    let gross_profit = recorder.gross_profit().to_f64().unwrap_or(0.0);
    let gross_loss = recorder.gross_loss().to_f64().unwrap_or(0.0);
    let profit_factor = if gross_loss > 0.0 { gross_profit / gross_loss } else { 0.0 };

    let equity = wallet.equity().to_f64().unwrap_or(100.0);
    let total_roi = ((equity - 100.0) / 100.0) * 100.0;
    let max_dd = wallet.drawdown_pct().to_f64().unwrap_or(0.0) * 100.0;

    Json(serde_json::json!({
        "total_roi": (total_roi * 10.0).round() / 10.0,
        "sharpe": 0.0,
        "max_drawdown": (max_dd * 10.0).round() / 10.0,
        "calmar": 0.0,
        "win_rate": (win_rate * 100.0).round() / 100.0 / 100.0,
        "profit_factor": (profit_factor * 100.0).round() / 100.0,
        "total_trades": recorder.total_trade_count(),
        "recovery_factor": 0.0,
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

// ---------------------------------------------------------------------------
// Per-Market Wallet & Trades
// ---------------------------------------------------------------------------

pub async fn get_wallet_by_market(
    State(state): State<SharedState>,
    axum::extract::Path(market): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    let (wallet, recorder) = match market.as_str() {
        "polymarket" => (&state.polymarket_wallet, &state.polymarket_recorder),
        "crypto" => (&state.crypto_wallet, &state.crypto_recorder),
        _ => (&state.wallet, &state.trade_recorder),
    };
    Json(serde_json::json!({
        "market": market,
        "mode": format!("{:?}", wallet.mode()),
        "balance": wallet.balance().to_string(),
        "equity": wallet.equity().to_string(),
        "unrealized_pnl": wallet.unrealized_pnl().to_string(),
        "realized_pnl": wallet.realized_pnl().to_string(),
        "drawdown_pct": wallet.drawdown_pct().to_string(),
        "total_fees": wallet.total_fees().to_string(),
        "open_positions": wallet.open_position_count(),
        "total_trades": recorder.total_trade_count(),
        "positions": wallet.positions().values().map(|p| serde_json::json!({
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

pub async fn get_trades_by_market(
    State(state): State<SharedState>,
    axum::extract::Path(market): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    let recorder = match market.as_str() {
        "polymarket" => &state.polymarket_recorder,
        "crypto" => &state.crypto_recorder,
        _ => &state.trade_recorder,
    };
    let trades = recorder.recent_trades(50);
    Json(serde_json::json!({
        "market": market,
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
        "total": recorder.total_trade_count(),
    }))
}
