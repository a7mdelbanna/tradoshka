use rust_decimal_macros::dec;
use tradoshka_api::state::{create_shared_state, SharedState};
use tradoshka_api::server;
use tradoshka_engine::TradeRecord;
use tradoshka_common::types::{Market, OrderSide};
use tradoshka_engine::TrackedCryptoAsset;
use tradoshka_engine::{ResearchEngine, ResearchConfig, MarketSnapshot};

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
                                let trade = TradeRecord {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    timestamp: chrono::Utc::now(),
                                    market: Market::Crypto,
                                    symbol: asset.symbol.clone(),
                                    market_question: format!("{} Spot", asset.symbol),
                                    direction: direction.into(),
                                    side: if is_long { OrderSide::Buy } else { OrderSide::Sell },
                                    shares: filled,
                                    price: fill_price,
                                    fee,
                                    strategy_id: "research".into(),
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
                                };
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

                // Poll Polymarket prices and update polymarket wallet
                {
                    let mut s = state.write().await;
                    s.market_data.poll_prices().await;
                    let prices = s.market_data.current_prices();
                    s.polymarket_wallet.update_prices(&prices);
                }
            }
        }
    }
}

/// Build a MarketSnapshot for the research engine from a TrackedCryptoAsset.
/// Uses estimated indicators since we don't have candle data yet.
fn build_crypto_snapshot(asset: &TrackedCryptoAsset) -> MarketSnapshot {
    let price = asset.price;
    MarketSnapshot {
        symbol: asset.symbol.clone(),
        market: tradoshka_common::types::Market::Crypto,
        current_price: price,
        price_24h_ago: None,
        volume_24h: asset.volume_24h,
        volume_7d_avg: Some(asset.volume_24h * 0.8), // Conservative 7d average estimate
        atr_14: price * dec!(0.02),  // 2% ATR estimate (real candle data not yet available)
        rsi_14: Some(55.0),          // Neutral RSI estimate
        ema_9: Some(price * dec!(1.005)),  // Slight upward EMA bias to detect trend
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
