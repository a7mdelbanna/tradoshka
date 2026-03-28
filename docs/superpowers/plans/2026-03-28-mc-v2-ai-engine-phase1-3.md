# MC-V2 AI-Powered Meme Coin Engine — Phase 1-3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an AI-powered meme coin trading engine (MC2) that runs alongside the existing V1, using Claude Sonnet to score tokens before trading. 40 strategy wallets with AI-specific params, separate evolution cycle.

**Architecture:** New `claude_scorer.rs` module calls `claude -p` CLI to score tokens post-safety-filter. Scores cached 5 min. 40 MC2-* strategies defined in mutation.rs with AI params (min_ai_score, trust_ai_exits, regime_filter). New trading block in main.rs reuses V1's scanner data but gates every trade on Claude's verdict. Market name `meme_coins_v2`, prefix `MC2-`.

**Tech Stack:** Rust (subprocess call to `claude -p` CLI), existing DexScreener scanner, Kelly position sizing, evolution engine.

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `core/engine/src/claude_scorer.rs` | Create | ClaudeScorer struct, score_token(), cache, JSON parsing |
| `core/engine/src/lib.rs` | Modify | Export ClaudeScorer, TokenScore |
| `core/engine/src/mutation.rs` | Modify | Add 40 MC2-* strategy definitions, update count to 200 |
| `core/engine/src/strategy_wallet.rs` | Modify | Add MC2- prefix routing to meme_coins_v2 |
| `core/api/src/state.rs` | Modify | Add ClaudeScorer to AppState |
| `core/api/src/main.rs` | Modify | Add MC2 trading block, wire Claude scoring |
| `core/engine/src/evolution.rs` | Modify | Add "MC2-" to evolution prefix list |

---

### Task 1: Create ClaudeScorer module

**Files:**
- Create: `core/engine/src/claude_scorer.rs`
- Modify: `core/engine/src/lib.rs`

- [ ] **Step 1: Create `claude_scorer.rs` with types and tests**

