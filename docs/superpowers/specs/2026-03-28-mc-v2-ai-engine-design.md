# MC-V2: AI-Powered Meme Coin Engine

**Date:** 2026-03-28
**Status:** Approved
**Goal:** Create a second meme coin engine (MC2) running alongside V1, powered by Claude AI for token scoring, entry decisions, exit recommendations, and self-improving feedback loops.

**Principle:** V1 stays untouched. MC2 shares infrastructure (DexScreener scanner, safety filter, Kelly sizing) but adds Claude AI as the decision layer. Evolution competes strategies that trust AI exits vs strategies that override them. Claude learns from its own mistakes via a feedback loop.

---

## 1. Architecture

### Separation from V1

| | MC-V1 (Classic) | MC-V2 (AI-Powered) |
|---|---|---|
| Prefix | `MC-` | `MC2-` |
| Market name | `meme_coins` | `meme_coins_v2` |
| Wallets | 40 ($100 each) | 40 ($100 each) |
| Entry decision | Numerical filters only | Safety filter → Claude scores → strategy threshold |
| Exit decision | Fixed SL/TP from params | Claude-suggested SL/TP OR fixed (per strategy) |
| Evolution | Independent hourly cycle | Independent hourly cycle |
| Scanner | DexScreener (shared) | Same scanner data (no extra API calls) |
| AI calls | None | 5-15 Claude calls per cycle (~$0.09 each) |

### Data Flow

```
DexScreener scan (shared with V1)
    → Safety + volume + liquidity filter (same as V1)
    → Claude scores each surviving token (~2-3 sec per token)
    → Cache score for 5 min per token address
    → Each MC2-* strategy checks: score >= min_ai_score? regime matches? verdict != AVOID?
    → If trust_ai_exits: use Claude's SL/TP. Else: use strategy's params.
    → Kelly position sizing
    → Execute trade + gas fee
    → Position monitor (same as V1, but SL/TP may be AI-sourced)
    → On close: record Claude prediction vs actual result
    → Every 10 cycles: Claude reviews its own predictions, generates lessons
    → Lessons prepended to future scoring prompts
```

---

## 2. Claude Token Scorer

### Prompt Template

```
You are a Solana meme coin analyst. Score this token for short-term trading potential.

Token: {name} ({symbol})
Address: {address}
Market Cap: ${mcap}
Liquidity: ${liquidity}
5min Volume: ${vol_5m}
24h Volume: ${vol_24h}
Price Change 5m: {change_5m}%
Price Change 1h: {change_1h}%
Price Change 24h: {change_24h}%
Token Age: {age_minutes} minutes
Safety Score: {safety}/100

{lessons_context}

Score 0-100 on:
1. Narrative freshness — is this concept original or a copycat?
2. Community signals — does volume/holders suggest real interest or wash trading?
3. Risk assessment — any red flags?
4. Timing — is this still early or already peaking?

Return JSON with: score (0-100), verdict (BUY/SKIP/AVOID), reasoning (one line),
regime (pump_phase/distribution/dead_cat/organic_growth),
suggested_stop_pct (number), suggested_target_pct (number).
```

### JSON Schema (enforced via `--json-schema`)

```json
{
  "type": "object",
  "properties": {
    "score": {"type": "integer", "minimum": 0, "maximum": 100},
    "verdict": {"type": "string", "enum": ["BUY", "SKIP", "AVOID"]},
    "reasoning": {"type": "string"},
    "regime": {"type": "string", "enum": ["pump_phase", "distribution", "dead_cat", "organic_growth"]},
    "suggested_stop_pct": {"type": "number", "minimum": 5, "maximum": 50},
    "suggested_target_pct": {"type": "number", "minimum": 5, "maximum": 100}
  },
  "required": ["score", "verdict", "reasoning", "regime", "suggested_stop_pct", "suggested_target_pct"]
}
```

### Execution

