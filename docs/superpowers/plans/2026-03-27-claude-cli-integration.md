# Claude CLI Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the OpenAI HTTP LLM client with a Claude Code CLI (`claude -p`) integration using the Max subscription, switching from 20 per-agent calls to a single batch prompt.

**Architecture:** `ClaudeClient` wraps `subprocess.run(["claude", "-p", ...])` with `--bare --max-turns 1 --output-format json --json-schema`. `PredictionSimulation` builds one batch prompt containing all 20 agent personas + market context, sends it in a single CLI call, and parses the structured JSON response into `AgentVote` objects. All downstream components (AgentFactory, Scorer, WorldBuilder, Predictor) remain unchanged.

**Tech Stack:** Python 3.12, subprocess, json, Claude Code CLI v2.1+

**Spec:** `docs/superpowers/specs/2026-03-27-claude-cli-integration-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `strategies/polymarket/tradoshka_poly/ai_predictor/claude_client.py` | Create | ClaudeConfig dataclass + ClaudeClient subprocess wrapper |
| `strategies/polymarket/tradoshka_poly/ai_predictor/simulation.py` | Modify | Replace LLM per-agent calls with single Claude batch call |
| `strategies/polymarket/tradoshka_poly/ai_predictor/predictor.py` | Modify | Swap LLMConfig → ClaudeConfig |
| `strategies/polymarket/tradoshka_poly/ai_predictor/__init__.py` | Modify | Export ClaudeClient/ClaudeConfig instead of LLMClient/LLMConfig |
| `strategies/polymarket/tradoshka_poly/ai_predictor/llm_client.py` | Delete | Replaced by claude_client.py |
| `strategies/polymarket/tradoshka_poly/ai_predictor/tests/__init__.py` | Create | Test package init |
| `strategies/polymarket/tradoshka_poly/ai_predictor/tests/test_claude_client.py` | Create | Unit tests for ClaudeClient with mocked subprocess |
| `strategies/polymarket/tradoshka_poly/ai_predictor/tests/test_simulation.py` | Create | Tests for batch simulation flow |

---

### Task 1: Create ClaudeClient with CLI availability check

**Files:**
- Create: `strategies/polymarket/tradoshka_poly/ai_predictor/tests/__init__.py`
- Create: `strategies/polymarket/tradoshka_poly/ai_predictor/tests/test_claude_client.py`
- Create: `strategies/polymarket/tradoshka_poly/ai_predictor/claude_client.py`

- [ ] **Step 1: Create test package init**

```python
# strategies/polymarket/tradoshka_poly/ai_predictor/tests/__init__.py
```

Empty file — just makes the directory a Python package.

- [ ] **Step 2: Write failing tests for ClaudeConfig and ClaudeClient.is_available()**

```python
# strategies/polymarket/tradoshka_poly/ai_predictor/tests/test_claude_client.py
import subprocess
from unittest.mock import patch, MagicMock
import pytest


def test_claude_config_defaults():
    from ai_predictor.claude_client import ClaudeConfig
    config = ClaudeConfig()
    assert config.model == "sonnet"
    assert config.max_tokens == 16384
    assert config.temperature == 0.7
    assert config.timeout == 120


def test_claude_client_is_available_when_cli_exists():
    from ai_predictor.claude_client import ClaudeClient
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0)
        client = ClaudeClient()
        assert client.is_available() is True
        mock_run.assert_called_once_with(
            ["claude", "--version"],
            capture_output=True, timeout=10,
        )


def test_claude_client_is_available_when_cli_missing():
    from ai_predictor.claude_client import ClaudeClient
    with patch("subprocess.run", side_effect=FileNotFoundError):
        client = ClaudeClient()
        assert client.is_available() is False
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd strategies/polymarket && python -m pytest tradoshka_poly/ai_predictor/tests/test_claude_client.py -v`
Expected: FAIL — `ModuleNotFoundError: No module named 'ai_predictor.claude_client'`

- [ ] **Step 4: Implement ClaudeConfig and ClaudeClient with is_available()**

```python
# strategies/polymarket/tradoshka_poly/ai_predictor/claude_client.py
import json
import subprocess
from dataclasses import dataclass


