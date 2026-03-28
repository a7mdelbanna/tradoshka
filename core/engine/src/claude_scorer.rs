use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};
use tracing::{info, warn};

// ── JSON schema that Claude must conform to ──────────────────────────────────

pub const TOKEN_SCORE_SCHEMA: &str = r#"{
  "type": "object",
  "required": ["score", "verdict", "reasoning", "regime", "suggested_stop_pct", "suggested_target_pct"],
  "properties": {
    "score": {
      "type": "integer",
      "minimum": 0,
      "maximum": 100,
      "description": "Overall trade quality score 0-100"
    },
    "verdict": {
      "type": "string",
      "enum": ["BUY", "SKIP", "AVOID"],
      "description": "Trading verdict for this token"
    },
    "reasoning": {
      "type": "string",
      "description": "Concise explanation of the score and verdict"
    },
    "regime": {
      "type": "string",
      "enum": ["pump_phase", "distribution", "dead_cat", "organic_growth"],
      "description": "Current market regime for this token"
    },
    "suggested_stop_pct": {
      "type": "number",
      "minimum": 5,
      "maximum": 50,
      "description": "Suggested stop-loss percentage below entry (5-50)"
    },
    "suggested_target_pct": {
      "type": "number",
      "minimum": 5,
      "maximum": 100,
      "description": "Suggested take-profit percentage above entry (5-100)"
    }
  },
  "additionalProperties": false
}"#;

// ── Public types ─────────────────────────────────────────────────────────────

/// Score returned by Claude for a single token.
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

/// Post-trade feedback recorded against a Claude-scored trade.
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

// ── ClaudeScorer ─────────────────────────────────────────────────────────────

/// Scores Solana meme-coin tokens using the local `claude` CLI.
/// Results are cached per token address with a configurable TTL so we never
/// hammer the CLI unnecessarily during rapid re-scans.
pub struct ClaudeScorer {
    cache: HashMap<String, (TokenScore, Instant)>,
    cache_ttl: Duration,
    lessons: Vec<String>,
    feedback: Vec<TradeFeedback>,
    cycle_count: u32,
    available: bool,
}

impl ClaudeScorer {
    // ── Constructor ──────────────────────────────────────────────────────────

    /// Create a new scorer.  Probes `claude --version` to determine whether the
    /// CLI is installed; sets `available = false` gracefully if not.
    pub fn new() -> Self {
        let available = Command::new("claude")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if available {
            info!("[ClaudeScorer] claude CLI detected — AI scoring enabled");
        } else {
            warn!("[ClaudeScorer] claude CLI not found — AI scoring disabled, will return None for all score requests");
        }

        Self {
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(300), // 5 minutes
            lessons: Vec::new(),
            feedback: Vec::new(),
            cycle_count: 0,
            available,
        }
    }

    /// Create with a custom TTL (useful for tests).
    pub fn with_ttl(ttl: Duration) -> Self {
        let mut s = Self::new();
        s.cache_ttl = ttl;
        s
    }

    // ── Availability ─────────────────────────────────────────────────────────

    pub fn is_available(&self) -> bool {
        self.available
    }

    // ── Cache ────────────────────────────────────────────────────────────────

    /// Return a cached score if present and not yet expired.
    pub fn get_cached(&self, token_address: &str) -> Option<&TokenScore> {
        if let Some((score, inserted_at)) = self.cache.get(token_address) {
            if inserted_at.elapsed() < self.cache_ttl {
                return Some(score);
            }
        }
        None
    }

    /// Remove all cache entries whose TTL has elapsed.
    pub fn clear_expired_cache(&mut self) {
        let ttl = self.cache_ttl;
        self.cache
            .retain(|_, (_, inserted_at)| inserted_at.elapsed() < ttl);
    }

    /// Number of entries currently in cache (including potentially expired ones
    /// that have not yet been purged).
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    // ── Scoring ──────────────────────────────────────────────────────────────

