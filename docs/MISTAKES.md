# Mistakes Log

Document mistakes with root cause analysis and prevention rules.

| Date | Mistake | Root Cause | Prevention |
|------|---------|------------|------------|
| — | — | — | — |
| 2026-03-27 | Tailwind v4 CSS wrong syntax | Used v3 directives with v4 — zero styles applied | Check framework version before writing CSS |
| 2026-03-27 | Fake indicators for strategies | Hardcoded EMA/RSI values instead of computing real ones | Always compute from actual data |
| 2026-03-27 | Positions never closing | Position monitor not wired to strategy wallets | Wire ALL wallet types to position monitor |
| 2026-03-27 | Cross-market evolution | PM strategies killed by crypto performance | Always evolve within same market |
| 2026-03-27 | PM strategies identical | All PM strategies use same trading logic | Each strategy must use its params for differentiation |
| 2026-03-27 | API lock contention | Write lock held too long, blocking reads | Collect data quickly, release lock, then serialize |
| 2026-03-27 | `--bare` flag breaks auth on Windows | `claude -p --bare` skips credential loading, returns "Not logged in" | Never use `--bare` with `claude -p` on Windows; use `--system-prompt` to override project context instead |
| 2026-03-27 | `--max-turns 1` breaks `--json-schema` | `--json-schema` is implemented as a tool internally, needs 2 turns to complete | Always use `--max-turns 2` (or more) when using `--json-schema` with `claude -p` |
| 2026-03-27 | `--json-schema` output in `structured_output` not `result` | Assumed CLI puts JSON in `result` field like regular responses | When using `--json-schema`, parse from `response["structured_output"]`, not `response["result"]` |
| 2026-03-27 | `except Exception` in fallback masks bugs | Catching all exceptions hides logic bugs (TypeError, ValueError) behind silent fallback | Use `except RuntimeError` to only catch infrastructure errors; let logic bugs crash loudly |
| 2026-03-27 | Unseeded RNG in statistical fallback | `random.Random()` without seed makes fallback non-deterministic even when seed is provided | Store seed in `self._seed` and pass it to `random.Random(self._seed)` |
| 2026-03-27 | Negative crypto trade fees (-2%) | Polymarket fee formula `2% * min(price, 1-price)` used for all markets. Crypto price=$600 → `1-600=-599` → negative fee | Every market MUST have its own `fee_rate` field. Never share fee formulas across markets |
| 2026-03-27 | MC strategies routed to crypto_perps | `initialize_defaults()` only checked PM-/CS- prefixes, defaulting MC- to crypto_perps | Always add explicit prefix checks for every market type |
| 2026-03-27 | DexScreener keyword search finds wrong tokens | Search "pepe" returns established $1B tokens, not fresh meme launches. Zero tokens pass entry criteria | Use `token-boosts` and `token-profiles` endpoints for trending tokens, not keyword search |
| 2026-03-27 | MC safety test assertion off-by-one | quick_check gives score 85 (all pass except mint_revoked=15pts), test asserted `< 85` | Calculate exact expected score from parameters, use `assert_eq!` not range checks |
| 2026-03-28 | **MC strategy params catastrophically wrong** | V2 spec designed 50% stops, 2-5x TP, 5 positions, 1x leverage based on sniper theory. But system does trend-riding, not sniping. All 37 designed strategies lost 85-97%. Random explorers with tight stops and quick TP made +78%. | NEVER design strategy params from theory alone. Run evolution first, study winners, then apply their params. The market tells you what works — listen to the data, not the spec |
| 2026-03-28 | Meme coin exit targets unrealistic | V2 profit ladder (30% at 2x, 30% at 5x, 40% trailing) was for snipe plays at token launch. Trend riders enter AFTER the initial pump — waiting for 2x from there rarely works | Match exit strategy to entry strategy. Snipers can wait for 5x. Trend riders should take 10-30% and move on |