@dataclass
class ClaudeConfig:
    model: str = "sonnet"
    max_tokens: int = 16384
    temperature: float = 0.7
    timeout: int = 120


class ClaudeClient:
    """Claude Code CLI wrapper. Uses claude -p for inference via Max subscription."""

    def __init__(self, config: ClaudeConfig | None = None):
        self.config = config or ClaudeConfig()

    def is_available(self) -> bool:
        try:
            result = subprocess.run(
                ["claude", "--version"],
                capture_output=True, timeout=10,
            )
            return result.returncode == 0
        except (FileNotFoundError, subprocess.TimeoutExpired):
            return False
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd strategies/polymarket && python -m pytest tradoshka_poly/ai_predictor/tests/test_claude_client.py -v`
Expected: 3 passed

- [ ] **Step 6: Commit**

```bash
git add strategies/polymarket/tradoshka_poly/ai_predictor/claude_client.py strategies/polymarket/tradoshka_poly/ai_predictor/tests/__init__.py strategies/polymarket/tradoshka_poly/ai_predictor/tests/test_claude_client.py
git commit -m "feat: add ClaudeConfig and ClaudeClient with CLI availability check"
```

---

### Task 2: Implement ClaudeClient.predict() with subprocess call

**Files:**
- Modify: `strategies/polymarket/tradoshka_poly/ai_predictor/tests/test_claude_client.py`
- Modify: `strategies/polymarket/tradoshka_poly/ai_predictor/claude_client.py`

- [ ] **Step 1: Write failing tests for predict()**

Append to `tests/test_claude_client.py`:

```python
def test_predict_returns_parsed_json():
    from ai_predictor.claude_client import ClaudeClient, ClaudeConfig
    config = ClaudeConfig(model="sonnet", timeout=30)
    client = ClaudeClient(config)

    mock_response = json.dumps({
        "result": '{"votes": [{"agent_id": "agent_0000", "probability_yes": 0.72, "confidence": 0.85, "reasoning": "test"}]}',
        "session_id": "test-session",
    })

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout=mock_response,
        )
        result = client.predict("system prompt", "user message")
        assert result == {
            "votes": [{
                "agent_id": "agent_0000",
                "probability_yes": 0.72,
                "confidence": 0.85,
                "reasoning": "test",
            }]
        }

        call_args = mock_run.call_args
        cmd = call_args[0][0]
        assert cmd[0] == "claude"
        assert "-p" in cmd
        assert "--model" in cmd
        assert "--output-format" in cmd
        assert "--bare" in cmd
        assert "--max-turns" in cmd


def test_predict_raises_on_cli_failure():
    from ai_predictor.claude_client import ClaudeClient
    client = ClaudeClient()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(
            returncode=1,
            stdout="",
            stderr="auth expired",
        )
        with pytest.raises(RuntimeError, match="Claude CLI error"):
            client.predict("system", "user")


def test_predict_raises_on_timeout():
    from ai_predictor.claude_client import ClaudeClient
    client = ClaudeClient()

    with patch("subprocess.run", side_effect=subprocess.TimeoutExpired(cmd="claude", timeout=120)):
        with pytest.raises(RuntimeError, match="timed out"):
            client.predict("system", "user")
