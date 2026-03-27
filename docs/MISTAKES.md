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