Create `core/engine/src/claude_scorer.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};

/// Score returned by Claude for a meme coin token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenScore {
    pub score: u32,
    pub verdict: String,
    pub reasoning: String,
    pub regime: String,
    pub suggested_stop_pct: f64,
    pub suggested_target_pct: f64,
    pub scored_at: DateTime<Utc>,
}

/// Feedback record: Claude's prediction vs actual trade result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeFeedback {
    pub token_name: String,
    pub token_address: String,
    pub claude_score: u32,
    pub claude_verdict: String,
    pub claude_regime: String,
    pub claude_reasoning: String,
    pub claude_stop: f64,
    pub claude_tp: f64,
    pub actual_pnl_pct: f64,
    pub actual_exit_reason: String,
    pub was_profitable: bool,
    pub strategy_used_ai_exits: bool,
    pub timestamp: DateTime<Utc>,
}

/// JSON schema for Claude's token scoring response.
const TOKEN_SCORE_SCHEMA: &str = r#"{
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
}"#;

pub struct ClaudeScorer {
    cache: HashMap<String, (TokenScore, Instant)>,
    cache_ttl: Duration,
    lessons: Vec<String>,
    feedback: Vec<TradeFeedback>,
    cycle_count: u32,
    available: bool,
}

impl ClaudeScorer {
    pub fn new() -> Self {
        // Check if claude CLI is available at startup
        let available = Command::new("claude")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if available {
            tracing::info!("ClaudeScorer: claude CLI available, AI scoring enabled");
        } else {
            tracing::warn!("ClaudeScorer: claude CLI not found, AI scoring disabled");
        }

        Self {
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(300), // 5 min cache
            lessons: Vec::new(),
            feedback: Vec::new(),
            cycle_count: 0,
            available,
        }
    }

    /// Check if Claude CLI is available.
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Get cached score if still valid.
    pub fn get_cached(&self, token_address: &str) -> Option<&TokenScore> {
        self.cache.get(token_address).and_then(|(score, instant)| {
            if instant.elapsed() < self.cache_ttl {
                Some(score)
            } else {
                None
            }
        })
    }

    /// Score a token using Claude CLI. Returns None if CLI unavailable or call fails.
    pub fn score_token(
        &mut self,
        token_name: &str,
        token_symbol: &str,
        token_address: &str,
        mcap: f64,
        liquidity: f64,
        vol_5m: f64,
        vol_24h: f64,
        change_5m: f64,
        change_1h: f64,
        change_24h: f64,
        age_minutes: i64,
        safety_score: u32,
    ) -> Option<TokenScore> {
        if !self.available {
            return None;
        }

        // Check cache first
        if let Some(cached) = self.get_cached(token_address) {
            return Some(cached.clone());
        }

        // Build lessons context
        let lessons_ctx = if self.lessons.is_empty() {
            String::new()
        } else {
            format!(
                "\nLESSONS FROM YOUR PAST PREDICTIONS:\n{}",
                self.lessons.iter().enumerate()
                    .map(|(i, l)| format!("{}. {}", i + 1, l))
                    .collect::<Vec<_>>().join("\n")
            )
        };

        let prompt = format!(
            "Token: {} ({})\nAddress: {}\nMarket Cap: ${:.0}\nLiquidity: ${:.0}\n\
             5min Volume: ${:.0}\n24h Volume: ${:.0}\n\
             Price Change 5m: {:.1}%\nPrice Change 1h: {:.1}%\nPrice Change 24h: {:.1}%\n\
             Token Age: {} minutes\nSafety Score: {}/100\n{}",
            token_name, token_symbol, token_address,
            mcap, liquidity, vol_5m, vol_24h,
            change_5m, change_1h, change_24h,
            age_minutes, safety_score, lessons_ctx
        );

        let system = "You are a Solana meme coin analyst. Score this token for short-term trading potential. \
            Score 0-100 on: 1) Narrative freshness (original or copycat?), 2) Community signals (real interest or wash?), \
            3) Risk assessment (red flags?), 4) Timing (early or peaking?). \
            Return JSON with score, verdict (BUY/SKIP/AVOID), reasoning (one line), \
            regime (pump_phase/distribution/dead_cat/organic_growth), \
            suggested_stop_pct, suggested_target_pct.";

        let output = Command::new("claude")
            .args([
                "-p", &prompt,
                "--system-prompt", system,
                "--model", "sonnet",
                "--output-format", "json",
                "--json-schema", TOKEN_SCORE_SCHEMA,
                "--max-turns", "2",
            ])
            .output();

        match output {
            Ok(result) if result.status.success() => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                match serde_json::from_str::<serde_json::Value>(&stdout) {
                    Ok(json) => {
                        // Claude CLI with --json-schema puts output in "structured_output"
                        let data = json.get("structured_output")
                            .or_else(|| json.get("result"))
                            .unwrap_or(&json);

                        // Parse the inner data (might be a string that needs parsing)
                        let parsed: serde_json::Value = if data.is_string() {
                            serde_json::from_str(data.as_str().unwrap_or("{}")).unwrap_or_default()
                        } else {
                            data.clone()
                        };

                        let score = TokenScore {
                            score: parsed["score"].as_u64().unwrap_or(0) as u32,
                            verdict: parsed["verdict"].as_str().unwrap_or("SKIP").to_string(),
                            reasoning: parsed["reasoning"].as_str().unwrap_or("").to_string(),
                            regime: parsed["regime"].as_str().unwrap_or("pump_phase").to_string(),
                            suggested_stop_pct: parsed["suggested_stop_pct"].as_f64().unwrap_or(15.0),
                            suggested_target_pct: parsed["suggested_target_pct"].as_f64().unwrap_or(20.0),
                            scored_at: Utc::now(),
                        };

                        tracing::info!(
                            "Claude scored {} ({}): {}/100 {} | regime={} | SL={:.0}% TP={:.0}% | {}",
                            token_symbol, token_name, score.score, score.verdict,
                            score.regime, score.suggested_stop_pct, score.suggested_target_pct,
                            score.reasoning.chars().take(60).collect::<String>()
                        );

                        self.cache.insert(token_address.to_string(), (score.clone(), Instant::now()));
                        Some(score)
                    }
                    Err(e) => {
                        tracing::warn!("Claude JSON parse error: {} | raw: {}", e, &stdout[..stdout.len().min(200)]);
                        None
                    }
                }
            }
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                tracing::warn!("Claude CLI error (code {}): {}", result.status, &stderr[..stderr.len().min(200)]);
                None
            }
            Err(e) => {
                tracing::warn!("Claude CLI exec error: {}", e);
                None
            }
        }
    }

    /// Record a trade feedback for the learning loop.
    pub fn record_feedback(&mut self, feedback: TradeFeedback) {
        self.feedback.push(feedback);
    }

    /// Get all accumulated feedback records.
    pub fn feedback(&self) -> &[TradeFeedback] {
        &self.feedback
    }

    /// Get current lessons.
    pub fn lessons(&self) -> &[String] {
        &self.lessons
    }

    /// Set lessons (loaded from file or from learning cycle).
    pub fn set_lessons(&mut self, lessons: Vec<String>) {
        self.lessons = lessons;
    }

    /// Increment cycle counter.
    pub fn tick_cycle(&mut self) {
        self.cycle_count += 1;
    }

    /// Current cycle count.
    pub fn cycle_count(&self) -> u32 {
        self.cycle_count
    }

    /// Clear expired cache entries.
    pub fn clear_expired_cache(&mut self) {
        self.cache.retain(|_, (_, instant)| instant.elapsed() < self.cache_ttl);
    }

    /// Cache size.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_scorer() {
        let scorer = ClaudeScorer::new();
        assert_eq!(scorer.cache_size(), 0);
        assert_eq!(scorer.cycle_count(), 0);
        assert!(scorer.lessons().is_empty());
        assert!(scorer.feedback().is_empty());
    }

    #[test]
    fn test_cache_miss_on_empty() {
        let scorer = ClaudeScorer::new();
        assert!(scorer.get_cached("nonexistent").is_none());
    }

    #[test]
    fn test_cache_insert_and_retrieve() {
        let mut scorer = ClaudeScorer::new();
        let score = TokenScore {
            score: 75,
            verdict: "BUY".into(),
            reasoning: "Test".into(),
            regime: "pump_phase".into(),
            suggested_stop_pct: 12.0,
            suggested_target_pct: 25.0,
            scored_at: Utc::now(),
        };
        scorer.cache.insert("tok1".into(), (score, Instant::now()));
        assert!(scorer.get_cached("tok1").is_some());
        assert_eq!(scorer.get_cached("tok1").unwrap().score, 75);
        assert_eq!(scorer.cache_size(), 1);
    }

    #[test]
    fn test_cache_expires() {
        let mut scorer = ClaudeScorer::new();
        scorer.cache_ttl = Duration::from_millis(1); // 1ms TTL for test
        let score = TokenScore {
            score: 50,
            verdict: "SKIP".into(),
            reasoning: "Test".into(),
            regime: "dead_cat".into(),
            suggested_stop_pct: 15.0,
            suggested_target_pct: 20.0,
            scored_at: Utc::now(),
        };
        scorer.cache.insert("tok2".into(), (score, Instant::now()));
        std::thread::sleep(Duration::from_millis(5));
        assert!(scorer.get_cached("tok2").is_none()); // Expired
    }

    #[test]
    fn test_feedback_recording() {
        let mut scorer = ClaudeScorer::new();
        let fb = TradeFeedback {
            token_name: "FROGGY".into(),
            token_address: "abc".into(),
            claude_score: 78,
            claude_verdict: "BUY".into(),
            claude_regime: "pump_phase".into(),
            claude_reasoning: "Good concept".into(),
            claude_stop: 12.0,
            claude_tp: 25.0,
            actual_pnl_pct: -18.5,
            actual_exit_reason: "stop_loss".into(),
            was_profitable: false,
            strategy_used_ai_exits: true,
            timestamp: Utc::now(),
        };
        scorer.record_feedback(fb);
        assert_eq!(scorer.feedback().len(), 1);
        assert!(!scorer.feedback()[0].was_profitable);
    }

    #[test]
    fn test_lessons_management() {
        let mut scorer = ClaudeScorer::new();
        scorer.set_lessons(vec!["Avoid pump_phase < $50K mcap".into()]);
        assert_eq!(scorer.lessons().len(), 1);
        scorer.set_lessons(vec!["Rule 1".into(), "Rule 2".into()]);
        assert_eq!(scorer.lessons().len(), 2);
    }

    #[test]
    fn test_cycle_counter() {
        let mut scorer = ClaudeScorer::new();
        assert_eq!(scorer.cycle_count(), 0);
        scorer.tick_cycle();
        scorer.tick_cycle();
        assert_eq!(scorer.cycle_count(), 2);
    }

    #[test]
    fn test_clear_expired_cache() {
        let mut scorer = ClaudeScorer::new();
        scorer.cache_ttl = Duration::from_millis(1);
        let score = TokenScore {
            score: 60,
            verdict: "BUY".into(),
            reasoning: "Test".into(),
            regime: "organic_growth".into(),
            suggested_stop_pct: 15.0,
            suggested_target_pct: 20.0,
            scored_at: Utc::now(),
        };
        scorer.cache.insert("old".into(), (score.clone(), Instant::now()));
        std::thread::sleep(Duration::from_millis(5));
        scorer.cache.insert("new".into(), (score, Instant::now()));
        scorer.clear_expired_cache();
        assert_eq!(scorer.cache_size(), 1); // Only "new" survives
        assert!(scorer.get_cached("old").is_none());
        assert!(scorer.get_cached("new").is_some());
    }
}
```

