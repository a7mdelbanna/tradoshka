use thiserror::Error;
use crate::types::OrderStatus;

#[derive(Debug, Error)]
pub enum TradoshkaError {
    #[error("Invalid order state transition: {from:?} → {to:?}")]
    InvalidTransition { from: OrderStatus, to: OrderStatus },
    #[error("Order not found: {0}")]
    OrderNotFound(String),
    #[error("Risk rejected: {0}")]
    RiskRejected(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Market adapter error: {0}")]
    AdapterError(String),
    #[error("Strategy error: {0}")]
    StrategyError(String),
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance {
        required: rust_decimal::Decimal,
        available: rust_decimal::Decimal,
    },
    #[error("Circuit breaker tripped: {0}")]
    CircuitBreakerTripped(String),
}

pub type Result<T> = std::result::Result<T, TradoshkaError>;
