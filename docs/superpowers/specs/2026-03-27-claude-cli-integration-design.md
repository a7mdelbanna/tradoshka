# Claude CLI Integration — Design Spec

## Summary

Replace the OpenAI HTTP-based LLM client with a Claude Code CLI (`claude -p`) integration that uses the user's Max subscription. Switch from 20 sequential per-agent LLM calls to a single batch prompt where Claude simulates all agent perspectives in one call.

## Motivation

- **Cost**: Flat-rate Max subscription ($100-200/mo) vs. variable API billing
- **Speed**: One CLI call (~5-10s) vs. 20 sequential API calls (~3-10 min)
- **Quality**: Claude Sonnet/Opus produces better multi-perspective reasoning than GPT-4o-mini
- **Legal**: `claude -p` in scripts is officially supported by Anthropic

## Architecture

```
AIPredictorStrategy
  └─ PredictionSimulation
       ├─ AgentFactory.create_agents(20)     [UNCHANGED]
       ├─ WorldBuilder.build(market_data)     [UNCHANGED]
       ├─ ClaudeClient.predict(prompt)        [NEW — replaces LLMClient]
       │    └─ subprocess: claude -p --model sonnet --output-format json
       │         --json-schema '...' --bare --max-turns 1
       └─ PredictionScorer.aggregate(votes)   [UNCHANGED]
```

## Component: ClaudeClient (`claude_client.py`)

Replaces `llm_client.py`. Wraps the Claude Code CLI via subprocess.

### ClaudeConfig

```python
@dataclass
class ClaudeConfig:
    model: str = "sonnet"       # sonnet | opus | haiku
    max_tokens: int = 16384     # enough for 20 agent JSON responses
    temperature: float = 0.7
    timeout: int = 120          # seconds
```

No env vars for API keys — authentication is handled by the CLI's existing OAuth session.

### ClaudeClient

```python
class ClaudeClient:
    def __init__(self, config: ClaudeConfig | None = None)
    def is_available(self) -> bool        # checks claude CLI exists
    def predict(self, system_prompt: str, user_message: str) -> dict
```

### CLI Invocation

```bash
claude -p "user message" \
  --system-prompt "system instructions" \
  --model sonnet \
  --output-format json \
  --json-schema '<schema>' \
  --max-turns 2
```

Flag rationale:
- `--system-prompt`: Fully replaces the default system prompt. Prevents project context (CLAUDE.md) from polluting the prediction. Do NOT use `--bare` (breaks auth on Windows) or `--append-system-prompt` (inherits project context, triggers tool use).
- `--max-turns 2`: Required because `--json-schema` is implemented as an internal tool that needs 2 turns. Using `--max-turns 1` causes `error_max_turns`.
- `--json-schema`: Enforce structured output. Response lands in `structured_output` field (NOT `result`).
- `--output-format json`: Clean JSON response envelope.

### Prompt Construction

The system prompt is passed via `--system-prompt` (full override) and the user message is passed as the `-p` argument. This keeps system instructions separate from user content and prevents project context from polluting predictions.

## Component: Batch Prompt Design

### System Instructions

```
You are a prediction market simulation engine. You will receive N agent
personas and a market question. For EACH agent, independently estimate
the probability of YES.

CRITICAL: Generate genuinely diverse opinions. Contrarian agents MUST
disagree with the majority. Do NOT converge toward consensus. Each agent
should reason from their unique perspective, biases, and information access.
```

### User Message

```
MARKET: {question}
Description: {description}
Current: YES={yes_price}, NO={no_price}
Volume: ${volume_24h}
End Date: {end_date}
Context:
- {context_data[0]}
- {context_data[1]}
...

AGENTS:
[1] Institutional Analyst #0 | risk=0.35 | bias=0.12 | confidence=0.78 | info=high | contrarian=0.15
[2] Contrarian #1 | risk=0.72 | bias=-0.45 | confidence=0.55 | info=medium | contrarian=0.82
...
```

### JSON Schema (enforced output)

```json
{
  "type": "object",
  "properties": {
    "votes": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "agent_id": {"type": "string"},
          "probability_yes": {"type": "number", "minimum": 0.01, "maximum": 0.99},
          "confidence": {"type": "number", "minimum": 0.0, "maximum": 1.0},
          "reasoning": {"type": "string"}
        },
        "required": ["agent_id", "probability_yes", "confidence", "reasoning"]
      }
    }
  },
  "required": ["votes"]
}
```

## Component: Simulation Changes (`simulation.py`)

### New Flow

```
Old: for each agent → LLM call → AgentVote    (20 HTTP calls)
New: build batch prompt → single claude -p → parse JSON → list[AgentVote]    (1 CLI call)
```

### Changes

- Constructor accepts `ClaudeConfig` instead of `LLMConfig`
- `_simulate_with_llm()` → `_simulate_with_claude()`: builds batch prompt, calls `ClaudeClient.predict()`, maps response to `AgentVote` list
- `_simulate_statistical()`: unchanged, serves as fallback
- CLI availability detected at init via `ClaudeClient.is_available()`

### Unchanged

- `AgentFactory` — same archetypes, same persona generation
- `PredictionScorer` — same weighted aggregation
- `WorldBuilder` — same market context construction
- `AIPredictorStrategy` — calls `sim.run()` which returns the same `PredictionResult`

## Error Handling

Three failure modes, all fall back to statistical mode:

1. **CLI not installed**: Detected at init. Warning logged. Statistical mode used.
2. **CLI call fails** (timeout, crash, auth expired): Caught per-cycle. Statistical fallback. Next cycle retries Claude.
3. **Bad JSON**: Shouldn't happen with `--json-schema`. If it does, statistical fallback for that cycle.

No retry loops. Statistical mode provides immediate answers. Next natural prediction cycle retries Claude.

Timeout: 120 seconds. Process killed after that.

## Files Changed

| File | Action | Description |
|------|--------|-------------|
| `llm_client.py` | Delete | Replaced entirely by claude_client.py |
| `claude_client.py` | Create | ClaudeConfig + ClaudeClient subprocess wrapper |
| `simulation.py` | Modify | Batch prompt logic, swap LLM → Claude |
| `__init__.py` | Modify | Export ClaudeClient/ClaudeConfig instead of LLMClient/LLMConfig |
| `predictor.py` | Modify | ClaudeConfig instead of LLMConfig |
| `tests/test_claude_client.py` | Create | Unit tests with mocked subprocess |
| `tests/test_simulation.py` | Create | Tests for batch simulation |

## Future Work (Not in This Spec)

- Enhance AgentFactory with Claude-powered persona generation
- Use Claude for deeper market research and context gathering
- Expand Claude integration to crypto strategies
- Evaluate Opus vs. Sonnet prediction quality
