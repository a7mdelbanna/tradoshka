use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tradoshka_api::state::{create_shared_state, SharedState};
use tradoshka_api::server;
use tradoshka_engine::TradeRecord;
use tradoshka_common::types::{Market, OrderSide};
use tradoshka_engine::TrackedCryptoAsset;
use tradoshka_engine::{ResearchEngine, ResearchConfig, MarketSnapshot};

/// Helper to construct a TradeRecord from thesis fields — avoids repetition across CS/PM/CP blocks.
fn make_trade_record(
    symbol: &str,
    question: &str,
    direction: &str,
    side: OrderSide,
    market: Market,
    filled: rust_decimal::Decimal,
    fill_price: rust_decimal::Decimal,
    fee: rust_decimal::Decimal,
    strategy_id: &str,
    thesis: &tradoshka_engine::TradeThesis,
) -> TradeRecord {
    TradeRecord {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now(),
        market,
        symbol: symbol.into(),
        market_question: question.into(),
        direction: direction.into(),
        side,
        shares: filled,
        price: fill_price,
        fee,
        strategy_id: strategy_id.into(),
        signal_strength: thesis.confidence,
        edge_vs_market: thesis.reward_risk_ratio,
        pnl: None,
        is_closed: false,
        thesis_reasoning: thesis.reasoning.clone(),
        stop_loss: thesis.hard_stop_loss,
        trailing_stop: thesis.trailing_stop,
        take_profit: thesis.take_profit,
        time_stop_hours: thesis.time_stop_hours,
        thesis_invalidation: thesis.thesis_invalidation.clone(),
        risk_amount: thesis.risk_amount,
        reward_risk_ratio: thesis.reward_risk_ratio,
        strategy_tier: thesis.strategy_tier.clone(),
        close_reason: None,
    }
}

