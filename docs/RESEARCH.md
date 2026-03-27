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
