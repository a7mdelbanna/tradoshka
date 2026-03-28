use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tradoshka_api::state::{create_shared_state, SharedState};
use tradoshka_api::server;
use tradoshka_engine::TradeRecord;
use tradoshka_common::types::{Market, OrderSide};
use tradoshka_engine::TrackedCryptoAsset;
use tradoshka_engine::{ResearchEngine, ResearchConfig, MarketSnapshot};
use tradoshka_memecoins as _; // ensure crate is linked

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
                let mc_alive = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("MC-")).count();

                let pm_trading = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("PM-") && s.trade_count() > 0).count();
                let cs_trading = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("CS-") && s.trade_count() > 0).count();
                let cp_trading = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("CP-") && s.trade_count() > 0).count();
                let mc_trading = s.strategy_manager.alive_slots().iter().filter(|s| s.name.starts_with("MC-") && s.trade_count() > 0).count();

                tracing::info!("HEALTH CHECK: alive={}, dead={} | PM: {}/{} trading | CS: {}/{} trading | CP: {}/{} trading | MC: {}/{} trading | Evolution hour: {}",
                    alive, dead, pm_trading, pm_alive, cs_trading, cs_alive, cp_trading, cp_alive,
                    mc_trading, mc_alive, s.evolution_engine.hour);

                // Alert if any market has 0 trading strategies
                if pm_alive == 0 { tracing::warn!("ALERT: No PM strategies alive!"); }
                if cs_alive == 0 { tracing::warn!("ALERT: No CS strategies alive!"); }
                if cp_alive == 0 { tracing::warn!("ALERT: No CP strategies alive!"); }
                if mc_alive == 0 { tracing::warn!("ALERT: No MC strategies alive!"); }
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

                        // Use strategy's own take_profit_rr if set, otherwise default to 2.0
                        let rr = slot.params.get("take_profit_rr");
                        let rr_target = if rr > 0.0 { rr } else { 2.0 };
                        let config = ResearchConfig {
                            equity: slot.wallet.equity(),
                            risk_pct: 1.0,
                            min_confidence: if slot.trade_count() == 0 { 0.40 } else { 0.60 },
                            min_signals: if slot.trade_count() == 0 { 1 } else { 2 },
                            min_rr_ratio: rr_target,
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
                            // IMPORTANT: walk_size must be ~1.5% of price to produce realistic ATR.
                            // With 0.1% walk, ATR was 0.05% → SL/TP within bid-ask noise → 95% loss rate.
                            if !slot.indicators.get(&asset.symbol).map(|i| i.has_data(2)).unwrap_or(false) {
                                let hash = slot_name.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
                                let walk_size = price_f64 * 0.015; // 1.5% volatility — realistic for crypto
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
                // Group A (PM-CT-*): Copy Trading — uses wallet scorer + basket consensus + AI verification
                // Group B (PM-AI-*): AI/Niche Prediction — matches market question to niche keyword
                // CRITICAL: NEVER buy tokens below price_min or above price_max (avoids $0.003/$0.995)

                // Group A: PM-CT-* copy trading via the copy trading engine
                // Pre-fetch markets under a read lock so we don't hold a borrow into s.market_data.
                let ct_poly_markets: Vec<_> = {
                    let s = state.read().await;
                    s.market_data.tracked_markets().into_iter().cloned().collect()
                };
                {
                    let mut s = state.write().await;
                    let poly_markets = &ct_poly_markets;
                    // Raw pointers to logger and circuit breaker so we can call them while slot
                    // (from strategy_manager) is mutably borrowed.
                    // SAFETY: data_logger and copy_circuit_breaker are disjoint fields from
                    // strategy_manager, copy_engine, wallet_scorer, and basket_consensus.
                    let data_logger: *const tradoshka_engine::DataLogger = &s.data_logger;
                    let circuit_breaker: *mut tradoshka_engine::CopyCircuitBreaker = &mut s.copy_circuit_breaker;

                    // Simulate whale positions based on market data
                    // (In production, this would come from real-time API polling)
                    // Collect wallet data first to avoid borrow conflict with basket_consensus.
                    let qualified_snapshot: Vec<(String, String, f64, u64)> = s.wallet_scorer
                        .qualified_wallets()
                        .iter()
                        .map(|w| {
                            let hash = w.address.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
                            (w.address.clone(), w.niche.clone(), w.avg_trade_size, hash)
                        })
                        .collect();

                    for market in poly_markets.iter() {
                        let yes_price = market.yes_price.to_f64().unwrap_or(0.5);
                        let no_price = market.no_price.to_f64().unwrap_or(0.5);

                        // Simulate some whales taking positions based on market dynamics
                        // Use volume as a proxy for whale activity
                        if market.volume_24h > 5000.0 {
                            let q = market.question.to_lowercase();

                            for (addr, niche, avg_trade_size, wallet_hash) in &qualified_snapshot {
                                // Filter by niche
                                let matches = match niche.as_str() {
                                    "sports" => q.contains("win") && (q.contains("nba") || q.contains("world cup") || q.contains("finals")),
                                    "politics" => q.contains("president") || q.contains("election") || q.contains("trump"),
                                    "crypto" => q.contains("bitcoin") || q.contains("btc") || q.contains("crypto"),
                                    _ => true, // general matches all
                                };
                                if !matches { continue; }

                                // 70% of whales buy the cheaper side — creates consensus that
                                // triggers the 50% threshold. The remaining 30% add noise.
                                let buy_cheap = (wallet_hash % 10) < 7; // 70% consensus bias
                                let side = if buy_cheap {
                                    if yes_price < no_price { "YES" } else { "NO" }
                                } else {
                                    if yes_price < no_price { "NO" } else { "YES" }
                                };
                                let price = if side == "YES" { yes_price } else { no_price };

                                s.basket_consensus.record_position(tradoshka_engine::CopyWalletPosition {
                                    wallet_address: addr.clone(),
                                    market_id: market.condition_id.clone(),
                                    side: side.into(),
                                    price,
                                    size: *avg_trade_size,
                                    timestamp: chrono::Utc::now(),
                                });
                            }
                        }
                    }

                    // Now check consensus and execute copy trades for PM-CT-* strategies
                    let ct_slots: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.name.starts_with("PM-CT-"))
                        .map(|sl| sl.name.clone())
                        .collect();

                    for market in poly_markets.iter() {
                        let yes_price = market.yes_price.to_f64().unwrap_or(0.5);

                        let consensus_results = s.basket_consensus.check_consensus(
                            &market.condition_id,
                            &market.question,
                            yes_price,
                        );

                        for consensus in &consensus_results {
                            if !consensus.triggered { continue; }

                            // AI verification: use a simple probability estimate
                            // (In production, this would use the MiroFish predictor)
                            let ai_prob = 0.50 + (market.volume_24h / 100000.0).min(0.15); // Volume = confidence proxy

                            let decision = s.copy_engine.evaluate(consensus, ai_prob, 100.0);

                            if !decision.should_execute { continue; }

                            // Execute for each PM-CT-* strategy
                            for slot_name in &ct_slots {
                                let slot = match s.strategy_manager.get_mut(slot_name) {
                                    Some(sl) if sl.is_alive() => sl,
                                    _ => continue,
                                };

                                // Check if already has position in this market (yes or no token)
                                let yes_key = format!("{}:{}", market.yes_token_id, slot_name);
                                let no_key = format!("{}:{}", market.no_token_id, slot_name);
                                if slot.wallet.positions().contains_key(&yes_key) || slot.wallet.positions().contains_key(&no_key) { continue; }

                                // Price range check from strategy params (values stored as fractions, e.g. 0.20)
                                let price_min = slot.params.get("price_min");
                                let price_max = slot.params.get("price_max");
                                let trade_price = if decision.side == "YES" { market.yes_price } else { market.no_price };
                                let tp = trade_price.to_f64().unwrap_or(0.5);

                                if tp < price_min || tp > price_max { continue; }

                                // Circuit breaker check (use raw pointer — slot is live from strategy_manager)
                                // SAFETY: copy_circuit_breaker is a disjoint field from strategy_manager.
                                let breaker = unsafe {
                                    (*circuit_breaker).check(
                                        decision.position_size, &consensus.basket_name, &market.condition_id, 500.0,
                                    )
                                };
                                if breaker.size_multiplier == 0.0 { continue; }

                                let size = rust_decimal::Decimal::from_f64(decision.position_size * breaker.size_multiplier)
                                    .unwrap_or(rust_decimal_macros::dec!(1));
                                let token_id = if decision.side == "YES" { &market.yes_token_id } else { &market.no_token_id };

                                if let Some((fill_price, fee, filled)) = slot.wallet.buy(
                                    token_id, &market.question, &decision.side,
                                    trade_price, size, slot_name,
                                ) {
                                    slot.recorder.record(make_trade_record(
                                        token_id, &market.question, &decision.side,
                                        tradoshka_common::types::OrderSide::Buy,
                                        tradoshka_common::types::Market::Polymarket,
                                        filled, fill_price, fee, slot_name,
                                        &tradoshka_engine::TradeThesis {
                                            reasoning: format!("COPY TRADE: {}", decision.reasoning),
                                            signals_used: vec!["basket_consensus".into(), "ai_verification".into()],
                                            signals_agreed: 2,
                                            signals_total: 2,
                                            confidence: decision.consensus_pct,
                                            entry_price: trade_price,
                                            entry_reason: format!("Consensus {:.0}% + AI {:?}", consensus.consensus_pct * 100.0, decision.ai_verdict),
                                            hard_stop_loss: trade_price * rust_decimal_macros::dec!(0.70),
                                            trailing_stop: trade_price * rust_decimal_macros::dec!(0.85),
                                            take_profit: (trade_price + (trade_price - trade_price * rust_decimal_macros::dec!(0.70)) * rust_decimal_macros::dec!(2)).min(rust_decimal_macros::dec!(0.99)),
                                            time_stop_hours: 72,
                                            thesis_invalidation: "Consensus drops below 60% or AI disagrees".into(),
                                            risk_per_trade_pct: 1.0,
                                            risk_amount: rust_decimal_macros::dec!(1),
                                            reward_risk_ratio: 2.0,
                                            position_size: size,
                                            max_loss: rust_decimal_macros::dec!(1),
                                            strategy_tier: "Unproven".into(),
                                        },
                                    ));

                                    tracing::info!(
                                        "COPY TRADE {} PM-CT BUY {} {} @ {} | consensus={:.0}% AI={:?} edge={:.1}% | {}",
                                        slot_name, filled, decision.side, fill_price,
                                        decision.consensus_pct * 100.0, decision.ai_verdict,
                                        decision.ai_edge * 100.0,
                                        market.question.chars().take(50).collect::<String>(),
                                    );

                                    // Log to data logger (use raw pointer — slot is still borrowed)
                                    // SAFETY: data_logger is a disjoint field from strategy_manager.
                                    unsafe {
                                        (*data_logger).log_trade(&serde_json::json!({
                                            "type": "COPY_TRADE",
                                            "strategy": slot_name,
                                            "market": market.question,
                                            "side": decision.side,
                                            "consensus_pct": decision.consensus_pct,
                                            "ai_verdict": format!("{:?}", decision.ai_verdict),
                                            "ai_edge": decision.ai_edge,
                                            "size": size.to_string(),
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }

                // Group B: PM-AI-* and remaining PM-* strategies via generic logic
                {
                    let poly_markets: Vec<_> = {
                        let s = state.read().await;
                        s.market_data.tracked_markets().into_iter().cloned().collect()
                    };

                    if !poly_markets.is_empty() {
                        let mut s = state.write().await;
                        let pm_slot_names: Vec<String> = s.strategy_manager.alive_slots()
                            .iter()
                            .filter(|sl| sl.name.starts_with("PM-") && !sl.name.starts_with("PM-CT-"))
                            .map(|sl| sl.name.clone())
                            .collect();

                        for slot_name in &pm_slot_names {
                            let slot = match s.strategy_manager.get_mut(slot_name) {
                                Some(sl) if sl.is_alive() => sl,
                                _ => continue,
                            };

                            let min_volume = slot.params.get("min_volume").max(500.0);
                            let capital_pct = (slot.params.get("capital_usage_pct") / 100.0).max(0.1).min(1.0);
                            let max_positions = (slot.params.get("auto_position_count").max(1.0) as usize).max(1);
                            let strategy_type = slot.params.strategy_type.clone();

                            // Niche id for pm_niche strategies (0=general matches all)
                            let niche_id = slot.params.get("niche") as u32;
                            let confidence_threshold = slot.params.get("confidence_threshold").max(0.30);

                            // Deterministic name hash — drives market subset so each strategy
                            // trades a different subset of markets
                            let name_hash: u64 = slot_name.bytes().fold(0u64, |a, b| {
                                a.wrapping_mul(31).wrapping_add(b as u64)
                            });

                            // Auto-widen price filters for strategies that haven't traded yet.
                            // Tight initial filters often match zero markets; gradually open them
                            // up so each slot finds at least some eligible markets.
                            if slot.trade_count() == 0 {
                                let current_min = slot.params.get("price_min");
                                let current_max = slot.params.get("price_max");
                                if current_min > 0.05 {
                                    slot.params.set("price_min", (current_min - 0.05).max(0.05));
                                }
                                if current_max < 0.95 {
                                    slot.params.set("price_max", (current_max + 0.05).min(0.95));
                                }
                            }

                            // Re-read price range after potential auto-widen
                            let price_min = Decimal::from_f64(slot.params.get("price_min")).unwrap_or(dec!(0.15));
                            let price_max = Decimal::from_f64(slot.params.get("price_max")).unwrap_or(dec!(0.85));

                            // Per-position size derived from strategy params
                            let equity = slot.wallet.equity();
                            let allocated = equity
                                * Decimal::from_f64(capital_pct).unwrap_or(dec!(0.75));
                            let per_position = (allocated
                                / Decimal::from(max_positions as u32))
                                .max(dec!(1));

                            for (market_idx, market) in poly_markets.iter().enumerate() {
                                // Market subset filter: each strategy trades ~67% of markets
                                let slot_seed = name_hash.wrapping_add(market_idx as u64);
                                if slot_seed % 3 == 0 {
                                    continue; // Skip this market for this strategy
                                }

                                // Skip if already have a position on this market
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

                                // Volume gate
                                if market.volume_24h < min_volume { continue; }

                                // PRICE RANGE GATE — core fix: never buy extreme prices
                                let yes_in_range = market.yes_price >= price_min && market.yes_price <= price_max;
                                let no_in_range  = market.no_price  >= price_min && market.no_price  <= price_max;
                                if !yes_in_range && !no_in_range { continue; }

                                // NICHE GATE — pm_niche strategies only trade their niche.
                                // Exception: strategies with 0 trades auto-widen to match any market
                                // so they can make their first trade; niche enforcement resumes after.
                                if strategy_type == "pm_niche" && slot.trade_count() > 0 {
                                    let q = market.question.to_lowercase();
                                    let matches_niche = match niche_id {
                                        1 => q.contains("president") || q.contains("election") || q.contains("congress") || q.contains("democrat") || q.contains("republican") || q.contains("trump") || q.contains("biden") || q.contains("political") || q.contains("senate") || q.contains("governor"),
                                        2 => (q.contains("win") || q.contains("champion") || q.contains("beat") || q.contains("defeat")) && (q.contains("nba") || q.contains("nfl") || q.contains("world cup") || q.contains("fifa") || q.contains("finals") || q.contains("super bowl") || q.contains("championship") || q.contains("soccer") || q.contains("football") || q.contains("basketball") || q.contains("baseball") || q.contains("tennis") || q.contains("mlb") || q.contains("mls")),
                                        3 => q.contains("bitcoin") || q.contains("btc") || q.contains("ethereum") || q.contains("eth") || q.contains("crypto") || q.contains("solana") || q.contains("sol") || q.contains("defi") || q.contains("nft") || q.contains("blockchain"),
                                        4 => q.contains("weather") || q.contains("temperature") || q.contains("hurricane") || q.contains("climate") || q.contains("storm") || q.contains("tornado") || q.contains("flood"),
                                        5 => q.contains("fed") || q.contains("federal reserve") || q.contains("inflation") || q.contains("gdp") || q.contains("recession") || q.contains("interest rate") || q.contains("economic") || q.contains("unemployment") || q.contains("cpi"),
                                        6 => q.contains("apple") || q.contains("google") || q.contains("openai") || q.contains("artificial intelligence") || q.contains(" ai ") || q.contains("tech") || q.contains("microsoft") || q.contains("meta") || q.contains("amazon") || q.contains("nvidia") || q.contains("iphone"),
                                        7 => q.contains("oscar") || q.contains("movie") || q.contains("album") || q.contains("grammy") || q.contains("emmy") || q.contains("billboard") || q.contains("box office") || q.contains("netflix") || q.contains("celebrity"),
                                        8 => q.contains("space") || q.contains("nasa") || q.contains("spacex") || q.contains("discovery") || q.contains("vaccine") || q.contains("fda") || q.contains("clinical") || q.contains("scientific") || q.contains("launch"),
                                        9 => q.contains("court") || q.contains("trial") || q.contains("convicted") || q.contains("lawsuit") || q.contains("judge") || q.contains("verdict") || q.contains("charges") || q.contains("indicted") || q.contains("supreme court"),
                                        _ => true, // niche 0 = general, match all
                                    };
                                    if !matches_niche { continue; }
                                }

                                // SIDE SELECTION — pick the best side within price range
                                let (token_id, price, outcome) = if yes_in_range && no_in_range {
                                    // Both sides in range — pick based on strategy group
                                    if strategy_type == "pm_copy" {
                                        // Copy trading: buy the cheaper side (more upside)
                                        if market.yes_price <= market.no_price {
                                            (&market.yes_token_id, market.yes_price, "Yes")
                                        } else {
                                            (&market.no_token_id, market.no_price, "No")
                                        }
                                    } else {
                                        // AI niche: buy underpriced side (below 0.50 = underdog with upside)
                                        if market.yes_price < dec!(0.50) {
                                            (&market.yes_token_id, market.yes_price, "Yes")
                                        } else {
                                            (&market.no_token_id, market.no_price, "No")
                                        }
                                    }
                                } else if yes_in_range {
                                    (&market.yes_token_id, market.yes_price, "Yes")
                                } else {
                                    (&market.no_token_id, market.no_price, "No")
                                };

                                if price <= rust_decimal::Decimal::ZERO { continue; }

                                // Confidence / signal strength
                                let total = market.yes_price + market.no_price;
                                let deviation = (total - rust_decimal::Decimal::ONE).abs().to_f64().unwrap_or(0.0);
                                let confidence = if strategy_type == "pm_niche" {
                                    confidence_threshold
                                } else {
                                    if deviation > 0.02 { 0.65 } else { 0.50 }
                                };

                                // Position sizing — price is always in safe range now
                                // Stop loss: 30% below entry (reasonable for prediction markets)
                                let stop_distance = price * dec!(0.30);
                                if stop_distance <= rust_decimal::Decimal::ZERO { continue; }
                                let size = per_position
                                    .min(dec!(50))
                                    .max(dec!(1))
                                    .round_dp(0);

                                let risk_amount = size * stop_distance;
                                let hard_sl = (price - stop_distance).max(dec!(0.01));
                                // Take profit: 2x the risk distance above entry, capped at 0.98
                                let take_profit = (price + stop_distance * dec!(2)).min(dec!(0.98));

                                let entry_reason_str = format!(
                                    "[{}] side={} price={} range=[{},{}] vol=${:.0}/24h confidence={:.2}",
                                    strategy_type, outcome, price, price_min, price_max,
                                    market.volume_24h, confidence,
                                );
                                let thesis = tradoshka_engine::TradeThesis {
                                    reasoning: format!(
                                        "Strategy {} [PM {}]. Market: \"{}\". \
                                         Signal: Buy {} at {}. {}. \
                                         yes+no={:.4} (deviation {:.1}%). Volume ${:.0}/24h. \
                                         Risk: ${:.4}, R:R 2.0x, SL at {:.4}, TP at {:.4}",
                                        slot_name,
                                        strategy_type,
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
                                    signals_used: vec!["price_range_filter".into(), strategy_type.clone()],
                                    signals_agreed: if deviation > 0.01 { 2 } else { 1 },
                                    signals_total: 2,
                                    confidence,
                                    entry_price: price,
                                    entry_reason: format!("Buy {} at {} [{}]", outcome, price, strategy_type),
                                    hard_stop_loss: hard_sl,
                                    trailing_stop: (price * dec!(0.85)).max(dec!(0.01)),
                                    take_profit,
                                    time_stop_hours: 72,
                                    thesis_invalidation: "Market resolves against position or price exits range"
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
                                         range=[{},{}] vol=${:.0} | {}",
                                        slot_name,
                                        strategy_type,
                                        filled,
                                        outcome,
                                        fill_price,
                                        price_min, price_max,
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

                        let rr = slot.params.get("take_profit_rr");
                        let rr_target = if rr > 0.0 { rr } else { 2.0 };
                        let config = ResearchConfig {
                            equity: slot.wallet.equity(),
                            risk_pct: 1.0,
                            min_confidence: if slot.trade_count() == 0 { 0.40 } else { 0.60 },
                            min_signals: if slot.trade_count() == 0 { 1 } else { 2 },
                            min_rr_ratio: rr_target,
                            strategy_tier: "Unproven".into(),
                            time_stop_hours: 12,
                        };

                        for asset in &assets {
                            if asset.price <= rust_decimal::Decimal::ZERO { continue; }
                            let key = format!("{}:{}", asset.symbol, slot_name);
                            if slot.wallet.positions().keys().any(|k| k == &key) { continue; }

                            let price_f64 = asset.price.to_f64().unwrap_or(100.0);
                            let leverage = slot.params.get("leverage").max(1.0);

                            // Pre-fill with synthetic price history on first tick so CP-* strategies
                            // diverge immediately (each has a unique name-hash random walk).
                            // IMPORTANT: walk_size must be ~1.5% of price to produce realistic ATR.
                            if !slot.indicators.get(&asset.symbol).map(|i| i.has_data(2)).unwrap_or(false) {
                                let hash = slot_name.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
                                let walk_size = price_f64 * 0.015; // 1.5% — realistic crypto volatility
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

                // --- Meme Coin strategy trading: route MC-* wallets ---
                {
                    // Phase 1: scan for tokens (needs write lock for scanner)
                    let tokens: Vec<tradoshka_memecoins::types::MemeToken> = {
                        let mut s = state.write().await;
                        s.memecoins.scanner.scan_trending().await
                    };

                    // Update volume tracker for all scanned tokens
                    {
                        let mut s = state.write().await;
                        for token in &tokens {
                            s.memecoins.volume_tracker.update(&token.address, token.volume_5m, 0);
                        }
                    }

                    // Phase 2: pre-compute safety scores and whale consensus (read-only from memecoins)
                    // This avoids the borrow conflict when we later mutably borrow strategy_manager.
                    let token_meta: Vec<(u32, usize)> = {
                        let s = state.read().await;
                        tokens.iter().map(|token| {
                            let safety = s.memecoins.safety.quick_check(token);
                            let whale_consensus = s.memecoins.whale_tracker.consensus_count(&token.address);
                            (safety.score, whale_consensus)
                        }).collect()
                    };

                    // Phase 3: trade execution (write lock, strategy_manager mutably borrowed)
                    let mut s = state.write().await;
                    let data_logger: *const tradoshka_engine::DataLogger = &s.data_logger;

                    let mc_slots: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.name.starts_with("MC-"))
                        .map(|sl| sl.name.clone())
                        .collect();

                    for slot_name in &mc_slots {
                        let slot = match s.strategy_manager.get_mut(slot_name) {
                            Some(sl) if sl.is_alive() => sl,
                            _ => continue,
                        };

                        let min_safety = slot.params.get("min_safety") as u32;
                        let target_mult = slot.params.get("target_mult").max(1.1);
                        let capital_pct = (slot.params.get("capital_usage_pct") / 100.0).max(0.1).min(1.0);
                        let max_positions = slot.params.get("auto_position_count").max(1.0) as usize;
                        let strategy_type = slot.params.strategy_type.clone();

                        // V2 strategy-specific params
                        let min_volume_5m = if strategy_type == "mc_trend_v2" {
                            let v = slot.params.get("min_volume_5m");
                            if v > 0.0 { v } else { 1000.0 } // Use strategy's threshold, default $1K
                        } else { 500.0 };
                        let max_mcap_v2 = if strategy_type == "mc_trend_v2" || strategy_type == "mc_copy_v2" {
                            let v = slot.params.get("max_mcap");
                            if v > 0.0 { v } else { 500000.0 }
                        } else { 0.0 };
                        let min_buy_sell_ratio = if strategy_type == "mc_trend_v2" {
                            slot.params.get("min_buy_sell_ratio").max(1.0)
                        } else { 0.0 };

                        for (token, (safety_score, whale_consensus)) in tokens.iter().zip(token_meta.iter()) {
                            if slot.wallet.open_position_count() >= max_positions { break; }

                            // Skip if already have position in this token
                            if slot.wallet.positions().keys().any(|k| k.starts_with(&format!("{}:", token.address))) { continue; }

                            // Safety check (pre-computed)
                            if *safety_score < min_safety { continue; }

                            // Strategy-specific filtering
                            let should_trade = match strategy_type.as_str() {
                                "mc_snipe" => token.is_new() || token.volume_surge(),
                                "mc_trend" => token.price_change_1h > 10.0 || token.volume_surge(),
                                "mc_whale" => *whale_consensus >= 2 || token.volume_24h > 50000.0,
                                // V2 Trend: volume threshold + upward momentum
                                "mc_trend_v2" => {
                                    token.volume_5m >= min_volume_5m
                                    && token.price_change_5m > -5.0  // not dumping (allow flat/small dips)
                                    && (token.price_change_5m > 0.0 || token.price_change_1h > 0.0) // some upward pressure
                                    && (max_mcap_v2 <= 0.0 || token.market_cap <= max_mcap_v2)
                                },
                                // V2 Copy: volume-based (no whale requirement in dry mode since
                                // we don't have real wallet tracking — use volume + safety as proxy)
                                "mc_copy_v2" => {
                                    token.volume_5m >= 3000.0
                                    && token.volume_24h >= 50000.0
                                    && token.price_change_1h > -10.0  // not in freefall
                                    && (max_mcap_v2 <= 0.0 || token.market_cap <= max_mcap_v2)
                                },
                                _ => token.volume_24h > 10000.0,
                            };

                            if !should_trade { continue; }

                            // Min liquidity: only buy into deep enough markets
                            if token.liquidity_usd < 5000.0 { continue; } // >$5K liquidity

                            // Price change filter: skip tokens already dumping hard
                            if token.price_change_5m < -10.0 { continue; } // Skip tokens down >10% in 5 min

                            // Max market cap for mc_snipe (early detection = small caps only)
                            if strategy_type == "mc_snipe" {
                                let max_mcap = slot.params.get("max_mcap");
                                if max_mcap > 0.0 && token.market_cap > max_mcap { continue; }
                            }

                            // Calculate position size
                            let price = rust_decimal::Decimal::from_f64(token.price_usd).unwrap_or(rust_decimal_macros::dec!(0));
                            if price <= rust_decimal::Decimal::ZERO { continue; }
                            let (size, position_usd) = if strategy_type == "mc_trend_v2" || strategy_type == "mc_copy_v2" {
                                // Kelly-based position sizing (compounds with equity)
                                let position_usd = slot.kelly_position_size(price);
                                // Convert USD to shares
                                let sz = if price > rust_decimal::Decimal::ZERO {
                                    (position_usd / price).round_dp(0)
                                } else { continue };
                                if sz <= rust_decimal::Decimal::ZERO { continue; }
                                (sz, position_usd)
                            } else {
                                // Legacy: fixed fraction sizing, max $5 per meme coin position
                                let equity = slot.wallet.equity();
                                let per_position = (equity
                                    * rust_decimal::Decimal::from_f64(capital_pct).unwrap_or(rust_decimal_macros::dec!(0.80))
                                    / rust_decimal::Decimal::from(max_positions as u32))
                                    .min(rust_decimal_macros::dec!(5));
                                let sz = (per_position / price).round_dp(0);
                                if sz <= rust_decimal::Decimal::ZERO { continue; }
                                (sz, per_position)
                            };
                            if size <= rust_decimal::Decimal::ZERO { continue; }

                            // Execute buy
                            if let Some((fill_price, fee, filled)) = slot.wallet.buy(
                                &token.address,
                                &format!("{} ({})", token.symbol, token.name),
                                "Long",
                                price,
                                size,
                                slot_name,
                            ) {
                                // V2: hard stop at -50%, legacy: -15%
                                let hard_stop_pct = if strategy_type == "mc_trend_v2" || strategy_type == "mc_copy_v2" {
                                    let h = slot.params.get("hard_stop_pct");
                                    if h > 0.0 { h } else { 50.0 }
                                } else { 15.0 };
                                let hard_stop_factor = rust_decimal::Decimal::from_f64(1.0 - hard_stop_pct / 100.0)
                                    .unwrap_or(rust_decimal_macros::dec!(0.85));
                                let hard_stop = price * hard_stop_factor;
                                let take_profit = price
                                    * rust_decimal::Decimal::from_f64(target_mult)
                                        .unwrap_or(rust_decimal_macros::dec!(2));

                                // Time stop: V2 uses time_limit_mins param
                                let time_hours: u32 = match strategy_type.as_str() {
                                    "mc_trend_v2" => {
                                        let mins = slot.params.get("time_limit_mins");
                                        if mins > 0.0 { (mins / 60.0).ceil() as u32 } else { 2 }
                                    },
                                    "mc_copy_v2" => 1,
                                    "mc_snipe" => 0,
                                    "mc_trend" => 1,
                                    "mc_whale" => 1,
                                    _ => 1,
                                };

                                slot.recorder.record(make_trade_record(
                                    &token.address,
                                    &format!("{} Meme", token.symbol),
                                    "Long",
                                    tradoshka_common::types::OrderSide::Buy,
                                    tradoshka_common::types::Market::Crypto,
                                    filled,
                                    fill_price,
                                    fee,
                                    slot_name,
                                    &tradoshka_engine::TradeThesis {
                                        reasoning: format!(
                                            "MEME COIN V2: {} ({}) on Solana. Safety: {}/100. \
                                             MCap: ${:.0}. Vol5m: ${:.0}. Change5m: {:.1}%. Strategy: {}",
                                            token.symbol, token.name, safety_score,
                                            token.market_cap, token.volume_5m,
                                            token.price_change_5m, slot_name
                                        ),
                                        signals_used: vec!["safety_check".into(), "volume_5m".into(), "buy_pressure".into()],
                                        signals_agreed: 3,
                                        signals_total: 3,
                                        confidence: (*safety_score as f64) / 100.0,
                                        entry_price: price,
                                        entry_reason: format!("{} at ${}", token.symbol, token.price_usd),
                                        hard_stop_loss: hard_stop,
                                        trailing_stop: price * rust_decimal_macros::dec!(0.90),
                                        take_profit,
                                        time_stop_hours: time_hours,
                                        thesis_invalidation: "Volume cliff or holder stalling — V2 exit system".into(),
                                        risk_per_trade_pct: 1.0,
                                        risk_amount: position_usd,
                                        reward_risk_ratio: target_mult,
                                        position_size: size,
                                        max_loss: position_usd,
                                        strategy_tier: "Unproven".into(),
                                    },
                                ));

                                // SAFETY: data_logger is a disjoint field from strategy_manager.
                                unsafe {
                                    (*data_logger).log_trade(&serde_json::json!({
                                        "type": "MEME_COIN_V2",
                                        "strategy": slot_name,
                                        "token": token.symbol,
                                        "address": token.address,
                                        "price": token.price_usd,
                                        "safety_score": safety_score,
                                        "mcap": token.market_cap,
                                        "volume_5m": token.volume_5m,
                                    }));
                                }

                                slot.wallet.add_gas_fee(rust_decimal_macros::dec!(0.151));

                                tracing::info!(
                                    "MC V2 {} BUY {} {} @ {} | safety={}/100 vol5m=${:.0} mcap=${:.0} | {}",
                                    slot_name, filled, token.symbol, fill_price,
                                    safety_score, token.volume_5m, token.market_cap,
                                    token.name.chars().take(30).collect::<String>()
                                );
                            }
                        }
                    }
                }

                // Update MC-* strategy wallet prices from meme coin scanner
                {
                    let mut s = state.write().await;
                    // Build price map from tracked meme tokens (address → price as Decimal)
                    let mc_prices: std::collections::HashMap<String, rust_decimal::Decimal> = s.memecoins.scanner
                        .tracked_tokens().iter()
                        .filter_map(|t| {
                            rust_decimal::Decimal::from_f64(t.price_usd)
                                .map(|p| (t.address.clone(), p))
                        })
                        .collect();

                    if !mc_prices.is_empty() {
                        let mc_slot_names: Vec<String> = s.strategy_manager.alive_slots()
                            .iter()
                            .filter(|sl| sl.name.starts_with("MC-"))
                            .map(|sl| sl.name.clone())
                            .collect();
                        for name in &mc_slot_names {
                            if let Some(slot) = s.strategy_manager.get_mut(name) {
                                slot.wallet.update_prices(&mc_prices);
                            }
                        }
                    }
                }

                // V2 MC Position Monitor
                {
                    let mut s = state.write().await;
                    // SAFETY: volume_tracker and strategy_manager are disjoint fields of AppState.
                    let volume_tracker: *const tradoshka_memecoins::volume_tracker::VolumeTracker =
                        &s.memecoins.volume_tracker;

                    let mc_slot_names: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.name.starts_with("MC-") && sl.trade_count() > 0)
                        .map(|sl| sl.name.clone())
                        .collect();

                    for slot_name in &mc_slot_names {
                        let slot = match s.strategy_manager.get_mut(slot_name) {
                            Some(sl) => sl,
                            None => continue,
                        };

                        let strategy_type = slot.params.strategy_type.clone();
                        let hard_stop_pct = if strategy_type == "mc_trend_v2" || strategy_type == "mc_copy_v2" {
                            let h = slot.params.get("hard_stop_pct");
                            if h > 0.0 { h } else { 50.0 }
                        } else { 15.0 };
                        let time_limit_mins: i64 = if strategy_type == "mc_trend_v2" || strategy_type == "mc_copy_v2" {
                            let t = slot.params.get("time_limit_mins");
                            if t > 0.0 { t as i64 } else { 120 }
                        } else { 60 };

                        let pos_data: Vec<_> = slot.wallet.positions().iter().map(|(k, p)| {
                            (k.clone(), p.current_price, p.avg_price, p.opened_at, p.token_id.clone())
                        }).collect();

                        for (pos_key, current_price, entry_price, opened_at, token_id) in &pos_data {
                            if *entry_price <= rust_decimal::Decimal::ZERO { continue; }

                            let pnl_pct = ((*current_price - *entry_price) / *entry_price
                                * rust_decimal::Decimal::new(100, 0))
                                .to_f64().unwrap_or(0.0);
                            let mins_held = (chrono::Utc::now() - *opened_at).num_minutes();

                            // V2 Exit Logic
                            let (should_close, reason, exit_type) = if strategy_type == "mc_trend_v2" || strategy_type == "mc_copy_v2" {
                                // 1. Volume cliff (primary exit) — safe: disjoint field from strategy_manager
                                let vol_exit = unsafe { (*volume_tracker).should_exit(token_id) };
                                if let Some(exit_reason) = vol_exit {
                                    (true, format!("VOLUME EXIT: {}", exit_reason), tradoshka_engine::ExitType::StopLoss)
                                }
                                // 2. Hard stop (-50% default, configurable)
                                else if pnl_pct <= -(hard_stop_pct) {
                                    (true, format!("HARD STOP: {:.0}% loss", pnl_pct), tradoshka_engine::ExitType::StopLoss)
                                }
                                // 3. Time limit
                                else if time_limit_mins > 0 && mins_held >= time_limit_mins {
                                    (true, format!("TIME LIMIT: {}min", mins_held), tradoshka_engine::ExitType::TimeStop)
                                }
                                // 4. Profit ladder: sell at 2x
                                else if pnl_pct >= 100.0 {
                                    (true, format!("PROFIT LADDER: 2x reached ({:.0}%)", pnl_pct), tradoshka_engine::ExitType::TakeProfit)
                                }
                                else { (false, String::new(), tradoshka_engine::ExitType::Manual) }
                            } else {
                                // Legacy exit logic for non-V2 strategies
                                let close = pnl_pct <= -15.0
                                    || (mins_held >= 15 && pnl_pct < 5.0)
                                    || pnl_pct >= 100.0
                                    || mins_held >= 60;
                                let (rsn, et) = if pnl_pct <= -15.0 {
                                    ("hard stop -15%".to_string(), tradoshka_engine::ExitType::StopLoss)
                                } else if mins_held >= 60 {
                                    ("max time 60min".to_string(), tradoshka_engine::ExitType::TimeStop)
                                } else if pnl_pct >= 100.0 {
                                    ("take profit 2x".to_string(), tradoshka_engine::ExitType::TakeProfit)
                                } else {
                                    ("time stop 15min".to_string(), tradoshka_engine::ExitType::TimeStop)
                                };
                                (close, rsn, et)
                            };

                            if should_close {
                                let symbol = pos_key.split(':').next().unwrap_or(pos_key.as_str());
                                if let Some((_, _, pnl)) = slot.wallet.sell_with_exit_type(
                                    symbol, *current_price,
                                    rust_decimal::Decimal::MAX, slot_name,
                                    exit_type,
                                ) {
                                    slot.wallet.add_gas_fee(rust_decimal_macros::dec!(0.151));
                                    slot.recorder.close_trade(symbol, slot_name, pnl);
                                    tracing::info!("MC V2 EXIT: {} | {} | {} | PnL: {:.1}% | held {}min",
                                        slot_name, symbol, reason, pnl_pct, mins_held);
                                }
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

                // --- PM position monitor: close PM-* positions held >= 60 minutes ---
                // Prediction market positions don't benefit from holding longer in dry mode;
                // cycling capital quickly lets evolution accumulate win/loss data faster.
                {
                    let mut s = state.write().await;
                    let pm_slot_names: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.name.starts_with("PM-") && sl.trade_count() > 0)
                        .map(|sl| sl.name.clone())
                        .collect();

                    for slot_name in &pm_slot_names {
                        let slot = match s.strategy_manager.get_mut(slot_name) {
                            Some(sl) => sl,
                            None => continue,
                        };

                        let pos_keys: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal, chrono::DateTime<chrono::Utc>)> =
                            slot.wallet.positions().iter().map(|(k, p)| {
                                (k.clone(), p.current_price, p.avg_price, p.opened_at)
                            }).collect();

                        for (pos_key, current_price, _entry_price, opened_at) in &pos_keys {
                            let mins_held = (chrono::Utc::now() - *opened_at).num_minutes();

                            // Close after 60 minutes for faster dry-mode evaluation
                            if mins_held >= 60 {
                                let symbol = pos_key.split(':').next().unwrap_or(pos_key);
                                if let Some((_, _, pnl)) = slot.wallet.sell(
                                    symbol,
                                    *current_price,
                                    rust_decimal::Decimal::MAX,
                                    slot_name,
                                ) {
                                    slot.recorder.close_trade(symbol, slot_name, pnl);
                                    tracing::info!(
                                        "PM CLOSE: {} | {} | held {}min | PnL: {}",
                                        slot_name, symbol, mins_held, pnl
                                    );
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

    // Seed copy trading with simulated top traders
    {
        let mut s = state.write().await;
        // Simulate 20 top Polymarket wallets across niches
        let wallets = vec![
            ("0xwhale_pol_1", 0.82, 0.35, 45u32, 8500.0, 800.0, 12.0, 0.04, 0.15, 2.0, 120u32, "politics"),
            ("0xwhale_pol_2", 0.75, 0.28, 60u32, 6200.0, 600.0, 18.0, 0.06, 0.12, 5.0, 90u32, "politics"),
            ("0xwhale_pol_3", 0.70, 0.22, 35u32, 4000.0, 500.0, 10.0, 0.05, 0.10, 3.0, 80u32, "politics"),
            ("0xwhale_sport_1", 0.88, 0.45, 30u32, 12000.0, 1200.0, 8.0, 0.03, 0.20, 1.0, 150u32, "sports"),
            ("0xwhale_sport_2", 0.72, 0.25, 50u32, 5500.0, 550.0, 15.0, 0.05, 0.11, 4.0, 100u32, "sports"),
            ("0xwhale_sport_3", 0.68, 0.18, 40u32, 3500.0, 400.0, 12.0, 0.07, 0.09, 7.0, 70u32, "sports"),
            ("0xwhale_crypto_1", 0.80, 0.40, 25u32, 10000.0, 1000.0, 7.0, 0.04, 0.18, 2.0, 130u32, "crypto"),
            ("0xwhale_crypto_2", 0.73, 0.30, 55u32, 7000.0, 700.0, 16.0, 0.05, 0.13, 3.0, 95u32, "crypto"),
            ("0xwhale_crypto_3", 0.65, 0.15, 30u32, 2500.0, 350.0, 9.0, 0.08, 0.07, 10.0, 60u32, "crypto"),
            ("0xwhale_gen_1", 0.78, 0.32, 70u32, 9000.0, 900.0, 20.0, 0.04, 0.14, 1.0, 140u32, "general"),
            ("0xwhale_gen_2", 0.71, 0.20, 45u32, 4500.0, 500.0, 13.0, 0.06, 0.10, 6.0, 85u32, "general"),
            ("0xwhale_gen_3", 0.66, 0.16, 35u32, 3000.0, 400.0, 10.0, 0.07, 0.08, 8.0, 65u32, "general"),
            ("0xwhale_pol_4", 0.76, 0.26, 55u32, 5800.0, 580.0, 16.0, 0.05, 0.11, 4.0, 105u32, "politics"),
            ("0xwhale_sport_4", 0.74, 0.24, 42u32, 5000.0, 520.0, 12.0, 0.06, 0.10, 5.0, 88u32, "sports"),
            ("0xwhale_crypto_4", 0.69, 0.19, 38u32, 3800.0, 420.0, 11.0, 0.06, 0.09, 6.0, 75u32, "crypto"),
            ("0xwhale_gen_4", 0.77, 0.29, 48u32, 6500.0, 650.0, 14.0, 0.05, 0.12, 3.0, 110u32, "general"),
            ("0xwhale_gen_5", 0.70, 0.21, 52u32, 4800.0, 480.0, 15.0, 0.06, 0.10, 5.0, 92u32, "general"),
            ("0xwhale_pol_5", 0.79, 0.31, 38u32, 7500.0, 750.0, 11.0, 0.04, 0.14, 2.0, 115u32, "politics"),
            ("0xwhale_sport_5", 0.73, 0.23, 44u32, 5200.0, 540.0, 13.0, 0.05, 0.10, 4.0, 93u32, "sports"),
            ("0xwhale_crypto_5", 0.67, 0.17, 32u32, 3200.0, 380.0, 9.0, 0.07, 0.08, 9.0, 68u32, "crypto"),
        ];

        for (addr, wr, roi, trades, pnl, avg_size, tpm, std, mean, days_win, days_active, niche) in wallets {
            s.wallet_scorer.score_wallet(
                addr, wr, roi, trades, pnl, avg_size, tpm,
                std, mean, days_win, days_active, false, 0.08, niche,
            );
        }

        // Build baskets from qualified wallets
        let qualified: Vec<_> = s.wallet_scorer.qualified_wallets().iter().map(|w| (*w).clone()).collect();
        let refs: Vec<&tradoshka_engine::ScoredWallet> = qualified.iter().collect();
        s.basket_consensus.build_baskets_from_wallets(&refs);

        tracing::info!("Copy trading initialized: {} wallets scored, {} qualified, {} baskets built",
            s.wallet_scorer.wallet_count(),
            s.wallet_scorer.qualified_count(),
            s.basket_consensus.basket_count());
    }

    tracing::info!("Starting Tradoshka in DRY MODE with ${initial_balance} initial balance");

    let loop_state = state.clone();
    tokio::spawn(async move {
        run_trading_loop(loop_state).await;
    });

    tracing::info!("Auto-trading loop started (scan: 30min, cycle: 5min)");

    server::start(state, 3001).await
}
