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
