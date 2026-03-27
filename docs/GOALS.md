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

## Phase 1: Polymarket — In Progress
## Phase 2: Crypto — Not started
## Phase 3: Forex — Not started
## Phase 4: Stocks — Not started