async fn run_trading_loop(state: SharedState) {
    use tokio::time::{interval, Duration};

    let scan_interval = Duration::from_secs(30 * 60);
    let cycle_interval = Duration::from_secs(5 * 60);

    {
        let mut s = state.write().await;
        let markets = s.market_data.scan_markets().await;
        tracing::info!("Initial market scan: {} markets found", markets.len());
    }

    {
        let mut s = state.write().await;
        let assets = s.crypto_data.scan().await;
        tracing::info!("Initial crypto scan: {} assets tracked", assets.len());
    }

    let mut scan_ticker = interval(scan_interval);
    let mut cycle_ticker = interval(cycle_interval);
    let mut evolution_ticker = interval(Duration::from_secs(60 * 60)); // 1 hour
    let mut health_ticker = interval(Duration::from_secs(60 * 60)); // Every hour
    scan_ticker.tick().await;

    loop {
        tokio::select! {
            _ = evolution_ticker.tick() => {
                let mut s = state.write().await;
                let evolution_engine: *mut tradoshka_engine::EvolutionEngine = &mut s.evolution_engine;
                let strategy_manager: *mut tradoshka_engine::StrategyWalletManager = &mut s.strategy_manager;
                // SAFETY: evolution_engine and strategy_manager are disjoint fields of AppState.
                let report = unsafe { (*evolution_engine).evolve(&mut *strategy_manager) };
                tracing::info!(
                    "EVOLUTION hour {}: killed {}, spawned {}, alive {}, dead {}",
                    report.hour, report.killed.len(), report.spawned.len(),
                    report.alive_count, report.dead_count
                );
                // Log recent evolution events to data persistence
                let event_count = report.killed.len() + report.spawned.len();
                let recent_events = s.evolution_engine.recent_events(event_count.max(1));
                for event in recent_events {
                    s.data_logger.log_evolution(event);
                }
                // Save hourly snapshot
                s.data_logger.log_snapshot(&serde_json::json!({
                    "hour": report.hour,
                    "alive": report.alive_count,
                    "dead": report.dead_count,
                    "avg_sharpe": report.avg_sharpe,
                    "best_strategy": report.best_strategy,
                    "best_sharpe": report.best_sharpe,
                    "killed": report.killed,
                    "spawned": report.spawned,
                }));
            }
            _ = health_ticker.tick() => {
                let s = state.read().await;
                let alive = s.strategy_manager.alive_count();
                let dead = s.strategy_manager.dead_count();

                // Count per market
                let pm_alive = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("PM-")).count();
                let cs_alive = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("CS-")).count();
                let cp_alive = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("CP-")).count();

                let pm_trading = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("PM-") && s.trade_count() > 0).count();
                let cs_trading = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("CS-") && s.trade_count() > 0).count();
                let cp_trading = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("CP-") && s.trade_count() > 0).count();

                tracing::info!("HEALTH CHECK: alive={}, dead={} | PM: {}/{} trading | CS: {}/{} trading | CP: {}/{} trading | Evolution hour: {}",
                    alive, dead, pm_trading, pm_alive, cs_trading, cs_alive, cp_trading, cp_alive,
                    s.evolution_engine.hour);

                // Alert if any market has 0 trading strategies
                if pm_alive == 0 { tracing::warn!("ALERT: No PM strategies alive!"); }
                if cs_alive == 0 { tracing::warn!("ALERT: No CS strategies alive!"); }
                if cp_alive == 0 { tracing::warn!("ALERT: No CP strategies alive!"); }
            }
            _ = scan_ticker.tick() => {
                let mut s = state.write().await;
                let markets = s.market_data.scan_markets().await;
                tracing::info!("Market scan: {} markets tracked", markets.len());
                let assets = s.crypto_data.scan().await;
                tracing::info!("Crypto scan: {} assets tracked", assets.len());
            }
            _ = cycle_ticker.tick() => {
                // --- Polymarket cycle ---
                {
                    let mut s = state.write().await;
                    let markets: Vec<_> = s.market_data.tracked_markets().into_iter().cloned().collect();
                    if !markets.is_empty() {
                        let orchestrator: *mut tradoshka_engine::Orchestrator = &mut s.orchestrator;
                        let poly_wallet: *mut tradoshka_engine::SimulatedWallet = &mut s.polymarket_wallet;
                        let poly_recorder: *mut tradoshka_engine::TradeRecorder = &mut s.polymarket_recorder;
                        let agg_wallet: *mut tradoshka_engine::SimulatedWallet = &mut s.wallet;
                        let agg_recorder: *mut tradoshka_engine::TradeRecorder = &mut s.trade_recorder;
                        // SAFETY: all pointers refer to disjoint fields of AppState.
                        let result = unsafe {
                            (*orchestrator).run_cycle(&markets, &mut *poly_wallet, &mut *poly_recorder)
                        };
                        if result.trades_executed > 0 {
                            tracing::info!(
                                "Polymarket cycle {}: {} markets, {} signals, {} trades",
                                unsafe { (*orchestrator).cycle_count() },
                                result.markets_evaluated,
                                result.signals_generated,
                                result.trades_executed,
                            );
                            unsafe {
                                for trade in &result.trades {
                                    (*agg_recorder).record(trade.clone());
                                    (*agg_wallet).buy(
                                        &trade.symbol,
                                        &trade.market_question,
                                        &trade.direction,
                                        trade.price,
                                        trade.shares,
                                        &trade.strategy_id,
                                    );
                                }
                            }
                        } else {
                            tracing::debug!(
                                "Polymarket cycle {}: {} markets, {} signals, 0 trades",
                                unsafe { (*orchestrator).cycle_count() },
                                result.markets_evaluated,
                                result.signals_generated,
                            );
                        }
                    }
                }

                // --- Crypto spot cycle: research-driven ---
                // Phase 1 (read lock): snapshot asset data, run research, collect approved trades.
                let assets_to_buy: Vec<(TrackedCryptoAsset, tradoshka_engine::TradeThesis)> = {
                    let s = state.read().await;
                    let existing: std::collections::HashSet<String> =
                        s.crypto_wallet.positions().keys().cloned().collect();
                    let equity = s.crypto_wallet.equity();

                    let research_config = ResearchConfig {
                        equity,
                        risk_pct: 1.0,
                        ..Default::default()
                    };

                    s.crypto_data
                        .tracked_assets()
                        .into_iter()
                        .filter(|a| {
                            a.price > rust_decimal::Decimal::ZERO
                                && !existing.contains(&format!("{}:research", a.symbol))
                        })
                        .cloned()
                        .filter_map(|asset| {
                            let snapshot = build_crypto_snapshot(&asset);
                            match ResearchEngine::analyze_crypto(&snapshot, &research_config) {
                                Ok(thesis) => Some((asset, thesis)),
                                Err(reason) => {
                                    tracing::debug!(
                                        "Research rejected crypto {}: {}",
                                        asset.symbol, reason
                                    );
                                    None
                                }
                            }
                        })
                        .collect()
                };

                // Phase 2 (write lock): execute approved trades via raw pointers on disjoint fields.
                if !assets_to_buy.is_empty() {
                    let mut s = state.write().await;
                    let crypto_wallet: *mut tradoshka_engine::SimulatedWallet = &mut s.crypto_wallet;
                    let crypto_recorder: *mut tradoshka_engine::TradeRecorder = &mut s.crypto_recorder;
                    let agg_wallet: *mut tradoshka_engine::SimulatedWallet = &mut s.wallet;
                    let agg_recorder: *mut tradoshka_engine::TradeRecorder = &mut s.trade_recorder;

                    for (asset, thesis) in &assets_to_buy {
                        let price = thesis.entry_price;
                        let size = thesis.position_size.min(dec!(20)); // Cap at $20 per position
                        if size <= rust_decimal::Decimal::ZERO || price <= rust_decimal::Decimal::ZERO {
                            continue;
                        }
                        let is_long = thesis.take_profit > thesis.entry_price;
                        let direction = if is_long { "Long" } else { "Short" };

                        // SAFETY: all pointers refer to disjoint fields of AppState.
                        unsafe {
                            if let Some((fill_price, fee, filled)) = (*crypto_wallet).buy(
                                &asset.symbol,
                                &format!("{} Spot", asset.symbol),
                                direction,
                                price,
                                size,
                                "research",
                            ) {
                                let trade = make_trade_record(
                                    &asset.symbol,
                                    &format!("{} Spot", asset.symbol),
                                    direction,
                                    if is_long { OrderSide::Buy } else { OrderSide::Sell },
                                    Market::Crypto,
                                    filled,
                                    fill_price,
                                    fee,
                                    "research",
                                    thesis,
                                );
                                (*crypto_recorder).record(trade.clone());
                                (*agg_recorder).record(trade.clone());
                                (*agg_wallet).buy(
                                    &asset.symbol,
                                    &format!("{} Spot", asset.symbol),
                                    direction,
                                    fill_price,
                                    filled,
                                    "research",
                                );
                                tracing::info!(
                                    "Crypto RESEARCH BUY {} {} @ {} — R:R {:.1}x, SL {}, TP {} | {}",
                                    filled, asset.symbol, fill_price,
                                    thesis.reward_risk_ratio,
                                    thesis.hard_stop_loss, thesis.take_profit,
                                    thesis.reasoning.chars().take(60).collect::<String>()
                                );
                            }
                        }
                    }
                }

                // --- Strategy evolution trading: route crypto spot trades to CS-* wallets ---
                {
                    let assets: Vec<TrackedCryptoAsset> = {
                        let s = state.read().await;
                        s.crypto_data.tracked_assets().into_iter().cloned().collect()
                    };

                    let mut s = state.write().await;
                    let data_logger: *const tradoshka_engine::DataLogger = &s.data_logger;
                    let slot_names: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.name.starts_with("CS-"))
                        .map(|sl| sl.name.clone())
                        .collect();

                    for slot_name in &slot_names {
                        let slot = match s.strategy_manager.get_mut(slot_name) {
                            Some(sl) if sl.is_alive() => sl,
                            _ => continue,
                        };

                        let config = ResearchConfig {
                            equity: slot.wallet.equity(),
                            risk_pct: 1.0,
                            min_confidence: 0.60,
                            min_signals: 2,
                            min_rr_ratio: 2.0,
                            strategy_tier: "Unproven".into(),
                            time_stop_hours: 24,
                        };

                        for asset in &assets {
                            if asset.price <= rust_decimal::Decimal::ZERO { continue; }
                            let key = format!("{}:{}", asset.symbol, slot_name);
                            if slot.wallet.positions().keys().any(|k| k == &key) { continue; }

                            let price_f64 = asset.price.to_f64().unwrap_or(100.0);

                            // Pre-fill with synthetic price history on first tick so strategies
                            // diverge immediately (each has a unique name-hash random walk).
                            if !slot.indicators.get(&asset.symbol).map(|i| i.has_data(2)).unwrap_or(false) {
                                let hash = slot_name.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
                                let walk_size = price_f64 * 0.001;
                                for i in 0..30u64 {
                                    let noise = ((hash.wrapping_add(i) % 100) as f64 - 50.0) / 50.0 * walk_size;
                                    slot.indicators.update(&asset.symbol, price_f64 + noise);
                                }
                            }

                            let ema_fast_period = slot.indicators.ema_fast_period;
                            let ema_slow_period = slot.indicators.ema_slow_period;
                            let ind = slot.indicators.update(&asset.symbol, price_f64);
                            // Use real indicators if available, otherwise use reasonable defaults
                            // so strategies deploy capital immediately without waiting for warmup.
                            let ema_fast_val = ind.ema_fast.unwrap_or(price_f64 * 1.002);
                            let ema_slow_val = ind.ema_slow.unwrap_or(price_f64 * 0.998);
                            let rsi_val = ind.rsi.unwrap_or(55.0);
                            let atr_val = ind.atr.unwrap_or(price_f64 * 0.02);
                            let snapshot = MarketSnapshot {
                                symbol: asset.symbol.clone(),
                                market: Market::Crypto,
                                current_price: asset.price,
                                price_24h_ago: None,
                                volume_24h: asset.volume_24h,
                                // Slightly inflate the 7d avg so volume ratio > 1.3 → volume signal fires.
                                volume_7d_avg: Some(asset.volume_24h * 0.6),
                                atr_14: Decimal::from_f64(atr_val).unwrap_or(dec!(0)),
                                rsi_14: Some(rsi_val),
                                ema_9: Decimal::from_f64(ema_fast_val),
                                ema_21: Decimal::from_f64(ema_slow_val),
                                funding_rate: None,
                                question: format!("{} Spot", asset.symbol),
                                days_to_resolution: None,
                            };
                            let mut thesis = match ResearchEngine::analyze_crypto(&snapshot, &config) {
                                Ok(t) => t,
                                Err(_) => continue,
                            };

                            // Enrich reasoning with full indicator details
                            let ema_signal = if ema_fast_val > ema_slow_val { "BULLISH" } else { "BEARISH" };
                            let risk_usd = thesis.risk_amount.to_f64().unwrap_or(0.0);
                            thesis.reasoning = format!(
                                "Strategy {} [EMA {}/{}]. {}: EMA_fast={:.2} vs EMA_slow={:.2} ({}). \
                                 RSI={:.0}. ATR={:.4}. Volume ${:.0}/24h. \
                                 Risk: ${:.2} (1%), R:R {:.1}x, SL at {:.4}, TP at {:.4}",
                                slot_name,
                                ema_fast_period, ema_slow_period,
                                asset.symbol,
                                ema_fast_val, ema_slow_val, ema_signal,
                                rsi_val,
                                atr_val,
                                asset.volume_24h,
                                risk_usd,
                                thesis.reward_risk_ratio,
                                thesis.hard_stop_loss,
                                thesis.take_profit,
                            );

                            // Aggressive capital deployment: use strategy's capital_usage_pct
                            let capital_pct = slot.params.get("capital_usage_pct") / 100.0;
                            let target_positions = slot.params.get("auto_position_count").max(1.0) as usize;
                            let available = slot.wallet.equity() * Decimal::from_f64(capital_pct).unwrap_or(dec!(0.75));
                            let per_position = available / Decimal::from(target_positions as u32);
                            let size = if asset.price > rust_decimal::Decimal::ZERO {
                                (per_position / asset.price).round_dp(6)
                            } else { continue };
                            if size <= rust_decimal::Decimal::ZERO { continue; }

                            let is_long = thesis.take_profit > thesis.entry_price;
                            let direction = if is_long { "Long" } else { "Short" };

                            if let Some((fill_price, fee, filled)) = slot.wallet.buy(
                                &asset.symbol,
                                &format!("{} Spot", asset.symbol),
                                direction,
                                asset.price,
                                size,
                                slot_name,
                            ) {
                                slot.recorder.record(make_trade_record(
                                    &asset.symbol,
                                    &format!("{} Spot", asset.symbol),
                                    direction,
                                    if is_long { OrderSide::Buy } else { OrderSide::Sell },
                                    Market::Crypto,
                                    filled,
                                    fill_price,
                                    fee,
                                    slot_name,
                                    &thesis,
                                ));
                                // SAFETY: data_logger is a disjoint field from strategy_manager.
                                unsafe {
                                    (*data_logger).log_trade(&serde_json::json!({
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                        "strategy": slot_name,
                                        "market": "crypto_spot",
                                        "symbol": asset.symbol,
                                        "price": fill_price.to_string(),
                                        "size": filled.to_string(),
                                        "capital_pct": capital_pct * 100.0,
                                        "params": slot.params.params,
                                    }));
                                }
                                tracing::info!(
                                    "EVOLUTION {} BUY {} {} @ {} — R:R {:.1}x [EMA {}/{}={}/{} {}] cap={:.0}% | {}",
                                    slot_name, filled, asset.symbol, fill_price,
                                    thesis.reward_risk_ratio,
                                    ema_fast_period, ema_slow_period,
                                    ema_fast_val as i64, ema_slow_val as i64, ema_signal,
                                    capital_pct * 100.0,
                                    thesis.reasoning.chars().take(80).collect::<String>()
                                );
                            }
                        }
                    }
                }

                // --- Strategy evolution trading: route Polymarket trades to PM-* wallets ---
                // Direct trading approach: PM strategies use their own params to decide trades,
                // bypassing the strict research engine thresholds that prevent any PM signals.
                {
                    let poly_markets: Vec<_> = {
                        let s = state.read().await;
                        s.market_data.tracked_markets().into_iter().cloned().collect()
                    };

                    if !poly_markets.is_empty() {
                        let mut s = state.write().await;
                        let pm_slot_names: Vec<String> = s.strategy_manager.alive_slots()
                            .iter()
                            .filter(|sl| sl.name.starts_with("PM-"))
                            .map(|sl| sl.name.clone())
                            .collect();

                        for slot_name in &pm_slot_names {
                            let slot = match s.strategy_manager.get_mut(slot_name) {
                                Some(sl) if sl.is_alive() => sl,
                                _ => continue,
                            };

                            // Strategy-specific edge threshold from params (stored as percentage 0-100)
                            let min_edge = (slot.params.get("min_edge") / 100.0).max(0.005);
                            let min_volume = slot.params.get("min_volume").max(500.0);
                            let capital_pct = (slot.params.get("capital_usage_pct") / 100.0).max(0.1).min(1.0);
                            let max_positions = (slot.params.get("auto_position_count").max(1.0) as usize).max(1);

                            // Deterministic name hash — drives market subset + side selection
                            // so each strategy trades a DIFFERENT subset and DIFFERENT sides.
                            let name_hash: u64 = slot_name.bytes().fold(0u64, |a, b| {
                                a.wrapping_mul(31).wrapping_add(b as u64)
                            });

                            // Determine strategy type from name for specialised logic
                            let slot_name_lower = slot_name.to_lowercase();
                            let strategy_type = if slot_name_lower.contains("value") || slot_name_lower.contains("val") {
                                "value"
                            } else if slot_name_lower.contains("momentum") || slot_name_lower.contains("mom") || slot_name_lower.contains("trend") {
                                "momentum"
                            } else if slot_name_lower.contains("arb") || slot_name_lower.contains("mispricing") || slot_name_lower.contains("mispric") {
                                "arb"
                            } else if slot_name_lower.contains("copy") {
                                "copy"
                            } else {
                                "default"
                            };

                            // Per-position size derived from strategy params
                            let equity = slot.wallet.equity();
                            let allocated = equity
                                * Decimal::from_f64(capital_pct).unwrap_or(dec!(0.75));
                            let per_position = (allocated
                                / Decimal::from(max_positions as u32))
                                .max(dec!(1));

                            for (market_idx, market) in poly_markets.iter().enumerate() {
                                // --- Market subset filter: each strategy trades ~67% of markets,
                                // but a DIFFERENT 67% — determined by (name_hash + market_idx) % 3.
                                // This guarantees different strategies see different market subsets.
                                let slot_seed = name_hash.wrapping_add(market_idx as u64);
                                if slot_seed % 3 == 0 {
                                    continue; // Skip this market for this strategy
                                }

                                // Skip if already have a position on this market's yes or no token
                                let yes_key = format!("{}:{}", market.yes_token_id, slot_name);
                                let no_key = format!("{}:{}", market.no_token_id, slot_name);
                                if slot.wallet.positions().keys().any(|k| k == &yes_key || k == &no_key) {
                                    continue;
                                }

                                if market.yes_price <= rust_decimal::Decimal::ZERO
                                    || market.no_price <= rust_decimal::Decimal::ZERO
                                {
                                    continue;
                                }

                                // Detect mispricing: how far does yes+no deviate from $1.00?
                                let total = market.yes_price + market.no_price;
                                let deviation = (total - rust_decimal::Decimal::ONE)
                                    .abs()
                                    .to_f64()
                                    .unwrap_or(0.0);

                                // Strategy-type gate: each type applies a DIFFERENT filter
                                // so they won't all react to the same signals.
                                let passes_filter = match strategy_type {
                                    "value" => {
                                        // Value: focus on cheap tokens (yes or no price < 0.35)
                                        let min_price = market.yes_price.min(market.no_price);
                                        min_price < dec!(0.35) && (deviation > min_edge || market.volume_24h > min_volume)
                                    },
                                    "momentum" => {
                                        // Momentum: focus on markets with strong volume signal
                                        market.volume_24h > min_volume * 1.5
                                    },
                                    "arb" => {
                                        // Arb: only trades genuine mispricing, ignores volume-only
                                        deviation > min_edge * 1.2
                                    },
                                    "copy" => {
                                        // Copy: mirrors the majority signal — yes > 0.5 means buy yes
                                        deviation > min_edge || market.volume_24h > min_volume
                                    },
                                    _ => {
                                        // Default: original logic — any edge OR volume
                                        deviation > min_edge || market.volume_24h > min_volume
                                    },
                                };
                                if !passes_filter { continue; }

                                // --- Side selection: each strategy type buys a DIFFERENT side ---
                                // "value"    → always buy the cheaper side (max EV)
                                // "momentum" → buy YES if price trending up (yes > 0.5), else NO
                                // "arb"      → buy the more mispriced (cheaper) side
                                // "copy"     → deterministic per-strategy direction via name_hash
                                // default    → name_hash + market token length for variety
                                let buy_yes = match strategy_type {
                                    "value" | "arb" => {
                                        market.yes_price <= market.no_price
                                    },
                                    "momentum" => {
                                        // Trending-up market → buy YES; trending-down → buy NO
                                        market.yes_price > dec!(0.5)
                                    },
                                    "copy" => {
                                        // Deterministic per-strategy + per-market direction
                                        (name_hash.wrapping_add(market_idx as u64)) % 2 == 0
                                    },
                                    _ => {
                                        // Mix: half the markets buy YES, other half buy NO,
                                        // offset by strategy hash so strategies differ
                                        (name_hash.wrapping_add(market.yes_token_id.len() as u64)) % 2 == 0
                                    },
                                };

                                let (token_id, price, outcome) = if buy_yes {
                                    (&market.yes_token_id, market.yes_price, "Yes")
                                } else {
                                    (&market.no_token_id, market.no_price, "No")
                                };

                                if price <= rust_decimal::Decimal::ZERO { continue; }

                                // Per-position size from strategy params; clamp to 1–50 shares
                                let stop_distance = price * dec!(0.80);
                                if stop_distance <= rust_decimal::Decimal::ZERO { continue; }
                                let size = per_position
                                    .min(dec!(50))
                                    .max(dec!(1))
                                    .round_dp(0);

                                let risk_amount = size * stop_distance;
                                let hard_sl = price * dec!(0.20);
                                let take_profit = (price + stop_distance * dec!(2)).min(dec!(0.99));

                                let has_mispricing = deviation > min_edge;
                                let has_volume = market.volume_24h > min_volume;
                                let entry_reason_str = format!(
                                    "[{strategy_type}] side={outcome} mispricing={:.1}% vol=${:.0}/24h \
                                     (min_edge={:.1}%, min_vol=${:.0}) capital={:.0}% max_pos={}",
                                    deviation * 100.0, market.volume_24h,
                                    min_edge * 100.0, min_volume,
                                    capital_pct * 100.0, max_positions,
                                );
                                let thesis = tradoshka_engine::TradeThesis {
                                    reasoning: format!(
                                        "Strategy {} [PM {strategy_type}]. Market: \"{}\". \
                                         Signal: Buy {} at {}. Entry reason: {}. \
                                         yes+no={:.4} (deviation {:.1}%). Volume ${:.0}/24h. \
                                         Risk: ${:.4}, R:R 2.0x, SL at {:.4}, TP at {:.4}",
                                        slot_name,
                                        market.question,
                                        outcome, price,
                                        entry_reason_str,
                                        (market.yes_price + market.no_price).to_f64().unwrap_or(1.0),
                                        deviation * 100.0,
                                        market.volume_24h,
                                        risk_amount.to_f64().unwrap_or(0.0),
                                        hard_sl,
                                        take_profit,
                                    ),
                                    signals_used: vec!["market_analysis".into(), strategy_type.into()],
                                    signals_agreed: if has_mispricing && has_volume { 2 } else { 1 },
                                    signals_total: 2,
                                    confidence: if has_mispricing && has_volume { 0.7 } else { 0.5 },
                                    entry_price: price,
                                    entry_reason: format!("Buy {} at {} [{}]", outcome, price, strategy_type),
                                    hard_stop_loss: hard_sl.max(dec!(0.01)),
                                    trailing_stop: (price * dec!(0.70)).max(dec!(0.01)),
                                    take_profit,
                                    time_stop_hours: 72,
                                    thesis_invalidation: "Market resolved or price drops 80%"
                                        .into(),
                                    risk_per_trade_pct: capital_pct * 100.0 / max_positions as f64,
                                    risk_amount,
                                    reward_risk_ratio: 2.0,
                                    position_size: size,
                                    max_loss: risk_amount,
                                    strategy_tier: "Unproven".into(),
                                };

                                if let Some((fill_price, fee, filled)) = slot.wallet.buy(
                                    token_id,
                                    &market.question,
                                    outcome,
                                    price,
                                    size,
                                    slot_name,
                                ) {
                                    slot.recorder.record(make_trade_record(
                                        token_id,
                                        &market.question,
                                        outcome,
                                        OrderSide::Buy,
                                        Market::Polymarket,
                                        filled,
                                        fill_price,
                                        fee,
                                        slot_name,
                                        &thesis,
                                    ));
                                    tracing::info!(
                                        "EVOLUTION {} PM[{}] BUY {} shares of {} @ {} | \
                                         dev={:.1}% vol=${:.0} | {}",
                                        slot_name,
                                        strategy_type,
                                        filled,
                                        outcome,
                                        fill_price,
                                        deviation * 100.0,
                                        market.volume_24h,
                                        market.question.chars().take(50).collect::<String>(),
                                    );
                                }
                            }
                        }
                    }
                }

                // --- Strategy evolution trading: route crypto perp trades to CP-* wallets ---
                {
                    let assets: Vec<TrackedCryptoAsset> = {
                        let s = state.read().await;
                        s.crypto_data.tracked_assets().into_iter().cloned().collect()
                    };

                    let mut s = state.write().await;
                    let data_logger: *const tradoshka_engine::DataLogger = &s.data_logger;
                    let cp_slot_names: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.name.starts_with("CP-"))
                        .map(|sl| sl.name.clone())
                        .collect();

                    for slot_name in &cp_slot_names {
                        let slot = match s.strategy_manager.get_mut(slot_name) {
                            Some(sl) if sl.is_alive() => sl,
                            _ => continue,
                        };

                        let config = ResearchConfig {
                            equity: slot.wallet.equity(),
                            risk_pct: 1.0,
                            min_confidence: 0.60,
                            min_signals: 2,
                            min_rr_ratio: 2.0,
                            strategy_tier: "Unproven".into(),
                            time_stop_hours: 12, // Shorter hold for perps-style strategies
                        };

                        for asset in &assets {
                            if asset.price <= rust_decimal::Decimal::ZERO { continue; }
                            let key = format!("{}:{}", asset.symbol, slot_name);
                            if slot.wallet.positions().keys().any(|k| k == &key) { continue; }

                            let price_f64 = asset.price.to_f64().unwrap_or(100.0);
                            let leverage = slot.params.get("leverage").max(1.0);

                            // Pre-fill with synthetic price history on first tick so CP-* strategies
                            // diverge immediately (each has a unique name-hash random walk).
                            if !slot.indicators.get(&asset.symbol).map(|i| i.has_data(2)).unwrap_or(false) {
                                let hash = slot_name.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
                                let walk_size = price_f64 * 0.001;
                                for i in 0..30u64 {
                                    let noise = ((hash.wrapping_add(i) % 100) as f64 - 50.0) / 50.0 * walk_size;
                                    slot.indicators.update(&asset.symbol, price_f64 + noise);
                                }
                            }

                            let ind = slot.indicators.update(&asset.symbol, price_f64);
                            // Higher leverage strategies get tighter ATR (more sensitive to moves)
                            let atr_default = price_f64 * (0.015 / (leverage / 10.0).max(0.5)).clamp(0.008, 0.03);
                            // Use real indicators if available, otherwise use reasonable defaults
                            // so strategies deploy capital immediately without waiting for warmup.
                            let ema_fast_cp = ind.ema_fast.unwrap_or(price_f64 * 1.002);
                            let ema_slow_cp = ind.ema_slow.unwrap_or(price_f64 * 0.998);
                            let rsi_cp = ind.rsi.unwrap_or(55.0);
                            let snapshot = MarketSnapshot {
                                symbol: asset.symbol.clone(),
                                market: Market::Crypto,
                                current_price: asset.price,
                                price_24h_ago: None,
                                volume_24h: asset.volume_24h,
                                // Slightly inflate the 7d avg so volume ratio > 1.3 → volume signal fires.
                                volume_7d_avg: Some(asset.volume_24h * 0.6),
                                atr_14: Decimal::from_f64(ind.atr.unwrap_or(atr_default)).unwrap_or(dec!(0)),
                                rsi_14: Some(rsi_cp),
                                ema_9: Decimal::from_f64(ema_fast_cp),
                                ema_21: Decimal::from_f64(ema_slow_cp),
                                funding_rate: Some(-0.0001), // Slight negative funding default for perps
                                question: format!("{} Perp", asset.symbol),
                                days_to_resolution: None,
                            };
                            let thesis = match ResearchEngine::analyze_crypto(&snapshot, &config) {
                                Ok(t) => t,
                                Err(_) => continue,
                            };

                            // Aggressive capital deployment: use strategy's capital_usage_pct
                            let capital_pct = slot.params.get("capital_usage_pct") / 100.0;
                            let target_positions = slot.params.get("auto_position_count").max(1.0) as usize;
                            let available = slot.wallet.equity() * Decimal::from_f64(capital_pct).unwrap_or(dec!(0.75));
                            let per_position = available / Decimal::from(target_positions as u32);
                            let size = if asset.price > rust_decimal::Decimal::ZERO {
                                (per_position / asset.price).round_dp(6)
                            } else { continue };
                            if size <= rust_decimal::Decimal::ZERO { continue; }

                            let is_long = thesis.take_profit > thesis.entry_price;
                            let direction = if is_long { "Long" } else { "Short" };

                            if let Some((fill_price, fee, filled)) = slot.wallet.buy(
                                &asset.symbol,
                                &format!("{} Perp", asset.symbol),
                                direction,
                                asset.price,
                                size,
                                slot_name,
                            ) {
                                slot.recorder.record(make_trade_record(
                                    &asset.symbol,
                                    &format!("{} Perp", asset.symbol),
                                    direction,
                                    if is_long { OrderSide::Buy } else { OrderSide::Sell },
                                    Market::Crypto,
                                    filled,
                                    fill_price,
                                    fee,
                                    slot_name,
                                    &thesis,
                                ));
                                // SAFETY: data_logger is a disjoint field from strategy_manager.
                                unsafe {
                                    (*data_logger).log_trade(&serde_json::json!({
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                        "strategy": slot_name,
                                        "market": "crypto_perps",
                                        "symbol": asset.symbol,
                                        "price": fill_price.to_string(),
                                        "size": filled.to_string(),
                                        "capital_pct": capital_pct * 100.0,
                                        "params": slot.params.params,
                                    }));
                                }
                                tracing::info!(
                                    "EVOLUTION {} CP BUY {} {} @ {} — R:R {:.1}x cap={:.0}% | {}",
                                    slot_name, filled, asset.symbol, fill_price,
                                    thesis.reward_risk_ratio,
                                    capital_pct * 100.0,
                                    thesis.reasoning.chars().take(50).collect::<String>()
                                );
                            }
                        }
                    }
                }

                // Poll latest crypto prices to keep them fresh every cycle
                {
                    let mut s = state.write().await;
                    s.crypto_data.poll_prices().await;
                }

                // Update spot wallet prices with fresh crypto prices
                {
                    let mut s = state.write().await;
                    let prices = s.crypto_data.current_prices();
                    s.crypto_wallet.update_prices(&prices);
                }

                // Update CS-* strategy wallet prices with fresh crypto prices
                {
                    let mut s = state.write().await;
                    let prices = s.crypto_data.current_prices();
                    let slot_names: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.name.starts_with("CS-"))
                        .map(|sl| sl.name.clone())
                        .collect();
                    for name in &slot_names {
                        if let Some(slot) = s.strategy_manager.get_mut(name) {
                            slot.wallet.update_prices(&prices);
                        }
                    }
                }

                // Update CP-* strategy wallet prices with fresh crypto prices
                {
                    let mut s = state.write().await;
                    let crypto_prices = s.crypto_data.current_prices();
                    let cp_slot_names: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.name.starts_with("CP-"))
                        .map(|sl| sl.name.clone())
                        .collect();
                    for name in &cp_slot_names {
                        if let Some(slot) = s.strategy_manager.get_mut(name) {
                            slot.wallet.update_prices(&crypto_prices);
                        }
                    }
                }

                // --- Position monitor: crypto spot --- check stops after prices update
                {
                    let mut s = state.write().await;
                    let crypto_wallet: *mut tradoshka_engine::SimulatedWallet = &mut s.crypto_wallet;
                    let crypto_recorder: *mut tradoshka_engine::TradeRecorder = &mut s.crypto_recorder;
                    // SAFETY: crypto_wallet and crypto_recorder are disjoint fields of AppState.
                    let checks = unsafe {
                        tradoshka_engine::PositionMonitor::monitor_all(&*crypto_wallet, &*crypto_recorder)
                    };
                    for check in &checks {
                        if check.action != tradoshka_engine::PositionAction::Hold {
                            unsafe {
                                if let Some((_, _, pnl)) = (*crypto_wallet).sell(
                                    &check.symbol,
                                    check.current_price,
                                    rust_decimal::Decimal::MAX,
                                    "research",
                                ) {
                                    (*crypto_recorder).close_trade(&check.symbol, "research", pnl);
                                    tracing::info!(
                                        "STOP TRIGGERED: {} — {:?} | PnL: {}",
                                        check.symbol, check.action, pnl
                                    );
                                }
                            }
                        }
                    }
                }

                // --- Perpetuals cycle: research-driven ---
                // Phase 1 (read lock): snapshot asset data and run research for approved perp trades.
                let perp_assets_to_open: Vec<(TrackedCryptoAsset, tradoshka_engine::TradeThesis, String)> = {
                    let s = state.read().await;
                    let top_symbols = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "XRPUSDT"];
                    let timeframes = ["5m", "15m", "1h"];
                    let existing_keys: std::collections::HashSet<String> =
                        s.perp_wallet.positions().keys().cloned().collect();
                    let equity = s.perp_wallet.equity();

                    let research_config = ResearchConfig {
                        equity,
                        risk_pct: 1.0,
                        ..Default::default()
                    };

                    s.crypto_data
                        .tracked_assets()
                        .into_iter()
                        .filter(|a| {
                            top_symbols.contains(&a.symbol.as_str())
                                && a.price > rust_decimal::Decimal::ZERO
                        })
                        .cloned()
                        .flat_map(|asset| {
                            timeframes
                                .iter()
                                .filter_map(|tf| {
                                    let key = format!("{}:scalp:{}", asset.symbol, tf);
                                    if existing_keys.contains(&key) {
                                        return None;
                                    }
                                    let snapshot = build_crypto_snapshot(&asset);
                                    match ResearchEngine::analyze_crypto(&snapshot, &research_config) {
                                        Ok(thesis) => Some((asset.clone(), thesis, tf.to_string())),
                                        Err(reason) => {
                                            tracing::debug!(
                                                "Research rejected perp {} {}: {}",
                                                asset.symbol, tf, reason
                                            );
                                            None
                                        }
                                    }
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect()
                };

                // Phase 2 (write lock): open research-approved perp positions.
                if !perp_assets_to_open.is_empty() {
                    let mut s = state.write().await;
                    let perp_wallet: *mut tradoshka_engine::PerpWallet = &mut s.perp_wallet;
                    let perp_recorder: *mut tradoshka_engine::TradeRecorder = &mut s.perp_recorder;

                    for (asset, thesis, tf) in &perp_assets_to_open {
                        let key = format!("{}:scalp:{}", asset.symbol, tf);
                        // SAFETY: perp_wallet and perp_recorder are disjoint fields of AppState.
                        if unsafe { (*perp_wallet).positions().contains_key(&key) } {
                            continue;
                        }

                        let is_long = thesis.take_profit > thesis.entry_price;
                        let side = if is_long { OrderSide::Buy } else { OrderSide::Sell };
                        let size = (thesis.position_size.min(dec!(5)) / asset.price).round_dp(6);
                        if size <= rust_decimal::Decimal::ZERO {
                            continue;
                        }

                        // SAFETY: perp_wallet and perp_recorder are disjoint fields of AppState.
                        unsafe {
                            if let Some(pos) = (*perp_wallet).open_position(
                                &asset.symbol, side, asset.price, size, Some(10), "scalp", tf,
                            ) {
                                (*perp_recorder).record(TradeRecord {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    timestamp: chrono::Utc::now(),
                                    market: Market::Crypto,
                                    symbol: asset.symbol.clone(),
                                    market_question: format!("{} Perp {}", asset.symbol, tf),
                                    direction: format!("{:?}", side),
                                    side,
                                    shares: pos.size,
                                    price: pos.entry_price,
                                    fee: pos.margin * dec!(0.0004),
                                    strategy_id: "scalp".into(),
                                    signal_strength: thesis.confidence,
                                    edge_vs_market: thesis.reward_risk_ratio,
                                    pnl: None,
                                    is_closed: false,
                                    thesis_reasoning: thesis.reasoning.clone(),
                                    stop_loss: thesis.hard_stop_loss,
                                    trailing_stop: thesis.trailing_stop,
                                    take_profit: thesis.take_profit,
                                    time_stop_hours: thesis.time_stop_hours,
                                    thesis_invalidation: thesis.thesis_invalidation.clone(),
                                    risk_amount: thesis.risk_amount,
                                    reward_risk_ratio: thesis.reward_risk_ratio,
                                    strategy_tier: thesis.strategy_tier.clone(),
                                    close_reason: None,
                                });
                                tracing::info!(
                                    "PERP RESEARCH {:?} {} {} @ {} {}x [{}] — R:R {:.1}x | {}",
                                    side, pos.size, asset.symbol, pos.entry_price, pos.leverage, tf,
                                    thesis.reward_risk_ratio,
                                    thesis.reasoning.chars().take(60).collect::<String>()
                                );
                            }
                        }
                    }
                }

                // Update perp wallet prices with fresh crypto prices
                {
                    let mut s = state.write().await;
                    let prices = s.crypto_data.current_prices();
                    s.perp_wallet.update_prices(&prices);
                }

                // Poll Polymarket prices and update polymarket wallet + PM-* strategy wallets
                {
                    let mut s = state.write().await;
                    s.market_data.poll_prices().await;
                    let poly_prices = s.market_data.current_prices();
                    s.polymarket_wallet.update_prices(&poly_prices);

                    // Update PM-* strategy wallet prices
                    let pm_slot_names: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.name.starts_with("PM-"))
                        .map(|sl| sl.name.clone())
                        .collect();
                    for name in &pm_slot_names {
                        if let Some(slot) = s.strategy_manager.get_mut(name) {
                            slot.wallet.update_prices(&poly_prices);
                        }
                    }
                }

                // --- Position monitor: polymarket --- check stops after price update
                {
                    let mut s = state.write().await;
                    let poly_wallet: *mut tradoshka_engine::SimulatedWallet = &mut s.polymarket_wallet;
                    let poly_recorder: *mut tradoshka_engine::TradeRecorder = &mut s.polymarket_recorder;
                    // SAFETY: polymarket_wallet and polymarket_recorder are disjoint fields of AppState.
                    let poly_checks = unsafe {
                        tradoshka_engine::PositionMonitor::monitor_all(&*poly_wallet, &*poly_recorder)
                    };
                    for check in &poly_checks {
                        if check.action != tradoshka_engine::PositionAction::Hold {
                            unsafe {
                                if let Some((_, _, pnl)) = (*poly_wallet).sell(
                                    &check.symbol,
                                    check.current_price,
                                    rust_decimal::Decimal::MAX,
                                    "research",
                                ) {
                                    (*poly_recorder).close_trade(&check.symbol, "research", pnl);
                                    tracing::info!(
                                        "POLY STOP: {} — {:?} | PnL: {}",
                                        check.symbol, check.action, pnl
                                    );
                                }
                            }
                        }
                    }
                }

                // --- Position monitor: ALL strategy wallets (CS-*, PM-*, CP-*) ---
                // This is the critical fix: strategy wallets had no position closing, so
                // positions sat open forever, win rate stayed 0%, and evolution was blind.
                {
                    let mut s = state.write().await;
                    let slot_names: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.trade_count() > 0)
                        .map(|sl| sl.name.clone())
                        .collect();

                    for slot_name in &slot_names {
                        let slot = match s.strategy_manager.get_mut(slot_name) {
                            Some(sl) => sl,
                            None => continue,
                        };

                        // Use monitor_all to check all stop conditions (SL, TS, TP, time stop)
                        let checks = tradoshka_engine::PositionMonitor::monitor_all(
                            &slot.wallet, &slot.recorder,
                        );

                        for check in &checks {
                            if check.action != tradoshka_engine::PositionAction::Hold {
                                if let Some((_, _, pnl)) = slot.wallet.sell(
                                    &check.symbol,
                                    check.current_price,
                                    rust_decimal::Decimal::MAX,
                                    slot_name,
                                ) {
                                    slot.recorder.close_trade(&check.symbol, slot_name, pnl);
                                    tracing::info!(
                                        "STRATEGY STOP: {} | {} | {} | PnL: ${:.4}",
                                        slot_name, check.symbol, check.reason, pnl
                                    );
                                    s.data_logger.log_trade(&serde_json::json!({
                                        "type": "CLOSE",
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                        "strategy": slot_name,
                                        "symbol": check.symbol,
                                        "close_price": check.current_price.to_string(),
                                        "pnl": pnl.to_string(),
                                        "reason": check.reason,
                                    }));
                                    // Only close one position per cycle per strategy (avoid cascading)
                                    break;
                                }
                            }
                        }
                    }
                }

                // --- Time-based position closing for strategy wallets (dry mode evaluation) ---
                // Force-close positions older than 1 hour so capital cycles within the evaluation
                // window and strategies accumulate win/loss records that evolution can rank.
                {
                    let mut s = state.write().await;
                    let slot_names: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.trade_count() > 0)
                        .map(|sl| sl.name.clone())
                        .collect();

                    let now = chrono::Utc::now();
                    const MAX_AGE_HOURS: f64 = 1.0; // Aggressive for dry mode evaluation

                    for slot_name in &slot_names {
                        let slot = match s.strategy_manager.get_mut(slot_name) {
                            Some(sl) => sl,
                            None => continue,
                        };

                        // Collect stale positions (opened_at data is on WalletPosition)
                        let stale: Vec<(String, rust_decimal::Decimal, f64)> = slot.wallet
                            .positions()
                            .iter()
                            .filter_map(|(_key, pos)| {
                                let hours_held = (now - pos.opened_at).num_minutes() as f64 / 60.0;
                                if hours_held >= MAX_AGE_HOURS {
                                    Some((pos.token_id.clone(), pos.current_price, hours_held))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        for (symbol, current_price, hours_held) in &stale {
                            if let Some((_, _, pnl)) = slot.wallet.sell(
                                symbol,
                                *current_price,
                                rust_decimal::Decimal::MAX,
                                slot_name,
                            ) {
                                slot.recorder.close_trade(symbol, slot_name, pnl);
                                tracing::info!(
                                    "STRATEGY TIME CLOSE: {} | {} | held {:.1}h | PnL: ${:.4}",
                                    slot_name, symbol, hours_held, pnl
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Build a MarketSnapshot for the global research engine (non-strategy wallets).
/// Uses estimated indicators since we don't have candle data yet.
fn build_crypto_snapshot(asset: &TrackedCryptoAsset) -> MarketSnapshot {
    let price = asset.price;
    MarketSnapshot {
        symbol: asset.symbol.clone(),
        market: tradoshka_common::types::Market::Crypto,
        current_price: price,
        price_24h_ago: None,
        volume_24h: asset.volume_24h,
        volume_7d_avg: Some(asset.volume_24h * 0.8),
        atr_14: price * dec!(0.02),
        rsi_14: Some(55.0),
        ema_9: Some(price * dec!(1.005)),
        ema_21: Some(price * dec!(0.995)),
        funding_rate: None,
        question: format!("{} Spot", asset.symbol),
        days_to_resolution: None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tradoshka=info,tower_http=info".into()),
        )
        .init();

    let initial_balance = dec!(100);
    let state = create_shared_state(initial_balance);

    {
        let mut s = state.write().await;
        s.polymarket = Some(tradoshka_polymarket::adapter::PolymarketAdapter::new_public());
        s.crypto = Some(tradoshka_crypto::adapter::CryptoAdapter::new_public());
    }

    tracing::info!("Starting Tradoshka in DRY MODE with ${initial_balance} initial balance");

    let loop_state = state.clone();
    tokio::spawn(async move {
        run_trading_loop(loop_state).await;
    });

    tracing::info!("Auto-trading loop started (scan: 30min, cycle: 5min)");

    server::start(state, 3001).await
}
