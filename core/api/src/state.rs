use std::sync::Arc;
use tokio::sync::RwLock;
use tradoshka_engine::{OrderManager, PortfolioTracker, DryModeEngine, SimulatedWallet, TradeRecorder, ReadinessScorer, MarketDataService, MarketDataConfig};
use tradoshka_risk::TradoshkaRiskManager;
use tradoshka_polymarket::adapter::PolymarketAdapter;
use rust_decimal_macros::dec;

pub type SharedState = Arc<RwLock<AppState>>;

pub struct AppState {
    pub order_manager: OrderManager,
    pub portfolio: PortfolioTracker,
    pub risk_manager: TradoshkaRiskManager,
    pub dry_engine: DryModeEngine,
    pub polymarket: Option<PolymarketAdapter>,
    pub wallet: SimulatedWallet,
    pub trade_recorder: TradeRecorder,
    pub readiness_scorer: ReadinessScorer,
    pub market_data: MarketDataService,
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
            wallet: SimulatedWallet::new(initial_balance, dec!(5)),
            trade_recorder: TradeRecorder::new(),
            readiness_scorer: ReadinessScorer::default(),
            market_data: MarketDataService::new(MarketDataConfig::default()),
        }
    }

    pub fn with_polymarket(mut self) -> Self {
        self.polymarket = Some(PolymarketAdapter::new_public());
        self
    }
}

pub fn create_shared_state(initial_balance: rust_decimal::Decimal) -> SharedState {
    Arc::new(RwLock::new(AppState::new_dry_mode(initial_balance)))
}
