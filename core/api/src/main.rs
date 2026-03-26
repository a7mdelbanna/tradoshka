use rust_decimal_macros::dec;
use tradoshka_api::state::{create_shared_state, SharedState};
use tradoshka_api::server;

async fn run_trading_loop(state: SharedState) {
    use tokio::time::{interval, Duration};

    let scan_interval = Duration::from_secs(30 * 60); // 30 minutes
    let cycle_interval = Duration::from_secs(5 * 60);  // 5 minutes

    // Initial scan immediately
    {
        let mut s = state.write().await;
        let markets = s.market_data.scan_markets().await;
        tracing::info!("Initial market scan: {} markets found", markets.len());
    }

    // Initial crypto scan
    {
        let mut s = state.write().await;
        let assets = s.crypto_data.scan().await;
        tracing::info!("Initial crypto scan: {} assets tracked", assets.len());
    }

    let mut scan_ticker = interval(scan_interval);
    let mut cycle_ticker = interval(cycle_interval);

    // Skip the first tick (we already did the initial scan)
    scan_ticker.tick().await;

    loop {
        tokio::select! {
            _ = scan_ticker.tick() => {
                let mut s = state.write().await;
                let markets = s.market_data.scan_markets().await;
                tracing::info!("Market scan: {} markets tracked", markets.len());
                // Also scan crypto
                let assets = s.crypto_data.scan().await;
                tracing::info!("Crypto scan: {} assets tracked", assets.len());
            }
            _ = cycle_ticker.tick() => {
                let mut s = state.write().await;
                let markets: Vec<_> = s.market_data.tracked_markets().into_iter().cloned().collect();
                if markets.is_empty() {
                    tracing::debug!("Skipping cycle: no tracked markets");
                    continue;
                }
                // Use raw pointers to avoid multiple-mutable-borrow error on disjoint fields
                let orchestrator: *mut tradoshka_engine::Orchestrator = &mut s.orchestrator;
                let wallet: *mut tradoshka_engine::SimulatedWallet = &mut s.wallet;
                let recorder: *mut tradoshka_engine::TradeRecorder = &mut s.trade_recorder;
                // SAFETY: orchestrator, wallet, and trade_recorder are disjoint fields of AppState.
                let result = unsafe {
                    (*orchestrator).run_cycle(&markets, &mut *wallet, &mut *recorder)
                };
                if result.trades_executed > 0 {
                    tracing::info!(
                        "Cycle {}: {} markets, {} signals, {} trades executed",
                        s.orchestrator.cycle_count(),
                        result.markets_evaluated,
                        result.signals_generated,
                        result.trades_executed,
                    );
                } else {
                    tracing::debug!(
                        "Cycle {}: {} markets, {} signals, 0 trades",
                        s.orchestrator.cycle_count(),
                        result.markets_evaluated,
                        result.signals_generated,
                    );
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

    // Enable Polymarket in read-only mode
    {
        let mut s = state.write().await;
        s.polymarket = Some(tradoshka_polymarket::adapter::PolymarketAdapter::new_public());
        s.crypto = Some(tradoshka_crypto::adapter::CryptoAdapter::new_public());
    }

    tracing::info!("Starting Tradoshka in DRY MODE with ${initial_balance} initial balance");

    // Spawn auto-trading loop
    let loop_state = state.clone();
    tokio::spawn(async move {
        run_trading_loop(loop_state).await;
    });

    tracing::info!("Auto-trading loop started (scan: 30min, cycle: 5min)");

    server::start(state, 3001).await
}