- [ ] **Step 2: Add module declaration to engine lib.rs**

In `core/engine/src/lib.rs`, add the module and exports. Add the module declaration alongside existing modules:

```rust
pub mod claude_scorer;
```

And add the pub use export:

```rust
pub use claude_scorer::{ClaudeScorer, TokenScore, TradeFeedback};
```

- [ ] **Step 3: Run tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p tradoshka-engine -- claude_scorer 2>&1`

Expected: All 7 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add core/engine/src/claude_scorer.rs core/engine/src/lib.rs
git commit -m "feat(engine): add ClaudeScorer module with caching, feedback, and tests"
```

---

### Task 2: Add 40 MC2 strategy definitions

**Files:**
- Modify: `core/engine/src/mutation.rs`

- [ ] **Step 1: Add MC2-TR and MC2-CT strategies to initial_strategies()**

At the end of the `vec![]` in `initial_strategies()`, before the closing `]`, add:

```rust
        // ═══ MC-V2 AI-Powered — Trend Riding (20) — prefix MC2-TR- ═══
        // Claude scores tokens 0-100. Strategies vary by min_ai_score, trust_ai_exits, regime_filter.
        // regime_filter bitmask: 1=pump_phase, 2=distribution, 4=dead_cat, 8=organic_growth
        ("MC2-TR-ai-sniper".into(),       StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 80.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 9.0).with_param("min_volume_5m", 3000.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.20).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-TR-ai-aggressive".into(),   StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 40.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_volume_5m", 1000.0).with_param("min_safety", 30.0).with_param("max_mcap", 10000000.0).with_param("hard_stop_pct", 20.0).with_param("target_mult", 1.25).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 17.0).with_param("capital_usage_pct", 70.0).with_param("auto_leverage", 6.0)),
        ("MC2-TR-ai-conservative".into(), StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 85.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 8.0).with_param("min_volume_5m", 3000.0).with_param("min_safety", 70.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 10.0).with_param("target_mult", 1.12).with_param("time_limit_mins", 30.0).with_param("auto_position_count", 12.0).with_param("capital_usage_pct", 55.0).with_param("auto_leverage", 3.0)),
        ("MC2-TR-ai-override".into(),     StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 60.0).with_param("trust_ai_exits", 0.0).with_param("regime_filter", 15.0).with_param("min_volume_5m", 2000.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-TR-ai-pump-rider".into(),   StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 50.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 1.0).with_param("min_volume_5m", 3000.0).with_param("min_safety", 40.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 12.0).with_param("target_mult", 1.20).with_param("time_limit_mins", 15.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 65.0).with_param("auto_leverage", 5.0)),
        ("MC2-TR-ai-organic".into(),      StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 70.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 8.0).with_param("min_volume_5m", 2000.0).with_param("min_safety", 60.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 12.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 30.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 4.0)),
        ("MC2-TR-ai-high-vol".into(),     StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 60.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 9.0).with_param("min_volume_5m", 5000.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.18).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-TR-ai-micro-cap".into(),    StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 50.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_volume_5m", 1000.0).with_param("min_safety", 40.0).with_param("max_mcap", 100000.0).with_param("hard_stop_pct", 20.0).with_param("target_mult", 1.30).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 6.0)),
        ("MC2-TR-ai-mid-cap".into(),      StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 60.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 9.0).with_param("min_volume_5m", 3000.0).with_param("min_safety", 50.0).with_param("max_mcap", 2000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 4.0)),
        ("MC2-TR-ai-balanced".into(),     StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 65.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 9.0).with_param("min_volume_5m", 2000.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-TR-ai-trust-test".into(),   StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 70.0).with_param("trust_ai_exits", 0.0).with_param("regime_filter", 15.0).with_param("min_volume_5m", 2000.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-TR-ai-tight-sl".into(),     StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 60.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_volume_5m", 2000.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 10.0).with_param("target_mult", 1.12).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 4.0)),
        ("MC2-TR-ai-wide-tp".into(),      StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 55.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 1.0).with_param("min_volume_5m", 3000.0).with_param("min_safety", 40.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 18.0).with_param("target_mult", 1.30).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-TR-ai-fast-exit".into(),    StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 60.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_volume_5m", 2000.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 12.0).with_param("target_mult", 1.12).with_param("time_limit_mins", 10.0).with_param("auto_position_count", 17.0).with_param("capital_usage_pct", 65.0).with_param("auto_leverage", 4.0)),
        ("MC2-TR-ai-patient".into(),      StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 75.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 8.0).with_param("min_volume_5m", 2000.0).with_param("min_safety", 60.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.20).with_param("time_limit_mins", 45.0).with_param("auto_position_count", 12.0).with_param("capital_usage_pct", 55.0).with_param("auto_leverage", 4.0)),
        ("MC2-TR-ai-diversified".into(),  StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 50.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_volume_5m", 1000.0).with_param("min_safety", 40.0).with_param("max_mcap", 10000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 20.0).with_param("capital_usage_pct", 65.0).with_param("auto_leverage", 4.0)),
        ("MC2-TR-ai-concentrated".into(), StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 80.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 9.0).with_param("min_volume_5m", 5000.0).with_param("min_safety", 60.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 18.0).with_param("target_mult", 1.25).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 8.0).with_param("capital_usage_pct", 55.0).with_param("auto_leverage", 6.0)),
        ("MC2-TR-ai-momentum".into(),     StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 55.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 1.0).with_param("min_volume_5m", 5000.0).with_param("min_safety", 40.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 12.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 15.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 65.0).with_param("auto_leverage", 5.0)),
        ("MC2-TR-ai-safe".into(),         StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 70.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 8.0).with_param("min_volume_5m", 3000.0).with_param("min_safety", 80.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 10.0).with_param("target_mult", 1.10).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 12.0).with_param("capital_usage_pct", 55.0).with_param("auto_leverage", 3.0)),
        ("MC2-TR-ai-explorer".into(),     StrategyParams::new("mc2_ai_trend").with_param("min_ai_score", 55.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_volume_5m", 2000.0).with_param("min_safety", 40.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.18).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 65.0).with_param("auto_leverage", 5.0)),

        // ═══ MC-V2 AI-Powered — Copy Trading (20) — prefix MC2-CT- ═══
        ("MC2-CT-ai-top-pnl".into(),      StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 60.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 9.0).with_param("min_safety", 40.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-CT-ai-selective".into(),     StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 80.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 8.0).with_param("min_safety", 60.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 12.0).with_param("target_mult", 1.12).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 12.0).with_param("capital_usage_pct", 55.0).with_param("auto_leverage", 4.0)),
        ("MC2-CT-ai-fast".into(),         StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 50.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 1.0).with_param("min_safety", 40.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 10.0).with_param("target_mult", 1.10).with_param("time_limit_mins", 10.0).with_param("auto_position_count", 17.0).with_param("capital_usage_pct", 65.0).with_param("auto_leverage", 4.0)),
        ("MC2-CT-ai-override".into(),     StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 55.0).with_param("trust_ai_exits", 0.0).with_param("regime_filter", 15.0).with_param("min_safety", 40.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-CT-ai-whale".into(),        StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 65.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 9.0).with_param("min_safety", 50.0).with_param("max_mcap", 10000000.0).with_param("hard_stop_pct", 12.0).with_param("target_mult", 1.12).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 4.0)),
        ("MC2-CT-ai-consensus".into(),    StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 70.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 9.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-CT-ai-early".into(),        StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 55.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 1.0).with_param("min_safety", 40.0).with_param("max_mcap", 2000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.20).with_param("time_limit_mins", 15.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-CT-ai-balanced".into(),     StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 65.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 9.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-CT-ai-safe".into(),         StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 75.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 8.0).with_param("min_safety", 70.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 10.0).with_param("target_mult", 1.08).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 12.0).with_param("capital_usage_pct", 55.0).with_param("auto_leverage", 3.0)),
        ("MC2-CT-ai-aggressive".into(),   StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 40.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_safety", 30.0).with_param("max_mcap", 10000000.0).with_param("hard_stop_pct", 20.0).with_param("target_mult", 1.25).with_param("time_limit_mins", 15.0).with_param("auto_position_count", 17.0).with_param("capital_usage_pct", 70.0).with_param("auto_leverage", 6.0)),
        ("MC2-CT-ai-micro".into(),        StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 50.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_safety", 40.0).with_param("max_mcap", 100000.0).with_param("hard_stop_pct", 20.0).with_param("target_mult", 1.30).with_param("time_limit_mins", 15.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 6.0)),
        ("MC2-CT-ai-patient".into(),      StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 75.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 8.0).with_param("min_safety", 60.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 12.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 45.0).with_param("auto_position_count", 12.0).with_param("capital_usage_pct", 55.0).with_param("auto_leverage", 4.0)),
        ("MC2-CT-ai-volume".into(),       StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 60.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 9.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-CT-ai-contrarian".into(),   StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 60.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 6.0).with_param("min_safety", 40.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 18.0).with_param("target_mult", 1.20).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-CT-ai-trust-test".into(),   StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 65.0).with_param("trust_ai_exits", 0.0).with_param("regime_filter", 15.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 14.0).with_param("capital_usage_pct", 60.0).with_param("auto_leverage", 5.0)),
        ("MC2-CT-ai-tight".into(),        StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 60.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_safety", 50.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 10.0).with_param("target_mult", 1.10).with_param("time_limit_mins", 15.0).with_param("auto_position_count", 17.0).with_param("capital_usage_pct", 65.0).with_param("auto_leverage", 4.0)),
        ("MC2-CT-ai-wide".into(),         StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 55.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 1.0).with_param("min_safety", 40.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 20.0).with_param("target_mult", 1.30).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 13.0).with_param("capital_usage_pct", 55.0).with_param("auto_leverage", 6.0)),
        ("MC2-CT-ai-diversified".into(),  StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 50.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_safety", 40.0).with_param("max_mcap", 10000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.15).with_param("time_limit_mins", 25.0).with_param("auto_position_count", 20.0).with_param("capital_usage_pct", 65.0).with_param("auto_leverage", 4.0)),
        ("MC2-CT-ai-explorer".into(),     StrategyParams::new("mc2_ai_copy").with_param("min_ai_score", 55.0).with_param("trust_ai_exits", 1.0).with_param("regime_filter", 15.0).with_param("min_safety", 40.0).with_param("max_mcap", 5000000.0).with_param("hard_stop_pct", 15.0).with_param("target_mult", 1.18).with_param("time_limit_mins", 20.0).with_param("auto_position_count", 15.0).with_param("capital_usage_pct", 65.0).with_param("auto_leverage", 5.0)),
```