- CLI command: `claude -p "{prompt}" --system-prompt "{system}" --model sonnet --output-format json --json-schema "{schema}" --max-turns 2`
- Timeout: 10 seconds per call
- On failure/timeout: skip token (don't trade without AI approval), log error
- Cache: score cached 5 minutes by token address (HashMap<String, (CachedScore, Instant)>)

---

## 3. Strategy Definitions (40 Wallets)

### AI-Specific Params (new, added to standard StrategyParams)

| Param | Type | Range | Description |
|-------|------|-------|-------------|
| `min_ai_score` | f64 | 40-85 | Minimum Claude score to enter trade |
| `trust_ai_exits` | f64 | 0.0 or 1.0 | Use Claude's suggested SL/TP (1.0) or strategy's own (0.0) |
| `regime_filter` | f64 | Bitmask 1-15 | Which regimes to trade: 1=pump, 2=distribution, 4=dead_cat, 8=organic |

### Group A: AI Trend Riding (20) — prefix MC2-TR-

| Strategy | min_ai_score | trust_ai_exits | regime_filter | Other Key Params |
|----------|-------------|----------------|---------------|-----------------|
| MC2-TR-ai-sniper | 80 | 1.0 | 9 (pump+organic) | 15 pos, 5x lev, 15% SL |
| MC2-TR-ai-aggressive | 40 | 1.0 | 15 (all) | 17 pos, 6x lev, 20% SL |
| MC2-TR-ai-conservative | 85 | 1.0 | 8 (organic only) | 12 pos, 3x lev, 10% SL |
| MC2-TR-ai-override | 60 | 0.0 | 15 (all) | 15 pos, 5x lev, own 15% SL / 1.15x TP |
| MC2-TR-ai-pump-rider | 50 | 1.0 | 1 (pump only) | 15 pos, 5x lev, 12% SL |
| MC2-TR-ai-organic | 70 | 1.0 | 8 (organic only) | 14 pos, 4x lev, 12% SL |
| MC2-TR-ai-high-vol | 60 | 1.0 | 9 (pump+organic) | vol_5m >= 5000, 15 pos, 5x lev |
| MC2-TR-ai-micro-cap | 50 | 1.0 | 15 (all) | mcap <= 100K, 15 pos, 6x lev |
| MC2-TR-ai-mid-cap | 60 | 1.0 | 9 (pump+organic) | mcap <= 2M, 14 pos, 4x lev |
| MC2-TR-ai-balanced | 65 | 1.0 | 9 (pump+organic) | 14 pos, 5x lev, 15% SL |
| MC2-TR-ai-trust-test | 70 | 0.0 | 15 (all) | Tests fixed exits vs AI exits |
| MC2-TR-ai-tight-sl | 60 | 1.0 | 15 (all) | Override: max 10% SL even if AI says higher |
| MC2-TR-ai-wide-tp | 55 | 1.0 | 1 (pump only) | Override: min 30% TP even if AI says lower |
| MC2-TR-ai-fast-exit | 60 | 1.0 | 15 (all) | time_limit 10 min |
| MC2-TR-ai-patient | 75 | 1.0 | 8 (organic only) | time_limit 45 min |
| MC2-TR-ai-diversified | 50 | 1.0 | 15 (all) | 20 pos, 4x lev (many small bets) |
| MC2-TR-ai-concentrated | 80 | 1.0 | 9 (pump+organic) | 8 pos, 6x lev (few big bets) |
| MC2-TR-ai-momentum | 55 | 1.0 | 1 (pump only) | vol_5m >= 3000, 15 pos, 5x lev |
| MC2-TR-ai-safe | 70 | 1.0 | 8 (organic only) | min_safety 80, 12 pos, 3x lev |
| MC2-TR-ai-explorer | 55 | 1.0 | 15 (all) | Random AI params, evolution mutates |

### Group B: AI Copy Trading (20) — prefix MC2-CT-

Same structure as Group A but with copy trading logic — Claude validates whether a token being bought by tracked wallets is worth copying. Differentiated by min_ai_score thresholds, regime filters, and trust_ai_exits.

---

## 4. Claude Learning Loop

### Feedback Record (per closed trade)

```json
{
  "token_address": "abc123pump",
  "token_name": "FROGGY",
  "claude_score": 78,
  "claude_verdict": "BUY",
  "claude_regime": "pump_phase",
  "claude_reasoning": "Original frog concept, strong 5m volume surge...",
  "claude_suggested_stop": 12.0,
  "claude_suggested_tp": 25.0,
  "actual_pnl_pct": -18.5,
  "actual_exit_reason": "stop_loss",
  "actual_hold_mins": 8,
  "was_profitable": false,
  "strategy_used_ai_exits": true,
  "timestamp": "2026-03-28T15:00:00Z"
}
```

Stored in `data/mc2_ai_feedback/YYYY-MM-DD.jsonl` (one file per day).

### Learning Cycle (every 10 cycles / ~50 minutes)

1. Load last 20 closed MC2 trade feedbacks
2. Call Claude with feedback data:

```
Here are your last 20 meme coin predictions and what actually happened.

[feedback records as JSON array]

Analyze patterns in your mistakes and wins. Return JSON:
{
  "lessons": ["avoid X pattern because...", "Y signal was reliable because..."],
  "regime_accuracy": {"pump_phase": 0.35, "organic_growth": 0.62, ...},
  "overconfident_patterns": ["tokens with mcap < $50K and pump_phase"],
  "reliable_patterns": ["organic_growth + score > 75 had 65% win rate"],
  "updated_rules": ["Lower scores for pump_phase tokens by 10 points", ...]
}
```

3. Save lessons to `data/mc2_ai_lessons.json`
4. Prepend lessons to future scoring prompts as `{lessons_context}`:

```
LESSONS FROM YOUR PAST PREDICTIONS:
- You were overconfident on pump_phase tokens with mcap < $50K (25% win rate)
- organic_growth tokens scoring 75+ had 65% win rate — trust these more
- Tokens with "baby" or "inu" in name scored well but dumped 80% of the time
```

### Lesson Persistence

- `data/mc2_ai_lessons.json` loaded at startup
- Updated after each learning cycle (overwrite with latest)
- Max 20 lessons kept (oldest dropped when new ones added)
- Lessons are human-readable and visible in the dashboard

---

## 5. Dashboard Integration

### Trading Page

New sub-tabs under Meme Coins:
```
[V1 Classic 40] [V2 AI-Powered 40]
```

Within V2:
```
[All 40] [Trend Riding 20] [Copy Trading 20]
```

### Trade Feed (V2 trades show AI context)

```
BUY  Long  FROGGY Meme
Score: 78/100 | Regime: pump_phase | AI SL: 12% TP: 25%
"Original frog concept, strong 5m volume surge..."
```

### AI Learning Panel (new section on MC2 strategy detail)

```
AI PREDICTION ACCURACY
  Last 20 predictions: 7 correct, 13 wrong (35%)

  By Regime:
    pump_phase:     2/8  (25%) — overconfident
    organic_growth: 4/6  (67%) — reliable
    distribution:   1/4  (25%) — avoid
    dead_cat:       0/2  (0%)  — avoid

LATEST LESSONS (3):
  1. "Tokens with mcap < $50K in pump_phase regime dump 75% of the time"
  2. "organic_growth tokens with vol_24h > $100K are the most reliable"
  3. "Avoid tokens where 5m change > 20% — usually already peaking"
```

### Evolution Page

- `meme_coins_v2` appears as a separate market filter
- Sub-tabs: `[All MC2] [Trend Riding] [Copy Trading]`
- Strategy params show AI-specific fields (min_ai_score, trust_ai_exits, regime_filter)

---

## 6. Implementation in Rust

### New Module: `core/engine/src/claude_scorer.rs`

```rust
pub struct ClaudeScorer {
    cache: HashMap<String, (TokenScore, Instant)>,
    lessons: Vec<String>,
    feedback: Vec<TradeFeedback>,
    cycle_count: u32,
}

pub struct TokenScore {
    pub score: u32,
    pub verdict: String,        // BUY / SKIP / AVOID
    pub reasoning: String,
    pub regime: String,         // pump_phase / distribution / dead_cat / organic_growth
    pub suggested_stop_pct: f64,
    pub suggested_target_pct: f64,
    pub scored_at: DateTime<Utc>,
}

pub struct TradeFeedback {
    pub token_name: String,
    pub claude_score: u32,
    pub claude_regime: String,
    pub claude_reasoning: String,
    pub claude_stop: f64,
    pub claude_tp: f64,
    pub actual_pnl_pct: f64,
    pub actual_exit_reason: String,
    pub was_profitable: bool,
    pub timestamp: DateTime<Utc>,
}
```

### Claude Call (subprocess)

```rust
impl ClaudeScorer {
    pub async fn score_token(&mut self, token: &MemeToken, safety_score: u32) -> Option<TokenScore> {
        // Check cache first
        if let Some((cached, instant)) = self.cache.get(&token.address) {
            if instant.elapsed() < Duration::from_secs(300) {
                return Some(cached.clone());
            }
        }
        // Build prompt, call claude -p, parse JSON
        // Cache result, return score
    }

    pub async fn learn_from_feedback(&mut self) {
        // Every 10 cycles, call Claude with last 20 feedbacks
        // Update self.lessons, save to data/mc2_ai_lessons.json
    }
}
```

### Strategy Routing in main.rs

New block after MC1 trading:

```rust
// --- MC2 AI-powered strategy trading ---
// 1. Reuse tokens from MC1 scanner
// 2. Apply safety/volume filter
// 3. Score with Claude (cached)
// 4. Route to MC2-* strategies
// Same position monitor as MC1 but with AI-sourced SL/TP
```

### market routing in strategy_wallet.rs

Add `MC2-` prefix detection:
```rust
} else if name.starts_with("MC2-") {
    "meme_coins_v2"
}
```

---

## 7. Implementation Order

| Phase | What | Depends On |
|-------|------|-----------|
| 1 | `claude_scorer.rs` — ClaudeScorer struct, score_token(), cache, JSON parsing | Nothing |
| 2 | 40 MC2 strategy definitions in mutation.rs | Phase 1 |
| 3 | Wire MC2 trading block in main.rs (reuse scanner, add Claude scoring) | Phase 1+2 |
| 4 | Feedback recording + learning loop | Phase 3 |
| 5 | Dashboard: V1/V2 tabs, AI context in trade feed, learning panel | Phase 3+4 |
| 6 | Evolution page: meme_coins_v2 market filter | Phase 2 |

---

## 8. Cost Estimate

| Item | Per Cycle (5 min) | Per Hour | Per Day |
|------|-------------------|----------|---------|
| Claude scoring (~10 tokens × Sonnet) | ~$0.90 | ~$10.80 | ~$259 |
| Claude learning (1 call / 10 cycles) | ~$0.09 | ~$0.11 | ~$2.59 |
| **Total** | **~$0.99** | **~$10.91** | **~$262** |

With Max subscription (flat rate): $0 extra cost — all included.
Without Max: ~$262/day — significant, but the system is designed to prove ROI.
