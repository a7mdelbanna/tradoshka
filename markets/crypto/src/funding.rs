use crate::types::FundingRateInfo;

pub struct FundingMonitor;

impl FundingMonitor {
    /// Find symbols with high absolute funding rates (arbitrage opportunities).
    pub fn find_opportunities(rates: &[FundingRateInfo], min_rate: f64) -> Vec<&FundingRateInfo> {
        rates.iter()
            .filter(|r| {
                r.last_funding_rate.parse::<f64>().unwrap_or(0.0).abs() >= min_rate
            })
            .collect()
    }

    /// Calculate annualized funding rate.
    pub fn annualized_rate(funding_rate: f64) -> f64 {
        funding_rate * 3.0 * 365.0  // 3 settlements per day * 365 days
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rate(symbol: &str, rate: &str) -> FundingRateInfo {
        FundingRateInfo {
            symbol: symbol.into(),
            mark_price: "35000.00".into(),
            last_funding_rate: rate.into(),
            next_funding_time: 0,
        }
    }

    #[test]
    fn test_find_high_funding() {
        let rates = vec![
            make_rate("BTCUSDT", "0.0001"),   // 0.01% — low
            make_rate("ETHUSDT", "0.001"),     // 0.1% — high
            make_rate("SOLUSDT", "-0.0005"),   // -0.05% — moderate
        ];
        let opps = FundingMonitor::find_opportunities(&rates, 0.0003);
        assert_eq!(opps.len(), 2); // ETH and SOL
    }

    #[test]
    fn test_annualized_rate() {
        let annual = FundingMonitor::annualized_rate(0.0001);
        // 0.01% * 3 * 365 = 10.95%
        assert!((annual - 0.1095).abs() < 0.001);
    }
}
