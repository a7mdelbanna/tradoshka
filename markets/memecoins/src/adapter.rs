use async_trait::async_trait;
use rust_decimal::Decimal;
use tradoshka_common::types::*;
use tradoshka_common::traits::MarketAdapter;
use tradoshka_common::error::Result;
use crate::token_scanner::TokenScanner;
use crate::safety::SafetyFilter;
use crate::whale_tracker::MemeWhaleTracker;

pub struct MemeCoinAdapter {
    pub scanner: TokenScanner,
    pub safety: SafetyFilter,
    pub whale_tracker: MemeWhaleTracker,
    connected: bool,
}

impl MemeCoinAdapter {
    pub fn new() -> Self {
        Self {
            scanner: TokenScanner::new(),
            safety: SafetyFilter::new(),
            whale_tracker: MemeWhaleTracker::new(),
            connected: false,
        }
    }
}

#[async_trait]
impl MarketAdapter for MemeCoinAdapter {
    fn name(&self) -> &str { "memecoins" }
    fn market(&self) -> Market { Market::Crypto } // Reuse Crypto for now

    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        tracing::info!("Connected to Solana DEX (DexScreener)");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    async fn place_order(&self, order: &Order) -> Result<OrderId> {
        Ok(order.id) // Dry mode
    }

    async fn cancel_order(&self, _id: &OrderId) -> Result<()> { Ok(()) }

    async fn get_positions(&self) -> Result<Vec<Position>> { Ok(Vec::new()) }

    async fn get_balances(&self) -> Result<Balances> {
        Ok(Balances { total: Decimal::ZERO, available: Decimal::ZERO, in_positions: Decimal::ZERO })
    }
}
