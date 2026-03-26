use base64::{engine::general_purpose::URL_SAFE, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::config::ApiCredentials;

type HmacSha256 = Hmac<Sha256>;

pub struct L2Auth {
    creds: ApiCredentials,
}

impl L2Auth {
    pub fn new(creds: ApiCredentials) -> Self {
        Self { creds }
    }

    pub fn sign(&self, method: &str, path: &str, body: Option<&str>, timestamp: i64) -> String {
        let mut message = format!("{}{}{}", timestamp, method, path);
        if let Some(b) = body {
            message.push_str(b);
        }
        let secret_bytes = URL_SAFE
            .decode(&self.creds.api_secret)
            .expect("Invalid base64 API secret");
        let mut mac = HmacSha256::new_from_slice(&secret_bytes).expect("HMAC key");
        mac.update(message.as_bytes());
        URL_SAFE.encode(mac.finalize().into_bytes())
    }

    pub fn headers(&self, method: &str, path: &str, body: Option<&str>) -> Vec<(String, String)> {
        let timestamp = chrono::Utc::now().timestamp();
        let signature = self.sign(method, path, body, timestamp);
        vec![
            ("POLY_ADDRESS".into(), self.creds.wallet_address.clone()),
            ("POLY_SIGNATURE".into(), signature),
            ("POLY_TIMESTAMP".into(), timestamp.to_string()),
            ("POLY_API_KEY".into(), self.creds.api_key.clone()),
            ("POLY_PASSPHRASE".into(), self.creds.passphrase.clone()),
        ]
    }

    pub fn wallet_address(&self) -> &str {
        &self.creds.wallet_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiCredentials;

    fn make_creds() -> ApiCredentials {
        // base64url-encoded secret: "my-test-secret-key-1234567890ab"
        // URL_SAFE encode of that string
        let raw_secret = "my-test-secret-key-1234567890ab";
        let encoded_secret = URL_SAFE.encode(raw_secret.as_bytes());
        ApiCredentials {
            api_key: "test-api-key".into(),
            api_secret: encoded_secret,
            passphrase: "test-passphrase".into(),
            wallet_address: "0xDeAdBeEf0000000000000000000000000000cafe".into(),
        }
    }

    #[test]
    fn test_sign_deterministic() {
        let auth = L2Auth::new(make_creds());
        let ts = 1_700_000_000_i64;
        let sig1 = auth.sign("GET", "/markets", None, ts);
        let sig2 = auth.sign("GET", "/markets", None, ts);
        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn test_sign_different_with_body() {
        let auth = L2Auth::new(make_creds());
        let ts = 1_700_000_000_i64;
        let sig_no_body = auth.sign("POST", "/order", None, ts);
        let sig_with_body = auth.sign("POST", "/order", Some(r#"{"price":"0.5"}"#), ts);
        assert_ne!(sig_no_body, sig_with_body);
    }

    #[test]
    fn test_sign_different_methods() {
        let auth = L2Auth::new(make_creds());
        let ts = 1_700_000_000_i64;
        let sig_get = auth.sign("GET", "/order", None, ts);
        let sig_post = auth.sign("POST", "/order", None, ts);
        assert_ne!(sig_get, sig_post);
    }

    #[test]
    fn test_sign_different_timestamps() {
        let auth = L2Auth::new(make_creds());
        let sig1 = auth.sign("GET", "/markets", None, 1_700_000_000);
        let sig2 = auth.sign("GET", "/markets", None, 1_700_000_001);
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_headers_contain_all_fields() {
        let auth = L2Auth::new(make_creds());
        let headers = auth.headers("GET", "/markets", None);
        let keys: Vec<&str> = headers.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"POLY_ADDRESS"), "missing POLY_ADDRESS");
        assert!(keys.contains(&"POLY_SIGNATURE"), "missing POLY_SIGNATURE");
        assert!(keys.contains(&"POLY_TIMESTAMP"), "missing POLY_TIMESTAMP");
        assert!(keys.contains(&"POLY_API_KEY"), "missing POLY_API_KEY");
        assert!(keys.contains(&"POLY_PASSPHRASE"), "missing POLY_PASSPHRASE");
        assert_eq!(headers.len(), 5);

        // Verify values are non-empty
        for (k, v) in &headers {
            assert!(!v.is_empty(), "header {k} has empty value");
        }
    }
}
