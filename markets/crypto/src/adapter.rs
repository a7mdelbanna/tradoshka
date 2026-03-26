use async_trait::async_trait;
use rust_decimal::Decimal;
use tradoshka_common::types::*;
use tradoshka_common::traits::MarketAdapter;
use tradoshka_common::error::Result;
use crate::client::BinanceClient;
use crate::config::BinanceMarketType;
use crate::rate_limiter::RateLimiter;

pub struct CryptoAdapter {
    spot_client: BinanceClient,
    futures_client: BinanceClient,
    spot_limiter: RateLimiter,
    futures_limiter: RateLimiter,
    connected: bool,
}

impl CryptoAdapter {
    pub fn new_public() -> Self {
        Self {
            spot_client: BinanceClient::new_public(BinanceMarketType::Spot),
            futures_client: BinanceClient::new_public(BinanceMarketType::UsdtFutures),
            spot_limiter: RateLimiter::new(6000, 60),
            futures_limiter: RateLimiter::new(2400, 60),
            connected: false,
        }
    }

    pub fn spot_client(&self) -> &BinanceClient { &self.spot_client }
    pub fn futures_client(&self) -> &BinanceClient { &self.futures_client }
}

#[async_trait]
impl MarketAdapter for CryptoAdapter {
    fn name(&self) -> &str { "binance" }
    fn market(&self) -> Market { Market::Crypto }

    async fn connect(&mut self) -> Result<()> {
        self.spot_client.ping().await?;
        self.connected = true;
        tracing::info!("Connected to Binance API");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    async fn place_order(&self, order: &Order) -> Result<OrderId> {
        Ok(order.id) // Dry mode — no real orders
    }

    async fn cancel_order(&self, _id: &OrderId) -> Result<()> {
        Ok(()) // Dry mode
    }

    async fn get_positions(&self) -> Result<Vec<Position>> {
        Ok(Vec::new())
    }

    async fn get_balances(&self) -> Result<Balances> {
        Ok(Balances { total: Decimal::ZERO, available: Decimal::ZERO, in_positions: Decimal::ZERO })
    }
}
