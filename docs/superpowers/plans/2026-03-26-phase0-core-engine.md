# Phase 0: Core Engine + Infrastructure — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust core engine (types, traits, risk management, order engine, data layer, API) and Python strategy base, plus all development infrastructure (CLAUDE.md, hooks, CI).

**Architecture:** Cargo workspace with 5 crates (common, data, risk, engine, api) communicating through shared traits. Python strategies connect via PyO3 bridge. Axum serves REST + WebSocket API for the dashboard.

**Tech Stack:** Rust (stable), Python 3.13, PyO3, Axum, Tokio, rust_decimal, chrono, serde, uuid

---

## Prerequisites

Before starting any task, ensure these are installed:

```bash
# Install Rust (if not present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup default stable

# Verify
rustc --version   # Expected: rustc 1.8x.x
cargo --version   # Expected: cargo 1.8x.x

# Python should already be available (3.13.1)
python --version  # Expected: Python 3.13.1

# Install maturin for PyO3 builds
pip install maturin
```

---

## File Structure

```
tradoshka/
├── Cargo.toml                          # Workspace root
├── CLAUDE.md                           # Master Claude rules
├── .gitignore
├── core/
│   ├── common/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Re-exports
│   │       ├── types.rs                # Order, Position, Signal, MarketEvent, etc.
│   │       ├── traits.rs               # MarketAdapter, RiskManager, Strategy
│   │       └── error.rs               # Error types
│   ├── data/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ring_buffer.rs          # Generic lock-free ring buffer
│   │       └── candle_aggregator.rs    # Trade → Candle conversion
│   ├── risk/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── position_sizer.rs       # Half-Kelly position sizing
│   │       ├── circuit_breaker.rs      # Drawdown-based circuit breaker
│   │       ├── drawdown_tracker.rs     # Real-time drawdown tracking
│   │       └── manager.rs             # Combined RiskManager implementation
│   ├── engine/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── order_manager.rs        # Order lifecycle state machine
│   │       ├── portfolio.rs            # Portfolio tracking, P&L calculation
│   │       └── dry_mode.rs            # Simulated execution engine
│   └── api/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── server.rs               # Axum server setup
│           ├── routes.rs              # REST endpoint handlers
│           ├── ws.rs                  # WebSocket handler
│           └── state.rs              # Shared application state
├── bridge/
│   ├── Cargo.toml                      # PyO3 crate
│   └── src/
│       └── lib.rs                     # Python ↔ Rust bridge
├── strategies/
│   └── shared/
│       ├── pyproject.toml
│       └── tradoshka_strategy/
│           ├── __init__.py
│           ├── base.py                 # Base Strategy class
│           ├── signal.py               # Signal dataclass
│           └── indicators.py           # Common technical indicators
├── docs/
│   ├── MISTAKES.md
│   ├── RESEARCH.md
│   ├── BENCHMARKS.md
│   └── GOALS.md
├── core/CLAUDE.md
├── strategies/CLAUDE.md
└── infra/
    ├── docker/
    │   └── Dockerfile
    └── ci/
        └── test.yml                   # GitHub Actions CI
```

---

### Task 1: Workspace Setup + Git Structure

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `core/common/Cargo.toml`
- Create: `core/data/Cargo.toml`
- Create: `core/risk/Cargo.toml`
- Create: `core/engine/Cargo.toml`
- Create: `core/api/Cargo.toml`
- Create: `bridge/Cargo.toml`
- Create: `.gitignore`

- [ ] **Step 1: Create branch**

```bash
git checkout -b feature/core-engine
```

- [ ] **Step 2: Create workspace Cargo.toml**

Create `Cargo.toml` at project root:

```toml
[workspace]
resolver = "2"
members = [
    "core/common",
    "core/data",
    "core/risk",
    "core/engine",
    "core/api",
    "bridge",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "AGPL-3.0"
repository = "https://github.com/a7mdelbanna/tradoshka"

[workspace.dependencies]
tokio = { version = "1.43", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rust_decimal = { version = "1.36", features = ["serde-with-str"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.11", features = ["v4", "serde"] }
thiserror = "2.0"
anyhow = "1.0"
async-trait = "0.1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 3: Create core/common/Cargo.toml**

```toml
[package]
name = "tradoshka-common"
version.workspace = true
edition.workspace = true

[dependencies]
rust_decimal = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
tokio = { workspace = true }
```

- [ ] **Step 4: Create core/data/Cargo.toml**

```toml
[package]
name = "tradoshka-data"
version.workspace = true
edition.workspace = true

[dependencies]
tradoshka-common = { path = "../common" }
rust_decimal = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 5: Create core/risk/Cargo.toml**

```toml
[package]
name = "tradoshka-risk"
version.workspace = true
edition.workspace = true

[dependencies]
tradoshka-common = { path = "../common" }
rust_decimal = { workspace = true }
rust_decimal_macros = "1.36"
chrono = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 6: Create core/engine/Cargo.toml**

```toml
[package]
name = "tradoshka-engine"
version.workspace = true
edition.workspace = true

[dependencies]
tradoshka-common = { path = "../common" }
tradoshka-risk = { path = "../risk" }
tradoshka-data = { path = "../data" }
rust_decimal = { workspace = true }
rust_decimal_macros = "1.36"
chrono = { workspace = true }
uuid = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
```

- [ ] **Step 7: Create core/api/Cargo.toml**

```toml
[package]
name = "tradoshka-api"
version.workspace = true
edition.workspace = true