```

Add `import json` at the top of the test file if not already there.

- [ ] **Step 2: Run tests to verify new tests fail**

Run: `cd strategies/polymarket && python -m pytest tradoshka_poly/ai_predictor/tests/test_claude_client.py -v`
Expected: 3 passed, 3 failed (predict tests fail with AttributeError)

- [ ] **Step 3: Implement predict()**

Add to `claude_client.py`:

```python
    # JSON schema for enforced structured output
    VOTE_SCHEMA = json.dumps({
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
                        "reasoning": {"type": "string"},
                    },
                    "required": ["agent_id", "probability_yes", "confidence", "reasoning"],
                },
            },
        },
        "required": ["votes"],
    })

    def predict(self, system_prompt: str, user_message: str) -> dict:
        cmd = [
            "claude", "-p", user_message,
            "--append-system-prompt", system_prompt,
            "--model", self.config.model,
            "--output-format", "json",
            "--json-schema", self.VOTE_SCHEMA,
            "--bare",
            "--max-turns", "1",
        ]
        try:
            proc = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=self.config.timeout,
            )
        except subprocess.TimeoutExpired:
            raise RuntimeError(f"Claude CLI timed out after {self.config.timeout}s")

        if proc.returncode != 0:
            raise RuntimeError(f"Claude CLI error (code {proc.returncode}): {proc.stderr}")

        response = json.loads(proc.stdout)
        # claude --output-format json wraps the response in {"result": "...", "session_id": "..."}
        result_text = response.get("result", proc.stdout)
        if isinstance(result_text, str):
            return json.loads(result_text)
        return result_text
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd strategies/polymarket && python -m pytest tradoshka_poly/ai_predictor/tests/test_claude_client.py -v`
Expected: 6 passed

- [ ] **Step 5: Commit**

```bash
git add strategies/polymarket/tradoshka_poly/ai_predictor/claude_client.py strategies/polymarket/tradoshka_poly/ai_predictor/tests/test_claude_client.py
git commit -m "feat: implement ClaudeClient.predict() with subprocess call and JSON schema"
```

---

### Task 3: Rewrite PredictionSimulation to use Claude batch prompt

**Files:**
- Create: `strategies/polymarket/tradoshka_poly/ai_predictor/tests/test_simulation.py`
- Modify: `strategies/polymarket/tradoshka_poly/ai_predictor/simulation.py`

- [ ] **Step 1: Write failing tests for batch simulation**

```python
# strategies/polymarket/tradoshka_poly/ai_predictor/tests/test_simulation.py
import json
from unittest.mock import patch, MagicMock
import pytest


def test_simulation_uses_statistical_when_claude_unavailable():
    from ai_predictor.simulation import PredictionSimulation
    from ai_predictor.world_builder import SimulationWorld

    sim = PredictionSimulation(claude_config=None, agent_count=5, seed=42)
    world = SimulationWorld(
        market_question="Will X happen?",
        market_description="Test market",
        current_yes_price=0.6,
        current_no_price=0.4,
        volume_24h=10000.0,
        end_date="2026-04-01",
    )
    result = sim.run(world)
    assert result.num_agents == 5
    assert 0.01 <= result.probability_yes <= 0.99


def test_simulation_builds_batch_prompt_with_all_agents():
    from ai_predictor.simulation import PredictionSimulation
    from ai_predictor.claude_client import ClaudeConfig
    from ai_predictor.world_builder import SimulationWorld

    config = ClaudeConfig()

    mock_votes = {
        "votes": [
            {"agent_id": f"agent_{i:04d}", "probability_yes": 0.5 + (i * 0.02),
             "confidence": 0.7, "reasoning": f"Agent {i} reasoning"}
            for i in range(5)
        ]
    }

    with patch("ai_predictor.simulation.ClaudeClient") as MockClient:
        mock_instance = MagicMock()
        mock_instance.is_available.return_value = True
        mock_instance.predict.return_value = mock_votes
        MockClient.return_value = mock_instance

        sim = PredictionSimulation(claude_config=config, agent_count=5, seed=42)
        world = SimulationWorld(
            market_question="Will X happen?",
            market_description="Test market",
            current_yes_price=0.6,
            current_no_price=0.4,
            volume_24h=10000.0,
            end_date="2026-04-01",
        )
        result = sim.run(world)

        assert result.num_agents == 5
        mock_instance.predict.assert_called_once()
        call_args = mock_instance.predict.call_args
        system_prompt = call_args[0][0]
        user_message = call_args[0][1]
        assert "prediction market simulation engine" in system_prompt.lower()
        assert "Will X happen?" in user_message
        assert "agent_0000" in user_message or "[1]" in user_message


