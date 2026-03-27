# Tradoshka Session Log

## Session 2026-03-27

### What was built
- Evolution V2 with crossover breeding, random explorers, self-adjusting params
- 120 strategies (40 per market) trading 24/7
- Position monitor with stop losses, trailing stops, take profits
- Per-strategy indicator engines (real EMA/RSI/ATR/BB computation)
- Data persistence (JSONL for trades/evolution/snapshots)

### Bugs Found & Fixed
1. Tailwind v4 CSS not applying (wrong import syntax) — FIXED
2. Polymarket strategies never trading (research engine too strict) — FIXED
3. All strategies showing identical PnL (fake indicators) — FIXED
4. Positions never closing (position monitor not wired to strategy wallets) — FIXED
5. Evolution killing PM strategies (cross-market competition) — FIXED per-market evolution
6. 41 strategies idle at $100 (indicator warmup too slow) — FIXED with fallback defaults
7. API response times 2-5 seconds (lock contention) — FIXED with snapshot approach
8. PM strategies all identical (same trading logic for all) — FIXED (market subset filter + side selection + per-param sizing)

### Key Learnings
- Research engine must use REAL indicator values, not estimated/hardcoded ones
- Each strategy MUST have its own indicator state
- Position closing is essential for evolution evaluation
- Per-market evolution prevents unfair cross-market competition
- Random explorers prevent convergence to local optima
- 1-hour time stops force capital turnover for faster evaluation
- PM differentiation needs: (a) different market subsets, (b) different side selection, (c) different sizing from params

### Performance Results (after 6 evolution hours)
- Best: CP-funding-arb-wide-v2 at +$10.99 (73% WR, evolved variant!)
- Crypto strategies: +$24 total across spot+perps
- Polymarket: -$23 total (strategies not differentiated — bug)
- 67 strategies killed, evolution working

### Enhancement Vision
- Connect real wallet for live trading once strategies prove themselves
- Add more sophisticated crossover (multi-parent breeding)
- AI meta-learner that analyzes accumulated data to generate new strategies
- Multi-exchange support (Bybit, OKX after Binance proven)
- Real candle data for better indicator computation
- WebSocket-based real-time indicator updates instead of polling

### Checkpoint 2026-03-27_1324
- Alive: 120
- Dead: 10
- Capital: $12013
- Avg Sharpe: 69.83
- Best: CP-funding-arb-wide