[dependencies]
tradoshka-common = { path = "../common" }
tradoshka-engine = { path = "../engine" }
tradoshka-risk = { path = "../risk" }
tradoshka-data = { path = "../data" }
axum = { version = "0.8", features = ["ws"] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
uuid = { workspace = true }
```

- [ ] **Step 8: Create bridge/Cargo.toml**

```toml
[package]
name = "tradoshka-bridge"
version.workspace = true
edition.workspace = true

[lib]
name = "tradoshka_bridge"
crate-type = ["cdylib"]

[dependencies]
tradoshka-common = { path = "../core/common" }
pyo3 = { version = "0.23", features = ["extension-module"] }
rust_decimal = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
```

- [ ] **Step 9: Create .gitignore**

```gitignore
# Rust
target/
**/*.rs.bk
Cargo.lock

# Python
__pycache__/
*.py[cod]
*$py.class
*.so
*.egg-info/
dist/
build/
.venv/
venv/

# Node / Dashboard
node_modules/
.next/
out/

# IDE
.idea/
.vscode/
*.swp
*.swo

# Environment
.env
.env.local
*.pem
*.key

# OS
.DS_Store
Thumbs.db

# Trading specific - NEVER commit
api_keys.json
credentials.json
secrets/
```

- [ ] **Step 10: Create stub lib.rs files for all crates**

Create `core/common/src/lib.rs`:
```rust
pub mod types;
pub mod traits;
pub mod error;
```

Create `core/data/src/lib.rs`:
```rust
pub mod ring_buffer;
pub mod candle_aggregator;
```

Create `core/risk/src/lib.rs`:
```rust
pub mod position_sizer;
pub mod circuit_breaker;
pub mod drawdown_tracker;
pub mod manager;
```

Create `core/engine/src/lib.rs`:
```rust
pub mod order_manager;
pub mod portfolio;
pub mod dry_mode;
```

Create `core/api/src/lib.rs`:
```rust
pub mod server;
pub mod routes;
pub mod ws;
pub mod state;
```

Create `bridge/src/lib.rs`:
```rust
use pyo3::prelude::*;

#[pymodule]
fn tradoshka_bridge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}
```

Create empty stub files for each module (just `// TODO: implement` placeholder to compile):
- `core/common/src/types.rs` → empty
- `core/common/src/traits.rs` → empty
- `core/common/src/error.rs` → empty
- `core/data/src/ring_buffer.rs` → empty
- `core/data/src/candle_aggregator.rs` → empty
- `core/risk/src/position_sizer.rs` → empty
- `core/risk/src/circuit_breaker.rs` → empty
- `core/risk/src/drawdown_tracker.rs` → empty
- `core/risk/src/manager.rs` → empty
- `core/engine/src/order_manager.rs` → empty
- `core/engine/src/portfolio.rs` → empty
- `core/engine/src/dry_mode.rs` → empty
- `core/api/src/server.rs` → empty
- `core/api/src/routes.rs` → empty
- `core/api/src/ws.rs` → empty
- `core/api/src/state.rs` → empty

- [ ] **Step 11: Verify workspace compiles**

```bash
cargo check
```

Expected: compiles with no errors (may have warnings about unused modules)

- [ ] **Step 12: Commit**

```bash
git add -A
git commit -m "feat: initialize Cargo workspace with 6 crates

Sets up monorepo structure: common, data, risk, engine, api, bridge.
All crates compile as empty stubs."
```

---

### Task 2: Common Types

**Files:**
- Create: `core/common/src/types.rs`
- Create: `core/common/src/error.rs`

- [ ] **Step 1: Write tests for core types**

Create `core/common/src/types.rs` with types and inline tests:

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── Core IDs ──

pub type OrderId = Uuid;
pub type TradeId = Uuid;
pub type StrategyId = String;
pub type Symbol = String;

// ── Market ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Market {
    Polymarket,
    Crypto,
    Forex,
    Stocks,
}

// ── Orders ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit { price: Decimal },
    StopLoss { trigger: Decimal },
    TrailingStop { offset_pct: Decimal },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Submitted,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

impl OrderStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Filled | Self::Cancelled | Self::Rejected)
    }

    pub fn can_transition_to(&self, next: &OrderStatus) -> bool {
        match (self, next) {
            (Self::Pending, Self::Submitted) => true,
            (Self::Pending, Self::Rejected) => true,
            (Self::Submitted, Self::PartiallyFilled) => true,
            (Self::Submitted, Self::Filled) => true,
            (Self::Submitted, Self::Cancelled) => true,
            (Self::Submitted, Self::Rejected) => true,
            (Self::PartiallyFilled, Self::Filled) => true,
            (Self::PartiallyFilled, Self::Cancelled) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub symbol: Symbol,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub strategy_id: StrategyId,
    pub market: Market,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Order {
    pub fn new(
        symbol: Symbol,
        side: OrderSide,
        order_type: OrderType,
        quantity: Decimal,
        strategy_id: StrategyId,
        market: Market,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            symbol,
            side,
            order_type,
            quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::Pending,
            strategy_id,
            market,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn remaining_quantity(&self) -> Decimal {
        self.quantity - self.filled_quantity
    }
}

// ── Market Data ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub symbol: Symbol,
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub symbol: Symbol,
    pub price: Decimal,
    pub quantity: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: Decimal,
    pub quantity: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketEvent {
    TradeEvent(Trade),
    OrderBookUpdate {
        symbol: Symbol,
        bids: Vec<OrderBookLevel>,
        asks: Vec<OrderBookLevel>,
        timestamp: DateTime<Utc>,
    },
    CandleEvent(Candle),
}

impl MarketEvent {
    pub fn symbol(&self) -> &str {
        match self {
            Self::TradeEvent(t) => &t.symbol,
            Self::OrderBookUpdate { symbol, .. } => symbol,
            Self::CandleEvent(c) => &c.symbol,
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::TradeEvent(t) => t.timestamp,
            Self::OrderBookUpdate { timestamp, .. } => *timestamp,
            Self::CandleEvent(c) => c.timestamp,
        }
    }
}

// ── Signals ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalDirection {
    Long,
    Short,
    Close,
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub strategy_id: StrategyId,
    pub symbol: Symbol,
    pub direction: SignalDirection,
    pub strength: f64,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl Signal {
    pub fn new(
        strategy_id: StrategyId,
        symbol: Symbol,
        direction: SignalDirection,
        strength: f64,
    ) -> Self {
        Self {
            strategy_id,
            symbol,
            direction,
            strength: strength.clamp(0.0, 1.0),
            stop_loss: None,
            take_profit: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

// ── Risk ──

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskDecision {
    Approved { adjusted_quantity: Decimal },
    Rejected { reason: String },
}

// ── Portfolio ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: Symbol,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub strategy_id: StrategyId,
    pub market: Market,
    pub opened_at: DateTime<Utc>,
}

impl Position {
    pub fn update_price(&mut self, price: Decimal) {
        self.current_price = price;
        let diff = price - self.entry_price;
        self.unrealized_pnl = match self.side {
            OrderSide::Buy => diff * self.quantity,
            OrderSide::Sell => -diff * self.quantity,
        };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub balance: Decimal,
    pub equity: Decimal,
    pub positions: Vec<Position>,
    pub daily_pnl: Decimal,
    pub peak_equity: Decimal,
    pub current_drawdown_pct: Decimal,
}

impl Portfolio {
    pub fn new(initial_balance: Decimal) -> Self {
        Self {
            balance: initial_balance,
            equity: initial_balance,
            positions: Vec::new(),
            daily_pnl: Decimal::ZERO,
            peak_equity: initial_balance,
            current_drawdown_pct: Decimal::ZERO,
        }
    }

    pub fn update_equity(&mut self) {
        let unrealized: Decimal = self.positions.iter().map(|p| p.unrealized_pnl).sum();
        self.equity = self.balance + unrealized;
        if self.equity > self.peak_equity {
            self.peak_equity = self.equity;
        }
        if self.peak_equity > Decimal::ZERO {
            self.current_drawdown_pct =
                (self.peak_equity - self.equity) / self.peak_equity;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balances {
    pub total: Decimal,
    pub available: Decimal,
    pub in_positions: Decimal,
}

// ── Fills ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub order_id: OrderId,
    pub trade_id: TradeId,
    pub symbol: Symbol,
    pub side: OrderSide,
    pub price: Decimal,
    pub quantity: Decimal,
    pub fee: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_order_status_terminal() {
        assert!(!OrderStatus::Pending.is_terminal());
        assert!(!OrderStatus::Submitted.is_terminal());
        assert!(!OrderStatus::PartiallyFilled.is_terminal());
        assert!(OrderStatus::Filled.is_terminal());
        assert!(OrderStatus::Cancelled.is_terminal());
        assert!(OrderStatus::Rejected.is_terminal());
    }

    #[test]
    fn test_order_status_valid_transitions() {
        assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Submitted));
        assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Rejected));
        assert!(OrderStatus::Submitted.can_transition_to(&OrderStatus::Filled));
        assert!(OrderStatus::Submitted.can_transition_to(&OrderStatus::Cancelled));
        assert!(OrderStatus::PartiallyFilled.can_transition_to(&OrderStatus::Filled));
    }

    #[test]
    fn test_order_status_invalid_transitions() {
        assert!(!OrderStatus::Pending.can_transition_to(&OrderStatus::Filled));
        assert!(!OrderStatus::Filled.can_transition_to(&OrderStatus::Pending));
        assert!(!OrderStatus::Cancelled.can_transition_to(&OrderStatus::Submitted));
    }

    #[test]
    fn test_order_new() {
        let order = Order::new(
            "BTC/USD".into(),
            OrderSide::Buy,
            OrderType::Market,
            dec!(1.5),
            "test_strategy".into(),
            Market::Crypto,
        );
        assert_eq!(order.status, OrderStatus::Pending);
        assert_eq!(order.filled_quantity, Decimal::ZERO);
        assert_eq!(order.remaining_quantity(), dec!(1.5));
    }

    #[test]
    fn test_signal_clamps_strength() {
        let signal = Signal::new("s1".into(), "BTC".into(), SignalDirection::Long, 1.5);
        assert_eq!(signal.strength, 1.0);

        let signal = Signal::new("s1".into(), "BTC".into(), SignalDirection::Short, -0.5);
        assert_eq!(signal.strength, 0.0);
    }

    #[test]
    fn test_position_update_price_long() {
        let mut pos = Position {
            symbol: "BTC/USD".into(),
            side: OrderSide::Buy,
            quantity: dec!(2),
            entry_price: dec!(50000),
            current_price: dec!(50000),
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            strategy_id: "test".into(),
            market: Market::Crypto,
            opened_at: Utc::now(),
        };
        pos.update_price(dec!(55000));
        assert_eq!(pos.unrealized_pnl, dec!(10000)); // (55000-50000)*2
    }

    #[test]
    fn test_position_update_price_short() {
        let mut pos = Position {
            symbol: "BTC/USD".into(),
            side: OrderSide::Sell,
            quantity: dec!(2),
            entry_price: dec!(50000),
            current_price: dec!(50000),
            unrealized_pnl: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            strategy_id: "test".into(),
            market: Market::Crypto,
            opened_at: Utc::now(),
        };
        pos.update_price(dec!(45000));
        assert_eq!(pos.unrealized_pnl, dec!(10000)); // -(45000-50000)*2
    }

    #[test]
    fn test_portfolio_drawdown() {
        let mut portfolio = Portfolio::new(dec!(10000));
        assert_eq!(portfolio.current_drawdown_pct, Decimal::ZERO);

        // Simulate profit then loss
        portfolio.balance = dec!(12000);
        portfolio.update_equity();
        assert_eq!(portfolio.peak_equity, dec!(12000));

        portfolio.balance = dec!(10800);
        portfolio.update_equity();
        assert_eq!(portfolio.current_drawdown_pct, dec!(0.1)); // 10% drawdown
    }

    #[test]
    fn test_market_event_symbol() {
        let trade = MarketEvent::TradeEvent(Trade {
            symbol: "ETH/USD".into(),
            price: dec!(3000),
            quantity: dec!(1),
            timestamp: Utc::now(),
        });
        assert_eq!(trade.symbol(), "ETH/USD");
    }
}
```

- [ ] **Step 2: Write error types**

Create `core/common/src/error.rs`:

```rust
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
```

- [ ] **Step 3: Add rust_decimal_macros to common Cargo.toml for tests**

Add to `core/common/Cargo.toml` under `[dev-dependencies]`:

```toml
[dev-dependencies]
rust_decimal_macros = "1.36"
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p tradoshka-common
```

Expected: all 8 tests pass

- [ ] **Step 5: Commit**

```bash
git add core/common/
git commit -m "feat(common): add core types, error types, and tests

Order lifecycle with state machine, Position with P&L tracking,
Signal with strength clamping, Portfolio with drawdown calculation,
MarketEvent enum for unified market data."
```

---

### Task 3: Common Traits

**Files:**
- Create: `core/common/src/traits.rs`

- [ ] **Step 1: Define all core traits**

```rust
use async_trait::async_trait;
use crate::types::*;
use crate::error::Result;

/// Adapter for connecting to a specific market/exchange.
/// Each market (Polymarket, Binance, OANDA, Alpaca) implements this.
#[async_trait]
pub trait MarketAdapter: Send + Sync {
    /// Unique name of this adapter (e.g., "binance", "polymarket")
    fn name(&self) -> &str;

    /// Which market this adapter serves
    fn market(&self) -> Market;

    /// Connect to the exchange/broker
    async fn connect(&mut self) -> Result<()>;

    /// Disconnect cleanly
    async fn disconnect(&mut self) -> Result<()>;

    /// Place an order
    async fn place_order(&self, order: &Order) -> Result<OrderId>;

    /// Cancel an existing order
    async fn cancel_order(&self, id: &OrderId) -> Result<()>;

    /// Get all open positions
    async fn get_positions(&self) -> Result<Vec<Position>>;

    /// Get account balances
    async fn get_balances(&self) -> Result<Balances>;
}

/// Risk management — validates orders and calculates position sizes.
pub trait RiskManager: Send + Sync {
    /// Validate whether an order should be placed given current portfolio state
    fn validate_order(&self, order: &Order, portfolio: &Portfolio) -> RiskDecision;

    /// Check if any circuit breakers have been triggered
    fn check_circuit_breakers(&self, portfolio: &Portfolio) -> bool;

    /// Calculate appropriate position size for a signal
    fn calculate_position_size(&self, signal: &Signal, portfolio: &Portfolio) -> rust_decimal::Decimal;
}

/// Strategy — receives market data, produces trading signals.
pub trait Strategy: Send + Sync {
    /// Unique name of this strategy
    fn name(&self) -> &str;

    /// Which market this strategy operates on
    fn market(&self) -> Market;

    /// Process a market event and optionally produce a signal
    fn on_market_event(&mut self, event: &MarketEvent) -> Option<Signal>;

    /// Notify strategy that an order was filled
    fn on_fill(&mut self, fill: &Fill);
}
```

- [ ] **Step 2: Update lib.rs re-exports**

Update `core/common/src/lib.rs`:

```rust
pub mod types;
pub mod traits;
pub mod error;

// Re-export commonly used items
pub use types::*;
pub use traits::*;
pub use error::{TradoshkaError, Result};
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p tradoshka-common
```

Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add core/common/
git commit -m "feat(common): add MarketAdapter, RiskManager, Strategy traits

Async MarketAdapter for exchange connectivity, RiskManager for order
validation and position sizing, Strategy for signal generation."
```

---

### Task 4: Ring Buffer

**Files:**
- Create: `core/data/src/ring_buffer.rs`

- [ ] **Step 1: Implement ring buffer with tests**

```rust
/// Fixed-capacity circular buffer optimized for time-series data.
/// When full, oldest items are overwritten. O(1) push, O(1) latest.
#[derive(Debug)]
pub struct RingBuffer<T> {
    buffer: Vec<Option<T>>,
    capacity: usize,
    head: usize,
    len: usize,
}

impl<T: Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "RingBuffer capacity must be > 0");
        Self {
            buffer: (0..capacity).map(|_| None).collect(),
            capacity,
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        self.buffer[self.head] = Some(item);
        self.head = (self.head + 1) % self.capacity;
        if self.len < self.capacity {
            self.len += 1;
        }
    }

    /// Returns the most recently pushed item
    pub fn latest(&self) -> Option<&T> {
        if self.len == 0 {
            return None;
        }
        let idx = if self.head == 0 {
            self.capacity - 1
        } else {
            self.head - 1
        };
        self.buffer[idx].as_ref()
    }

    /// Returns items from oldest to newest
    pub fn iter(&self) -> RingBufferIter<'_, T> {
        let start = if self.len < self.capacity {
            0
        } else {
            self.head
        };
        RingBufferIter {
            buffer: &self.buffer,
            capacity: self.capacity,
            pos: start,
            remaining: self.len,
        }
    }

    /// Returns items from newest to oldest
    pub fn iter_rev(&self) -> impl Iterator<Item = &T> {
        let items: Vec<&T> = self.iter().collect();
        items.into_iter().rev().collect::<Vec<_>>().into_iter()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn clear(&mut self) {
        self.buffer.iter_mut().for_each(|slot| *slot = None);
        self.head = 0;
        self.len = 0;
    }
}

pub struct RingBufferIter<'a, T> {
    buffer: &'a [Option<T>],
    capacity: usize,
    pos: usize,
    remaining: usize,
}

impl<'a, T> Iterator for RingBufferIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let item = self.buffer[self.pos].as_ref();
        self.pos = (self.pos + 1) % self.capacity;
        self.remaining -= 1;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer_is_empty() {
        let buf: RingBuffer<i32> = RingBuffer::new(5);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert!(!buf.is_full());
        assert_eq!(buf.latest(), None);
    }

    #[test]
    fn test_push_and_latest() {
        let mut buf = RingBuffer::new(3);
        buf.push(10);
        assert_eq!(buf.latest(), Some(&10));
        assert_eq!(buf.len(), 1);

        buf.push(20);
        assert_eq!(buf.latest(), Some(&20));
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_overwrites_when_full() {
        let mut buf = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        assert!(buf.is_full());

        buf.push(4); // overwrites 1
        assert_eq!(buf.len(), 3);
        let items: Vec<&i32> = buf.iter().collect();
        assert_eq!(items, vec![&2, &3, &4]);
    }

    #[test]
    fn test_iter_ordering() {
        let mut buf = RingBuffer::new(5);
        for i in 1..=5 {
            buf.push(i);
        }
        let items: Vec<&i32> = buf.iter().collect();
        assert_eq!(items, vec![&1, &2, &3, &4, &5]);
    }

    #[test]
    fn test_iter_after_wrap() {
        let mut buf = RingBuffer::new(3);
        for i in 1..=7 {
            buf.push(i);
        }
        let items: Vec<&i32> = buf.iter().collect();
        assert_eq!(items, vec![&5, &6, &7]);
    }

    #[test]
    fn test_clear() {
        let mut buf = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.latest(), None);
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn test_zero_capacity_panics() {
        let _: RingBuffer<i32> = RingBuffer::new(0);
    }

    #[test]
    fn test_single_capacity() {
        let mut buf = RingBuffer::new(1);
        buf.push(42);
        assert_eq!(buf.latest(), Some(&42));
        assert!(buf.is_full());

        buf.push(99);
        assert_eq!(buf.latest(), Some(&99));
        assert_eq!(buf.len(), 1);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p tradoshka-data ring_buffer
```

Expected: all 8 tests pass

- [ ] **Step 3: Commit**

```bash
git add core/data/
git commit -m "feat(data): add high-performance ring buffer

O(1) push, O(1) latest access. Fixed capacity with oldest-overwrite
semantics. Includes ordered iteration and reverse iteration."
```

---

### Task 5: Candle Aggregator

**Files:**
- Create: `core/data/src/candle_aggregator.rs`

- [ ] **Step 1: Implement candle aggregator with tests**

```rust
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use tradoshka_common::types::{Candle, Symbol};

/// Aggregates individual trades into OHLCV candles at a specified interval.
pub struct CandleAggregator {
    interval_secs: i64,
    current: Option<CandleBuilder>,
}

struct CandleBuilder {
    symbol: Symbol,
    interval_start: DateTime<Utc>,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: Decimal,
}

impl CandleBuilder {
    fn new(symbol: Symbol, timestamp: DateTime<Utc>, interval_secs: i64, price: Decimal, volume: Decimal) -> Self {
        let interval_start = align_to_interval(timestamp, interval_secs);
        Self {
            symbol,
            interval_start,
            open: price,
            high: price,
            low: price,
            close: price,
            volume,
        }
    }

    fn update(&mut self, price: Decimal, volume: Decimal) {
        if price > self.high {
            self.high = price;
        }
        if price < self.low {
            self.low = price;
        }
        self.close = price;
        self.volume += volume;
    }

    fn to_candle(&self) -> Candle {
        Candle {
            symbol: self.symbol.clone(),
            timestamp: self.interval_start,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
        }
    }
}

fn align_to_interval(timestamp: DateTime<Utc>, interval_secs: i64) -> DateTime<Utc> {
    let ts = timestamp.timestamp();
    let aligned = ts - (ts % interval_secs);
    DateTime::from_timestamp(aligned, 0).unwrap()
}

impl CandleAggregator {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval_secs: interval.num_seconds(),
            current: None,
        }
    }

    /// Process a trade. Returns a completed candle if the interval rolled over.
    pub fn process_trade(
        &mut self,
        symbol: &Symbol,
        price: Decimal,
        quantity: Decimal,
        timestamp: DateTime<Utc>,
    ) -> Option<Candle> {
        let trade_interval = align_to_interval(timestamp, self.interval_secs);

        match &mut self.current {
            Some(builder) if builder.interval_start == trade_interval => {
                builder.update(price, quantity);
                None
            }
            Some(builder) => {
                let completed = builder.to_candle();
                self.current = Some(CandleBuilder::new(
                    symbol.clone(),
                    timestamp,
                    self.interval_secs,
                    price,
                    quantity,
                ));
                Some(completed)
            }
            None => {
                self.current = Some(CandleBuilder::new(
                    symbol.clone(),
                    timestamp,
                    self.interval_secs,
                    price,
                    quantity,
                ));
                None
            }
        }
    }

    /// Get the current in-progress candle (not yet completed)
    pub fn current_candle(&self) -> Option<Candle> {
        self.current.as_ref().map(|b| b.to_candle())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn ts(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    #[test]
    fn test_first_trade_starts_candle() {
        let mut agg = CandleAggregator::new(Duration::seconds(60));
        let result = agg.process_trade(&"BTC".into(), dec!(50000), dec!(1), ts(1000));
        assert!(result.is_none()); // No completed candle yet
        let current = agg.current_candle().unwrap();
        assert_eq!(current.open, dec!(50000));
    }

    #[test]
    fn test_trades_within_interval_update_candle() {
        let mut agg = CandleAggregator::new(Duration::seconds(60));
        agg.process_trade(&"BTC".into(), dec!(100), dec!(1), ts(1000));
        agg.process_trade(&"BTC".into(), dec!(110), dec!(2), ts(1030));
        agg.process_trade(&"BTC".into(), dec!(95), dec!(3), ts(1059));

        let current = agg.current_candle().unwrap();
        assert_eq!(current.open, dec!(100));
        assert_eq!(current.high, dec!(110));
        assert_eq!(current.low, dec!(95));
        assert_eq!(current.close, dec!(95));
        assert_eq!(current.volume, dec!(6));
    }

    #[test]
    fn test_new_interval_completes_candle() {
        let mut agg = CandleAggregator::new(Duration::seconds(60));
        agg.process_trade(&"BTC".into(), dec!(100), dec!(1), ts(1000));
        agg.process_trade(&"BTC".into(), dec!(110), dec!(2), ts(1030));

        // Trade in next interval
        let completed = agg.process_trade(&"BTC".into(), dec!(120), dec!(1), ts(1060));
        assert!(completed.is_some());

        let candle = completed.unwrap();
        assert_eq!(candle.open, dec!(100));
        assert_eq!(candle.high, dec!(110));
        assert_eq!(candle.close, dec!(110));
        assert_eq!(candle.volume, dec!(3));
        assert_eq!(candle.timestamp, ts(960)); // aligned to 60s boundary

        // New candle started
        let current = agg.current_candle().unwrap();
        assert_eq!(current.open, dec!(120));
    }

    #[test]
    fn test_interval_alignment() {
        let aligned = align_to_interval(ts(1025), 60);
        assert_eq!(aligned, ts(960)); // 1025 - (1025 % 60) = 960

        let aligned = align_to_interval(ts(3600), 3600);
        assert_eq!(aligned, ts(3600)); // exactly on boundary
    }
}
```

- [ ] **Step 2: Add dev-dependency for data crate**

Add to `core/data/Cargo.toml`:

```toml
[dev-dependencies]
rust_decimal_macros = "1.36"
```

- [ ] **Step 3: Update data lib.rs**

`core/data/src/lib.rs` should already have:
```rust
pub mod ring_buffer;
pub mod candle_aggregator;
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p tradoshka-data
```

Expected: all 12 tests pass (8 ring buffer + 4 candle aggregator)

- [ ] **Step 5: Commit**

```bash
git add core/data/
git commit -m "feat(data): add candle aggregator for OHLCV construction

Aggregates trades into candles at configurable intervals. Handles
interval alignment, OHLCV tracking, and candle completion on rollover."
```

---

### Task 6: Half-Kelly Position Sizer

**Files:**
- Create: `core/risk/src/position_sizer.rs`

- [ ] **Step 1: Implement with tests**

```rust
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use tradoshka_common::types::Portfolio;

/// Half-Kelly position sizer — captures ~75% of growth with much less drawdown
/// than full Kelly. Industry standard for professional trading.
///
/// Kelly% = (W × R - L) / R
/// Half Kelly = Kelly% / 2
/// Position = min(Half Kelly × equity, max_risk_pct × equity)
pub struct HalfKellySizer {
    /// Maximum risk per trade as fraction (e.g., 0.02 = 2%)
    max_risk_pct: Decimal,
    /// Maximum single position as fraction of equity (e.g., 0.10 = 10%)
    max_position_pct: Decimal,
}

impl HalfKellySizer {
    pub fn new(max_risk_pct: Decimal, max_position_pct: Decimal) -> Self {
        Self {
            max_risk_pct,
            max_position_pct,
        }
    }

    /// Calculate position size in quote currency.
    ///
    /// - `win_rate`: historical win probability (0.0 to 1.0)
    /// - `avg_win_loss_ratio`: average win / average loss
    /// - `portfolio`: current portfolio state
    /// - `entry_price`: intended entry price
    /// - `stop_price`: stop loss price
    pub fn calculate(
        &self,
        win_rate: f64,
        avg_win_loss_ratio: f64,
        portfolio: &Portfolio,
        entry_price: Decimal,
        stop_price: Decimal,
    ) -> Decimal {
        if portfolio.equity <= Decimal::ZERO || entry_price <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        // Kelly criterion: (W × R - L) / R
        let w = win_rate;
        let r = avg_win_loss_ratio;
        let l = 1.0 - w;
        let kelly = if r > 0.0 { (w * r - l) / r } else { 0.0 };

        // Half Kelly for safety
        let half_kelly = (kelly / 2.0).max(0.0);
        let half_kelly_dec = Decimal::from_f64(half_kelly).unwrap_or(Decimal::ZERO);

        // Kelly-based risk amount
        let kelly_risk = half_kelly_dec * portfolio.equity;

        // Max risk per trade
        let max_risk = self.max_risk_pct * portfolio.equity;

        // Use the smaller of Kelly or max risk
        let risk_amount = kelly_risk.min(max_risk);

        // Convert risk amount to position size based on stop distance
        let stop_distance = (entry_price - stop_price).abs();
        let position_value = if stop_distance > Decimal::ZERO {
            risk_amount / stop_distance * entry_price
        } else {
            risk_amount
        };

        // Cap at max position
        let max_position_value = self.max_position_pct * portfolio.equity;
        position_value.min(max_position_value).max(Decimal::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn test_portfolio(equity: Decimal) -> Portfolio {
        Portfolio::new(equity)
    }

    #[test]
    fn test_positive_kelly_returns_position() {
        let sizer = HalfKellySizer::new(dec!(0.02), dec!(0.10));
        let portfolio = test_portfolio(dec!(10000));
        let size = sizer.calculate(0.6, 1.5, &portfolio, dec!(100), dec!(95));
        assert!(size > Decimal::ZERO);
    }

    #[test]
    fn test_negative_kelly_returns_zero() {
        // Win rate too low for the reward/risk ratio
        let sizer = HalfKellySizer::new(dec!(0.02), dec!(0.10));
        let portfolio = test_portfolio(dec!(10000));
        let size = sizer.calculate(0.2, 0.5, &portfolio, dec!(100), dec!(95));
        assert_eq!(size, Decimal::ZERO);
    }

    #[test]
    fn test_respects_max_risk_pct() {
        let sizer = HalfKellySizer::new(dec!(0.02), dec!(1.0)); // 2% max risk, no position cap
        let portfolio = test_portfolio(dec!(10000));
        // Very high win rate → Kelly wants large position, but capped at 2% risk = $200
        let size = sizer.calculate(0.9, 3.0, &portfolio, dec!(100), dec!(98));
        // Risk = $200, stop distance = $2, so position value = 200/2 * 100 = $10000
        // But max position = 100% * 10000 = $10000
        assert!(size <= dec!(10000));
    }

    #[test]
    fn test_respects_max_position_pct() {
        let sizer = HalfKellySizer::new(dec!(0.50), dec!(0.10)); // high risk ok, but 10% max position
        let portfolio = test_portfolio(dec!(10000));
        let size = sizer.calculate(0.9, 3.0, &portfolio, dec!(100), dec!(99));
        assert!(size <= dec!(1000)); // 10% of $10000
    }

    #[test]
    fn test_zero_equity_returns_zero() {
        let sizer = HalfKellySizer::new(dec!(0.02), dec!(0.10));
        let portfolio = test_portfolio(Decimal::ZERO);
        let size = sizer.calculate(0.6, 1.5, &portfolio, dec!(100), dec!(95));
        assert_eq!(size, Decimal::ZERO);
    }

    #[test]
    fn test_zero_stop_distance() {
        let sizer = HalfKellySizer::new(dec!(0.02), dec!(0.10));
        let portfolio = test_portfolio(dec!(10000));
        // Entry = stop → zero distance
        let size = sizer.calculate(0.6, 1.5, &portfolio, dec!(100), dec!(100));
        assert!(size > Decimal::ZERO); // Falls back to risk_amount directly
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p tradoshka-risk position_sizer
```

Expected: all 6 tests pass

- [ ] **Step 3: Commit**

```bash
git add core/risk/
git commit -m "feat(risk): add Half-Kelly position sizer

Implements Kelly criterion / 2 for position sizing. Respects max risk
per trade and max position percentage. Returns zero for negative edge."
```

---

### Task 7: Circuit Breaker + Drawdown Tracker

**Files:**
- Create: `core/risk/src/circuit_breaker.rs`
- Create: `core/risk/src/drawdown_tracker.rs`

- [ ] **Step 1: Implement drawdown tracker**

Create `core/risk/src/drawdown_tracker.rs`:

```rust
use chrono::{DateTime, Utc, Datelike};
use rust_decimal::Decimal;

/// Tracks drawdown metrics in real-time.
pub struct DrawdownTracker {
    peak_equity: Decimal,
    daily_start_equity: Decimal,
    current_day: u32,
}

impl DrawdownTracker {
    pub fn new(initial_equity: Decimal) -> Self {
        let today = Utc::now().ordinal();
        Self {
            peak_equity: initial_equity,
            daily_start_equity: initial_equity,
            current_day: today,
        }
    }

    /// Update with current equity value. Call on every portfolio update.
    /// Returns (portfolio_drawdown_pct, daily_drawdown_pct)
    pub fn update(&mut self, equity: Decimal, now: DateTime<Utc>) -> (Decimal, Decimal) {
        // Check for new day
        let today = now.ordinal();
        if today != self.current_day {
            self.daily_start_equity = equity;
            self.current_day = today;
        }

        // Update peak
        if equity > self.peak_equity {
            self.peak_equity = equity;
        }

        let portfolio_dd = if self.peak_equity > Decimal::ZERO {
            (self.peak_equity - equity) / self.peak_equity
        } else {
            Decimal::ZERO
        };

        let daily_dd = if self.daily_start_equity > Decimal::ZERO {
            (self.daily_start_equity - equity) / self.daily_start_equity
        } else {
            Decimal::ZERO
        };

        (portfolio_dd.max(Decimal::ZERO), daily_dd.max(Decimal::ZERO))
    }

    pub fn peak_equity(&self) -> Decimal {
        self.peak_equity
    }

    pub fn reset_daily(&mut self, equity: Decimal) {
        self.daily_start_equity = equity;
        self.current_day = Utc::now().ordinal();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn test_no_drawdown_at_peak() {
        let mut tracker = DrawdownTracker::new(dec!(10000));
        let (portfolio_dd, daily_dd) = tracker.update(dec!(10000), now());
        assert_eq!(portfolio_dd, Decimal::ZERO);
        assert_eq!(daily_dd, Decimal::ZERO);
    }

    #[test]
    fn test_portfolio_drawdown() {
        let mut tracker = DrawdownTracker::new(dec!(10000));
        tracker.update(dec!(12000), now()); // new peak
        let (portfolio_dd, _) = tracker.update(dec!(10800), now());
        assert_eq!(portfolio_dd, dec!(0.1)); // 10% from peak of 12000
    }

    #[test]
    fn test_peak_updates_on_new_high() {
        let mut tracker = DrawdownTracker::new(dec!(10000));
        tracker.update(dec!(15000), now());
        assert_eq!(tracker.peak_equity(), dec!(15000));
    }
}
```

- [ ] **Step 2: Implement circuit breaker**

Create `core/risk/src/circuit_breaker.rs`:

```rust
use rust_decimal::Decimal;
use tracing::warn;

/// Multi-level circuit breaker that halts trading when drawdown limits are hit.
///
/// Three levels:
/// 1. Daily drawdown limit (e.g., 5%) — pauses new trades for the day
/// 2. Portfolio drawdown halt (e.g., 15%) — stops all trading until manual review
/// 3. Hard stop (e.g., 20%) — closes all positions and halts completely
#[derive(Debug)]
pub struct CircuitBreaker {
    daily_limit: Decimal,
    portfolio_halt: Decimal,
    hard_stop: Decimal,
    state: BreakerState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    Normal,
    DailyPause,
    PortfolioHalt,
    HardStop,
}

impl BreakerState {
    pub fn allows_new_trades(&self) -> bool {
        matches!(self, Self::Normal)
    }

    pub fn requires_close_all(&self) -> bool {
        matches!(self, Self::HardStop)
    }
}

impl CircuitBreaker {
    pub fn new(daily_limit: Decimal, portfolio_halt: Decimal, hard_stop: Decimal) -> Self {
        Self {
            daily_limit,
            portfolio_halt,
            hard_stop,
            state: BreakerState::Normal,
        }
    }

    /// Check drawdown levels and update breaker state.
    /// Returns the current state after evaluation.
    pub fn evaluate(&mut self, portfolio_drawdown: Decimal, daily_drawdown: Decimal) -> BreakerState {
        if portfolio_drawdown >= self.hard_stop {
            if self.state != BreakerState::HardStop {
                warn!(
                    "HARD STOP triggered: portfolio drawdown {:.2}% >= {:.2}%",
                    portfolio_drawdown * Decimal::ONE_HUNDRED,
                    self.hard_stop * Decimal::ONE_HUNDRED,
                );
            }
            self.state = BreakerState::HardStop;
        } else if portfolio_drawdown >= self.portfolio_halt {
            if self.state != BreakerState::PortfolioHalt && self.state != BreakerState::HardStop {
                warn!(
                    "PORTFOLIO HALT: drawdown {:.2}% >= {:.2}%",
                    portfolio_drawdown * Decimal::ONE_HUNDRED,
                    self.portfolio_halt * Decimal::ONE_HUNDRED,
                );
            }
            self.state = BreakerState::PortfolioHalt;
        } else if daily_drawdown >= self.daily_limit {
            if self.state == BreakerState::Normal {
                warn!(
                    "DAILY PAUSE: drawdown {:.2}% >= {:.2}%",
                    daily_drawdown * Decimal::ONE_HUNDRED,
                    self.daily_limit * Decimal::ONE_HUNDRED,
                );
            }
            self.state = BreakerState::DailyPause;
        } else {
            self.state = BreakerState::Normal;
        }
        self.state
    }

    pub fn state(&self) -> BreakerState {
        self.state
    }

    /// Manual reset — only for DailyPause and PortfolioHalt (not HardStop)
    pub fn reset(&mut self) -> bool {
        match self.state {
            BreakerState::DailyPause | BreakerState::PortfolioHalt => {
                self.state = BreakerState::Normal;
                true
            }
            BreakerState::HardStop => false, // Requires manual intervention
            BreakerState::Normal => true,
        }
    }

    /// Force reset — even from HardStop (for manual override)
    pub fn force_reset(&mut self) {
        self.state = BreakerState::Normal;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_normal_state_allows_trades() {
        let mut cb = CircuitBreaker::new(dec!(0.05), dec!(0.15), dec!(0.20));
        let state = cb.evaluate(dec!(0.02), dec!(0.01));
        assert_eq!(state, BreakerState::Normal);
        assert!(state.allows_new_trades());
    }

    #[test]
    fn test_daily_pause() {
        let mut cb = CircuitBreaker::new(dec!(0.05), dec!(0.15), dec!(0.20));
        let state = cb.evaluate(dec!(0.03), dec!(0.06)); // daily > 5%
        assert_eq!(state, BreakerState::DailyPause);
        assert!(!state.allows_new_trades());
    }

    #[test]
    fn test_portfolio_halt() {
        let mut cb = CircuitBreaker::new(dec!(0.05), dec!(0.15), dec!(0.20));
        let state = cb.evaluate(dec!(0.16), dec!(0.03)); // portfolio > 15%
        assert_eq!(state, BreakerState::PortfolioHalt);
        assert!(!state.allows_new_trades());
    }

    #[test]
    fn test_hard_stop() {
        let mut cb = CircuitBreaker::new(dec!(0.05), dec!(0.15), dec!(0.20));
        let state = cb.evaluate(dec!(0.21), dec!(0.10)); // portfolio > 20%
        assert_eq!(state, BreakerState::HardStop);
        assert!(state.requires_close_all());
    }

    #[test]
    fn test_hard_stop_cannot_normal_reset() {
        let mut cb = CircuitBreaker::new(dec!(0.05), dec!(0.15), dec!(0.20));
        cb.evaluate(dec!(0.21), dec!(0.10));
        assert!(!cb.reset()); // Cannot reset from hard stop
        assert_eq!(cb.state(), BreakerState::HardStop);
    }

    #[test]
    fn test_force_reset_from_hard_stop() {
        let mut cb = CircuitBreaker::new(dec!(0.05), dec!(0.15), dec!(0.20));
        cb.evaluate(dec!(0.21), dec!(0.10));
        cb.force_reset();
        assert_eq!(cb.state(), BreakerState::Normal);
    }

    #[test]
    fn test_recovery_to_normal() {
        let mut cb = CircuitBreaker::new(dec!(0.05), dec!(0.15), dec!(0.20));
        cb.evaluate(dec!(0.03), dec!(0.06)); // daily pause
        assert_eq!(cb.state(), BreakerState::DailyPause);

        cb.evaluate(dec!(0.02), dec!(0.03)); // recovered
        assert_eq!(cb.state(), BreakerState::Normal);
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-risk
```

Expected: all 16 tests pass (6 position sizer + 3 drawdown + 7 circuit breaker)

- [ ] **Step 4: Commit**

```bash
git add core/risk/
git commit -m "feat(risk): add circuit breaker and drawdown tracker

Three-level circuit breaker (daily pause, portfolio halt, hard stop).
Real-time drawdown tracker with daily and portfolio-level monitoring."
```

---

### Task 8: Risk Manager

**Files:**
- Create: `core/risk/src/manager.rs`

- [ ] **Step 1: Implement combined risk manager**

```rust
use rust_decimal::Decimal;
use tradoshka_common::types::*;
use tradoshka_common::traits::RiskManager;
use crate::position_sizer::HalfKellySizer;
use crate::circuit_breaker::{CircuitBreaker, BreakerState};
use crate::drawdown_tracker::DrawdownTracker;

pub struct TradoshkaRiskManager {
    sizer: HalfKellySizer,
    breaker: CircuitBreaker,
    tracker: DrawdownTracker,
    max_open_positions: usize,
    max_exposure_pct: Decimal,
    default_win_rate: f64,
    default_win_loss_ratio: f64,
}

pub struct RiskConfig {
    pub max_risk_pct: Decimal,
    pub max_position_pct: Decimal,
    pub daily_drawdown_limit: Decimal,
    pub portfolio_drawdown_halt: Decimal,
    pub hard_stop: Decimal,
    pub max_open_positions: usize,
    pub max_exposure_pct: Decimal,
    pub initial_equity: Decimal,
    pub default_win_rate: f64,
    pub default_win_loss_ratio: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_risk_pct: Decimal::new(2, 2),         // 2%
            max_position_pct: Decimal::new(10, 2),     // 10%
            daily_drawdown_limit: Decimal::new(5, 2),  // 5%
            portfolio_drawdown_halt: Decimal::new(15, 2), // 15%
            hard_stop: Decimal::new(20, 2),            // 20%
            max_open_positions: 10,
            max_exposure_pct: Decimal::new(30, 2),     // 30%
            initial_equity: Decimal::new(10000, 0),
            default_win_rate: 0.55,
            default_win_loss_ratio: 1.5,
        }
    }
}

impl TradoshkaRiskManager {
    pub fn new(config: RiskConfig) -> Self {
        Self {
            sizer: HalfKellySizer::new(config.max_risk_pct, config.max_position_pct),
            breaker: CircuitBreaker::new(
                config.daily_drawdown_limit,
                config.portfolio_drawdown_halt,
                config.hard_stop,
            ),
            tracker: DrawdownTracker::new(config.initial_equity),
            max_open_positions: config.max_open_positions,
            max_exposure_pct: config.max_exposure_pct,
            default_win_rate: config.default_win_rate,
            default_win_loss_ratio: config.default_win_loss_ratio,
        }
    }

    pub fn update_equity(&mut self, equity: Decimal) {
        let now = chrono::Utc::now();
        let (portfolio_dd, daily_dd) = self.tracker.update(equity, now);
        self.breaker.evaluate(portfolio_dd, daily_dd);
    }

    pub fn breaker_state(&self) -> BreakerState {
        self.breaker.state()
    }

    pub fn force_reset_breaker(&mut self) {
        self.breaker.force_reset();
    }
}

impl RiskManager for TradoshkaRiskManager {
    fn validate_order(&self, order: &Order, portfolio: &Portfolio) -> RiskDecision {
        // Check circuit breaker
        if !self.breaker.state().allows_new_trades() {
            return RiskDecision::Rejected {
                reason: format!("Circuit breaker active: {:?}", self.breaker.state()),
            };
        }

        // Check max open positions
        if portfolio.positions.len() >= self.max_open_positions {
            return RiskDecision::Rejected {
                reason: format!(
                    "Max open positions reached: {} / {}",
                    portfolio.positions.len(),
                    self.max_open_positions,
                ),
            };
        }

        // Check total exposure
        let current_exposure: Decimal = portfolio
            .positions
            .iter()
            .map(|p| p.quantity * p.current_price)
            .sum();
        let max_exposure = self.max_exposure_pct * portfolio.equity;
        let order_value = order.quantity * match &order.order_type {
            OrderType::Limit { price } => *price,
            _ => portfolio.positions.first()
                .map(|p| p.current_price)
                .unwrap_or(Decimal::ONE),
        };

        if current_exposure + order_value > max_exposure {
            return RiskDecision::Rejected {
                reason: format!(
                    "Total exposure would exceed {:.0}% of equity",
                    self.max_exposure_pct * Decimal::ONE_HUNDRED,
                ),
            };
        }

        // Check sufficient balance
        if order_value > portfolio.balance {
            return RiskDecision::Rejected {
                reason: "Insufficient balance".into(),
            };
        }

        RiskDecision::Approved {
            adjusted_quantity: order.quantity,
        }
    }

    fn check_circuit_breakers(&self, _portfolio: &Portfolio) -> bool {
        self.breaker.state().allows_new_trades()
    }

    fn calculate_position_size(&self, signal: &Signal, portfolio: &Portfolio) -> Decimal {
        let entry_price = Decimal::ONE; // Placeholder — real price from market data
        let stop_price = signal.stop_loss.unwrap_or(entry_price * Decimal::new(95, 2)); // 5% default stop

        self.sizer.calculate(
            self.default_win_rate,
            self.default_win_loss_ratio,
            portfolio,
            entry_price,
            stop_price,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use chrono::Utc;

    fn test_config() -> RiskConfig {
        RiskConfig {
            initial_equity: dec!(10000),
            max_open_positions: 3,
            ..Default::default()
        }
    }

    fn test_order() -> Order {
        Order::new(
            "BTC/USD".into(),
            OrderSide::Buy,
            OrderType::Limit { price: dec!(100) },
            dec!(5), // $500 value
            "test".into(),
            Market::Crypto,
        )
    }

    #[test]
    fn test_approves_valid_order() {
        let rm = TradoshkaRiskManager::new(test_config());
        let portfolio = Portfolio::new(dec!(10000));
        let order = test_order();
        let decision = rm.validate_order(&order, &portfolio);
        assert!(matches!(decision, RiskDecision::Approved { .. }));
    }

    #[test]
    fn test_rejects_when_max_positions_reached() {
        let rm = TradoshkaRiskManager::new(test_config());
        let mut portfolio = Portfolio::new(dec!(10000));
        // Add 3 positions (max)
        for i in 0..3 {
            portfolio.positions.push(Position {
                symbol: format!("SYM{i}"),
                side: OrderSide::Buy,
                quantity: dec!(1),
                entry_price: dec!(100),
                current_price: dec!(100),
                unrealized_pnl: Decimal::ZERO,
                realized_pnl: Decimal::ZERO,
                strategy_id: "test".into(),
                market: Market::Crypto,
                opened_at: Utc::now(),
            });
        }
        let decision = rm.validate_order(&test_order(), &portfolio);
        assert!(matches!(decision, RiskDecision::Rejected { .. }));
    }

    #[test]
    fn test_rejects_when_circuit_breaker_tripped() {
        let mut rm = TradoshkaRiskManager::new(test_config());
        // Simulate large drawdown
        rm.update_equity(dec!(10000));
        rm.update_equity(dec!(7500)); // 25% drawdown → hard stop
        let portfolio = Portfolio::new(dec!(7500));
        let decision = rm.validate_order(&test_order(), &portfolio);
        assert!(matches!(decision, RiskDecision::Rejected { reason } if reason.contains("Circuit breaker")));
    }

    #[test]
    fn test_rejects_insufficient_balance() {
        let rm = TradoshkaRiskManager::new(test_config());
        let portfolio = Portfolio::new(dec!(100)); // Only $100
        let mut order = test_order();
        order.quantity = dec!(50); // $5000 value at $100/unit
        let decision = rm.validate_order(&order, &portfolio);
        assert!(matches!(decision, RiskDecision::Rejected { .. }));
    }
}
```

- [ ] **Step 2: Update risk lib.rs**

`core/risk/src/lib.rs`:
```rust
pub mod position_sizer;
pub mod circuit_breaker;
pub mod drawdown_tracker;
pub mod manager;

pub use manager::{TradoshkaRiskManager, RiskConfig};
pub use circuit_breaker::BreakerState;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p tradoshka-risk
```

Expected: all 20 tests pass

- [ ] **Step 4: Commit**

```bash
git add core/risk/
git commit -m "feat(risk): add combined risk manager with config

TradoshkaRiskManager integrates Half-Kelly, circuit breaker, drawdown
tracker. Validates positions, exposure, balance, and breaker state."
```

---

### Task 9: Order Manager (State Machine)

**Files:**
- Create: `core/engine/src/order_manager.rs`

- [ ] **Step 1: Implement order state machine**

```rust
use std::collections::HashMap;
use tradoshka_common::types::*;
use tradoshka_common::error::{TradoshkaError, Result};
use uuid::Uuid;

/// Manages the lifecycle of all orders. Enforces valid state transitions.
pub struct OrderManager {
    orders: HashMap<OrderId, Order>,
}

impl OrderManager {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
        }
    }

    /// Register a new order (status: Pending)
    pub fn register(&mut self, order: Order) -> OrderId {
        let id = order.id;
        self.orders.insert(id, order);
        id
    }

    /// Transition order to a new status. Validates the transition.
    pub fn transition(&mut self, id: &OrderId, new_status: OrderStatus) -> Result<()> {
        let order = self.orders.get_mut(id)
            .ok_or_else(|| TradoshkaError::OrderNotFound(id.to_string()))?;

        if !order.status.can_transition_to(&new_status) {
            return Err(TradoshkaError::InvalidTransition {
                from: order.status,
                to: new_status,
            });
        }

        order.status = new_status;
        order.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Record a fill against an order. Updates filled quantity and status.
    pub fn record_fill(&mut self, fill: &Fill) -> Result<()> {
        let order = self.orders.get_mut(&fill.order_id)
            .ok_or_else(|| TradoshkaError::OrderNotFound(fill.order_id.to_string()))?;

        order.filled_quantity += fill.quantity;
        order.updated_at = chrono::Utc::now();

        if order.filled_quantity >= order.quantity {
            order.status = OrderStatus::Filled;
        } else if order.filled_quantity > rust_decimal::Decimal::ZERO {
            if order.status == OrderStatus::Submitted {
                order.status = OrderStatus::PartiallyFilled;
            }
        }

        Ok(())
    }

    pub fn get(&self, id: &OrderId) -> Option<&Order> {
        self.orders.get(id)
    }

    pub fn get_mut(&mut self, id: &OrderId) -> Option<&mut Order> {
        self.orders.get_mut(id)
    }

    /// All orders with non-terminal status
    pub fn active_orders(&self) -> Vec<&Order> {
        self.orders.values()
            .filter(|o| !o.status.is_terminal())
            .collect()
    }

    /// All orders for a specific strategy
    pub fn orders_by_strategy(&self, strategy_id: &str) -> Vec<&Order> {
        self.orders.values()
            .filter(|o| o.strategy_id == strategy_id)
            .collect()
    }

    pub fn all_orders(&self) -> Vec<&Order> {
        self.orders.values().collect()
    }

    pub fn order_count(&self) -> usize {
        self.orders.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_order() -> Order {
        Order::new(
            "BTC/USD".into(),
            OrderSide::Buy,
            OrderType::Market,
            dec!(10),
            "test_strat".into(),
            Market::Crypto,
        )
    }

    fn make_fill(order_id: OrderId, quantity: rust_decimal::Decimal) -> Fill {
        Fill {
            order_id,
            trade_id: Uuid::new_v4(),
            symbol: "BTC/USD".into(),
            side: OrderSide::Buy,
            price: dec!(50000),
            quantity,
            fee: dec!(0.1),
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_register_order() {
        let mut mgr = OrderManager::new();
        let order = make_order();
        let id = mgr.register(order);
        assert!(mgr.get(&id).is_some());
        assert_eq!(mgr.order_count(), 1);
    }

    #[test]
    fn test_valid_transition() {
        let mut mgr = OrderManager::new();
        let order = make_order();
        let id = mgr.register(order);
        assert!(mgr.transition(&id, OrderStatus::Submitted).is_ok());
        assert_eq!(mgr.get(&id).unwrap().status, OrderStatus::Submitted);
    }

    #[test]
    fn test_invalid_transition() {
        let mut mgr = OrderManager::new();
        let order = make_order();
        let id = mgr.register(order);
        // Pending → Filled is invalid (must go through Submitted first)
        let result = mgr.transition(&id, OrderStatus::Filled);
        assert!(result.is_err());
    }

    #[test]
    fn test_partial_fill() {
        let mut mgr = OrderManager::new();
        let order = make_order(); // qty = 10
        let id = mgr.register(order);
        mgr.transition(&id, OrderStatus::Submitted).unwrap();

        let fill = make_fill(id, dec!(4));
        mgr.record_fill(&fill).unwrap();
        let o = mgr.get(&id).unwrap();
        assert_eq!(o.status, OrderStatus::PartiallyFilled);
        assert_eq!(o.filled_quantity, dec!(4));
        assert_eq!(o.remaining_quantity(), dec!(6));
    }

    #[test]
    fn test_complete_fill() {
        let mut mgr = OrderManager::new();
        let order = make_order(); // qty = 10
        let id = mgr.register(order);
        mgr.transition(&id, OrderStatus::Submitted).unwrap();

        mgr.record_fill(&make_fill(id, dec!(10))).unwrap();
        assert_eq!(mgr.get(&id).unwrap().status, OrderStatus::Filled);
    }

    #[test]
    fn test_multi_fill_to_complete() {
        let mut mgr = OrderManager::new();
        let order = make_order(); // qty = 10
        let id = mgr.register(order);
        mgr.transition(&id, OrderStatus::Submitted).unwrap();

        mgr.record_fill(&make_fill(id, dec!(3))).unwrap();
        assert_eq!(mgr.get(&id).unwrap().status, OrderStatus::PartiallyFilled);

        mgr.record_fill(&make_fill(id, dec!(7))).unwrap();
        assert_eq!(mgr.get(&id).unwrap().status, OrderStatus::Filled);
    }

    #[test]
    fn test_active_orders() {
        let mut mgr = OrderManager::new();
        let id1 = mgr.register(make_order());
        let id2 = mgr.register(make_order());
        mgr.transition(&id1, OrderStatus::Submitted).unwrap();
        mgr.transition(&id2, OrderStatus::Rejected).unwrap();

        let active = mgr.active_orders();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, id1);
    }

    #[test]
    fn test_nonexistent_order() {
        let mut mgr = OrderManager::new();
        let fake_id = Uuid::new_v4();
        assert!(mgr.transition(&fake_id, OrderStatus::Submitted).is_err());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p tradoshka-engine order_manager
```

Expected: all 8 tests pass

- [ ] **Step 3: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add order manager with state machine

Enforces valid order state transitions, tracks partial/complete fills,
provides active order queries and per-strategy filtering."
```

---

### Task 10: Portfolio Tracker

**Files:**
- Create: `core/engine/src/portfolio.rs`

- [ ] **Step 1: Implement portfolio tracker**

```rust
use rust_decimal::Decimal;
use chrono::Utc;
use std::collections::HashMap;
use tradoshka_common::types::*;

/// Real-time portfolio tracking with P&L calculation.
pub struct PortfolioTracker {
    balance: Decimal,
    positions: HashMap<String, Position>, // keyed by "symbol:strategy_id"
    realized_pnl: Decimal,
    total_fees: Decimal,
    trade_count: u64,
    winning_trades: u64,
}

impl PortfolioTracker {
    pub fn new(initial_balance: Decimal) -> Self {
        Self {
            balance: initial_balance,
            positions: HashMap::new(),
            realized_pnl: Decimal::ZERO,
            total_fees: Decimal::ZERO,
            trade_count: 0,
            winning_trades: 0,
        }
    }

    fn position_key(symbol: &str, strategy_id: &str) -> String {
        format!("{symbol}:{strategy_id}")
    }

    /// Open or add to a position based on a fill
    pub fn process_fill(&mut self, fill: &Fill, strategy_id: &str, market: Market) {
        let key = Self::position_key(&fill.symbol, strategy_id);
        self.total_fees += fill.fee;
        self.balance -= fill.fee;

        if let Some(pos) = self.positions.get_mut(&key) {
            if pos.side == fill.side {
                // Adding to position — average entry price
                let total_cost = pos.entry_price * pos.quantity + fill.price * fill.quantity;
                let total_qty = pos.quantity + fill.quantity;
                pos.entry_price = total_cost / total_qty;
                pos.quantity = total_qty;
            } else {
                // Closing/reducing position
                let close_qty = fill.quantity.min(pos.quantity);
                let pnl = match pos.side {
                    OrderSide::Buy => (fill.price - pos.entry_price) * close_qty,
                    OrderSide::Sell => (pos.entry_price - fill.price) * close_qty,
                };
                self.realized_pnl += pnl;
                self.balance += pnl;
                pos.realized_pnl += pnl;
                pos.quantity -= close_qty;

                self.trade_count += 1;
                if pnl > Decimal::ZERO {
                    self.winning_trades += 1;
                }

                if pos.quantity == Decimal::ZERO {
                    self.positions.remove(&key);
                }
                return;
            }
        } else {
            // New position
            let cost = fill.price * fill.quantity;
            self.balance -= cost;
            self.positions.insert(key, Position {
                symbol: fill.symbol.clone(),
                side: fill.side,
                quantity: fill.quantity,
                entry_price: fill.price,
                current_price: fill.price,
                unrealized_pnl: Decimal::ZERO,
                realized_pnl: Decimal::ZERO,
                strategy_id: strategy_id.to_string(),
                market,
                opened_at: Utc::now(),
            });
        }
    }

    /// Update mark price for a symbol
    pub fn update_price(&mut self, symbol: &str, price: Decimal) {
        for pos in self.positions.values_mut() {
            if pos.symbol == symbol {
                pos.update_price(price);
            }
        }
    }

    /// Get current portfolio snapshot
    pub fn snapshot(&self) -> Portfolio {
        let positions: Vec<Position> = self.positions.values().cloned().collect();
        let unrealized: Decimal = positions.iter().map(|p| p.unrealized_pnl).sum();
        let equity = self.balance + unrealized
            + positions.iter().map(|p| p.quantity * p.current_price).sum::<Decimal>();
        let mut portfolio = Portfolio {
            balance: self.balance,
            equity,
            positions,
            daily_pnl: self.realized_pnl, // simplified — real impl tracks daily reset
            peak_equity: equity,
            current_drawdown_pct: Decimal::ZERO,
        };
        portfolio.update_equity();
        portfolio
    }

    pub fn win_rate(&self) -> f64 {
        if self.trade_count == 0 {
            return 0.0;
        }
        self.winning_trades as f64 / self.trade_count as f64
    }

    pub fn realized_pnl(&self) -> Decimal {
        self.realized_pnl
    }

    pub fn total_fees(&self) -> Decimal {
        self.total_fees
    }

    pub fn open_position_count(&self) -> usize {
        self.positions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    fn buy_fill(symbol: &str, price: Decimal, qty: Decimal) -> Fill {
        Fill {
            order_id: Uuid::new_v4(),
            trade_id: Uuid::new_v4(),
            symbol: symbol.into(),
            side: OrderSide::Buy,
            price,
            quantity: qty,
            fee: dec!(1),
            timestamp: Utc::now(),
        }
    }

    fn sell_fill(symbol: &str, price: Decimal, qty: Decimal) -> Fill {
        Fill {
            order_id: Uuid::new_v4(),
            trade_id: Uuid::new_v4(),
            symbol: symbol.into(),
            side: OrderSide::Sell,
            price,
            quantity: qty,
            fee: dec!(1),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_open_long_position() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        tracker.process_fill(&buy_fill("BTC", dec!(100), dec!(10)), "s1", Market::Crypto);
        assert_eq!(tracker.open_position_count(), 1);
        // Balance reduced by cost + fee: 10000 - 1000 - 1 = 8999
        assert_eq!(tracker.snapshot().balance, dec!(8999));
    }

    #[test]
    fn test_close_long_with_profit() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        tracker.process_fill(&buy_fill("BTC", dec!(100), dec!(10)), "s1", Market::Crypto);
        tracker.process_fill(&sell_fill("BTC", dec!(110), dec!(10)), "s1", Market::Crypto);
        assert_eq!(tracker.open_position_count(), 0);
        assert_eq!(tracker.realized_pnl(), dec!(100)); // (110-100)*10
        assert_eq!(tracker.win_rate(), 1.0);
    }

    #[test]
    fn test_close_long_with_loss() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        tracker.process_fill(&buy_fill("BTC", dec!(100), dec!(10)), "s1", Market::Crypto);
        tracker.process_fill(&sell_fill("BTC", dec!(90), dec!(10)), "s1", Market::Crypto);
        assert_eq!(tracker.realized_pnl(), dec!(-100));
        assert_eq!(tracker.win_rate(), 0.0);
    }

    #[test]
    fn test_partial_close() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        tracker.process_fill(&buy_fill("BTC", dec!(100), dec!(10)), "s1", Market::Crypto);
        tracker.process_fill(&sell_fill("BTC", dec!(110), dec!(5)), "s1", Market::Crypto);
        assert_eq!(tracker.open_position_count(), 1);
        assert_eq!(tracker.realized_pnl(), dec!(50)); // (110-100)*5
    }

    #[test]
    fn test_update_price_changes_unrealized() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        tracker.process_fill(&buy_fill("BTC", dec!(100), dec!(10)), "s1", Market::Crypto);
        tracker.update_price("BTC", dec!(120));
        let snapshot = tracker.snapshot();
        let pos = &snapshot.positions[0];
        assert_eq!(pos.unrealized_pnl, dec!(200)); // (120-100)*10
    }

    #[test]
    fn test_fees_tracked() {
        let mut tracker = PortfolioTracker::new(dec!(10000));
        tracker.process_fill(&buy_fill("BTC", dec!(100), dec!(1)), "s1", Market::Crypto);
        tracker.process_fill(&sell_fill("BTC", dec!(100), dec!(1)), "s1", Market::Crypto);
        assert_eq!(tracker.total_fees(), dec!(2)); // $1 per fill
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p tradoshka-engine portfolio
```

Expected: all 6 tests pass

- [ ] **Step 3: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add portfolio tracker with P&L calculation

Tracks positions, fills, realized/unrealized P&L, win rate, fees.
Supports partial closes, position averaging, and price updates."
```

---

### Task 11: Dry Mode Engine

**Files:**
- Create: `core/engine/src/dry_mode.rs`

- [ ] **Step 1: Implement dry mode (simulated execution)**

```rust
use rust_decimal::Decimal;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use tradoshka_common::types::*;
use tradoshka_common::error::Result;

/// Simulated order execution for paper trading.
/// Behaves identically to live trading but without real money.
pub struct DryModeEngine {
    last_prices: HashMap<Symbol, Decimal>,
    slippage_bps: Decimal, // basis points of simulated slippage
}

impl DryModeEngine {
    pub fn new(slippage_bps: Decimal) -> Self {
        Self {
            last_prices: HashMap::new(),
            slippage_bps,
        }
    }

    /// Update the simulated market price for a symbol
    pub fn update_price(&mut self, symbol: &Symbol, price: Decimal) {
        self.last_prices.insert(symbol.clone(), price);
    }

    /// Simulate order execution. Returns a Fill if the order can be executed.
    pub fn try_execute(&self, order: &Order) -> Result<Option<Fill>> {
        let market_price = match self.last_prices.get(&order.symbol) {
            Some(p) => *p,
            None => return Ok(None), // No price data yet
        };

        let should_fill = match &order.order_type {
            OrderType::Market => true,
            OrderType::Limit { price } => match order.side {
                OrderSide::Buy => market_price <= *price,
                OrderSide::Sell => market_price >= *price,
            },
            OrderType::StopLoss { trigger } => match order.side {
                OrderSide::Buy => market_price >= *trigger,
                OrderSide::Sell => market_price <= *trigger,
            },
            OrderType::TrailingStop { .. } => false, // Requires separate tracking
        };

        if !should_fill {
            return Ok(None);
        }

        // Apply slippage
        let slippage_mult = self.slippage_bps / Decimal::new(10000, 0);
        let fill_price = match order.side {
            OrderSide::Buy => market_price * (Decimal::ONE + slippage_mult),
            OrderSide::Sell => market_price * (Decimal::ONE - slippage_mult),
        };

        // Simulate fee (0.1% taker fee)
        let fee_rate = Decimal::new(1, 3); // 0.001
        let fee = fill_price * order.remaining_quantity() * fee_rate;

        Ok(Some(Fill {
            order_id: order.id,
            trade_id: Uuid::new_v4(),
            symbol: order.symbol.clone(),
            side: order.side,
            price: fill_price,
            quantity: order.remaining_quantity(),
            fee,
            timestamp: Utc::now(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn market_buy(symbol: &str, qty: Decimal) -> Order {
        Order::new(symbol.into(), OrderSide::Buy, OrderType::Market, qty, "test".into(), Market::Crypto)
    }

    fn limit_buy(symbol: &str, qty: Decimal, price: Decimal) -> Order {
        Order::new(symbol.into(), OrderSide::Buy, OrderType::Limit { price }, qty, "test".into(), Market::Crypto)
    }

    fn stop_sell(symbol: &str, qty: Decimal, trigger: Decimal) -> Order {
        Order::new(symbol.into(), OrderSide::Sell, OrderType::StopLoss { trigger }, qty, "test".into(), Market::Crypto)
    }

    #[test]
    fn test_market_order_fills_immediately() {
        let mut engine = DryModeEngine::new(dec!(0)); // no slippage
        engine.update_price(&"BTC".into(), dec!(50000));
        let order = market_buy("BTC", dec!(1));
        let fill = engine.try_execute(&order).unwrap().unwrap();
        assert_eq!(fill.price, dec!(50000));
        assert_eq!(fill.quantity, dec!(1));
    }

    #[test]
    fn test_limit_buy_fills_when_price_at_or_below() {
        let mut engine = DryModeEngine::new(dec!(0));
        engine.update_price(&"BTC".into(), dec!(49000));
        let order = limit_buy("BTC", dec!(1), dec!(50000));
        assert!(engine.try_execute(&order).unwrap().is_some());
    }

    #[test]
    fn test_limit_buy_does_not_fill_above_limit() {
        let mut engine = DryModeEngine::new(dec!(0));
        engine.update_price(&"BTC".into(), dec!(51000));
        let order = limit_buy("BTC", dec!(1), dec!(50000));
        assert!(engine.try_execute(&order).unwrap().is_none());
    }

    #[test]
    fn test_stop_loss_triggers_below() {
        let mut engine = DryModeEngine::new(dec!(0));
        engine.update_price(&"BTC".into(), dec!(45000));
        let order = stop_sell("BTC", dec!(1), dec!(48000));
        assert!(engine.try_execute(&order).unwrap().is_some());
    }

    #[test]
    fn test_slippage_applied() {
        let mut engine = DryModeEngine::new(dec!(10)); // 10 bps = 0.1%
        engine.update_price(&"BTC".into(), dec!(50000));
        let order = market_buy("BTC", dec!(1));
        let fill = engine.try_execute(&order).unwrap().unwrap();
        assert_eq!(fill.price, dec!(50050)); // 50000 * 1.001
    }

    #[test]
    fn test_no_price_returns_none() {
        let engine = DryModeEngine::new(dec!(0));
        let order = market_buy("UNKNOWN", dec!(1));
        assert!(engine.try_execute(&order).unwrap().is_none());
    }

    #[test]
    fn test_fee_calculated() {
        let mut engine = DryModeEngine::new(dec!(0));
        engine.update_price(&"BTC".into(), dec!(1000));
        let order = market_buy("BTC", dec!(10)); // $10000 value
        let fill = engine.try_execute(&order).unwrap().unwrap();
        assert_eq!(fill.fee, dec!(10)); // 0.1% of $10000
    }
}
```

- [ ] **Step 2: Update engine lib.rs**

```rust
pub mod order_manager;
pub mod portfolio;
pub mod dry_mode;

pub use order_manager::OrderManager;
pub use portfolio::PortfolioTracker;
pub use dry_mode::DryModeEngine;
```

- [ ] **Step 3: Run all engine tests**

```bash
cargo test -p tradoshka-engine
```

Expected: all 21 tests pass (8 order manager + 6 portfolio + 7 dry mode)

- [ ] **Step 4: Commit**

```bash
git add core/engine/
git commit -m "feat(engine): add dry mode simulated execution engine

Paper trading with configurable slippage, simulated fees. Supports
market, limit, and stop-loss order types. Full fill generation."
```

---

### Task 12: API Server

**Files:**
- Create: `core/api/src/state.rs`
- Create: `core/api/src/routes.rs`
- Create: `core/api/src/ws.rs`
- Create: `core/api/src/server.rs`
- Create: `core/api/src/main.rs`

- [ ] **Step 1: Implement shared state**

Create `core/api/src/state.rs`:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use tradoshka_engine::{OrderManager, PortfolioTracker, DryModeEngine};
use tradoshka_risk::TradoshkaRiskManager;
use rust_decimal_macros::dec;

pub type SharedState = Arc<RwLock<AppState>>;

pub struct AppState {
    pub order_manager: OrderManager,
    pub portfolio: PortfolioTracker,
    pub risk_manager: TradoshkaRiskManager,
    pub dry_engine: DryModeEngine,
}

impl AppState {
    pub fn new_dry_mode(initial_balance: rust_decimal::Decimal) -> Self {
        Self {
            order_manager: OrderManager::new(),
            portfolio: PortfolioTracker::new(initial_balance),
            risk_manager: TradoshkaRiskManager::new(tradoshka_risk::RiskConfig {
                initial_equity: initial_balance,
                ..Default::default()
            }),
            dry_engine: DryModeEngine::new(dec!(5)), // 5 bps slippage
        }
    }
}

pub fn create_shared_state(initial_balance: rust_decimal::Decimal) -> SharedState {
    Arc::new(RwLock::new(AppState::new_dry_mode(initial_balance)))
}
```

- [ ] **Step 2: Implement REST routes**

Create `core/api/src/routes.rs`:

```rust
use axum::{extract::State, Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use crate::state::SharedState;
use tradoshka_common::types::*;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

#[derive(Serialize)]
pub struct PortfolioResponse {
    pub balance: String,
    pub equity: String,
    pub unrealized_pnl: String,
    pub realized_pnl: String,
    pub open_positions: usize,
    pub win_rate: f64,
    pub total_fees: String,
}

pub async fn get_portfolio(
    State(state): State<SharedState>,
) -> Json<PortfolioResponse> {
    let state = state.read().await;
    let snapshot = state.portfolio.snapshot();
    let unrealized: rust_decimal::Decimal = snapshot.positions.iter()
        .map(|p| p.unrealized_pnl).sum();
    Json(PortfolioResponse {
        balance: snapshot.balance.to_string(),
        equity: snapshot.equity.to_string(),
        unrealized_pnl: unrealized.to_string(),
        realized_pnl: state.portfolio.realized_pnl().to_string(),
        open_positions: state.portfolio.open_position_count(),
        win_rate: state.portfolio.win_rate(),
        total_fees: state.portfolio.total_fees().to_string(),
    })
}

#[derive(Serialize)]
pub struct PositionsResponse {
    pub positions: Vec<PositionInfo>,
}

#[derive(Serialize)]
pub struct PositionInfo {
    pub symbol: String,
    pub side: String,
    pub quantity: String,
    pub entry_price: String,
    pub current_price: String,
    pub unrealized_pnl: String,
    pub strategy_id: String,
    pub market: String,
}

pub async fn get_positions(
    State(state): State<SharedState>,
) -> Json<PositionsResponse> {
    let state = state.read().await;
    let snapshot = state.portfolio.snapshot();
    let positions = snapshot.positions.iter().map(|p| PositionInfo {
        symbol: p.symbol.clone(),
        side: format!("{:?}", p.side),
        quantity: p.quantity.to_string(),
        entry_price: p.entry_price.to_string(),
        current_price: p.current_price.to_string(),
        unrealized_pnl: p.unrealized_pnl.to_string(),
        strategy_id: p.strategy_id.clone(),
        market: format!("{:?}", p.market),
    }).collect();
    Json(PositionsResponse { positions })
}

#[derive(Serialize)]
pub struct OrdersResponse {
    pub active_orders: usize,
    pub total_orders: usize,
}

pub async fn get_orders(
    State(state): State<SharedState>,
) -> Json<OrdersResponse> {
    let state = state.read().await;
    Json(OrdersResponse {
        active_orders: state.order_manager.active_orders().len(),
        total_orders: state.order_manager.order_count(),
    })
}

#[derive(Serialize)]
pub struct RiskResponse {
    pub breaker_state: String,
    pub allows_trading: bool,
}

pub async fn get_risk(
    State(state): State<SharedState>,
) -> Json<RiskResponse> {
    let state = state.read().await;
    let breaker = state.risk_manager.breaker_state();
    Json(RiskResponse {
        breaker_state: format!("{:?}", breaker),
        allows_trading: breaker.allows_new_trades(),
    })
}
```

- [ ] **Step 3: Implement WebSocket handler**

Create `core/api/src/ws.rs`:

```rust
use axum::{
    extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
};
use serde::Serialize;
use tokio::time::{interval, Duration};
use crate::state::SharedState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[derive(Serialize)]
struct WsUpdate {
    event: String,
    data: serde_json::Value,
}

async fn handle_socket(mut socket: WebSocket, state: SharedState) {
    let mut tick = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let state = state.read().await;
                let snapshot = state.portfolio.snapshot();
                let update = WsUpdate {
                    event: "portfolio_update".into(),
                    data: serde_json::json!({
                        "equity": snapshot.equity.to_string(),
                        "balance": snapshot.balance.to_string(),
                        "drawdown_pct": snapshot.current_drawdown_pct.to_string(),
                        "open_positions": snapshot.positions.len(),
                    }),
                };
                if let Ok(json) = serde_json::to_string(&update) {
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
```

- [ ] **Step 4: Implement server setup**

Create `core/api/src/server.rs`:

```rust
use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use crate::state::SharedState;
use crate::routes;
use crate::ws;

pub fn create_router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route("/api/portfolio", get(routes::get_portfolio))
        .route("/api/positions", get(routes::get_positions))
        .route("/api/orders", get(routes::get_orders))
        .route("/api/risk", get(routes::get_risk))
        .route("/ws", get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn start(state: SharedState, port: u16) -> anyhow::Result<()> {
    let app = create_router(state);
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("Tradoshka API server starting on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 5: Add binary target**

Add to `core/api/Cargo.toml`:

```toml
[[bin]]
name = "tradoshka"
path = "src/main.rs"
```

Create `core/api/src/main.rs`:

```rust
use rust_decimal_macros::dec;
use tradoshka_api::state::create_shared_state;
use tradoshka_api::server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tradoshka=info,tower_http=info".into()),
        )
        .init();

    let initial_balance = dec!(100); // Start with $100
    let state = create_shared_state(initial_balance);

    tracing::info!("Starting Tradoshka in DRY MODE with ${initial_balance} initial balance");
    server::start(state, 3001).await
}
```

Add `rust_decimal_macros` and `anyhow` to api Cargo.toml deps:

```toml
rust_decimal_macros = "1.36"
anyhow = { workspace = true }
```

- [ ] **Step 6: Update api lib.rs**

```rust
pub mod server;
pub mod routes;
pub mod ws;
pub mod state;
```

- [ ] **Step 7: Verify compilation**

```bash
cargo build -p tradoshka-api
```

Expected: compiles successfully

- [ ] **Step 8: Commit**

```bash
git add core/api/
git commit -m "feat(api): add Axum REST + WebSocket API server

Health, portfolio, positions, orders, risk endpoints.
WebSocket for real-time portfolio updates (1s interval).
CORS enabled, tracing middleware, dry mode startup."
```

---

### Task 13: Python Strategy Base

**Files:**
- Create: `strategies/shared/pyproject.toml`
- Create: `strategies/shared/tradoshka_strategy/__init__.py`
- Create: `strategies/shared/tradoshka_strategy/signal.py`
- Create: `strategies/shared/tradoshka_strategy/base.py`
- Create: `strategies/shared/tradoshka_strategy/indicators.py`
- Create: `strategies/shared/tests/test_signal.py`
- Create: `strategies/shared/tests/test_indicators.py`

- [ ] **Step 1: Create pyproject.toml**

```toml
[project]
name = "tradoshka-strategy"
version = "0.1.0"
description = "Tradoshka strategy base classes and utilities"
requires-python = ">=3.10"
dependencies = []

[project.optional-dependencies]
dev = ["pytest>=8.0"]

[build-system]
requires = ["setuptools>=75.0"]
build-backend = "setuptools.build_meta"
```

- [ ] **Step 2: Create signal dataclass**

Create `strategies/shared/tradoshka_strategy/signal.py`:

```python
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum


class SignalDirection(Enum):
    LONG = "long"
    SHORT = "short"
    CLOSE = "close"
    HOLD = "hold"


@dataclass
class Signal:
    strategy_id: str
    symbol: str
    direction: SignalDirection
    strength: float  # 0.0 to 1.0
    stop_loss: float | None = None
    take_profit: float | None = None
    timestamp: datetime = field(default_factory=lambda: datetime.now(timezone.utc))
    metadata: dict[str, str] = field(default_factory=dict)

    def __post_init__(self):
        self.strength = max(0.0, min(1.0, self.strength))

    @property
    def is_actionable(self) -> bool:
        return self.direction in (SignalDirection.LONG, SignalDirection.SHORT, SignalDirection.CLOSE)
```

- [ ] **Step 3: Create base strategy class**

Create `strategies/shared/tradoshka_strategy/base.py`:

```python
from abc import ABC, abstractmethod
from .signal import Signal


class MarketEvent:
    """Simplified market event for Python strategies."""

    def __init__(self, symbol: str, price: float, volume: float, timestamp: float):
        self.symbol = symbol
        self.price = price
        self.volume = volume
        self.timestamp = timestamp


class Fill:
    """Simplified fill notification."""

    def __init__(self, symbol: str, side: str, price: float, quantity: float, fee: float):
        self.symbol = symbol
        self.side = side
        self.price = price
        self.quantity = quantity
        self.fee = fee


class BaseStrategy(ABC):
    """Base class for all Tradoshka strategies."""

    def __init__(self, strategy_id: str, market: str):
        self.strategy_id = strategy_id
        self.market = market
        self._enabled = True

    @abstractmethod
    def name(self) -> str:
        """Return the strategy name."""
        ...

    @abstractmethod
    def on_market_event(self, event: MarketEvent) -> Signal | None:
        """Process a market event and optionally return a trading signal."""
        ...

    def on_fill(self, fill: Fill) -> None:
        """Called when an order is filled. Override for position tracking."""
        pass

    @property
    def enabled(self) -> bool:
        return self._enabled

    def enable(self) -> None:
        self._enabled = True

    def disable(self) -> None:
        self._enabled = False
```

- [ ] **Step 4: Create common indicators**

Create `strategies/shared/tradoshka_strategy/indicators.py`:

```python
from collections import deque
import math


class SMA:
    """Simple Moving Average."""

    def __init__(self, period: int):
        self.period = period
        self._values: deque[float] = deque(maxlen=period)

    def update(self, value: float) -> float | None:
        self._values.append(value)
        if len(self._values) < self.period:
            return None
        return sum(self._values) / self.period

    @property
    def ready(self) -> bool:
        return len(self._values) >= self.period

    @property
    def value(self) -> float | None:
        if not self.ready:
            return None
        return sum(self._values) / self.period


class EMA:
    """Exponential Moving Average."""

    def __init__(self, period: int):
        self.period = period
        self._multiplier = 2.0 / (period + 1)
        self._value: float | None = None
        self._count = 0

    def update(self, value: float) -> float | None:
        self._count += 1
        if self._value is None:
            self._value = value
        else:
            self._value = (value - self._value) * self._multiplier + self._value
        if self._count < self.period:
            return None
        return self._value

    @property
    def ready(self) -> bool:
        return self._count >= self.period

    @property
    def value(self) -> float | None:
        if not self.ready:
            return None
        return self._value


class RSI:
    """Relative Strength Index."""

    def __init__(self, period: int = 14):
        self.period = period
        self._gains: deque[float] = deque(maxlen=period)
        self._losses: deque[float] = deque(maxlen=period)
        self._prev_value: float | None = None

    def update(self, value: float) -> float | None:
        if self._prev_value is not None:
            change = value - self._prev_value
            self._gains.append(max(0, change))
            self._losses.append(max(0, -change))
        self._prev_value = value

        if len(self._gains) < self.period:
            return None

        avg_gain = sum(self._gains) / self.period
        avg_loss = sum(self._losses) / self.period

        if avg_loss == 0:
            return 100.0
        rs = avg_gain / avg_loss
        return 100.0 - (100.0 / (1.0 + rs))

    @property
    def ready(self) -> bool:
        return len(self._gains) >= self.period


class ATR:
    """Average True Range — measures volatility."""

    def __init__(self, period: int = 14):
        self.period = period
        self._ranges: deque[float] = deque(maxlen=period)
        self._prev_close: float | None = None

    def update(self, high: float, low: float, close: float) -> float | None:
        if self._prev_close is not None:
            true_range = max(
                high - low,
                abs(high - self._prev_close),
                abs(low - self._prev_close),
            )
        else:
            true_range = high - low

        self._prev_close = close
        self._ranges.append(true_range)

        if len(self._ranges) < self.period:
            return None
        return sum(self._ranges) / self.period

    @property
    def ready(self) -> bool:
        return len(self._ranges) >= self.period

    @property
    def value(self) -> float | None:
        if not self.ready:
            return None
        return sum(self._ranges) / self.period


class BollingerBands:
    """Bollinger Bands — SMA ± std_dev_mult * stdev."""

    def __init__(self, period: int = 20, std_dev_mult: float = 2.0):
        self.period = period
        self.std_dev_mult = std_dev_mult
        self._values: deque[float] = deque(maxlen=period)

    def update(self, value: float) -> tuple[float, float, float] | None:
        """Returns (upper, middle, lower) or None."""
        self._values.append(value)
        if len(self._values) < self.period:
            return None
        middle = sum(self._values) / self.period
        variance = sum((v - middle) ** 2 for v in self._values) / self.period
        std_dev = math.sqrt(variance)
        upper = middle + self.std_dev_mult * std_dev
        lower = middle - self.std_dev_mult * std_dev
        return upper, middle, lower

    @property
    def ready(self) -> bool:
        return len(self._values) >= self.period
```

- [ ] **Step 5: Create __init__.py**

```python
from .signal import Signal, SignalDirection
from .base import BaseStrategy, MarketEvent, Fill
from .indicators import SMA, EMA, RSI, ATR, BollingerBands

__all__ = [
    "Signal", "SignalDirection",
    "BaseStrategy", "MarketEvent", "Fill",
    "SMA", "EMA", "RSI", "ATR", "BollingerBands",
]
```

- [ ] **Step 6: Write tests**

Create `strategies/shared/tests/test_signal.py`:

```python
from tradoshka_strategy import Signal, SignalDirection


def test_signal_clamps_strength():
    s = Signal("s1", "BTC", SignalDirection.LONG, 1.5)
    assert s.strength == 1.0
    s = Signal("s1", "BTC", SignalDirection.SHORT, -0.5)
    assert s.strength == 0.0


def test_signal_is_actionable():
    assert Signal("s1", "BTC", SignalDirection.LONG, 0.8).is_actionable
    assert Signal("s1", "BTC", SignalDirection.CLOSE, 0.5).is_actionable
    assert not Signal("s1", "BTC", SignalDirection.HOLD, 0.5).is_actionable
```

Create `strategies/shared/tests/test_indicators.py`:

```python
from tradoshka_strategy import SMA, EMA, RSI, ATR, BollingerBands


def test_sma():
    sma = SMA(3)
    assert sma.update(10) is None
    assert sma.update(20) is None
    assert sma.update(30) == 20.0  # (10+20+30)/3


def test_sma_rolling():
    sma = SMA(3)
    sma.update(10)
    sma.update(20)
    sma.update(30)
    assert sma.update(40) == 30.0  # (20+30+40)/3


def test_ema():
    ema = EMA(3)
    assert ema.update(10) is None
    assert ema.update(20) is None
    result = ema.update(30)
    assert result is not None
    assert result > 20  # EMA weights recent values more


def test_rsi_overbought():
    rsi = RSI(3)
    # Feed increasing prices
    rsi.update(10)
    rsi.update(20)
    rsi.update(30)
    result = rsi.update(40)
    assert result is not None
    assert result == 100.0  # All gains, no losses


def test_rsi_oversold():
    rsi = RSI(3)
    rsi.update(40)
    rsi.update(30)
    rsi.update(20)
    result = rsi.update(10)
    assert result is not None
    assert result == 0.0  # All losses, no gains


def test_atr():
    atr = ATR(3)
    assert atr.update(12, 10, 11) is None  # First bar, no prev close
    assert atr.update(13, 11, 12) is None
    result = atr.update(14, 10, 13)
    assert result is not None
    assert result > 0


def test_bollinger_bands():
    bb = BollingerBands(3, 2.0)
    assert bb.update(10) is None
    assert bb.update(10) is None
    result = bb.update(10)
    assert result is not None
    upper, middle, lower = result
    assert middle == 10.0
    assert upper == 10.0  # No variance → bands at middle
    assert lower == 10.0
```

- [ ] **Step 7: Run Python tests**

```bash
cd strategies/shared && pip install -e ".[dev]" && python -m pytest tests/ -v
```

Expected: all 9 tests pass

- [ ] **Step 8: Commit**

```bash
git add strategies/
git commit -m "feat(strategies): add Python strategy base classes and indicators

BaseStrategy ABC, Signal dataclass, SMA, EMA, RSI, ATR, Bollinger
Bands indicators. All with tests. Foundation for all market strategies."
```

---

### Task 14: CLAUDE.md System + Infrastructure

**Files:**
- Create: `CLAUDE.md`
- Create: `core/CLAUDE.md`
- Create: `strategies/CLAUDE.md`
- Create: `docs/MISTAKES.md`
- Create: `docs/RESEARCH.md`
- Create: `docs/BENCHMARKS.md`
- Create: `docs/GOALS.md`

- [ ] **Step 1: Create root CLAUDE.md**

```markdown
# Tradoshka — Claude Rules

## Project

Multi-market auto trading platform. Rust core + Python strategies + Next.js dashboard.

## Architecture

- `core/` — Rust workspace: common, data, risk, engine, api
- `markets/` — Per-market Rust adapters (polymarket, crypto, forex, stocks)
- `strategies/` — Python strategies per market
- `ai/` — Python AI/ML engine (mirofish, sentiment, regime, learning)
- `dashboard/` — Next.js + React + Tailwind
- `bridge/` — PyO3 Rust↔Python bridge

## Critical Rules

### Every feature must be best-in-market
Before building any feature, research competing implementations. Benchmark ours against them. If it doesn't meet the bar, rewrite it.

### Security: NEVER install external repos
- Study source code of external projects, reimplement from scratch
- Write our own API clients from official exchange documentation
- Only allow established, audited libraries (tokio, serde, PyTorch, scikit-learn)
- Pin exact versions, audit before any update
- NEVER commit API keys, secrets, or credentials

### Branch strategy
- Every feature on its own branch: `feature/<name>`
- PR-based workflow into `dev`, then `dev` → `main`
- Never push directly to `main`

### Testing
- TDD: write tests first, then implement
- All Rust code: `cargo test`
- All Python code: `pytest`
- Benchmark critical paths against competitors

### Documentation
- Update relevant CLAUDE.md files when changing architecture
- Log mistakes to `docs/MISTAKES.md` with root cause and prevention
- Update `docs/GOALS.md` after completing features
- Keep `docs/RESEARCH.md` updated with competitive findings

## Commands

```bash
# Build all
cargo build --workspace

# Test all Rust
cargo test --workspace

# Test Python
cd strategies/shared && python -m pytest tests/ -v

# Run API server (dry mode)
cargo run -p tradoshka-api

# Format
cargo fmt --all
cargo clippy --workspace
```

## Performance Targets

- Backtesting: 10M+ candles/sec (beat NautilusTrader's 5M)
- API latency: <1ms for REST, <100ms for WebSocket updates
- Order execution: <5ms from signal to order submission
```

- [ ] **Step 2: Create core/CLAUDE.md**

```markdown
# Core Rust Rules

## Dependencies (workspace-level)
All deps are pinned in root Cargo.toml. Never add deps without:
1. Justification (why we need it)
2. Security audit (check for known vulns)
3. Approval in PR

## Patterns
- Use `thiserror` for library errors, `anyhow` for binary errors
- All async code uses `tokio`
- Prefer `rust_decimal::Decimal` over f64 for money values
- Use `chrono::DateTime<Utc>` for all timestamps
- Traits in `common`, implementations in their respective crates

## Testing
- Unit tests inline with `#[cfg(test)]`
- Integration tests in `tests/` directory
- Use `rust_decimal_macros::dec!()` for test values
```

- [ ] **Step 3: Create strategies/CLAUDE.md**

```markdown
# Python Strategy Rules

## Structure
Every strategy must:
1. Extend `BaseStrategy`
2. Implement `name()` and `on_market_event()`
3. Return `Signal` objects (never raw values)
4. Have tests in the corresponding `tests/` directory

## Indicators
- Use indicators from `tradoshka_strategy.indicators`
- Never use external TA libraries (ta-lib, etc.) — write our own
- All indicators must be stateful (update-based, not batch)

## Testing
- Test with known input/output sequences
- Test edge cases (empty data, single data point, NaN)
- Test signal generation logic separately from indicator logic
```

- [ ] **Step 4: Create docs files**

Create `docs/MISTAKES.md`:
```markdown
# Mistakes Log

Document mistakes with root cause analysis and prevention rules.

| Date | Mistake | Root Cause | Prevention |
|------|---------|------------|------------|
| — | — | — | — |
```

Create `docs/RESEARCH.md`:
```markdown
# Competitive Research

## Tracked Competitors
- Freqtrade (~25K stars) — crypto, Python, FreqAI
- NautilusTrader — multi-asset, Rust+Python, 5M rows/sec
- QuantConnect/LEAN — multi-asset, C#+Python, cloud
- Hummingbot (~6K stars) — market making, Python
- Jesse (~5K stars) — crypto, Python, clean backtesting

## Research Log

| Date | Topic | Findings | Action |
|------|-------|----------|--------|
| 2026-03-26 | Initial research | See design spec | Architecture chosen |
```

Create `docs/BENCHMARKS.md`:
```markdown
# Performance Benchmarks

## Targets vs Competitors

| Metric | NautilusTrader | Freqtrade | Tradoshka Target | Tradoshka Actual |
|--------|---------------|-----------|-----------------|-----------------|
| Backtest throughput | 5M candles/s | ~10K/s | 10M+/s | TBD |
| Order latency | ~1ms | ~10ms | <5ms | TBD |
| Memory (1M candles) | ~200MB | ~500MB | <150MB | TBD |
```

Create `docs/GOALS.md`:
```markdown
# Goals & Progress

## Phase 0: Core Engine
- [x] Workspace setup
- [x] Common types and traits
- [x] Ring buffer
- [x] Candle aggregator
- [x] Half-Kelly position sizer
- [x] Circuit breaker + drawdown tracker
- [x] Risk manager
- [x] Order manager (state machine)
- [x] Portfolio tracker
- [x] Dry mode engine
- [x] API server (Axum REST + WebSocket)
- [x] Python strategy base + indicators
- [x] CLAUDE.md system
- [ ] PyO3 bridge (Phase 1)
- [ ] CI/CD pipeline

## Phase 1: Polymarket — Not started
## Phase 2: Crypto — Not started
## Phase 3: Forex — Not started
## Phase 4: Stocks — Not started
```

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md core/CLAUDE.md strategies/CLAUDE.md docs/
git commit -m "feat: add CLAUDE.md rules system and project documentation

Master rules, Rust rules, Python rules. Mistakes log, research tracker,
benchmarks, and goals tracking. Foundation for self-improving workflow."
```

---

### Task 15: Full Workspace Verification

- [ ] **Step 1: Run all Rust tests**

```bash
cargo test --workspace
```

Expected: all tests pass across all crates

- [ ] **Step 2: Run all Python tests**

```bash
cd strategies/shared && python -m pytest tests/ -v
```

Expected: all 9 tests pass

- [ ] **Step 3: Run clippy**

```bash
cargo clippy --workspace -- -D warnings
```

Expected: no warnings (fix any that appear)

- [ ] **Step 4: Run format check**

```bash
cargo fmt --all -- --check
```

Expected: all files formatted

- [ ] **Step 5: Merge to dev**

```bash
git checkout -b dev
git merge feature/core-engine
```

- [ ] **Step 6: Update GOALS.md**

Mark Phase 0 tasks as complete, update dates.

- [ ] **Step 7: Final commit**

```bash
git add docs/GOALS.md
git commit -m "docs: update goals — Phase 0 core engine complete"
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | Workspace setup | Compilation check |
| 2 | Common types | 8 tests |
| 3 | Common traits | Compilation check |
| 4 | Ring buffer | 8 tests |
| 5 | Candle aggregator | 4 tests |
| 6 | Half-Kelly sizer | 6 tests |
| 7 | Circuit breaker + drawdown | 10 tests |
| 8 | Risk manager | 4 tests |
| 9 | Order manager | 8 tests |
| 10 | Portfolio tracker | 6 tests |
| 11 | Dry mode engine | 7 tests |
| 12 | API server | Compilation check |
| 13 | Python strategy base | 9 tests |
| 14 | CLAUDE.md + docs | N/A |
| 15 | Full verification | All tests |

**Total: 70 Rust tests + 9 Python tests = 79 tests**

Next plan: **Phase 1 — Polymarket module** (adapter, AI predictor, copy trading, dry mode dashboard)
