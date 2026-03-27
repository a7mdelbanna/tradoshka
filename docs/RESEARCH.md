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
| 2026-03-27 | Claude CLI for AI predictions | `claude -p` with Max subscription is officially supported for scripted automation. Third-party OAuth proxies (OpenClaw) are banned. `--bare` breaks Windows auth. `--json-schema` uses `structured_output` field and needs `--max-turns 2`. Haiku costs ~$0.02/cycle, Sonnet ~$0.09, Opus ~$0.75. Max sub: $0/extra (flat rate). | Switched from OpenAI API to Claude CLI |
| 2026-03-27 | Batch vs. per-agent LLM calls | Single batch prompt with 20 agent personas produces better diversity than 20 separate GPT-4o-mini calls. Claude simulates contrarian/bullish/bearish perspectives in one context window. 20x faster, 20x cheaper on quota. Trade-off: slight correlation between agents sharing context. | Implemented single batch approach |
| 2026-03-27 | DexScreener token discovery | Keyword search ("pepe","doge") returns established $1B tokens — 0 pass V2 entry criteria. `token-boosts/top/v1` returns actually trending tokens (mcap $50K-$5M, vol5m $1K-$50K). `token-profiles/latest/v1` finds recently listed. Batch fetch via comma-separated addresses. | Switched to boost+profile endpoints, keyword as fallback |
| 2026-03-27 | Dry mode fee realism | Polymarket fee formula `2% * min(p, 1-p)` applied to crypto (price=$600) → negative fees (-2%), fake profits. Must use market-specific fee_rate: spot 0.1%, perps 0.04%, PM 0.2%, meme 0.3% + 1% slippage. | Added `fee_rate` field to SimulatedWallet |
| 2026-03-28 | **Meme coin evolution analysis** | 6h live data, 5000+ trades: 3 random explorers made +4% to +78%, ALL 37 designed strategies lost -85% to -97%. Winners: tight stops (15-20%), quick TP (1.08-1.3x), 12-17 positions, 4-6x leverage. Losers: 50% stops, 2-5x TP, 5 positions, 1x leverage. Avg winner $6.07 vs avg loser $3.25 (R:R 1.87) at 30% WR = positive expectancy. Losers had R:R 0.78 = negative expectancy. Core insight: meme trend-riding = fast small wins, NOT moonshots. | Rewrote all 40 MC strategy params (V3 spec). See `docs/superpowers/specs/2026-03-28-meme-coin-v3-evolution-learned.md` |
| 2026-03-28 | Many small leveraged bets vs few large unleveraged | 17 positions at 3.7% each with 6x leverage beats 5 positions at 10% each with 1x. Single bad trade costs 3.7% vs 10% of capital. With 70% loss rate, smaller bets survive the losing streaks that meme coins guarantee. Leverage amplifies the 30% of trades that do win. | Default MC position count: 12-17, leverage: 3-6x |
