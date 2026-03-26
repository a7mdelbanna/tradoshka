use async_trait::async_trait;
use rust_decimal::Decimal;
use tradoshka_common::types::*;
use tradoshka_common::traits::MarketAdapter;
use tradoshka_common::error::{TradoshkaError, Result};
use crate::client::PolymarketClient;
use crate::config::ApiCredentials;
use crate::scanner::{MarketScanner, MarketFilter};
use crate::types::*;

pub struct PolymarketAdapter {
    client: PolymarketClient,
    scanner: MarketScanner,
    connected: bool,
}

impl PolymarketAdapter {
    /// Create a read-only adapter (no trading, useful for scanning and data)
    pub fn new_public() -> Self {
        Self {
            client: PolymarketClient::new_public(),
            scanner: MarketScanner::new(MarketFilter::default()),
            connected: false,
        }
    }

    /// Create a fully authenticated adapter for trading
    pub fn new_authenticated(creds: ApiCredentials) -> Self {
        Self {
            client: PolymarketClient::new_authenticated(creds),
            scanner: MarketScanner::new(MarketFilter::default()),
            connected: false,
        }
    }

    /// Access the underlying client for advanced operations
    pub fn client(&self) -> &PolymarketClient {
        &self.client
    }

    /// Access the market scanner
    pub fn scanner(&self) -> &MarketScanner {
        &self.scanner
    }

    /// Scan for tradeable markets
    pub async fn scan_markets(&self) -> Result<Vec<GammaMarket>> {
        let markets = self.client.get_gamma_markets(true, 100, 0).await?;
        Ok(self.scanner.filter_markets(&markets))
    }
}

#[async_trait]
impl MarketAdapter for PolymarketAdapter {
    fn name(&self) -> &str {
        "polymarket"
    }

    fn market(&self) -> Market {
        Market::Polymarket
    }

    async fn connect(&mut self) -> Result<()> {
        self.client.health().await?;
        self.connected = true;
        tracing::info!("Connected to Polymarket CLOB API");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        tracing::info!("Disconnected from Polymarket");
        Ok(())
    }

    async fn place_order(&self, order: &Order) -> Result<OrderId> {
        // Convert core Order to Polymarket order.
        // The symbol in Polymarket context is the token_id.
        let poly_order = PolyOrderRequest {
            token_id: order.symbol.clone(),
            price: match &order.order_type {
                OrderType::Limit { price } => *price,
                OrderType::Market => Decimal::ONE, // Market orders use FOK with best price
                _ => {
                    return Err(TradoshkaError::AdapterError(
                        "Polymarket only supports Limit and Market orders".into(),
                    ))
                }
            },
            size: order.quantity,
            side: match order.side {
                OrderSide::Buy => PolySide::Buy,
                OrderSide::Sell => PolySide::Sell,
            },
            order_type: match &order.order_type {
                OrderType::Market => PolyOrderType::Fok,
                _ => PolyOrderType::Gtc,
            },
        };

        let response = self.client.place_order(&poly_order).await?;

        if let Some(error_msg) = response.error_msg {
            return Err(TradoshkaError::AdapterError(error_msg));
        }

        // Use the order's existing ID since Polymarket returns its own ID format
        Ok(order.id)
    }

    async fn cancel_order(&self, id: &OrderId) -> Result<()> {
        self.client.cancel_order(&id.to_string()).await
    }

    async fn get_positions(&self) -> Result<Vec<Position>> {
        // In read-only mode or when we don't have an address, return empty.
        // When authenticated, we'd use the Data API.
        Ok(Vec::new())
    }

    async fn get_balances(&self) -> Result<Balances> {
        // Return a default balance structure.
        // In authenticated mode, this would call get_balance().
        Ok(Balances {
            total: Decimal::ZERO,
            available: Decimal::ZERO,
            in_positions: Decimal::ZERO,
        })
    }
}