def test_simulation_falls_back_on_claude_error():
    from ai_predictor.simulation import PredictionSimulation
    from ai_predictor.claude_client import ClaudeConfig
    from ai_predictor.world_builder import SimulationWorld

    config = ClaudeConfig()

    with patch("ai_predictor.simulation.ClaudeClient") as MockClient:
        mock_instance = MagicMock()
        mock_instance.is_available.return_value = True
        mock_instance.predict.side_effect = RuntimeError("CLI crashed")
        MockClient.return_value = mock_instance

        sim = PredictionSimulation(claude_config=config, agent_count=5, seed=42)
        world = SimulationWorld(
            market_question="Will X happen?",
            market_description="Test market",
            current_yes_price=0.6,
            current_no_price=0.4,
            volume_24h=10000.0,
            end_date="2026-04-01",
        )
        result = sim.run(world)
        # Should fall back to statistical, not crash
        assert result.num_agents == 5
        assert 0.01 <= result.probability_yes <= 0.99
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd strategies/polymarket && python -m pytest tradoshka_poly/ai_predictor/tests/test_simulation.py -v`
Expected: FAIL — simulation.py still imports LLMClient

- [ ] **Step 3: Rewrite simulation.py**

Replace the entire file:

```python
# strategies/polymarket/tradoshka_poly/ai_predictor/simulation.py
import logging
import random
from .agent_factory import AgentFactory, AgentPersona
from .scorer import AgentVote, PredictionScorer, PredictionResult
from .world_builder import SimulationWorld
from .claude_client import ClaudeClient, ClaudeConfig

logger = logging.getLogger(__name__)

SYSTEM_PROMPT = (
    "You are a prediction market simulation engine. You will receive N agent "
    "personas and a market question. For EACH agent, independently estimate "
    "the probability of YES.\n\n"
    "CRITICAL: Generate genuinely diverse opinions. Contrarian agents MUST "
    "disagree with the majority. Do NOT converge toward consensus. Each agent "
    "should reason from their unique perspective, biases, and information access.\n\n"
    "Agents with high contrarian_factor should actively oppose the market consensus. "
    "Agents with negative bias should lean bearish, positive bias should lean bullish. "
    "Low-confidence agents should have more uncertain estimates near 0.5."
)


