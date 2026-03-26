use reqwest::Client;
use serde_json::Value;
use crate::config::*;
use crate::auth::L2Auth;
use crate::types::*;
use tradoshka_common::error::{TradoshkaError, Result};

pub struct PolymarketClient {
    http: Client,
    auth: Option<L2Auth>,
}

impl PolymarketClient {
    /// Read-only client (public endpoints only)
    pub fn new_public() -> Self {
        Self { http: Client::new(), auth: None }
    }

    /// Authenticated client (trading enabled)
    pub fn new_authenticated(creds: ApiCredentials) -> Self {
        Self { http: Client::new(), auth: Some(L2Auth::new(creds)) }
    }

    // ── Helper for making authenticated requests ──

    async fn get_authed(&self, base_url: &str, path: &str) -> Result<Value> {
        let auth = self.auth.as_ref()
            .ok_or_else(|| TradoshkaError::AdapterError("Not authenticated".into()))?;
        let headers = auth.headers("GET", path, None);
        let url = format!("{}{}", base_url, path);
        let mut req = self.http.get(&url);
        for (k, v) in &headers {
            req = req.header(k, v);
        }
        let resp = req.send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(TradoshkaError::AdapterError(
                format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default())
            ));
        }
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    async fn post_authed(&self, base_url: &str, path: &str, body: &Value) -> Result<Value> {
        let auth = self.auth.as_ref()
            .ok_or_else(|| TradoshkaError::AdapterError("Not authenticated".into()))?;
        let body_str = serde_json::to_string(body)
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        let headers = auth.headers("POST", path, Some(&body_str));
        let url = format!("{}{}", base_url, path);
        let mut req = self.http.post(&url).header("Content-Type", "application/json").body(body_str);
        for (k, v) in &headers {
            req = req.header(k, v);
        }
        let resp = req.send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(TradoshkaError::AdapterError(
                format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default())
            ));
        }
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    async fn delete_authed(&self, base_url: &str, path: &str, body: Option<&Value>) -> Result<Value> {
        let auth = self.auth.as_ref()
            .ok_or_else(|| TradoshkaError::AdapterError("Not authenticated".into()))?;
        let body_str = body.map(|b| serde_json::to_string(b).unwrap_or_default());
        let headers = auth.headers("DELETE", path, body_str.as_deref());
        let url = format!("{}{}", base_url, path);
        let mut req = self.http.delete(&url);
        if let Some(bs) = &body_str {
            req = req.header("Content-Type", "application/json").body(bs.clone());
        }
        for (k, v) in &headers {
            req = req.header(k, v);
        }
        let resp = req.send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(TradoshkaError::AdapterError(
                format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default())
            ));
        }
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    // ── CLOB API — Public ──

    pub async fn health(&self) -> Result<()> {
        let url = format!("{}/", CLOB_BASE_URL);
        self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_server_time(&self) -> Result<String> {
        let url = format!("{}/time", CLOB_BASE_URL);
        let resp: Value = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?
            .json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(resp.to_string())
    }

    pub async fn get_order_book(&self, token_id: &str) -> Result<OrderBookResponse> {
        let url = format!("{}/book?token_id={}", CLOB_BASE_URL, token_id);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_price(&self, token_id: &str, side: &str) -> Result<String> {
        let url = format!("{}/price?token_id={}&side={}", CLOB_BASE_URL, token_id, side);
        let resp: Value = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?
            .json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(resp["price"].as_str().unwrap_or("0").to_string())
    }

    pub async fn get_midpoint(&self, token_id: &str) -> Result<String> {
        let url = format!("{}/midpoint?token_id={}", CLOB_BASE_URL, token_id);
        let resp: Value = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?
            .json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(resp["mid"].as_str().unwrap_or("0").to_string())
    }

    pub async fn get_spread(&self, token_id: &str) -> Result<String> {
        let url = format!("{}/spread?token_id={}", CLOB_BASE_URL, token_id);
        let resp: Value = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?
            .json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        Ok(resp["spread"].as_str().unwrap_or("0").to_string())
    }

    pub async fn get_price_history(&self, token_id: &str, interval: &str) -> Result<Vec<PricePoint>> {
        let url = format!("{}/prices-history?market={}&interval={}", CLOB_BASE_URL, token_id, interval);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        let data: Value = resp.json().await
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        // The response may be {"history": [...]} or just [...]
        let history = if let Some(arr) = data.get("history") {
            serde_json::from_value(arr.clone())
        } else {
            serde_json::from_value(data)
        };
        history.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_markets_clob(&self, cursor: Option<&str>) -> Result<Value> {
        let mut url = format!("{}/markets", CLOB_BASE_URL);
        if let Some(c) = cursor {
            url = format!("{}?next_cursor={}", url, c);
        }
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    // ── Gamma API — Public ──

    pub async fn get_events(&self, active: bool, closed: bool, limit: u32, offset: u32) -> Result<Vec<GammaEvent>> {
        let url = format!(
            "{}/events?active={}&closed={}&order=volume24hr&ascending=false&limit={}&offset={}",
            GAMMA_BASE_URL, active, closed, limit, offset
        );
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_gamma_markets(&self, active: bool, limit: u32, offset: u32) -> Result<Vec<GammaMarket>> {
        let url = format!(
            "{}/markets?active={}&closed=false&limit={}&offset={}",
            GAMMA_BASE_URL, active, limit, offset
        );
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_event_by_slug(&self, slug: &str) -> Result<Vec<GammaEvent>> {
        let url = format!("{}/events?slug={}", GAMMA_BASE_URL, slug);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    // ── Data API — Public ──

    pub async fn get_user_positions(&self, address: &str) -> Result<Vec<UserPosition>> {
        let url = format!("{}/positions?user={}", DATA_BASE_URL, address);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_user_trades(&self, address: &str, limit: u32) -> Result<Vec<UserTrade>> {
        let url = format!("{}/trades?user={}&limit={}", DATA_BASE_URL, address, limit);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_market_holders(&self, condition_id: &str) -> Result<Value> {
        let url = format!("{}/holders?market={}", DATA_BASE_URL, condition_id);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn get_open_interest(&self, condition_id: &str) -> Result<Value> {
        let url = format!("{}/oi?market={}", DATA_BASE_URL, condition_id);
        let resp = self.http.get(&url).send().await
            .map_err(|e| TradoshkaError::ConnectionError(e.to_string()))?;
        resp.json().await.map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    // ── CLOB API — Authenticated ──

    pub async fn place_order(&self, order: &PolyOrderRequest) -> Result<PolyOrderResponse> {
        let body = serde_json::to_value(order)
            .map_err(|e| TradoshkaError::AdapterError(e.to_string()))?;
        let resp = self.post_authed(CLOB_BASE_URL, "/order", &body).await?;
        serde_json::from_value(resp).map_err(|e| TradoshkaError::AdapterError(e.to_string()))
    }

    pub async fn cancel_order(&self, order_id: &str) -> Result<()> {
        let body = serde_json::json!({"orderID": order_id});
        self.delete_authed(CLOB_BASE_URL, "/order", Some(&body)).await?;
        Ok(())
    }

    pub async fn cancel_all_orders(&self) -> Result<()> {
        self.delete_authed(CLOB_BASE_URL, "/cancel-all", None).await?;
        Ok(())
    }

    pub async fn get_open_orders(&self) -> Result<Value> {
        self.get_authed(CLOB_BASE_URL, "/data/orders").await
    }

    pub async fn get_trade_history(&self) -> Result<Value> {
        self.get_authed(CLOB_BASE_URL, "/data/trades").await
    }

    pub async fn get_balance(&self) -> Result<Value> {
        self.get_authed(CLOB_BASE_URL, "/balance-allowance").await
    }
}
