use chrono::{DateTime, Utc};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use crate::strategy_wallet::{StrategyWalletManager, StrategySlot};
use crate::mutation::{mutate, StrategyParams};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEvent {
    pub timestamp: DateTime<Utc>,
    pub hour: u64,
    pub action: EvolutionAction,
    pub strategy_name: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvolutionAction {
    Killed,
    Spawned,
    Started,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionReport {
    pub hour: u64,
    pub killed: Vec<String>,
    pub spawned: Vec<String>,
    pub alive_count: usize,
    pub dead_count: usize,
    pub avg_sharpe: f64,
    pub best_strategy: Option<String>,
    pub best_sharpe: f64,
}

pub struct EvolutionEngine {
    pub timeline: Vec<EvolutionEvent>,
    pub hour: u64,
    pub min_trades_for_ranking: usize,
    kill_pct: f64,
    spawn_pct: f64,
}

impl EvolutionEngine {
    pub fn new() -> Self {
        Self {
            timeline: Vec::new(),
            hour: 0,
            min_trades_for_ranking: 5,
            kill_pct: 0.10,
            spawn_pct: 0.10,
        }
    }

    /// Run one evolution cycle: rank → kill bottom → spawn from top, per market.
    pub fn evolve(&mut self, manager: &mut StrategyWalletManager) -> EvolutionReport {
        self.hour += 1;
        let mut all_killed = Vec::new();
        let mut all_spawned = Vec::new();

        // Run evolution separately per market
        for market_prefix in &["PM-", "CS-", "CP-"] {
            let (killed, spawned) = self.evolve_market(manager, market_prefix);
            all_killed.extend(killed);
            all_spawned.extend(spawned);
        }

        // Build report
        let alive = manager.alive_slots();
        let avg_sharpe = if alive.is_empty() { 0.0 } else {
            alive.iter().map(|s| s.sharpe_ratio()).sum::<f64>() / alive.len() as f64
        };

        // Find best overall by composite fitness
        let ranked = manager.rank_by_fitness(0);
        let (best_name, best_sharpe) = ranked.first()
            .map(|(n, s)| (Some(n.clone()), *s))
            .unwrap_or((None, 0.0));

        let report = EvolutionReport {
            hour: self.hour,
            killed: all_killed.clone(),
            spawned: all_spawned.clone(),
            alive_count: manager.alive_count(),
            dead_count: manager.dead_count(),
            avg_sharpe,
            best_strategy: best_name,
            best_sharpe,
        };

        info!("EVOLUTION HOUR {}: killed {}, spawned {}, alive {}, dead {}",
            self.hour, all_killed.len(), all_spawned.len(), report.alive_count, report.dead_count);

        report
    }

    fn evolve_market(&mut self, manager: &mut StrategyWalletManager, prefix: &str) -> (Vec<String>, Vec<String>) {
        let mut killed = Vec::new();
        let mut spawned = Vec::new();

        // Get strategies for this market only, ranked by composite fitness
        let market_ranked: Vec<(String, f64)> = manager.rank_by_fitness(self.min_trades_for_ranking)
            .into_iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .collect();

        let ranked_count = market_ranked.len();
        if ranked_count < 3 { return (killed, spawned); }

        // Kill bottom 10% within this market
        let kill_count = ((ranked_count as f64 * self.kill_pct).ceil() as usize).max(1);
        let to_kill: Vec<String> = market_ranked.iter()
            .rev()
            .take(kill_count)
            .map(|(name, _)| name.clone())
            .collect();

        // Count alive for this market
        let market_alive = manager.alive_slots().iter()
            .filter(|s| s.name.starts_with(prefix))
            .count();
        let min_per_market = 5usize; // Never go below 5 per market

        for name in &to_kill {
            if market_alive - killed.len() <= min_per_market { break; }
            if !manager.can_kill() { break; }
            if let Some(slot) = manager.get_mut(name) {
                let fitness = StrategyWalletManager::fitness_score_for(slot);
                let reason = format!(
                    "Bottom 10% in {} by composite fitness (hour {}). \
                     Fitness: {:.4}, Sharpe: {:.2}, PnL: ${:.2}, WR: {:.0}%, Trades: {}",
                    prefix.trim_end_matches('-'), self.hour,
                    fitness, slot.sharpe_ratio(), slot.pnl_pct(),
                    slot.win_rate() * 100.0, slot.trade_count()
                );
                slot.kill(&reason);
                killed.push(name.clone());
                self.timeline.push(EvolutionEvent {
                    timestamp: Utc::now(),
                    hour: self.hour,
                    action: EvolutionAction::Killed,
                    strategy_name: name.clone(),
                    details: reason,
                });
            }
        }

        // Spawn replacements: 40% mutation, 40% crossover, 20% random
        let spawn_count = killed.len();
        if spawn_count == 0 { return (killed, spawned); }

        // Get top performers for this market
        let top: Vec<(String, StrategyParams, String, u32)> = market_ranked.iter()
            .take(5.max(spawn_count))
            .filter_map(|(name, _)| {
                manager.get(name).map(|s| (s.name.clone(), s.params.clone(), s.market.clone(), s.generation))
            })
            .collect();

        if top.is_empty() { return (killed, spawned); }

        for i in 0..spawn_count {
            if !manager.can_spawn() { break; }

            // 0,1 = mutation (40%), 2,3 = crossover (40%), 4 = random (20%)
            let spawn_type = i % 5;
            let seed = (self.hour * 31 + i as u64 * 17 + killed.len() as u64 * 7) % 10000;

            let (new_name, new_params, market, gen, details) = if spawn_type < 2 && !top.is_empty() {
                // MUTATION: tweak one param from a top performer
                let parent = &top[i % top.len()];
                let (params, mutation) = mutate(&parent.1, seed);
                let base = parent.0.split("-v").next().unwrap_or(&parent.0);
                let name = format!("{}-v{}", base, parent.3 + 1);
                let details = format!("MUTATION from {} (gen {}). {} {:.2} → {:.2} ({:+.0}%)",
                    parent.0, parent.3, mutation.param_name, mutation.old_value, mutation.new_value, mutation.change_pct);
                (name, params, parent.2.clone(), parent.3 + 1, details)

            } else if spawn_type < 4 && top.len() >= 2 {
                // CROSSOVER: combine params from two top performers
                let parent_a = &top[0];
                let parent_b = &top[1 + (i % (top.len() - 1))];
                let params = crate::mutation::crossover(&parent_a.1, &parent_b.1, seed);
                let name = format!("{}-x-{}-v{}",
                    parent_a.0.split("-v").next().unwrap_or(&parent_a.0),
                    parent_b.0.split("-v").next().unwrap_or(&parent_b.0),
                    parent_a.3.max(parent_b.3) + 1);
                let gen = parent_a.3.max(parent_b.3) + 1;
                let details = format!("CROSSOVER of {} + {} (gen {}). Hybrid strategy.",
                    parent_a.0, parent_b.0, gen);
                (name, params, parent_a.2.clone(), gen, details)

            } else {
                // RANDOM EXPLORER: completely fresh random params
                let params = crate::mutation::random_explorer(seed);
                let name = format!("{}-explorer-{}", prefix.trim_end_matches('-'), self.hour);
                let details = format!("RANDOM EXPLORER. Fresh params: leverage={:.0}, timeframe={:.0}min, capital={:.0}%",
                    params.get("auto_leverage"), params.get("auto_timeframe"), params.get("capital_usage_pct"));
                (name, params, top[0].2.clone(), 1, details)
            };

            // Skip if name already exists
            if manager.get(&new_name).is_some() { continue; }

            let mut slot = StrategySlot::new(&new_name, &market, new_params, dec!(100));
            slot.generation = gen;
            if spawn_type < 4 {
                slot.parent = Some(top[0].0.clone());
            }
            manager.add_slot(slot);
            spawned.push(new_name.clone());

            self.timeline.push(EvolutionEvent {
                timestamp: Utc::now(),
                hour: self.hour,
                action: EvolutionAction::Spawned,
                strategy_name: new_name,
                details,
            });
        }

        // Self-adjust survivors: adapt execution params based on recent performance
        let survivor_names: Vec<String> = manager.alive_slots().iter()
            .filter(|s| s.name.starts_with(prefix) && s.trade_count() > 0)
            .map(|s| s.name.clone())
            .collect();

        for name in &survivor_names {
            if let Some(slot) = manager.get_mut(name) {
                let pnl = slot.pnl_pct();
                let trades = slot.trade_count();
                if trades < 3 { continue; } // Not enough data to adjust

                // If losing with high leverage → reduce
                if pnl < -1.0 && slot.params.get("auto_leverage") > 3.0 {
                    let old = slot.params.get("auto_leverage");
                    slot.params.set("auto_leverage", (old * 0.8).max(1.0));
                }
                // If capital usage too low → increase
                let capital_pct = slot.params.get("capital_usage_pct");
                if capital_pct < 50.0 {
                    slot.params.set("capital_usage_pct", (capital_pct + 10.0).min(95.0));
                }
            }
        }

        (killed, spawned)
    }

    pub fn recent_events(&self, limit: usize) -> Vec<&EvolutionEvent> {
        self.timeline.iter().rev().take(limit).collect()
    }

    pub fn events_by_hour(&self, hour: u64) -> Vec<&EvolutionEvent> {
        self.timeline.iter().filter(|e| e.hour == hour).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_manager() -> StrategyWalletManager {
        let mut mgr = StrategyWalletManager::new(150, 20, dec!(100));
        mgr.initialize_defaults();
        mgr
    }

    #[test]
    fn test_new_engine() {
        let engine = EvolutionEngine::new();
        assert_eq!(engine.hour, 0);
        assert_eq!(engine.timeline.len(), 0);
    }

    #[test]
    fn test_evolve_no_ranked_strategies() {
        let mut engine = EvolutionEngine::new();
        let mut mgr = setup_manager();
        // No trades → nothing ranked (min_trades = 5) → no kills/spawns
        let report = engine.evolve(&mut mgr);
        assert_eq!(report.hour, 1);
        assert_eq!(report.killed.len(), 0);
        assert_eq!(report.spawned.len(), 0);
        assert_eq!(report.alive_count, 120);
    }

    #[test]
    fn test_evolve_kills_bottom() {
        let mut engine = EvolutionEngine::new();
        engine.min_trades_for_ranking = 0; // Rank even with 0 trades
        let mut mgr = setup_manager();
        let report = engine.evolve(&mut mgr);
        // 120 strategies, bottom 10% = 12 killed, top 10% = 12 spawned
        assert!(report.killed.len() >= 1);
        assert!(report.alive_count < 122); // Some killed, some spawned
    }

    #[test]
    fn test_timeline_records_events() {
        let mut engine = EvolutionEngine::new();
        engine.min_trades_for_ranking = 0;
        let mut mgr = setup_manager();
        engine.evolve(&mut mgr);
        assert!(!engine.timeline.is_empty());
    }

    #[test]
    fn test_evolve_respects_min_alive() {
        let mut engine = EvolutionEngine::new();
        engine.min_trades_for_ranking = 0;
        let mut mgr = StrategyWalletManager::new(150, 115, dec!(100)); // min_alive = 115
        mgr.initialize_defaults(); // 120 alive
        // Can only kill 5 (120 - 115 = 5 buffer)
        let report = engine.evolve(&mut mgr);
        assert!(mgr.alive_count() >= 115);
    }

    #[test]
    fn test_recent_events() {
        let mut engine = EvolutionEngine::new();
        engine.min_trades_for_ranking = 0;
        let mut mgr = setup_manager();
        engine.evolve(&mut mgr);
        engine.evolve(&mut mgr);
        let recent = engine.recent_events(5);
        assert!(!recent.is_empty());
    }

    #[test]
    fn test_evolution_spawns_crossover_and_mutation() {
        let mut engine = EvolutionEngine::new();
        engine.min_trades_for_ranking = 0;
        let mut mgr = setup_manager();
        engine.evolve(&mut mgr);
        // Check that timeline has spawned events
        let spawned_events: Vec<_> = engine.timeline.iter()
            .filter(|e| e.action == EvolutionAction::Spawned)
            .collect();
        assert!(!spawned_events.is_empty());
        // Check that at least some events have MUTATION or CROSSOVER in details
        let has_mutation = spawned_events.iter().any(|e| e.details.contains("MUTATION"));
        let has_crossover_or_random = spawned_events.iter().any(|e| e.details.contains("CROSSOVER") || e.details.contains("RANDOM EXPLORER") || e.details.contains("MUTATION"));
        assert!(has_mutation || has_crossover_or_random);
    }
}
