# Goals & Progress

## Phase 0: Core Engine — COMPLETE
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

## Phase 0.5: Claude CLI Integration — COMPLETE
- [x] Replace OpenAI HTTP client with Claude Code CLI (`claude -p`)
- [x] Single batch prompt (20 agents in 1 call vs. 20 sequential calls)
- [x] JSON schema enforcement via `--json-schema`
- [x] Statistical fallback when CLI unavailable
- [x] Seeded RNG for deterministic fallback
- [x] 9 unit tests (ClaudeClient + Simulation)
- [x] Live smoke test verified with Max subscription
- [ ] Enhance AgentFactory with Claude-powered personas
- [ ] Claude integration for crypto strategies
- [ ] Opus vs. Sonnet prediction quality comparison

## Phase 1: Polymarket — COMPLETE
- [x] Polymarket adapter (CLOB + Gamma + Data APIs)
- [x] 20 copy trading strategies (PM-CT-*)
- [x] 20 AI niche strategies (PM-AI-*)
- [x] Copy trading engine with wallet scoring
- [x] Basket consensus (topic-based)
- [x] 4-layer circuit breaker
- [x] Per-market evolution (PM evolves independently)

## Phase 2: Crypto — COMPLETE
- [x] Binance adapter (spot + USDT-M futures)
- [x] 40 crypto spot strategies (CS-*)
- [x] 40 crypto perps strategies (CP-*)
- [x] Market-aware fees (0.1% spot, 0.04% perps)
- [x] Per-strategy indicator engines (real EMA/RSI/ATR/BB)
- [x] Research-first trade decisions (TradeThesis)

## Phase 3: Meme Coins (Solana DEX) — COMPLETE
- [x] DexScreener adapter (boost + profile + search endpoints)
- [x] Token scanner with 3-channel discovery
- [x] 6-point safety filter + blacklist
- [x] Volume tracker (5-min rolling)
- [x] 20 trend riding strategies (MC-TR-*)
- [x] 20 copy trading strategies (MC-CT-*)
- [x] V3 evolution-learned params (tight stops, quick TP, many positions, leverage)
- [ ] Volume-based exit in position monitor
- [ ] Holder velocity tracking
- [ ] Dev wallet monitoring (Solana RPC)
- [ ] Claude AI token scoring integration

## Phase 4: Dashboard — COMPLETE
- [x] Next.js + Tailwind v4 dark theme
- [x] Market tabs (Polymarket, Crypto Spot/Perps, Meme Coins)
- [x] Strategy wallet picker with PnL
- [x] Evolution page (leaderboard, timeline, graveyard)
- [x] Copy trading page
- [x] Live trade feed

## Trading Infrastructure — COMPLETE
- [x] 160 strategy wallets ($100 each, $16K total)
- [x] Darwinian evolution (hourly kill/spawn per market)
- [x] SimulatedWallet with market-aware fees
- [x] Data persistence (JSONL)
- [x] Health check system
- [x] Scheduled remote health agent (hourly)
- [ ] Database persistence (SQLite)

## Phase 5: Forex — Not started
## Phase 6: Stocks — Not started
