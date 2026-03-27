use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use chrono::Utc;
use serde::Serialize;

pub struct DataLogger {
    base_dir: String,
}

impl DataLogger {
    pub fn new(base_dir: &str) -> Self {
        fs::create_dir_all(format!("{}/trades", base_dir)).ok();
        fs::create_dir_all(format!("{}/evolution", base_dir)).ok();
        fs::create_dir_all(format!("{}/snapshots", base_dir)).ok();
        Self { base_dir: base_dir.into() }
    }

    pub fn log_trade(&self, trade: &impl Serialize) {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let path = format!("{}/trades/{}_trades.jsonl", self.base_dir, date);
        if let Ok(json) = serde_json::to_string(trade) {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
                writeln!(f, "{}", json).ok();
            }
        }
    }

    pub fn log_evolution(&self, event: &impl Serialize) {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let path = format!("{}/evolution/{}_evolution.jsonl", self.base_dir, date);
        if let Ok(json) = serde_json::to_string(event) {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
                writeln!(f, "{}", json).ok();
            }
        }
    }

    pub fn log_snapshot(&self, snapshot: &impl Serialize) {
        let now = Utc::now();
        let path = format!("{}/snapshots/{}_{:02}h_snapshot.json",
            self.base_dir, now.format("%Y-%m-%d"), now.hour());
        if let Ok(json) = serde_json::to_string_pretty(snapshot) {
            fs::write(&path, json).ok();
        }
    }

    pub fn trade_count_today(&self) -> usize {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let path = format!("{}/trades/{}_trades.jsonl", self.base_dir, date);
        if let Ok(content) = fs::read_to_string(&path) {
            content.lines().count()
        } else { 0 }
    }
}

use chrono::Timelike;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_logger_creates_dirs() {
        let dir = "/tmp/tradoshka_test_log";
        let _ = fs::remove_dir_all(dir);
        let _logger = DataLogger::new(dir);
        assert!(Path::new(&format!("{}/trades", dir)).exists());
        assert!(Path::new(&format!("{}/evolution", dir)).exists());
        assert!(Path::new(&format!("{}/snapshots", dir)).exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_log_trade() {
        let dir = "/tmp/tradoshka_test_trade";
        let _ = fs::remove_dir_all(dir);
        let logger = DataLogger::new(dir);
        logger.log_trade(&serde_json::json!({"symbol": "BTC", "pnl": 1.5}));
        assert!(logger.trade_count_today() >= 1);
        let _ = fs::remove_dir_all(dir);
    }
}
