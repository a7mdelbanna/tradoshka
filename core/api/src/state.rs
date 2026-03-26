use std::sync::Arc;
use tokio::sync::RwLock;
use tradoshka_engine::{OrderManager, PortfolioTracker, DryModeEngine};
use tradoshka_risk::TradoshkaRiskManager;
use rust_decimal_macros::dec;

pub type SharedState = Arc<RwLock<AppState>>;

pub struct AppState {
    pub order_manager: OrderManager,
    pub portfolio: PortfolioTracker,
    pub risk_manager: TradoshkaRiskManager,
    pub dry_engine: DryModeEngine,
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
        }
    }
}

pub fn create_shared_state(initial_balance: rust_decimal::Decimal) -> SharedState {
    Arc::new(RwLock::new(AppState::new_dry_mode(initial_balance)))
}
