use axum::{extract::State, Json, extract::Path};
use crate::state::SharedState;
use tradoshka_engine;

pub async fn get_leaderboard(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let mut strategies: Vec<serde_json::Value> = state.strategy_manager.alive_slots().iter()
        .map(|s| serde_json::json!({
            "id": s.name,
            "name": s.name,
            "market": s.market,
            "strategy_type": s.params.strategy_type,
            "status": format!("{:?}", s.status),
            // Numeric fields — page calls .toFixed() on these; win_rate is 0–1 fraction
            "sharpe": (s.sharpe_ratio() * 100.0).round() / 100.0,
            "pnl": (s.pnl_pct() * 100.0).round() / 100.0,
            "win_rate": s.win_rate(),
            "trades": s.trade_count(),
            "age_hours": s.age_hours(),
            "generation": s.generation,
            "parent": s.parent,
            "params": s.params.params,
        }))
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
        "total_alive": state.strategy_manager.alive_count(),
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
    let state = state.read().await;
    let alive = state.strategy_manager.alive_slots();
    let avg_sharpe = if alive.is_empty() { 0.0 } else {
        alive.iter().map(|s| s.sharpe_ratio()).sum::<f64>() / alive.len() as f64
    };
    let best = alive.iter().max_by(|a, b|
        a.sharpe_ratio().partial_cmp(&b.sharpe_ratio()).unwrap_or(std::cmp::Ordering::Equal)
    );
    let total_capital: f64 = alive.iter().map(|s| s.wallet.equity().to_string().parse::<f64>().unwrap_or(100.0)).sum();

    Json(serde_json::json!({
        // Canonical field names expected by the dashboard
        "alive_count": state.strategy_manager.alive_count(),
        "dead_count": state.strategy_manager.dead_count(),
        "total_count": state.strategy_manager.total_count(),
        "hours_running": state.evolution_engine.hour as f64,
        "avg_sharpe": (avg_sharpe * 100.0).round() / 100.0,
        "total_capital": total_capital,
        "best_strategy": best.map(|s| s.name.clone()).unwrap_or_else(|| "—".to_string()),
        // Legacy aliases kept for backwards-compat
        "alive": state.strategy_manager.alive_count(),
        "dead": state.strategy_manager.dead_count(),
        "current_hour": state.evolution_engine.hour,
        "total_capital_deployed": format!("{:.0}", total_capital),
        "best_sharpe": best.map(|s| (s.sharpe_ratio() * 100.0).round() / 100.0).unwrap_or(0.0),
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
                "positions": slot.wallet.open_position_count(),
            }))
        }
        None => Json(serde_json::json!({"error": "Strategy not found"})),
    }
}
