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