- [ ] **Step 2: Update the strategy count assertion**

Change the test assertion from:
```rust
assert_eq!(initial_strategies().len(), 160);
```
To:
```rust
assert_eq!(initial_strategies().len(), 200);
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p tradoshka-engine -- test_initial_strategies 2>&1`

Expected: PASS with 200 strategies.

- [ ] **Step 4: Commit**

```bash
git add core/engine/src/mutation.rs
git commit -m "feat(mc2): add 40 AI-powered meme coin strategy definitions (MC2-TR + MC2-CT)"
```

---

### Task 3: Route MC2- strategies to meme_coins_v2 market

**Files:**
- Modify: `core/engine/src/strategy_wallet.rs`
- Modify: `core/engine/src/evolution.rs`

- [ ] **Step 1: Add MC2- routing in initialize_defaults()**

In `strategy_wallet.rs`, find the `initialize_defaults()` method. Change the MC- branch:

```rust
} else if name.starts_with("MC2-") {
    "meme_coins_v2"
} else if name.starts_with("MC-") {
    "meme_coins"
} else {
```

The `MC2-` check MUST come before `MC-` since `MC2-` also starts with `MC-`.

- [ ] **Step 2: Add MC2- to evolution prefix list**

In `core/engine/src/evolution.rs`, find the line:
```rust
for market_prefix in &["PM-", "CS-", "CP-", "MC-"] {
```

