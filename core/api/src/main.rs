use rust_decimal_macros::dec;
use tradoshka_api::state::{create_shared_state, SharedState};
use tradoshka_api::server;
use tradoshka_engine::TradeRecord;
use tradoshka_common::types::{Market, OrderSide};
use tradoshka_engine::TrackedCryptoAsset;

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
    scan_ticker.tick().await;

    loop {
        tokio::select! {
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

                // --- Crypto cycle: two-phase to avoid borrow conflicts ---
                // Phase 1 (read lock): snapshot asset data and filter out
                // symbols that already have an open position.
                let assets_to_buy: Vec<TrackedCryptoAsset> = {
                    let s = state.read().await;
                    let existing: std::collections::HashSet<String> =
                        s.crypto_wallet.positions().keys().cloned().collect();
                    s.crypto_data.tracked_assets()
                        .into_iter()
                        .filter(|a| {
                            a.price > rust_decimal::Decimal::ZERO
                                && !existing.contains(&format!("{}:momentum", a.symbol))
                        })
                        .cloned()
                        .collect()
                };

                // Phase 2 (write lock): execute buys via raw pointers on disjoint fields.
                if !assets_to_buy.is_empty() {
                    let mut s = state.write().await;
                    let crypto_wallet: *mut tradoshka_engine::SimulatedWallet = &mut s.crypto_wallet;
                    let crypto_recorder: *mut tradoshka_engine::TradeRecorder = &mut s.crypto_recorder;
                    let agg_wallet: *mut tradoshka_engine::SimulatedWallet = &mut s.wallet;
                    let agg_recorder: *mut tradoshka_engine::TradeRecorder = &mut s.trade_recorder;

                    for asset in &assets_to_buy {
                        let trade_amount = dec!(5);
                        let shares = (trade_amount / asset.price).round_dp(6);
                        if shares <= rust_decimal::Decimal::ZERO {
                            continue;
                        }
                        // SAFETY: all pointers refer to disjoint fields of AppState.
                        unsafe {
                            if let Some((fill_price, fee, filled)) = (*crypto_wallet).buy(
                                &asset.symbol,
                                &format!("{} Spot", asset.symbol),
                                "Long",
                                asset.price,
                                shares,
                                "momentum",
                            ) {
                                let trade = TradeRecord {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    timestamp: chrono::Utc::now(),
                                    market: Market::Crypto,
                                    symbol: asset.symbol.clone(),
                                    market_question: format!("{} Spot", asset.symbol),
                                    direction: "Long".into(),
                                    side: OrderSide::Buy,
                                    shares: filled,
                                    price: fill_price,
                                    fee,
                                    strategy_id: "momentum".into(),
                                    signal_strength: 0.5,
                                    edge_vs_market: 0.0,
                                    pnl: None,
                                    is_closed: false,
                                };
                                (*crypto_recorder).record(trade.clone());
                                (*agg_recorder).record(trade.clone());
                                (*agg_wallet).buy(
                                    &asset.symbol,
                                    &format!("{} Spot", asset.symbol),
                                    "Long",
                                    fill_price,
                                    filled,
                                    "momentum",
                                );
                                tracing::info!(
                                    "Crypto BUY {} {} @ {} (fee: {})",
                                    filled, asset.symbol, fill_price, fee
                                );
                            }
                        }
                    }
                }

                // Perpetuals cycle — open positions on different timeframes.
                // Phase 1 (read lock): snapshot asset data and filter candidates.
                let perp_assets_to_open: Vec<TrackedCryptoAsset> = {
                    let s = state.read().await;
                    let top_symbols = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "XRPUSDT"];
                    let timeframes = ["5m", "15m", "1h"];
                    let existing_keys: std::collections::HashSet<String> =
                        s.perp_wallet.positions().keys().cloned().collect();
                    s.crypto_data.tracked_assets()
                        .into_iter()
                        .filter(|a| {
                            top_symbols.contains(&a.symbol.as_str())
                                && a.price > rust_decimal::Decimal::ZERO
                                && timeframes.iter().any(|tf| {
                                    !existing_keys.contains(&format!("{}:scalp:{}", a.symbol, tf))
                                })
                        })
                        .cloned()
                        .collect()
                };

                // Phase 2 (write lock): open perp positions.
                if !perp_assets_to_open.is_empty() {
                    let timeframes = ["5m", "15m", "1h"];
                    let mut s = state.write().await;
                    let perp_wallet: *mut tradoshka_engine::PerpWallet = &mut s.perp_wallet;
                    let perp_recorder: *mut tradoshka_engine::TradeRecorder = &mut s.perp_recorder;

                    for asset in &perp_assets_to_open {
                        for tf in &timeframes {
                            let key = format!("{}:scalp:{}", asset.symbol, tf);
                            // SAFETY: perp_wallet and perp_recorder are disjoint fields of AppState.
                            if unsafe { (*perp_wallet).positions().contains_key(&key) } { continue; }

                            let size = (dec!(2) / asset.price).round_dp(6);
                            if size <= rust_decimal::Decimal::ZERO { continue; }

                            let side = if asset.symbol.contains("BTC") || asset.symbol.contains("ETH") {
                                OrderSide::Buy
                            } else {
                                OrderSide::Sell
                            };

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
                                        signal_strength: 0.5,
                                        edge_vs_market: 0.0,
                                        pnl: None,
                                        is_closed: false,
                                    });
                                    tracing::info!("PERP {:?} {} {} @ {} {}x [{}]",
                                        side, pos.size, asset.symbol, pos.entry_price, pos.leverage, tf);
                                }
                            }
                        }
                    }
                }
            }
        }
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