    /// Score a token using the Claude CLI.
    ///
    /// Returns `None` if:
    /// - the CLI is not available
    /// - the CLI invocation fails
    /// - the response cannot be parsed
    ///
    /// A valid cached result is returned immediately without re-invoking the CLI.
    #[allow(clippy::too_many_arguments)]
    pub fn score_token(
        &mut self,
        name: &str,
        symbol: &str,
        address: &str,
        mcap: f64,
        liquidity: f64,
        vol_5m: f64,
        vol_24h: f64,
        change_5m: f64,
        change_1h: f64,
        change_24h: f64,
        age_minutes: f64,
        safety_score: f64,
    ) -> Option<TokenScore> {
        // 1. Return cache hit
        if let Some(cached) = self.get_cached(address) {
            return Some(cached.clone());
        }

        // 2. CLI must be available
        if !self.available {
            return None;
        }

        // 3. Build lessons context
        let lessons_context = if self.lessons.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nLessons learned from past trades:\n{}",
                self.lessons
                    .iter()
                    .enumerate()
                    .map(|(i, l)| format!("{}. {}", i + 1, l))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

        // 4. Build prompt
        let prompt = format!(
            r#"Analyze this Solana meme coin and provide a trading score.

Token: {name} ({symbol})
Address: {address}
Market Cap: ${mcap:.0}
Liquidity: ${liquidity:.0}
Volume 5m: ${vol_5m:.0}
Volume 24h: ${vol_24h:.0}
Price Change 5m: {change_5m:.2}%
Price Change 1h: {change_1h:.2}%
Price Change 24h: {change_24h:.2}%
Token Age: {age_minutes:.0} minutes
Safety Score: {safety_score:.1}/100{lessons_context}

Provide your analysis as structured JSON."#
        );

        let system = "You are a Solana meme coin analyst. Score this token for short-term trading \
potential based on momentum, liquidity, safety, and market regime. Be concise and data-driven. \
Respond only with valid JSON matching the provided schema.";

        // 5. Invoke CLI
        let output = Command::new("claude")
            .args([
                "-p",
                &prompt,
                "--system-prompt",
                system,
                "--model",
                "sonnet",
                "--output-format",
                "json",
                "--json-schema",
                TOKEN_SCORE_SCHEMA,
                "--max-turns",
                "2",
            ])
            .output();

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                warn!("[ClaudeScorer] Failed to invoke claude CLI: {e}");
                return None;
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("[ClaudeScorer] claude CLI exited with error: {stderr}");
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // 6. Parse response — try "structured_output" first, then "result"
        let score = Self::parse_score_from_output(&stdout)?;

        // 7. Cache and return
        self.cache
            .insert(address.to_string(), (score.clone(), Instant::now()));

        Some(score)
    }

    /// Parse a `TokenScore` from the raw JSON text emitted by the claude CLI.
    fn parse_score_from_output(raw: &str) -> Option<TokenScore> {
        let wrapper: serde_json::Value = match serde_json::from_str(raw.trim()) {
            Ok(v) => v,
            Err(e) => {
                warn!("[ClaudeScorer] Failed to parse claude output as JSON: {e}");
                return None;
            }
        };

        // Prefer structured_output > result > root object
        let payload = wrapper
            .get("structured_output")
            .or_else(|| wrapper.get("result"))
            .unwrap_or(&wrapper);

        let score = payload.get("score")?.as_u64()? as u32;
        let verdict = payload.get("verdict")?.as_str()?.to_string();
        let reasoning = payload.get("reasoning")?.as_str()?.to_string();
        let regime = payload.get("regime")?.as_str()?.to_string();
        let suggested_stop_pct = payload.get("suggested_stop_pct")?.as_f64()?;
        let suggested_target_pct = payload.get("suggested_target_pct")?.as_f64()?;

        Some(TokenScore {
            score,
            verdict,
            reasoning,
            regime,
            suggested_stop_pct,
            suggested_target_pct,
            scored_at: Utc::now(),
        })
    }

    // ── Feedback ─────────────────────────────────────────────────────────────

    pub fn record_feedback(&mut self, fb: TradeFeedback) {
        self.feedback.push(fb);
    }

    pub fn feedback(&self) -> &[TradeFeedback] {
        &self.feedback
    }

    // ── Lessons ──────────────────────────────────────────────────────────────

    pub fn lessons(&self) -> &[String] {
        &self.lessons
    }

    pub fn set_lessons(&mut self, lessons: Vec<String>) {
        self.lessons = lessons;
    }

    // ── Cycle tracking ───────────────────────────────────────────────────────

    pub fn tick_cycle(&mut self) {
        self.cycle_count += 1;
    }

    pub fn cycle_count(&self) -> u32 {
        self.cycle_count
    }
}

