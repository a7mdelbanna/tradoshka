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

                            let ind = slot.indicators.update(&asset.symbol, price_f64);
                            let snapshot = MarketSnapshot {
                                symbol: asset.symbol.clone(),
                                market: Market::Crypto,
                                current_price: asset.price,
                                price_24h_ago: None,
                                volume_24h: asset.volume_24h,
                                volume_7d_avg: Some(asset.volume_24h * 0.8),
                                atr_14: Decimal::from_f64(ind.atr.unwrap_or(price_f64 * 0.02)).unwrap_or(dec!(0)),
                                rsi_14: ind.rsi,
                                ema_9: ind.ema_fast.and_then(|v| Decimal::from_f64(v)),
                                ema_21: ind.ema_slow.and_then(|v| Decimal::from_f64(v)),
                                funding_rate: None,
                                question: format!("{} Spot", asset.symbol),
                                days_to_resolution: None,
                            };
                            let thesis = match ResearchEngine::analyze_crypto(&snapshot, &config) {
                                Ok(t) => t,
                                Err(_) => continue,
                            };

                            // Cap to $10 and convert USD amount to coin quantity
                            let dollar_size = thesis.position_size.min(dec!(10));
                            let size = if asset.price > rust_decimal::Decimal::ZERO {
                                (dollar_size / asset.price).round_dp(8)
                            } else { rust_decimal::Decimal::ZERO };
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
                                tracing::info!(
                                    "EVOLUTION {} BUY {} {} @ {} — R:R {:.1}x | {}",
                                    slot_name, filled, asset.symbol, fill_price,
                                    thesis.reward_risk_ratio,
                                    thesis.reasoning.chars().take(50).collect::<String>()
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

                            for market in &poly_markets {
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

                                // Strategy decides: trade if deviation exceeds its min_edge
                                // OR if volume is strong enough to signal real interest
                                let has_mispricing = deviation > min_edge;
                                let has_volume = market.volume_24h > min_volume;
                                if !has_mispricing && !has_volume { continue; }

                                // Buy the cheaper side (better expected value)
                                let (token_id, price, outcome) =
                                    if market.yes_price <= market.no_price {
                                        (&market.yes_token_id, market.yes_price, "Yes")
                                    } else {
                                        (&market.no_token_id, market.no_price, "No")
                                    };

                                if price <= rust_decimal::Decimal::ZERO { continue; }

                                // Risk-based position sizing: risk 1% of equity, max 80% loss on PM
                                let equity = slot.wallet.equity();
                                let risk_amount = equity * dec!(0.01);
                                let stop_distance = price * dec!(0.80);
                                if stop_distance <= rust_decimal::Decimal::ZERO { continue; }
                                let size = (risk_amount / stop_distance)
                                    .round_dp(0)
                                    .min(dec!(50))
                                    .max(dec!(1));

                                let hard_sl = price * dec!(0.20);
                                let take_profit = (price + stop_distance * dec!(2)).min(dec!(0.99));
                                let thesis = tradoshka_engine::TradeThesis {
                                    reasoning: format!(
                                        "PM strategy {}. Market: {}. Buy {} at {}. \
                                         Deviation: {:.1}%. Volume: ${:.0}/24h",
                                        slot_name,
                                        market.question,
                                        outcome,
                                        price,
                                        deviation * 100.0,
                                        market.volume_24h,
                                    ),
                                    signals_used: vec!["market_analysis".into()],
                                    signals_agreed: 1,
                                    signals_total: 1,
                                    confidence: 0.5,
                                    entry_price: price,
                                    entry_reason: format!("Buy {} at {}", outcome, price),
                                    hard_stop_loss: hard_sl.max(dec!(0.01)),
                                    trailing_stop: (price * dec!(0.70)).max(dec!(0.01)),
                                    take_profit,
                                    time_stop_hours: 72,
                                    thesis_invalidation: "Market resolved or price drops 80%"
                                        .into(),
                                    risk_per_trade_pct: 1.0,
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
                                        "EVOLUTION {} PM BUY {} shares of {} @ {} | \
                                         dev={:.1}% vol=${:.0} | {}",
                                        slot_name,
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
                            let snapshot = MarketSnapshot {
                                symbol: asset.symbol.clone(),
                                market: Market::Crypto,
                                current_price: asset.price,
                                price_24h_ago: None,
                                volume_24h: asset.volume_24h,
                                volume_7d_avg: Some(asset.volume_24h * 0.9),
                                atr_14: Decimal::from_f64(ind.atr.unwrap_or(atr_default)).unwrap_or(dec!(0)),
                                rsi_14: ind.rsi,
                                ema_9: ind.ema_fast.and_then(|v| Decimal::from_f64(v)),
                                ema_21: ind.ema_slow.and_then(|v| Decimal::from_f64(v)),
                                funding_rate: Some(-0.0001), // Slight negative funding default for perps
                                question: format!("{} Perp", asset.symbol),
                                days_to_resolution: None,
                            };
                            let thesis = match ResearchEngine::analyze_crypto(&snapshot, &config) {
                                Ok(t) => t,
                                Err(_) => continue,
                            };

                            // Cap to $10 and convert USD amount to coin quantity
                            let dollar_size = thesis.position_size.min(dec!(10));
                            let size = if asset.price > rust_decimal::Decimal::ZERO {
                                (dollar_size / asset.price).round_dp(8)
                            } else { rust_decimal::Decimal::ZERO };
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
                                tracing::info!(
                                    "EVOLUTION {} CP BUY {} {} @ {} — R:R {:.1}x | {}",
                                    slot_name, filled, asset.symbol, fill_price,
                                    thesis.reward_risk_ratio,
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
