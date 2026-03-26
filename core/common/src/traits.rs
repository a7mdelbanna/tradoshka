use async_trait::async_trait;
use crate::types::*;
use crate::error::Result;

#[async_trait]
pub trait MarketAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn market(&self) -> Market;
    async fn connect(&mut self) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    async fn place_order(&self, order: &Order) -> Result<OrderId>;
    async fn cancel_order(&self, id: &OrderId) -> Result<()>;
    async fn get_positions(&self) -> Result<Vec<Position>>;
    async fn get_balances(&self) -> Result<Balances>;
}

pub trait RiskManager: Send + Sync {
    fn validate_order(&self, order: &Order, portfolio: &Portfolio) -> RiskDecision;
    fn check_circuit_breakers(&self, portfolio: &Portfolio) -> bool;
    fn calculate_position_size(
        &self,
        signal: &Signal,
        portfolio: &Portfolio,
    ) -> rust_decimal::Decimal;
}

pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn market(&self) -> Market;
    fn on_market_event(&mut self, event: &MarketEvent) -> Option<Signal>;
    fn on_fill(&mut self, fill: &Fill);
}
