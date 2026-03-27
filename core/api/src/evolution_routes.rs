use axum::{extract::State, Json, extract::Path};
use rust_decimal::prelude::*;
use crate::state::SharedState;
use tradoshka_engine;

struct SlotSnapshot {
    name: String,
    market: String,
    strategy_type: String,
    status: String,
    sharpe: f64,
    pnl: f64,
    equity: f64,
    balance: f64,
    unrealized_pnl: f64,
    realized_pnl: f64,
    win_rate: f64,
    trades: usize,
    age_hours: i64,
    generation: u32,
    parent: Option<String>,
    open_positions: usize,
    fees: f64,
    params: std::collections::HashMap<String, f64>,
}

pub async fn get_leaderboard(State(state): State<SharedState>) -> Json<serde_json::Value> {
    // Collect only what we need into owned data under a short read lock, then release
    // before any allocation-heavy serialization work.  This prevents the read lock from
    // blocking the trading-loop write lock (and vice-versa) for more than a few µs.
    let (rows, total_alive) = {
        let s = state.read().await;
        let rows: Vec<SlotSnapshot> = s.strategy_manager.alive_slots().iter().map(|sl| {
            SlotSnapshot {
                name: sl.name.clone(),
                market: sl.market.clone(),
                strategy_type: sl.params.strategy_type.clone(),
                status: format!("{:?}", sl.status),
                sharpe: sl.sharpe_ratio(),
                pnl: sl.pnl_pct(),
                equity: sl.wallet.equity().to_f64().unwrap_or(100.0),
                balance: sl.wallet.balance().to_f64().unwrap_or(100.0),
                unrealized_pnl: sl.wallet.unrealized_pnl().to_f64().unwrap_or(0.0),
                realized_pnl: sl.wallet.realized_pnl().to_f64().unwrap_or(0.0),
                win_rate: sl.win_rate(),
                trades: sl.trade_count(),
                age_hours: sl.age_hours(),
                generation: sl.generation,
                parent: sl.parent.clone(),
                open_positions: sl.wallet.open_position_count(),
                fees: sl.wallet.total_fees().to_f64().unwrap_or(0.0),
                params: sl.params.params.clone(),
            }
        }).collect();
        let total = s.strategy_manager.alive_count();
        (rows, total)
    };
    // Lock is now released — serialization happens without holding it.
    let mut strategies: Vec<serde_json::Value> = rows.into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.name,
                "name": r.name,
                "market": r.market,
                "strategy_type": r.strategy_type,
                "status": r.status,
                // Numeric fields — page calls .toFixed() on these; win_rate is 0–1 fraction
                "sharpe": (r.sharpe * 100.0).round() / 100.0,
                "pnl": (r.pnl * 100.0).round() / 100.0,
                "total_pnl": (r.pnl * 100.0).round() / 100.0,
                "unrealized_pnl": r.unrealized_pnl,
                "realized_pnl": r.realized_pnl,
                "equity": r.equity,
                "balance": r.balance,
                "open_positions": r.open_positions,
                "fees": r.fees,
                "win_rate": r.win_rate,
                "trades": r.trades,
                "age_hours": r.age_hours,
                "generation": r.generation,
                "parent": r.parent,
                "params": r.params,
            })
        })
        .collect();
    strategies.sort_by(|a, b| {
        let sa = a["sharpe"].as_f64().unwrap_or(0.0);
        let sb = b["sharpe"].as_f64().unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    // Add rank
    for (i, s) in strategies.iter_mut().enumerate() {
        s.as_object_mut().unwrap().insert("rank".into(), serde_json::json!(i + 1));
    }
    Json(serde_json::json!({
        "strategies": strategies,
        "total_alive": total_alive,
    }))
}

pub async fn get_timeline(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let events: Vec<serde_json::Value> = state.evolution_engine.recent_events(50).iter()
        .map(|e| {
            // Map action to the string literals the dashboard expects
            let event_type = match e.action {
                tradoshka_engine::EvolutionAction::Killed => "KILLED",
                tradoshka_engine::EvolutionAction::Spawned => "SPAWNED",
                tradoshka_engine::EvolutionAction::Started => "SPAWNED",
            };
            serde_json::json!({
                "timestamp": e.timestamp.to_rfc3339(),
                "hour": e.hour,
                // "type" is the field the dashboard TimelineEvent reads
                "type": event_type,
                "strategy_name": e.strategy_name,
                "details": e.details,
                // Legacy aliases
                "action": format!("{:?}", e.action),
                "strategy": e.strategy_name,
            })
        })
        .collect();
    Json(serde_json::json!({
        "events": events,
        "current_hour": state.evolution_engine.hour,
    }))
}

pub async fn get_graveyard(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let dead: Vec<serde_json::Value> = state.strategy_manager.dead_slots().iter()
        .map(|s| serde_json::json!({
            "name": s.name,
            "market": s.market,
            "strategy_type": s.params.strategy_type,
            // Numeric fields — dashboard calls .toFixed() on these
            "lifetime_hours": s.age_hours(),
            "trades": s.trade_count(),
            "final_pnl": (s.pnl_pct() * 100.0).round() / 100.0,
            "win_rate": s.win_rate(),
            "sharpe": (s.sharpe_ratio() * 100.0).round() / 100.0,
            "cause_of_death": s.cause_of_death.clone().unwrap_or_else(|| "Unknown".to_string()),
            "killed_at": s.killed_at.map(|t| t.to_rfc3339()),
            "generation": s.generation,
            "parent": s.parent,
        }))
        .collect();
    Json(serde_json::json!({
        "strategies": dead,
        "total_dead": state.strategy_manager.dead_count(),
    }))
}

pub async fn get_evolution_stats(State(state): State<SharedState>) -> Json<serde_json::Value> {
    // Collect summary data under a short read lock, then release before building JSON.
    let (alive_count, dead_count, total_count, evo_hour, avg_sharpe, total_capital, best_name, best_sharpe) = {
        let s = state.read().await;
        let alive = s.strategy_manager.alive_slots();
        let avg_sharpe = if alive.is_empty() { 0.0 } else {
            alive.iter().map(|sl| sl.sharpe_ratio()).sum::<f64>() / alive.len() as f64
        };
        let best = alive.iter().max_by(|a, b|
            a.sharpe_ratio().partial_cmp(&b.sharpe_ratio()).unwrap_or(std::cmp::Ordering::Equal)
        );
        let total_capital: f64 = alive.iter()
            .map(|sl| sl.wallet.equity().to_f64().unwrap_or(100.0))
            .sum();
        let best_name = best.map(|sl| sl.name.clone()).unwrap_or_else(|| "—".to_string());
        let best_sharpe = best.map(|sl| sl.sharpe_ratio()).unwrap_or(0.0);
        (
            s.strategy_manager.alive_count(),
            s.strategy_manager.dead_count(),
            s.strategy_manager.total_count(),
            s.evolution_engine.hour,
            avg_sharpe,
            total_capital,
            best_name,
            best_sharpe,
        )
    };

    Json(serde_json::json!({
        // Canonical field names expected by the dashboard
        "alive_count": alive_count,
        "dead_count": dead_count,
        "total_count": total_count,
        "hours_running": evo_hour as f64,
        "avg_sharpe": (avg_sharpe * 100.0).round() / 100.0,
        "total_capital": total_capital,
        "best_strategy": best_name,
        // Legacy aliases kept for backwards-compat
        "alive": alive_count,
        "dead": dead_count,
        "current_hour": evo_hour,
        "total_capital_deployed": format!("{:.0}", total_capital),
        "best_sharpe": (best_sharpe * 100.0).round() / 100.0,
    }))
}

pub async fn trigger_evolution(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let mut state = state.write().await;
    let evolution_engine: *mut tradoshka_engine::EvolutionEngine = &mut state.evolution_engine;
    let strategy_manager: *mut tradoshka_engine::StrategyWalletManager = &mut state.strategy_manager;
    // SAFETY: evolution_engine and strategy_manager are disjoint fields of AppState.
    let report = unsafe { (*evolution_engine).evolve(&mut *strategy_manager) };
    Json(serde_json::json!({
        "hour": report.hour,
        "killed": report.killed,
        "spawned": report.spawned,
        "alive": report.alive_count,
        "dead": report.dead_count,
        "avg_sharpe": (report.avg_sharpe * 100.0).round() / 100.0,
        "best_strategy": report.best_strategy,
    }))
}

pub async fn get_strategy_wallet(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    match state.strategy_manager.get(&name) {
        Some(slot) => {
            let trades_list: Vec<serde_json::Value> = slot.recorder.recent_trades(20).iter().map(|t| serde_json::json!({
                "id": t.id,
                "timestamp": t.timestamp.to_rfc3339(),
                "symbol": t.symbol,
                "question": t.market_question,
                "direction": t.direction,
                "side": format!("{:?}", t.side),
                "shares": t.shares.to_string(),
                "price": t.price.to_string(),
                "fee": t.fee.to_string(),
                "pnl": t.pnl.map(|p| p.to_string()),
                "closed": t.is_closed,
                "thesis_reasoning": t.thesis_reasoning,
                "stop_loss": t.stop_loss.to_string(),
                "take_profit": t.take_profit.to_string(),
                "strategy_tier": t.strategy_tier,
            })).collect();

            let positions_list: Vec<serde_json::Value> = slot.wallet.positions().values().map(|p| serde_json::json!({
                "token_id": p.token_id,
                "question": p.market_question,
                "outcome": p.outcome,
                "side": format!("{:?}", p.side),
                "shares": p.shares.to_string(),
                "avg_price": p.avg_price.to_string(),
                "current_price": p.current_price.to_string(),
                "unrealized_pnl": p.unrealized_pnl.to_string(),
            })).collect();

            Json(serde_json::json!({
                "name": slot.name,
                "market": slot.market,
                "status": format!("{:?}", slot.status),
                "strategy_type": slot.params.strategy_type,
                "params": slot.params.params,
                "balance": slot.wallet.balance().to_string(),
                "equity": slot.wallet.equity().to_string(),
                "pnl": format!("{:.2}", slot.pnl_pct()),
                "trades": slot.trade_count(),
                "win_rate": format!("{:.0}", slot.win_rate() * 100.0),
                "sharpe": (slot.sharpe_ratio() * 100.0).round() / 100.0,
                "generation": slot.generation,
                "parent": slot.parent,
                "age_hours": slot.age_hours(),
                "open_positions": slot.wallet.open_position_count(),
                "trades_list": trades_list,
                "positions": positions_list,
                "closed_trades": slot.recorder.closed_trade_count(),
                "winning_trades": slot.recorder.winning_trade_count(),
            }))
        }
        None => Json(serde_json::json!({"error": "Strategy not found"})),
    }
}