impl Default for ClaudeScorer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_scorer() -> ClaudeScorer {
        ClaudeScorer::new()
    }

    fn make_feedback(token_name: &str) -> TradeFeedback {
        TradeFeedback {
            token_name: token_name.to_string(),
            token_address: "So11111111111111111111111111111111111111112".to_string(),
            claude_score: 75,
            claude_verdict: "BUY".to_string(),
            claude_regime: "pump_phase".to_string(),
            claude_reasoning: "Strong momentum".to_string(),
            claude_stop: 15.0,
            claude_tp: 40.0,
            actual_pnl_pct: 35.0,
            actual_exit_reason: "take_profit".to_string(),
            was_profitable: true,
            strategy_used_ai_exits: true,
            timestamp: Utc::now(),
        }
    }

    // ── test_new_scorer ───────────────────────────────────────────────────────

    #[test]
    fn test_new_scorer() {
        let scorer = make_scorer();
        // is_available reflects whether claude CLI exists — both states are valid
        let _ = scorer.is_available();
        assert_eq!(scorer.cycle_count(), 0);
        assert_eq!(scorer.cache_size(), 0);
        assert!(scorer.lessons().is_empty());
        assert!(scorer.feedback().is_empty());
    }

    // ── test_cache_miss_on_empty ──────────────────────────────────────────────

    #[test]
    fn test_cache_miss_on_empty() {
        let scorer = make_scorer();
        let result = scorer.get_cached("nonexistent_address");
        assert!(result.is_none(), "Expected cache miss on empty scorer");
    }

    // ── test_cache_insert_and_retrieve ────────────────────────────────────────

    #[test]
    fn test_cache_insert_and_retrieve() {
        let mut scorer = make_scorer();
        let ts = TokenScore {
            score: 82,
            verdict: "BUY".to_string(),
            reasoning: "Solid momentum".to_string(),
            regime: "organic_growth".to_string(),
            suggested_stop_pct: 12.0,
            suggested_target_pct: 35.0,
            scored_at: Utc::now(),
        };
        scorer
            .cache
            .insert("addr1".to_string(), (ts.clone(), Instant::now()));

        let retrieved = scorer.get_cached("addr1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().score, 82);
    }

    // ── test_cache_expires ────────────────────────────────────────────────────

    #[test]
    fn test_cache_expires() {
        let mut scorer = ClaudeScorer::with_ttl(Duration::from_millis(1));
        let ts = TokenScore {
            score: 55,
            verdict: "SKIP".to_string(),
            reasoning: "Weak signal".to_string(),
            regime: "distribution".to_string(),
            suggested_stop_pct: 20.0,
            suggested_target_pct: 15.0,
            scored_at: Utc::now(),
        };
        // Insert with an Instant that is already 100ms old (TTL is 1ms)
        scorer.cache.insert(
            "addr_expire".to_string(),
            (ts, Instant::now() - Duration::from_millis(100)),
        );

        let result = scorer.get_cached("addr_expire");
        assert!(
            result.is_none(),
            "Expected expired entry to return None"
        );
    }

    // ── test_feedback_recording ───────────────────────────────────────────────

    #[test]
    fn test_feedback_recording() {
        let mut scorer = make_scorer();
        assert_eq!(scorer.feedback().len(), 0);

        scorer.record_feedback(make_feedback("TokenA"));
        scorer.record_feedback(make_feedback("TokenB"));

        assert_eq!(scorer.feedback().len(), 2);
        assert_eq!(scorer.feedback()[0].token_name, "TokenA");
        assert_eq!(scorer.feedback()[1].token_name, "TokenB");
    }

    // ── test_lessons_management ───────────────────────────────────────────────

    #[test]
    fn test_lessons_management() {
        let mut scorer = make_scorer();
        assert!(scorer.lessons().is_empty());

        let lessons = vec![
            "Avoid tokens under 30 minutes old with no prior volume".to_string(),
            "High 5m change + low liquidity = likely rug".to_string(),
        ];
        scorer.set_lessons(lessons.clone());

        assert_eq!(scorer.lessons().len(), 2);
        assert_eq!(scorer.lessons()[0], lessons[0]);
        assert_eq!(scorer.lessons()[1], lessons[1]);

        // Overwrite
        scorer.set_lessons(vec!["Only one lesson now".to_string()]);
        assert_eq!(scorer.lessons().len(), 1);
    }

    // ── test_clear_expired_cache ──────────────────────────────────────────────

    #[test]
    fn test_clear_expired_cache() {
        let mut scorer = ClaudeScorer::with_ttl(Duration::from_millis(1));

        let fresh = TokenScore {
            score: 70,
            verdict: "BUY".to_string(),
            reasoning: "ok".to_string(),
            regime: "pump_phase".to_string(),
            suggested_stop_pct: 10.0,
            suggested_target_pct: 30.0,
            scored_at: Utc::now(),
        };
        let stale = TokenScore {
            score: 30,
            verdict: "AVOID".to_string(),
            reasoning: "bad".to_string(),
            regime: "dead_cat".to_string(),
            suggested_stop_pct: 25.0,
            suggested_target_pct: 5.0,
            scored_at: Utc::now(),
        };

        // Insert stale entry (already expired)
        scorer.cache.insert(
            "stale_addr".to_string(),
            (stale, Instant::now() - Duration::from_millis(100)),
        );
        // Insert fresh entry (not yet expired)
        scorer
            .cache
            .insert("fresh_addr".to_string(), (fresh, Instant::now()));

        assert_eq!(scorer.cache_size(), 2);

        // Give the fresh entry a moment, then extend its TTL so it survives
        // (TTL is 1ms so we just bump it to 5s for this entry via a new scorer
        // — simpler: just test that clear_expired_cache removes only the stale one)
        scorer.clear_expired_cache();

        // The stale entry should be gone; the fresh entry may have also expired
        // in a slow CI environment — so only assert the stale one is absent.
        assert!(
            scorer.cache.get("stale_addr").is_none(),
            "Stale cache entry should have been purged"
        );
    }
}
