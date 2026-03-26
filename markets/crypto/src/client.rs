use reqwest::Client;
use serde_json::Value;
use crate::config::*;
use crate::auth::BinanceAuth;
use crate::types::*;
use tradoshka_common::error::{TradoshkaError, Result};

pub struct BinanceClient {
    http: Client,
    auth: Option<BinanceAuth>,
    market_type: BinanceMarketType,
}

impl BinanceClient {
    pub fn new_public(market_type: BinanceMarketType) -> Self {
        Self { http: Client::new(), auth: None, market_type }
    }

    pub fn new_authenticated(creds: BinanceCredentials, market_type: BinanceMarketType) -> Self {
        Self {
            http: Client::new(),
            auth: Some(BinanceAuth::new(creds.api_key, creds.secret_key)),
            market_type,
        }
    }

    fn base_url(&self) -> &str { self.market_type.rest_url() }
    fn prefix(&self) -> &str { self.market_type.api_prefix() }

    // -- Public endpoints --

    pub async fn ping(&self) -> Result<()> {
        let url = format!("{}{}/ping", self.base_url(), self.prefix());
        self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_server_time(&self) -> Result<i64> {
        let url = format!("{}{}/time", self.base_url(), self.prefix());
        let resp: Value = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?
            .json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(resp["serverTime"].as_i64().unwrap_or(0))
    }

    pub async fn get_ticker_price(&self, symbol: &str) -> Result<BinanceTicker> {
        let url = format!("{}{}/ticker/price?symbol={}", self.base_url(), self.prefix(), symbol);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_all_tickers(&self) -> Result<Vec<BinanceTicker>> {
        let url = format!("{}{}/ticker/price", self.base_url(), self.prefix());
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_depth(&self, symbol: &str, limit: u32) -> Result<BinanceDepth> {
        let path = if self.market_type == BinanceMarketType::Spot { "/api/v3/depth" } else { "/fapi/v1/depth" };
        let url = format!("{}{}?symbol={}&limit={}", self.base_url(), path, symbol, limit);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_klines(&self, symbol: &str, interval: &str, limit: u32) -> Result<Vec<Vec<Value>>> {
        let path = if self.market_type == BinanceMarketType::Spot { "/api/v3/klines" } else { "/fapi/v1/klines" };
        let url = format!("{}{}?symbol={}&interval={}&limit={}", self.base_url(), path, symbol, interval, limit);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_book_ticker(&self, symbol: &str) -> Result<BinanceBookTicker> {
        let path = if self.market_type == BinanceMarketType::Spot { "/api/v3/ticker/bookTicker" } else { "/fapi/v1/ticker/bookTicker" };
        let url = format!("{}{}?symbol={}", self.base_url(), path, symbol);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    // -- Futures-specific public --

    pub async fn get_funding_rate(&self, symbol: &str) -> Result<Vec<FundingRateInfo>> {
        if self.market_type != BinanceMarketType::UsdtFutures {
            return Err(TradoshkaError::AdapterError("Funding rates only for futures".into()));
        }
        let url = format!("{}/fapi/v1/premiumIndex?symbol={}", self.base_url(), symbol);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        let single: FundingRateInfo = resp.json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(vec![single])
    }

    pub async fn get_all_funding_rates(&self) -> Result<Vec<FundingRateInfo>> {
        if self.market_type != BinanceMarketType::UsdtFutures {
            return Err(TradoshkaError::AdapterError("Funding rates only for futures".into()));
        }
        let url = format!("{}/fapi/v1/premiumIndex", self.base_url());
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    // -- Authenticated endpoints --

    async fn signed_get(&self, path: &str, params: &[(&str, &str)]) -> Result<Value> {
        let auth = self.auth.as_ref()
            .ok_or_else(|| TradoshkaError::AdapterError("Not authenticated".into()))?;
        let query = auth.sign_params(params);
        let url = format!("{}{}?{}", self.base_url(), path, query);
        let resp = self.http.get(&url)
            .header("X-MBX-APIKEY", auth.api_key())
            .send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(TradoshkaError::AdapterError(
                format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default())
            ));
        }
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    async fn signed_post(&self, path: &str, params: &[(&str, &str)]) -> Result<Value> {
        let auth = self.auth.as_ref()
            .ok_or_else(|| TradoshkaError::AdapterError("Not authenticated".into()))?;
        let query = auth.sign_params(params);
        let url = format!("{}{}?{}", self.base_url(), path, query);
        let resp = self.http.post(&url)
            .header("X-MBX-APIKEY", auth.api_key())
            .send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(TradoshkaError::AdapterError(
                format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default())
            ));
        }
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    async fn signed_delete(&self, path: &str, params: &[(&str, &str)]) -> Result<Value> {
        let auth = self.auth.as_ref()
            .ok_or_else(|| TradoshkaError::AdapterError("Not authenticated".into()))?;
        let query = auth.sign_params(params);
        let url = format!("{}{}?{}", self.base_url(), path, query);
        let resp = self.http.delete(&url)
            .header("X-MBX-APIKEY", auth.api_key())
            .send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(TradoshkaError::AdapterError(
                format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default())
            ));
        }
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn place_order(&self, symbol: &str, side: &str, order_type: &str,
                              quantity: &str, price: Option<&str>, time_in_force: Option<&str>) -> Result<BinanceOrderResponse> {
        let path = if self.market_type == BinanceMarketType::Spot { "/api/v3/order" } else { "/fapi/v1/order" };
        let mut params = vec![
            ("symbol", symbol),
            ("side", side),
            ("type", order_type),
            ("quantity", quantity),
        ];
        if let Some(p) = price { params.push(("price", p)); }
        if let Some(tif) = time_in_force { params.push(("timeInForce", tif)); }
        let resp = self.signed_post(path, &params).await?;
        serde_json::from_value(resp).map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<Value> {
        let path = if self.market_type == BinanceMarketType::Spot { "/api/v3/order" } else { "/fapi/v1/order" };
        self.signed_delete(path, &[("symbol", symbol), ("orderId", order_id)]).await
    }

    pub async fn get_account(&self) -> Result<Value> {
        let path = if self.market_type == BinanceMarketType::Spot { "/api/v3/account" } else { "/fapi/v2/account" };
        self.signed_get(path, &[]).await
    }

    pub async fn get_open_orders(&self, symbol: Option<&str>) -> Result<Value> {
        let path = if self.market_type == BinanceMarketType::Spot { "/api/v3/openOrders" } else { "/fapi/v1/openOrders" };
        let params = if let Some(s) = symbol { vec![("symbol", s)] } else { vec![] };
        self.signed_get(path, &params).await
    }
}
