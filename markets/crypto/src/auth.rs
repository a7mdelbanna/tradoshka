use hmac::{Hmac, Mac};
use sha2::Sha256;
use chrono::Utc;

type HmacSha256 = Hmac<Sha256>;

pub struct BinanceAuth {
    api_key: String,
    secret_key: String,
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

impl BinanceAuth {
    pub fn new(api_key: String, secret_key: String) -> Self {
        Self { api_key, secret_key }
    }

    /// Sign a query string and return `query_string&signature=hex`.
    pub fn sign_query(&self, query_string: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC key");
        mac.update(query_string.as_bytes());
        let signature = hex_encode(&mac.finalize().into_bytes());
        format!("{}&signature={}", query_string, signature)
    }

    /// Build a signed query string with timestamp injected.
    pub fn sign_params(&self, params: &[(&str, &str)]) -> String {
        let timestamp = Utc::now().timestamp_millis().to_string();
        let mut parts: Vec<String> = params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        parts.push(format!("timestamp={}", timestamp));
        let query = parts.join("&");
        self.sign_query(&query)
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auth() -> BinanceAuth {
        BinanceAuth::new("test-api-key".into(), "test-secret-key".into())
    }

    #[test]
    fn test_sign_deterministic() {
        let auth = make_auth();
        let query = "symbol=BTCUSDT&side=BUY&type=MARKET&quantity=0.001&timestamp=1700000000000";
        let sig1 = auth.sign_query(query);
        let sig2 = auth.sign_query(query);
        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn test_sign_includes_timestamp() {
        let auth = make_auth();
        let result = auth.sign_params(&[("symbol", "BTCUSDT")]);
        assert!(result.contains("timestamp="), "result should contain timestamp=");
    }

    #[test]
    fn test_sign_includes_signature() {
        let auth = make_auth();
        let result = auth.sign_params(&[("symbol", "BTCUSDT")]);
        assert!(result.contains("&signature="), "result should contain &signature=");
    }

    #[test]
    fn test_different_params_different_signatures() {
        let auth = make_auth();
        // We use sign_query with fixed inputs to get deterministic results
        let query1 = "symbol=BTCUSDT&timestamp=1700000000000";
        let query2 = "symbol=ETHUSDT&timestamp=1700000000000";
        let sig1 = auth.sign_query(query1);
        let sig2 = auth.sign_query(query2);
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_api_key_accessor() {
        let auth = make_auth();
        assert_eq!(auth.api_key(), "test-api-key");
    }
}