Change to:
```rust
for market_prefix in &["PM-", "CS-", "CP-", "MC2-", "MC-"] {
```

`MC2-` MUST come before `MC-` so strategies starting with `MC2-` match the more specific prefix first.

- [ ] **Step 3: Run tests and verify**

Run: `cargo test --workspace 2>&1`

Expected: All tests pass. Verify MC2 strategies route correctly by checking the 200 count.

- [ ] **Step 4: Commit**

```bash
git add core/engine/src/strategy_wallet.rs core/engine/src/evolution.rs
git commit -m "feat(mc2): route MC2- strategies to meme_coins_v2 market with own evolution"
```

---

### Task 4: Add ClaudeScorer to AppState

**Files:**
- Modify: `core/api/src/state.rs`

- [ ] **Step 1: Add claude_scorer field to AppState**

Add this field after the `pub memecoins: MemeCoinAdapter,` line:

```rust
    // MC-V2 AI scorer
    pub claude_scorer: tradoshka_engine::ClaudeScorer,
```

- [ ] **Step 2: Initialize in AppState construction**

Find where `AppState` is constructed (in `main.rs` or `state.rs`). Add:

```rust
claude_scorer: tradoshka_engine::ClaudeScorer::new(),
```

- [ ] **Step 3: Build**

Run: `cargo build --workspace 2>&1`

Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add core/api/src/state.rs core/api/src/main.rs
git commit -m "feat(mc2): add ClaudeScorer to AppState"
```

---

### Task 5: Wire MC2 trading block into main.rs

**Files:**
- Modify: `core/api/src/main.rs`

- [ ] **Step 1: Add MC2 AI-powered trading block**

After the existing MC1 position monitor block (search for the end of the MC1 section, before the CS/CP trading blocks), add a new block. This is the largest change — it reuses the MC1 scanned tokens but adds Claude scoring:

```rust
                // --- MC2 AI-powered meme coin strategy trading ---
                // Reuses tokens from MC1 scanner, adds Claude AI scoring before entry
                {
                    // Reuse the same tokens and safety scores from MC1 scan
                    // (tokens and token_meta are still in scope from the MC1 block above)

                    // Phase 1: Score tokens with Claude (only for tokens passing safety+volume)
                    let ai_scores: Vec<Option<tradoshka_engine::claude_scorer::TokenScore>> = {
                        let mut s = state.write().await;
                        s.claude_scorer.tick_cycle();
                        s.claude_scorer.clear_expired_cache();

                        tokens.iter().zip(token_meta.iter()).map(|(token, (safety, _))| {
                            if *safety < 30 || token.volume_5m < 1000.0 || token.liquidity_usd < 5000.0 {
                                return None; // Skip low-quality tokens
                            }
                            let age_mins = (chrono::Utc::now() - token.first_seen).num_minutes();
                            s.claude_scorer.score_token(
                                &token.name, &token.symbol, &token.address,
                                token.market_cap, token.liquidity_usd,
                                token.volume_5m, token.volume_24h,
                                token.price_change_5m, token.price_change_1h, token.price_change_24h,
                                age_mins, *safety,
                            )
                        }).collect()
                    };

                    // Phase 2: Trade execution for MC2-* strategies
                    let mut s = state.write().await;
                    let data_logger: *const tradoshka_engine::DataLogger = &s.data_logger;

                    let mc2_slots: Vec<String> = s.strategy_manager.alive_slots()
                        .iter()
                        .filter(|sl| sl.name.starts_with("MC2-"))
                        .map(|sl| sl.name.clone())
                        .collect();

                    for slot_name in &mc2_slots {
                        let slot = match s.strategy_manager.get_mut(slot_name) {
                            Some(sl) if sl.is_alive() => sl,
                            _ => continue,
                        };

                        let strategy_type = slot.params.strategy_type.clone();
                        if strategy_type != "mc2_ai_trend" && strategy_type != "mc2_ai_copy" { continue; }

                        let min_ai_score = slot.params.get("min_ai_score") as u32;
                        let trust_ai_exits = slot.params.get("trust_ai_exits") > 0.5;
                        let regime_filter = slot.params.get("regime_filter") as u32;
                        let min_safety = slot.params.get("min_safety") as u32;
                        let max_mcap = slot.params.get("max_mcap");
                        let max_positions = slot.params.get("auto_position_count").max(1.0) as usize;
                        let hard_stop_pct = slot.params.get("hard_stop_pct");
                        let target_mult = slot.params.get("target_mult").max(1.1);

                        for (i, (token, ai_score_opt)) in tokens.iter().zip(ai_scores.iter()).enumerate() {
                            if slot.wallet.open_position_count() >= max_positions { break; }

                            // Skip if already have position
                            let pos_key = format!("{}:{}", token.address, slot_name);
                            if slot.wallet.positions().keys().any(|k| k == &pos_key) { continue; }

                            // Safety check
                            let (safety_score, _) = token_meta[i];
                            if safety_score < min_safety { continue; }

                            // Must have AI score
                            let ai_score = match ai_score_opt {
                                Some(s) => s,
                                None => continue,
                            };

                            // AI score threshold
                            if ai_score.score < min_ai_score { continue; }

                            // AI verdict check
                            if ai_score.verdict == "AVOID" { continue; }

                            // Regime filter (bitmask: 1=pump, 2=dist, 4=dead_cat, 8=organic)
                            let regime_bit = match ai_score.regime.as_str() {
                                "pump_phase" => 1u32,
                                "distribution" => 2,
                                "dead_cat" => 4,
                                "organic_growth" => 8,
                                _ => 0,
                            };
                            if regime_filter & regime_bit == 0 { continue; }

                            // Mcap filter
                            if max_mcap > 0.0 && token.market_cap > max_mcap { continue; }

                            // Liquidity floor
                            if token.liquidity_usd < 5000.0 { continue; }

                            // Determine SL/TP — trust AI or use strategy params
                            let (use_stop_pct, use_target_mult) = if trust_ai_exits {
                                (ai_score.suggested_stop_pct, 1.0 + ai_score.suggested_target_pct / 100.0)
                            } else {
                                (hard_stop_pct, target_mult)
                            };

                            let price = rust_decimal::Decimal::from_f64(token.price_usd)
                                .unwrap_or(rust_decimal::Decimal::ZERO);
                            if price <= rust_decimal::Decimal::ZERO { continue; }

                            // Kelly position sizing
                            let position_usd = slot.kelly_position_size(price);
                            let size = (position_usd / price).round_dp(0);
                            if size <= rust_decimal::Decimal::ZERO { continue; }

                            if let Some((fill_price, fee, filled)) = slot.wallet.buy(
                                &token.address, &format!("{} Meme", token.symbol),
                                "Long", price, size, slot_name,
                            ) {
                                // Gas fee
                                slot.wallet.add_gas_fee(rust_decimal_macros::dec!(0.151));

                                // Build trade thesis with AI context
                                let hard_stop = fill_price
                                    * rust_decimal::Decimal::from_f64(1.0 - use_stop_pct / 100.0)
                                        .unwrap_or(rust_decimal_macros::dec!(0.85));
                                let take_profit = fill_price
                                    * rust_decimal::Decimal::from_f64(use_target_mult)
                                        .unwrap_or(rust_decimal_macros::dec!(1.15));

                                slot.recorder.record(tradoshka_engine::TradeRecord {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    timestamp: chrono::Utc::now(),
                                    market: tradoshka_common::types::Market::Crypto,
                                    symbol: token.address.clone(),
                                    market_question: format!("{} Meme", token.symbol),
                                    direction: "Long".into(),
                                    side: tradoshka_common::types::OrderSide::Buy,
                                    shares: filled,
                                    price: fill_price,
                                    fee,
                                    strategy_id: slot_name.clone(),
                                    signal_strength: ai_score.score as f64 / 100.0,
                                    edge_vs_market: 0.0,
                                    pnl: None,
                                    is_closed: false,
                                    thesis_reasoning: format!(
                                        "MC2 AI: {} Score={}/100 Regime={} | {} | SL={:.0}% TP={:.0}%{}",
                                        ai_score.verdict, ai_score.score, ai_score.regime,
                                        ai_score.reasoning,
                                        use_stop_pct, (use_target_mult - 1.0) * 100.0,
                                        if trust_ai_exits { " [AI exits]" } else { " [fixed exits]" }
                                    ),
                                    stop_loss: hard_stop,
                                    trailing_stop: hard_stop,
                                    take_profit,
                                    time_stop_hours: 1,
                                    thesis_invalidation: String::new(),
                                    risk_amount: position_usd,
                                    reward_risk_ratio: use_target_mult as f64,
                                    strategy_tier: "Unproven".into(),
                                    close_reason: None,
                                });

                                tracing::info!(
                                    "MC2 AI {} BUY {} {} @ {} | Score={}/100 Regime={} | SL={:.0}% TP={:.0}% | {}",
                                    slot_name, filled, token.symbol, fill_price,
                                    ai_score.score, ai_score.regime,
                                    use_stop_pct, (use_target_mult - 1.0) * 100.0,
                                    ai_score.reasoning.chars().take(50).collect::<String>()
                                );
                            }
                        }
                    }

                    // MC2 Position Monitor (reuse MC1 pattern with same exit logic)
                    // MC2 positions use the SL/TP from the trade record (which may be AI-sourced)
                    {
                        let mc2_slot_names: Vec<String> = s.strategy_manager.alive_slots()
                            .iter()
                            .filter(|sl| sl.name.starts_with("MC2-") && sl.trade_count() > 0)
                            .map(|sl| sl.name.clone())
                            .collect();

                        for slot_name in &mc2_slot_names {
                            let slot = match s.strategy_manager.get_mut(slot_name) {
                                Some(sl) => sl,
                                None => continue,
                            };

                            let time_limit_mins: i64 = {
                                let t = slot.params.get("time_limit_mins");
                                if t > 0.0 { t as i64 } else { 25 }
                            };

                            let pos_data: Vec<_> = slot.wallet.positions().iter().map(|(k, p)| {
                                (k.clone(), p.current_price, p.avg_price, p.opened_at, p.token_id.clone())
                            }).collect();

                            for (pos_key, current_price, entry_price, opened_at, token_id) in &pos_data {
                                if *entry_price <= rust_decimal::Decimal::ZERO { continue; }

                                let pnl_pct = ((*current_price - *entry_price) / *entry_price
                                    * rust_decimal::Decimal::new(100, 0))
                                    .to_f64().unwrap_or(0.0);
                                let mins_held = (chrono::Utc::now() - *opened_at).num_minutes();

                                // Find this trade's SL/TP from record
                                let (trade_sl_pct, trade_tp_pct) = slot.recorder.all_trades().iter().rev()
                                    .find(|t| t.symbol == *token_id && t.strategy_id == *slot_name && !t.is_closed)
                                    .map(|t| {
                                        let sl_pct = ((t.stop_loss - *entry_price) / *entry_price * rust_decimal::Decimal::new(100, 0))
                                            .to_f64().unwrap_or(-15.0).abs();
                                        let tp_pct = ((t.take_profit - *entry_price) / *entry_price * rust_decimal::Decimal::new(100, 0))
                                            .to_f64().unwrap_or(15.0);
                                        (sl_pct, tp_pct)
                                    })
                                    .unwrap_or((15.0, 15.0));

                                let (should_close, reason, exit_type) = if pnl_pct <= -trade_sl_pct {
                                    (true, format!("AI SL hit: {:.1}% <= -{:.0}%", pnl_pct, trade_sl_pct), tradoshka_engine::ExitType::StopLoss)
                                } else if pnl_pct >= trade_tp_pct {
                                    (true, format!("AI TP hit: {:.1}% >= +{:.0}%", pnl_pct, trade_tp_pct), tradoshka_engine::ExitType::TakeProfit)
                                } else if mins_held >= time_limit_mins {
                                    (true, format!("Time stop: {}min >= {}min limit", mins_held, time_limit_mins), tradoshka_engine::ExitType::TimeStop)
                                } else {
                                    (false, String::new(), tradoshka_engine::ExitType::Manual)
                                };

                                if should_close {
                                    let pos_shares = slot.wallet.positions().get(pos_key)
                                        .map(|p| p.shares).unwrap_or(rust_decimal::Decimal::ZERO);
                                    if let Some((fill, fee, pnl)) = slot.wallet.sell_with_exit_type(
                                        token_id, *current_price, pos_shares, slot_name, exit_type,
                                    ) {
                                        slot.wallet.add_gas_fee(rust_decimal_macros::dec!(0.151));
                                        slot.recorder.close_trade(token_id, slot_name, pnl);
                                        tracing::info!(
                                            "MC2 AI {} CLOSE {} @ {} pnl={:.4} | {}",
                                            slot_name, token_id.chars().take(20).collect::<String>(),
                                            fill, pnl, reason
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
```

- [ ] **Step 2: Ensure tokens and token_meta are accessible to MC2 block**

The MC2 block needs the `tokens` and `token_meta` variables from the MC1 scan. Make sure the MC2 block is within the same scope. If the MC1 block ends with `}` that drops these variables, move the MC2 block inside that scope or extract the scan to a shared scope.

- [ ] **Step 3: Build and test**

Run: `cargo build --workspace && cargo test --workspace 2>&1`

Expected: Compiles and all tests pass.

- [ ] **Step 4: Commit**

```bash
git add core/api/src/main.rs
git commit -m "feat(mc2): wire AI-powered trading block with Claude scoring + position monitor"
```

---

### Task 6: Integration test — verify MC2 strategies trading with Claude

**Files:**
- No new files

- [ ] **Step 1: Kill server, rebuild, restart**

```bash
taskkill //F //IM tradoshka.exe 2>/dev/null
sleep 2
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release
cargo run --release &
sleep 20
```

- [ ] **Step 2: Verify 200 strategies alive with MC2 market**

```bash
curl -s http://localhost:3001/api/evolution/leaderboard | python -c "
import json, sys
data = json.load(sys.stdin)
markets = {}
for s in data['strategies']:
    m = s['market']
    markets[m] = markets.get(m, 0) + 1
for m, c in sorted(markets.items()):
    print(f'  {m}: {c} strategies')
print(f'Total: {len(data[\"strategies\"])}')
"
```

Expected: `meme_coins_v2: 40` alongside existing markets. Total: 200.

- [ ] **Step 3: Check if Claude scorer is available**

```bash
curl -s http://localhost:3001/api/evolution/leaderboard | python -c "
import json, sys
data = json.load(sys.stdin)
mc2 = [s for s in data['strategies'] if s['market'] == 'meme_coins_v2']
print(f'MC2 strategies: {len(mc2)}')
trading = [s for s in mc2 if s.get('trades', 0) > 0]
print(f'MC2 with trades: {len(trading)}')
for s in mc2[:5]:
    print(f'  {s[\"name\"]} type={s[\"strategy_type\"]} trades={s.get(\"trades\",0)}')
"
```

Expected: 40 MC2 strategies. If Claude CLI is available, some should have trades after a few cycles. If not, 0 trades (expected — Claude scoring returns None without CLI).

- [ ] **Step 4: Push everything**

```bash
git push origin dev
```

---

## Spec Coverage Check

| Spec Requirement | Task |
|-----------------|------|
| ClaudeScorer struct with cache, score_token(), feedback | Task 1 |
| JSON schema for Claude response | Task 1 (TOKEN_SCORE_SCHEMA const) |
| 5-minute cache per token address | Task 1 (cache_ttl, get_cached) |
| Fallback: skip token if Claude fails | Task 1 (returns None) |
| 40 MC2 strategies (20 TR + 20 CT) | Task 2 |
| AI params: min_ai_score, trust_ai_exits, regime_filter | Task 2 (in strategy params) |
| Strategy types: mc2_ai_trend, mc2_ai_copy | Task 2 |
| MC2- routes to meme_coins_v2 market | Task 3 |
| MC2- has own evolution cycle | Task 3 (evolution.rs prefix) |
| ClaudeScorer in AppState | Task 4 |
| MC2 trading block reuses MC1 scanner data | Task 5 |
| Post-filter scoring (safety+volume first, then Claude) | Task 5 (Phase 1) |
| Regime bitmask filtering | Task 5 (regime_bit check) |
| Trust AI exits vs fixed exits | Task 5 (trust_ai_exits branch) |
| Kelly position sizing for MC2 | Task 5 (kelly_position_size) |
| Gas fees on buy/sell | Task 5 ($0.151) |
| Position monitor with AI-sourced SL/TP | Task 5 (trade record lookup) |
| Lessons prepended to prompt | Task 1 (lessons_ctx in prompt) |
| Feedback recording | Task 1 (record_feedback method) |
| Sonnet model | Task 1 (--model sonnet in CLI args) |