class PredictionSimulation:
    def __init__(self, claude_config: ClaudeConfig | None = None, agent_count: int = 20, seed: int | None = None):
        self.claude = None
        if claude_config is not None:
            client = ClaudeClient(claude_config)
            if client.is_available():
                self.claude = client
            else:
                logger.warning("Claude CLI not available, falling back to statistical mode")
        self.factory = AgentFactory(seed=seed)
        self.agent_count = agent_count

    def run(self, world: SimulationWorld) -> PredictionResult:
        agents = self.factory.create_agents(self.agent_count, topic_tags=["prediction_markets"])
        if self.claude is None:
            votes = self._simulate_statistical(agents, world)
        else:
            votes = self._simulate_with_claude(agents, world)
        scorer = PredictionScorer(market_price=world.current_yes_price)
        return scorer.aggregate(votes)

    def _simulate_statistical(self, agents: list[AgentPersona], world: SimulationWorld) -> list[AgentVote]:
        rng = random.Random()
        votes = []
        for agent in agents:
            base = world.current_yes_price + (agent.bias * 0.15)
            noise = rng.gauss(0, 0.1 * (1 - agent.confidence))
            if agent.contrarian_factor > 0.5:
                base = 1.0 - base
                base = world.current_yes_price + (base - world.current_yes_price) * agent.contrarian_factor
            prob = max(0.01, min(0.99, base + noise))
            votes.append(AgentVote(agent.agent_id, round(prob, 3), agent.confidence,
                                    f"Statistical estimate by {agent.archetype}"))
        return votes

    def _simulate_with_claude(self, agents: list[AgentPersona], world: SimulationWorld) -> list[AgentVote]:
        user_message = self._build_batch_prompt(agents, world)
        try:
            result = self.claude.predict(SYSTEM_PROMPT, user_message)
            votes = self._parse_votes(result, agents)
            if not votes:
                logger.warning("Claude returned no valid votes, falling back to statistical")
                return self._simulate_statistical(agents, world)
            return votes
        except Exception as e:
            logger.warning("Claude prediction failed (%s), falling back to statistical", e)
            return self._simulate_statistical(agents, world)

    def _build_batch_prompt(self, agents: list[AgentPersona], world: SimulationWorld) -> str:
        context = "\n".join(f"- {c}" for c in world.context_data[:10]) if world.context_data else "No additional context."
        end_line = f"\nEnd Date: {world.end_date}" if world.end_date else ""

        lines = [
            f"MARKET: {world.market_question}",
            f"Description: {world.market_description}",
            f"Current: YES={world.current_yes_price:.0%}, NO={world.current_no_price:.0%}",
            f"Volume: ${world.volume_24h:,.0f}",
        ]
        if world.end_date:
            lines.append(f"End Date: {world.end_date}")
        lines.append(f"Context:\n{context}")
        lines.append("")
        lines.append("AGENTS:")
        for i, agent in enumerate(agents):
            lines.append(
                f"[{i+1}] {agent.name} (id={agent.agent_id}) | "
                f"risk={agent.risk_tolerance} | bias={agent.bias} | "
                f"confidence={agent.confidence} | info={agent.information_access} | "
                f"contrarian={agent.contrarian_factor}"
            )
        lines.append("")
        lines.append("For EACH agent above, provide a vote with their agent_id, probability_yes, confidence, and brief reasoning.")

        return "\n".join(lines)

    def _parse_votes(self, result: dict, agents: list[AgentPersona]) -> list[AgentVote]:
        votes = []
        agent_map = {a.agent_id: a for a in agents}
        for vote_data in result.get("votes", []):
            agent_id = vote_data.get("agent_id", "")
            if agent_id not in agent_map:
                continue
            votes.append(AgentVote(
                agent_id=agent_id,
                probability_yes=max(0.01, min(0.99, float(vote_data.get("probability_yes", 0.5)))),
                confidence=max(0.0, min(1.0, float(vote_data.get("confidence", 0.5)))),
                reasoning=vote_data.get("reasoning", ""),
            ))
        return votes
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd strategies/polymarket && python -m pytest tradoshka_poly/ai_predictor/tests/test_simulation.py -v`
Expected: 3 passed

- [ ] **Step 5: Commit**

```bash
git add strategies/polymarket/tradoshka_poly/ai_predictor/simulation.py strategies/polymarket/tradoshka_poly/ai_predictor/tests/test_simulation.py
git commit -m "feat: rewrite PredictionSimulation for Claude batch prompt"
```

---

### Task 4: Update predictor.py and __init__.py, delete llm_client.py

**Files:**
- Modify: `strategies/polymarket/tradoshka_poly/ai_predictor/predictor.py`
- Modify: `strategies/polymarket/tradoshka_poly/ai_predictor/__init__.py`
- Delete: `strategies/polymarket/tradoshka_poly/ai_predictor/llm_client.py`

- [ ] **Step 1: Update predictor.py to use ClaudeConfig**

Replace the full file:

```python
# strategies/polymarket/tradoshka_poly/ai_predictor/predictor.py
import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..', '..', 'shared'))

from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection
from .simulation import PredictionSimulation
from .world_builder import WorldBuilder
from .scorer import PredictionScorer
from .claude_client import ClaudeConfig


