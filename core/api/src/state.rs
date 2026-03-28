use std::sync::Arc;
use tokio::sync::RwLock;
use tradoshka_engine::{OrderManager, PortfolioTracker, DryModeEngine, SimulatedWallet, TradeRecorder, ReadinessScorer, MarketDataService, MarketDataConfig, Orchestrator, OrchestratorConfig, CryptoDataService, PerpWallet, StrategyWalletManager, EvolutionEngine, DataLogger, WalletScorer, BasketConsensus, CopyEngine, CopyCircuitBreaker};
use tradoshka_risk::TradoshkaRiskManager;
use tradoshka_polymarket::adapter::PolymarketAdapter;
use tradoshka_crypto::adapter::CryptoAdapter;
use tradoshka_memecoins::adapter::MemeCoinAdapter;
use rust_decimal_macros::dec;

pub type SharedState = Arc<RwLock<AppState>>;

pub struct AppState {
    pub order_manager: OrderManager,
    pub portfolio: PortfolioTracker,
    pub risk_manager: TradoshkaRiskManager,
    pub dry_engine: DryModeEngine,
    pub polymarket: Option<PolymarketAdapter>,
    pub crypto: Option<CryptoAdapter>,
    /// Aggregate wallet (all markets combined -- kept for legacy /api/wallet endpoint)
    pub wallet: SimulatedWallet,
    /// Aggregate trade recorder (all markets -- kept for legacy endpoints)
    pub trade_recorder: TradeRecorder,
    /// Per-market wallet: Polymarket
    pub polymarket_wallet: SimulatedWallet,
    /// Per-market trade recorder: Polymarket
    pub polymarket_recorder: TradeRecorder,
    /// Per-market wallet: Crypto
    pub crypto_wallet: SimulatedWallet,
    /// Per-market trade recorder: Crypto
    pub crypto_recorder: TradeRecorder,
    /// Per-market wallet: Perpetuals
    pub perp_wallet: PerpWallet,
    /// Per-market trade recorder: Perpetuals
    pub perp_recorder: TradeRecorder,
    pub readiness_scorer: ReadinessScorer,
    pub market_data: MarketDataService,
    pub orchestrator: Orchestrator,
    pub crypto_data: CryptoDataService,
    pub strategy_manager: StrategyWalletManager,
    pub evolution_engine: EvolutionEngine,
    pub data_logger: DataLogger,
    // Copy trading components
    pub wallet_scorer: WalletScorer,
    pub basket_consensus: BasketConsensus,
    pub copy_engine: CopyEngine,
    pub copy_circuit_breaker: CopyCircuitBreaker,
    // Meme coin market adapter
    pub memecoins: MemeCoinAdapter,
    // MC-V2 AI scorer
    pub claude_scorer: tradoshka_engine::ClaudeScorer,
}

impl AppState {
    pub fn new_dry_mode(initial_balance: rust_decimal::Decimal) -> Self {
        Self {
            order_manager: OrderManager::new(),
            portfolio: PortfolioTracker::new(initial_balance),
            risk_manager: TradoshkaRiskManager::new(tradoshka_risk::RiskConfig {
                initial_equity: initial_balance,
                ..Default::default()
            }),
            dry_engine: DryModeEngine::new(dec!(5)),
            polymarket: None,
            crypto: None,
            wallet: SimulatedWallet::new(initial_balance, dec!(5), dec!(0.001)),
            trade_recorder: TradeRecorder::new(),
            polymarket_wallet: SimulatedWallet::new(dec!(100), dec!(5), dec!(0.002)),
            polymarket_recorder: TradeRecorder::new(),
            crypto_wallet: SimulatedWallet::new(dec!(100), dec!(5), dec!(0.001)),
            crypto_recorder: TradeRecorder::new(),
            perp_wallet: PerpWallet::new(dec!(100), 10), // $100, 10x default leverage
            perp_recorder: TradeRecorder::new(),
            readiness_scorer: ReadinessScorer::default(),
            market_data: MarketDataService::new(MarketDataConfig::default()),
            orchestrator: Orchestrator::new(OrchestratorConfig::default()),
            crypto_data: CryptoDataService::new(),
            strategy_manager: {
                let mut strategy_manager = StrategyWalletManager::new(200, 25, dec!(100));
                strategy_manager.initialize_defaults();
                strategy_manager
            },
            evolution_engine: EvolutionEngine::new(),
            data_logger: DataLogger::new("data"),
            wallet_scorer: WalletScorer::new(),
            basket_consensus: BasketConsensus::new().with_threshold(0.50).with_price_range(0.05, 0.95),
            copy_engine: CopyEngine::new(),
            copy_circuit_breaker: CopyCircuitBreaker::new(100.0), // $100 per strategy
            memecoins: MemeCoinAdapter::new(),
            claude_scorer: tradoshka_engine::ClaudeScorer::new(),
        }
    }

    pub fn with_polymarket(mut self) -> Self {
        self.polymarket = Some(PolymarketAdapter::new_public());
        self
    }

    pub fn with_crypto(mut self) -> Self {
        self.crypto = Some(CryptoAdapter::new_public());
        self
    }
}

pub fn create_shared_state(initial_balance: rust_decimal::Decimal) -> SharedState {
    Arc::new(RwLock::new(AppState::new_dry_mode(initial_balance)))
}