class AIPredictorStrategy(BaseStrategy):
    def __init__(self, claude_config: ClaudeConfig | None = None, agent_count: int = 20):
        super().__init__("ai_predictor", "polymarket")
        self.sim = PredictionSimulation(claude_config=claude_config, agent_count=agent_count)
        self.world_builder = WorldBuilder()
        self._last_predictions: dict[str, float] = {}

    @property
    def name(self) -> str:
        return "AI Predictor (MiroFish-inspired)"

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        world = self.world_builder.build(
            question=event.symbol, yes_price=event.price,
            no_price=1.0 - event.price, volume=event.volume,
        )
        result = self.sim.run(world)
        self._last_predictions[event.symbol] = result.probability_yes
        scorer = PredictionScorer(market_price=event.price)
        trade = scorer.should_trade(result, min_edge=0.05, min_confidence=0.3)
        if trade is None:
            return Signal(self.strategy_id, event.symbol, SignalDirection.HOLD, 0.0)
        direction = SignalDirection.LONG if trade[0] == "YES" else SignalDirection.SHORT
        return Signal(self.strategy_id, event.symbol, direction, trade[1])

    def on_fill(self, fill) -> None:
        pass
```

- [ ] **Step 2: Update __init__.py exports**

Replace the full file:

```python
# strategies/polymarket/tradoshka_poly/ai_predictor/__init__.py
from .predictor import AIPredictorStrategy
from .scorer import PredictionScorer, AgentVote, PredictionResult
from .agent_factory import AgentFactory, AgentPersona
from .world_builder import WorldBuilder, SimulationWorld
from .simulation import PredictionSimulation
from .claude_client import ClaudeClient, ClaudeConfig
```

- [ ] **Step 3: Delete llm_client.py**

```bash
git rm strategies/polymarket/tradoshka_poly/ai_predictor/llm_client.py
```

- [ ] **Step 4: Run all tests to verify nothing breaks**

Run: `cd strategies/polymarket && python -m pytest tradoshka_poly/ai_predictor/tests/ -v`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add strategies/polymarket/tradoshka_poly/ai_predictor/predictor.py strategies/polymarket/tradoshka_poly/ai_predictor/__init__.py
git commit -m "feat: swap LLMConfig/LLMClient for ClaudeConfig/ClaudeClient, delete llm_client.py"
```

---

### Task 5: Smoke test with live Claude CLI

**Files:** None (manual verification)

- [ ] **Step 1: Create a quick smoke test script**

```python
# Run from project root:
# python -c "
from strategies.polymarket.tradoshka_poly.ai_predictor import PredictionSimulation, ClaudeConfig
from strategies.polymarket.tradoshka_poly.ai_predictor.world_builder import WorldBuilder

config = ClaudeConfig(model='haiku')  # use haiku for cheap smoke test
sim = PredictionSimulation(claude_config=config, agent_count=3, seed=42)
world = WorldBuilder().build(
    question='Will Bitcoin exceed $100k by end of 2026?',
    description='Prediction market on BTC price target',
    yes_price=0.65, no_price=0.35, volume=50000.0,
    end_date='2026-12-31',
    context=['BTC currently at $87k', 'Fed holding rates steady'],
)
result = sim.run(world)
print(f'Probability YES: {result.probability_yes}')
print(f'Confidence: {result.confidence}')
print(f'Bulls: {result.bull_count}, Bears: {result.bear_count}')
print(f'Edge vs market: {result.edge_vs_market}')
print(f'Std dev: {result.std_dev}')
# "
```

- [ ] **Step 2: Verify output looks reasonable**

Expected: Probability between 0.01-0.99, confidence > 0, 3 agents reported, non-zero edge. If Claude CLI isn't authenticated, it should fall back to statistical mode gracefully.

- [ ] **Step 3: Commit any fixes if needed**

Only commit if the smoke test revealed issues that needed code changes.
