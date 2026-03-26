# Phase 1B: Polymarket Strategies — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build 5 Python strategy modules for Polymarket — AI Predictor (MiroFish-inspired), Copy Trading, Market Making, Arbitrage, and an Ensemble Combiner that fuses all signals.

**Architecture:** All strategies extend `BaseStrategy` from `tradoshka_strategy`. Each receives `MarketEvent` objects and returns `Signal` objects. The ensemble combiner weights signals from all strategies and produces final trading decisions. All strategies run in dry mode first.

**Tech Stack:** Python 3.13, no external trading libraries (reimplement from scratch per security policy). LLM integration via HTTP requests to OpenAI-compatible APIs.

---

## File Structure

```
strategies/polymarket/
├── pyproject.toml
├── tradoshka_poly/
│   ├── __init__.py
│   ├── ai_predictor/
│   │   ├── __init__.py
│   │   ├── predictor.py          # Main AI prediction strategy
│   │   ├── world_builder.py      # Build simulation context from real data
│   │   ├── agent_factory.py      # Generate diverse agent personas
│   │   ├── simulation.py         # Run multi-agent prediction simulation
│   │   ├── scorer.py             # Convert simulation outputs to probabilities
│   │   └── llm_client.py         # LLM API client (OpenAI-compatible)
│   ├── copy_trading/
│   │   ├── __init__.py
│   │   ├── strategy.py           # Main copy trading strategy
│   │   ├── wallet_tracker.py     # Track top Polymarket wallets
│   │   ├── signal_extractor.py   # Convert whale trades to signals
│   │   └── filters.py            # Trader quality filters
│   ├── market_making/
│   │   ├── __init__.py
│   │   ├── strategy.py           # Market making strategy
│   │   ├── spread.py             # Spread calculation and quoting
│   │   └── inventory.py          # Inventory risk management
│   ├── arbitrage/
│   │   ├── __init__.py
│   │   ├── strategy.py           # Arbitrage strategy
│   │   └── mispricing.py         # Yes+No mispricing detection
│   └── ensemble/
│       ├── __init__.py
│       └── combiner.py           # Weighted signal fusion
└── tests/
    ├── test_scorer.py
    ├── test_agent_factory.py
    ├── test_wallet_tracker.py
    ├── test_filters.py
    ├── test_spread.py
    ├── test_inventory.py
    ├── test_mispricing.py
    └── test_combiner.py
```

---

### Task 1: Package Setup

**Files:**
- Create: `strategies/polymarket/pyproject.toml`
- Create: `strategies/polymarket/tradoshka_poly/__init__.py`
- Create: all `__init__.py` files for subpackages

- [ ] **Step 1: Create branch**

```bash
git checkout dev
git checkout -b feature/polymarket-strategies
```

- [ ] **Step 2: Create pyproject.toml**

```toml
[project]
name = "tradoshka-poly"
version = "0.1.0"
description = "Tradoshka Polymarket trading strategies"
requires-python = ">=3.10"
dependencies = [
    "tradoshka-strategy",
]

[project.optional-dependencies]
dev = ["pytest>=8.0"]

[build-system]
requires = ["setuptools>=75.0"]
build-backend = "setuptools.build_meta"
```

- [ ] **Step 3: Create all __init__.py files**

`tradoshka_poly/__init__.py`:
```python
from .ai_predictor.predictor import AIPredictorStrategy
from .copy_trading.strategy import CopyTradingStrategy
from .market_making.strategy import MarketMakingStrategy
from .arbitrage.strategy import ArbitrageStrategy
from .ensemble.combiner import EnsembleCombiner
```

Create empty `__init__.py` in each subpackage directory.

- [ ] **Step 4: Commit**

```bash
git add strategies/polymarket/
git commit -m "feat(polymarket): initialize strategies package"
```

---

### Task 2: LLM Client

**Files:**
- Create: `strategies/polymarket/tradoshka_poly/ai_predictor/llm_client.py`
- Test: `strategies/polymarket/tests/test_llm_client.py`

- [ ] **Step 1: Implement LLM client**

A simple, secure HTTP client for OpenAI-compatible APIs. No SDK installation — raw HTTP requests.

```python
import json
import os
from dataclasses import dataclass
from urllib.request import Request, urlopen
from urllib.error import HTTPError


@dataclass
class LLMConfig:
    api_url: str = "https://api.openai.com/v1/chat/completions"
    api_key: str = ""
    model: str = "gpt-4o-mini"
    temperature: float = 0.7
    max_tokens: int = 4096

    @classmethod
    def from_env(cls) -> "LLMConfig":
        return cls(
            api_url=os.getenv("LLM_API_URL", "https://api.openai.com/v1/chat/completions"),
            api_key=os.getenv("LLM_API_KEY", ""),
            model=os.getenv("LLM_MODEL", "gpt-4o-mini"),
        )


class LLMClient:
    """Minimal LLM client using raw HTTP. No SDK dependencies."""

    def __init__(self, config: LLMConfig | None = None):
        self.config = config or LLMConfig.from_env()

    def chat(self, system_prompt: str, user_message: str, temperature: float | None = None) -> str:
        """Send a chat completion request and return the response text."""
        payload = {
            "model": self.config.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_message},
            ],
            "temperature": temperature or self.config.temperature,
            "max_tokens": self.config.max_tokens,
        }
        data = json.dumps(payload).encode("utf-8")
        req = Request(
            self.config.api_url,
            data=data,
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {self.config.api_key}",
            },
            method="POST",
        )
        try:
            with urlopen(req, timeout=60) as resp:
                result = json.loads(resp.read())
                return result["choices"][0]["message"]["content"]
        except HTTPError as e:
            error_body = e.read().decode() if e.fp else str(e)
            raise RuntimeError(f"LLM API error {e.code}: {error_body}")

    def chat_json(self, system_prompt: str, user_message: str, temperature: float | None = None) -> dict:
        """Chat and parse the response as JSON."""
        response = self.chat(system_prompt, user_message, temperature)
        # Strip markdown code blocks if present
        text = response.strip()
        if text.startswith("```json"):
            text = text[7:]
        if text.startswith("```"):
            text = text[3:]
        if text.endswith("```"):
            text = text[:-3]
        return json.loads(text.strip())
```

- [ ] **Step 2: Write tests**

```python
# tests/test_llm_client.py
from tradoshka_poly.ai_predictor.llm_client import LLMConfig, LLMClient


def test_config_defaults():
    config = LLMConfig()
    assert config.model == "gpt-4o-mini"
    assert config.temperature == 0.7
    assert "openai" in config.api_url


def test_config_from_env(monkeypatch):
    monkeypatch.setenv("LLM_MODEL", "claude-3")
    monkeypatch.setenv("LLM_API_KEY", "test-key")
    config = LLMConfig.from_env()
    assert config.model == "claude-3"
    assert config.api_key == "test-key"
```

- [ ] **Step 3: Run tests**

```bash
cd strategies/polymarket && pip install -e ".[dev]" && python -m pytest tests/test_llm_client.py -v
```

- [ ] **Step 4: Commit**

```bash
git add strategies/polymarket/
git commit -m "feat(polymarket): add LLM client for AI predictor"
```

---

### Task 3: Agent Factory

**Files:**
- Create: `strategies/polymarket/tradoshka_poly/ai_predictor/agent_factory.py`
- Test: `strategies/polymarket/tests/test_agent_factory.py`

- [ ] **Step 1: Implement agent factory**

Generates diverse agent personas for prediction simulation. Unlike MiroFish which extracts from documents, we generate personas tailored to prediction market scenarios.

```python
from dataclasses import dataclass, field
import random


@dataclass
class AgentPersona:
    """A simulated agent with distinct personality and biases."""
    agent_id: str
    name: str
    archetype: str          # e.g., "institutional_analyst", "retail_trader", "contrarian"
    expertise: list[str]     # e.g., ["crypto", "politics", "economics"]
    risk_tolerance: float   # 0.0 (very cautious) to 1.0 (very aggressive)
    bias: float             # -1.0 (bearish/no) to 1.0 (bullish/yes)
    confidence: float       # 0.0 to 1.0 — how strongly they hold opinions
    information_access: str  # "high", "medium", "low"
    contrarian_factor: float # 0.0 to 1.0 — tendency to go against the crowd


ARCHETYPES = [
    {"name": "Institutional Analyst", "archetype": "institutional_analyst",
     "risk_tolerance": (0.2, 0.5), "bias": (-0.3, 0.3), "confidence": (0.6, 0.9),
     "information_access": "high", "contrarian_factor": (0.1, 0.3)},
    {"name": "Retail Trader", "archetype": "retail_trader",
     "risk_tolerance": (0.4, 0.9), "bias": (-0.5, 0.5), "confidence": (0.3, 0.7),
     "information_access": "low", "contrarian_factor": (0.0, 0.2)},
    {"name": "Data Scientist", "archetype": "data_scientist",
     "risk_tolerance": (0.3, 0.6), "bias": (-0.1, 0.1), "confidence": (0.5, 0.8),
     "information_access": "high", "contrarian_factor": (0.2, 0.5)},
    {"name": "Political Insider", "archetype": "political_insider",
     "risk_tolerance": (0.3, 0.7), "bias": (-0.6, 0.6), "confidence": (0.7, 0.95),
     "information_access": "high", "contrarian_factor": (0.1, 0.2)},
    {"name": "Contrarian", "archetype": "contrarian",
     "risk_tolerance": (0.5, 0.9), "bias": (-0.8, 0.8), "confidence": (0.4, 0.8),
     "information_access": "medium", "contrarian_factor": (0.6, 0.9)},
    {"name": "Market Maker", "archetype": "market_maker",
     "risk_tolerance": (0.1, 0.3), "bias": (-0.1, 0.1), "confidence": (0.5, 0.7),
     "information_access": "high", "contrarian_factor": (0.3, 0.5)},
    {"name": "News Follower", "archetype": "news_follower",
     "risk_tolerance": (0.3, 0.7), "bias": (-0.4, 0.4), "confidence": (0.2, 0.6),
     "information_access": "medium", "contrarian_factor": (0.0, 0.1)},
    {"name": "Whale", "archetype": "whale",
     "risk_tolerance": (0.4, 0.8), "bias": (-0.5, 0.5), "confidence": (0.6, 0.9),
     "information_access": "high", "contrarian_factor": (0.2, 0.4)},
]


class AgentFactory:
    """Generate diverse agent populations for prediction simulations."""

    def __init__(self, seed: int | None = None):
        self.rng = random.Random(seed)

    def _rand_range(self, r: tuple[float, float]) -> float:
        return self.rng.uniform(r[0], r[1])

    def create_agents(self, count: int, topic_tags: list[str] | None = None) -> list[AgentPersona]:
        """Generate `count` agents with diverse archetypes."""
        agents = []
        for i in range(count):
            archetype = self.rng.choice(ARCHETYPES)
            expertise = topic_tags[:3] if topic_tags else ["general"]
            agent = AgentPersona(
                agent_id=f"agent_{i:04d}",
                name=f"{archetype['name']} #{i}",
                archetype=archetype["archetype"],
                expertise=expertise,
                risk_tolerance=round(self._rand_range(archetype["risk_tolerance"]), 2),
                bias=round(self._rand_range(archetype["bias"]), 2),
                confidence=round(self._rand_range(archetype["confidence"]), 2),
                information_access=archetype["information_access"],
                contrarian_factor=round(self._rand_range(archetype["contrarian_factor"]), 2),
            )
            agents.append(agent)
        return agents

    def archetype_distribution(self, agents: list[AgentPersona]) -> dict[str, int]:
        """Count agents per archetype."""
        dist: dict[str, int] = {}
        for a in agents:
            dist[a.archetype] = dist.get(a.archetype, 0) + 1
        return dist
```

- [ ] **Step 2: Write tests**

```python
# tests/test_agent_factory.py
from tradoshka_poly.ai_predictor.agent_factory import AgentFactory


def test_create_agents_count():
    factory = AgentFactory(seed=42)
    agents = factory.create_agents(100)
    assert len(agents) == 100


def test_create_agents_unique_ids():
    factory = AgentFactory(seed=42)
    agents = factory.create_agents(50)
    ids = [a.agent_id for a in agents]
    assert len(set(ids)) == 50


def test_create_agents_diverse_archetypes():
    factory = AgentFactory(seed=42)
    agents = factory.create_agents(100)
    dist = factory.archetype_distribution(agents)
    assert len(dist) >= 4  # Should have multiple archetypes


def test_agent_properties_in_range():
    factory = AgentFactory(seed=42)
    agents = factory.create_agents(50)
    for a in agents:
        assert 0.0 <= a.risk_tolerance <= 1.0
        assert -1.0 <= a.bias <= 1.0
        assert 0.0 <= a.confidence <= 1.0
        assert 0.0 <= a.contrarian_factor <= 1.0


def test_deterministic_with_seed():
    agents1 = AgentFactory(seed=123).create_agents(10)
    agents2 = AgentFactory(seed=123).create_agents(10)
    assert [a.agent_id for a in agents1] == [a.agent_id for a in agents2]
    assert [a.bias for a in agents1] == [a.bias for a in agents2]
```

- [ ] **Step 3: Run tests, commit**

```bash
python -m pytest tests/test_agent_factory.py -v
git add strategies/polymarket/ && git commit -m "feat(polymarket): add agent factory for AI prediction"
```

---

### Task 4: Scorer (Probability Estimation)

**Files:**
- Create: `strategies/polymarket/tradoshka_poly/ai_predictor/scorer.py`
- Test: `strategies/polymarket/tests/test_scorer.py`

- [ ] **Step 1: Implement scorer**

This is what MiroFish DOESN'T have — quantitative probability estimation from agent votes. Each agent "votes" on an outcome, and we aggregate into a probability.

```python
from dataclasses import dataclass
from .agent_factory import AgentPersona


@dataclass
class AgentVote:
    """An agent's prediction for a binary outcome."""
    agent_id: str
    probability_yes: float  # 0.0 to 1.0
    confidence: float       # 0.0 to 1.0
    reasoning: str = ""


@dataclass
class PredictionResult:
    """Aggregated prediction from all agents."""
    probability_yes: float
    probability_no: float
    confidence: float          # Overall confidence (agreement among agents)
    num_agents: int
    bull_count: int            # Agents predicting >0.5
    bear_count: int            # Agents predicting <0.5
    std_dev: float             # Disagreement measure
    edge_vs_market: float      # Our prob - market prob


class PredictionScorer:
    """Convert agent votes into probability estimates."""

    def __init__(self, market_price: float = 0.5):
        self.market_price = market_price

    def aggregate(self, votes: list[AgentVote]) -> PredictionResult:
        """Aggregate agent votes using confidence-weighted average."""
        if not votes:
            return PredictionResult(
                probability_yes=0.5, probability_no=0.5,
                confidence=0.0, num_agents=0,
                bull_count=0, bear_count=0,
                std_dev=0.0, edge_vs_market=0.0,
            )

        # Confidence-weighted average
        total_weight = sum(v.confidence for v in votes)
        if total_weight == 0:
            total_weight = len(votes)
            weighted_sum = sum(v.probability_yes for v in votes)
        else:
            weighted_sum = sum(v.probability_yes * v.confidence for v in votes)

        prob_yes = weighted_sum / total_weight
        prob_yes = max(0.01, min(0.99, prob_yes))  # Clamp to avoid extremes

        # Calculate standard deviation as disagreement measure
        mean = prob_yes
        variance = sum((v.probability_yes - mean) ** 2 for v in votes) / len(votes)
        std_dev = variance ** 0.5

        # Overall confidence: high agreement = high confidence
        # Max confidence when std_dev = 0, min when std_dev = 0.5
        agreement_confidence = max(0.0, 1.0 - (std_dev / 0.5))

        # Average individual confidence
        avg_confidence = sum(v.confidence for v in votes) / len(votes)

        # Combined confidence
        confidence = (agreement_confidence * 0.6) + (avg_confidence * 0.4)

        bull_count = sum(1 for v in votes if v.probability_yes > 0.5)
        bear_count = sum(1 for v in votes if v.probability_yes < 0.5)

        edge = prob_yes - self.market_price

        return PredictionResult(
            probability_yes=round(prob_yes, 4),
            probability_no=round(1.0 - prob_yes, 4),
            confidence=round(confidence, 4),
            num_agents=len(votes),
            bull_count=bull_count,
            bear_count=bear_count,
            std_dev=round(std_dev, 4),
            edge_vs_market=round(edge, 4),
        )

    def should_trade(self, result: PredictionResult, min_edge: float = 0.05, min_confidence: float = 0.3) -> tuple[str, float] | None:
        """Determine if the prediction has enough edge to trade.

        Returns: ("YES", strength) or ("NO", strength) or None
        """
        if result.confidence < min_confidence:
            return None

        if result.edge_vs_market > min_edge:
            strength = min(1.0, result.edge_vs_market / 0.20)  # Normalize: 20% edge = max strength
            return ("YES", round(strength, 2))
        elif result.edge_vs_market < -min_edge:
            strength = min(1.0, abs(result.edge_vs_market) / 0.20)
            return ("NO", round(strength, 2))

        return None
```

- [ ] **Step 2: Write tests**

```python
# tests/test_scorer.py
from tradoshka_poly.ai_predictor.scorer import PredictionScorer, AgentVote


def test_aggregate_unanimous_bullish():
    scorer = PredictionScorer(market_price=0.5)
    votes = [
        AgentVote("a1", 0.8, 0.9),
        AgentVote("a2", 0.85, 0.8),
        AgentVote("a3", 0.75, 0.7),
    ]
    result = scorer.aggregate(votes)
    assert result.probability_yes > 0.75
    assert result.confidence > 0.5
    assert result.bull_count == 3
    assert result.bear_count == 0


def test_aggregate_mixed_opinions():
    scorer = PredictionScorer(market_price=0.5)
    votes = [
        AgentVote("a1", 0.8, 0.9),
        AgentVote("a2", 0.2, 0.9),
        AgentVote("a3", 0.5, 0.5),
    ]
    result = scorer.aggregate(votes)
    assert 0.3 < result.probability_yes < 0.7  # Mixed = near 0.5
    assert result.std_dev > 0.1  # High disagreement


def test_aggregate_empty():
    scorer = PredictionScorer(market_price=0.5)
    result = scorer.aggregate([])
    assert result.probability_yes == 0.5
    assert result.confidence == 0.0


def test_should_trade_strong_edge():
    scorer = PredictionScorer(market_price=0.5)
    votes = [AgentVote(f"a{i}", 0.75, 0.8) for i in range(10)]
    result = scorer.aggregate(votes)
    trade = scorer.should_trade(result, min_edge=0.05)
    assert trade is not None
    assert trade[0] == "YES"
    assert trade[1] > 0.0


def test_should_trade_no_edge():
    scorer = PredictionScorer(market_price=0.5)
    votes = [AgentVote(f"a{i}", 0.51, 0.5) for i in range(10)]
    result = scorer.aggregate(votes)
    trade = scorer.should_trade(result, min_edge=0.05)
    assert trade is None  # Edge too small


def test_should_trade_low_confidence():
    scorer = PredictionScorer(market_price=0.5)
    votes = [
        AgentVote("a1", 0.9, 0.1),  # Low confidence
        AgentVote("a2", 0.1, 0.1),
    ]
    result = scorer.aggregate(votes)
    trade = scorer.should_trade(result, min_confidence=0.5)
    assert trade is None  # Confidence too low
```

- [ ] **Step 3: Run tests, commit**

```bash
python -m pytest tests/test_scorer.py -v
git add strategies/polymarket/ && git commit -m "feat(polymarket): add prediction scorer with probability estimation"
```

---

### Task 5: World Builder + Simulation + AI Predictor Strategy

**Files:**
- Create: `strategies/polymarket/tradoshka_poly/ai_predictor/world_builder.py`
- Create: `strategies/polymarket/tradoshka_poly/ai_predictor/simulation.py`
- Create: `strategies/polymarket/tradoshka_poly/ai_predictor/predictor.py`

- [ ] **Step 1: Implement world builder**

Constructs the context for a prediction simulation from market data.

```python
from dataclasses import dataclass


@dataclass
class SimulationWorld:
    """Context for a prediction simulation."""
    market_question: str
    market_description: str
    current_yes_price: float
    current_no_price: float
    volume_24h: float
    end_date: str | None
    context_data: list[str]  # News headlines, facts, data points

    @property
    def market_implied_probability(self) -> float:
        return self.current_yes_price


class WorldBuilder:
    """Constructs simulation worlds from Polymarket data."""

    def build(
        self,
        question: str,
        description: str = "",
        yes_price: float = 0.5,
        no_price: float = 0.5,
        volume: float = 0.0,
        end_date: str | None = None,
        context: list[str] | None = None,
    ) -> SimulationWorld:
        return SimulationWorld(
            market_question=question,
            market_description=description,
            current_yes_price=yes_price,
            current_no_price=no_price,
            volume_24h=volume,
            end_date=end_date,
            context_data=context or [],
        )
```

- [ ] **Step 2: Implement simulation runner**

Runs the multi-agent prediction without OASIS — our own lightweight version focused on voting rather than social media simulation.

```python
from dataclasses import dataclass
from .agent_factory import AgentFactory, AgentPersona
from .scorer import AgentVote, PredictionScorer, PredictionResult
from .world_builder import SimulationWorld
from .llm_client import LLMClient, LLMConfig
import json


PREDICTION_SYSTEM_PROMPT = """You are {agent_name}, a {archetype} with expertise in {expertise}.
Your risk tolerance is {risk_tolerance}/1.0 and you have {information_access} information access.
Your natural bias: {bias} (-1=bearish, +1=bullish). Contrarian tendency: {contrarian_factor}/1.0.

Analyze the prediction market question and provide your honest probability estimate."""

PREDICTION_USER_PROMPT = """Market Question: {question}
{description}

Current Market Price: YES={yes_price:.0%}, NO={no_price:.0%}
24h Volume: ${volume:,.0f}
{end_date_line}

{context_section}

Respond with JSON only:
{{"probability_yes": 0.XX, "confidence": 0.XX, "reasoning": "brief explanation"}}"""


class PredictionSimulation:
    """Run multi-agent prediction simulation."""

    def __init__(self, llm_config: LLMConfig | None = None, agent_count: int = 20, seed: int | None = None):
        self.llm = LLMClient(llm_config) if llm_config and llm_config.api_key else None
        self.factory = AgentFactory(seed=seed)
        self.agent_count = agent_count

    def run(self, world: SimulationWorld) -> PredictionResult:
        """Run simulation and return aggregated prediction."""
        agents = self.factory.create_agents(self.agent_count, topic_tags=["prediction_markets"])

        if self.llm is None:
            # Fallback: use statistical simulation without LLM
            votes = self._simulate_statistical(agents, world)
        else:
            votes = self._simulate_with_llm(agents, world)

        scorer = PredictionScorer(market_price=world.current_yes_price)
        return scorer.aggregate(votes)

    def _simulate_statistical(self, agents: list[AgentPersona], world: SimulationWorld) -> list[AgentVote]:
        """Statistical fallback when no LLM is available.
        Each agent's vote is based on their bias + noise + market price."""
        import random
        votes = []
        for agent in agents:
            # Base prediction influenced by market price and agent bias
            base = world.current_yes_price + (agent.bias * 0.15)
            # Add noise based on confidence (less confident = more noise)
            noise = random.gauss(0, 0.1 * (1 - agent.confidence))
            # Contrarians move away from market consensus
            if agent.contrarian_factor > 0.5:
                base = 1.0 - base  # Flip
                base = world.current_yes_price + (base - world.current_yes_price) * agent.contrarian_factor
            prob = max(0.01, min(0.99, base + noise))
            votes.append(AgentVote(
                agent_id=agent.agent_id,
                probability_yes=round(prob, 3),
                confidence=agent.confidence,
                reasoning=f"Statistical estimate by {agent.archetype}",
            ))
        return votes

    def _simulate_with_llm(self, agents: list[AgentPersona], world: SimulationWorld) -> list[AgentVote]:
        """Full LLM-driven simulation — each agent reasons independently."""
        votes = []
        for agent in agents:
            try:
                vote = self._get_agent_vote(agent, world)
                votes.append(vote)
            except Exception:
                # If LLM fails for one agent, skip
                continue
        return votes

    def _get_agent_vote(self, agent: AgentPersona, world: SimulationWorld) -> AgentVote:
        system = PREDICTION_SYSTEM_PROMPT.format(
            agent_name=agent.name, archetype=agent.archetype,
            expertise=", ".join(agent.expertise), risk_tolerance=agent.risk_tolerance,
            information_access=agent.information_access, bias=agent.bias,
            contrarian_factor=agent.contrarian_factor,
        )
        context_section = ""
        if world.context_data:
            context_section = "Context:\n" + "\n".join(f"- {c}" for c in world.context_data[:10])
        end_date_line = f"End Date: {world.end_date}" if world.end_date else ""
        user = PREDICTION_USER_PROMPT.format(
            question=world.market_question, description=world.market_description,
            yes_price=world.current_yes_price, no_price=world.current_no_price,
            volume=world.volume_24h, end_date_line=end_date_line,
            context_section=context_section,
        )
        result = self.llm.chat_json(system, user, temperature=0.8)
        return AgentVote(
            agent_id=agent.agent_id,
            probability_yes=max(0.01, min(0.99, float(result.get("probability_yes", 0.5)))),
            confidence=max(0.0, min(1.0, float(result.get("confidence", 0.5)))),
            reasoning=result.get("reasoning", ""),
        )
```

- [ ] **Step 3: Implement AI Predictor strategy**

```python
from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection


class AIPredictorStrategy(BaseStrategy):
    """MiroFish-inspired AI prediction strategy for Polymarket."""

    def __init__(self, llm_config=None, agent_count: int = 20):
        super().__init__("ai_predictor", "polymarket")
        from .simulation import PredictionSimulation
        from .world_builder import WorldBuilder
        self.sim = PredictionSimulation(llm_config=llm_config, agent_count=agent_count)
        self.world_builder = WorldBuilder()
        self._last_predictions: dict[str, float] = {}  # symbol -> prob_yes

    def name(self) -> str:
        return "AI Predictor (MiroFish-inspired)"

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None

        # Build world from market event
        world = self.world_builder.build(
            question=event.symbol,
            yes_price=event.price,
            no_price=1.0 - event.price,
            volume=event.volume,
        )

        # Run simulation
        result = self.sim.run(world)
        self._last_predictions[event.symbol] = result.probability_yes

        # Generate signal if edge exists
        from .scorer import PredictionScorer
        scorer = PredictionScorer(market_price=event.price)
        trade = scorer.should_trade(result, min_edge=0.05, min_confidence=0.3)

        if trade is None:
            return Signal(self.strategy_id, event.symbol, SignalDirection.HOLD, 0.0)

        direction = SignalDirection.LONG if trade[0] == "YES" else SignalDirection.SHORT
        return Signal(self.strategy_id, event.symbol, direction, trade[1])

    def on_fill(self, fill) -> None:
        pass
```

- [ ] **Step 4: Commit**

```bash
git add strategies/polymarket/
git commit -m "feat(polymarket): add AI predictor with multi-agent simulation"
```

---

### Task 6: Copy Trading Strategy

**Files:**
- Create: `strategies/polymarket/tradoshka_poly/copy_trading/wallet_tracker.py`
- Create: `strategies/polymarket/tradoshka_poly/copy_trading/filters.py`
- Create: `strategies/polymarket/tradoshka_poly/copy_trading/signal_extractor.py`
- Create: `strategies/polymarket/tradoshka_poly/copy_trading/strategy.py`
- Tests: `test_wallet_tracker.py`, `test_filters.py`

- [ ] **Step 1: Implement trader filters**

```python
from dataclasses import dataclass


@dataclass
class TraderProfile:
    address: str
    total_pnl: float
    roi: float
    win_rate: float
    total_trades: int
    markets_traded: int
    avg_position_size: float
    last_active_days_ago: int


@dataclass
class TraderFilter:
    min_pnl: float = 5000.0
    min_roi: float = 0.15
    min_win_rate: float = 0.55
    min_trades: int = 50
    min_markets: int = 10
    min_avg_position: float = 100.0
    max_inactive_days: int = 90


def filter_traders(traders: list[TraderProfile], criteria: TraderFilter | None = None) -> list[TraderProfile]:
    c = criteria or TraderFilter()
    return [t for t in traders if (
        t.total_pnl >= c.min_pnl and
        t.roi >= c.min_roi and
        t.win_rate >= c.min_win_rate and
        t.total_trades >= c.min_trades and
        t.markets_traded >= c.min_markets and
        t.avg_position_size >= c.min_avg_position and
        t.last_active_days_ago <= c.max_inactive_days
    )]
```

- [ ] **Step 2: Implement wallet tracker**

```python
from dataclasses import dataclass, field
from datetime import datetime, timezone


@dataclass
class WhaleTrade:
    address: str
    market_id: str
    token_id: str
    side: str          # "BUY" or "SELL"
    outcome: str       # "Yes" or "No"
    size: float        # Number of shares
    price: float       # Execution price
    timestamp: datetime
    title: str = ""


class WalletTracker:
    """Track and cache trades from monitored wallets."""

    def __init__(self):
        self._tracked_wallets: set[str] = set()
        self._trade_history: dict[str, list[WhaleTrade]] = {}  # address -> trades
        self._last_check: dict[str, datetime] = {}

    def add_wallet(self, address: str) -> None:
        self._tracked_wallets.add(address.lower())
        self._trade_history.setdefault(address.lower(), [])

    def remove_wallet(self, address: str) -> None:
        self._tracked_wallets.discard(address.lower())

    @property
    def tracked_wallets(self) -> set[str]:
        return self._tracked_wallets.copy()

    def record_trade(self, trade: WhaleTrade) -> None:
        addr = trade.address.lower()
        if addr in self._tracked_wallets:
            self._trade_history.setdefault(addr, []).append(trade)

    def get_recent_trades(self, address: str, limit: int = 10) -> list[WhaleTrade]:
        addr = address.lower()
        trades = self._trade_history.get(addr, [])
        return sorted(trades, key=lambda t: t.timestamp, reverse=True)[:limit]

    def get_new_trades_since(self, address: str, since: datetime) -> list[WhaleTrade]:
        addr = address.lower()
        trades = self._trade_history.get(addr, [])
        return [t for t in trades if t.timestamp > since]
```

- [ ] **Step 3: Implement signal extractor**

```python
from dataclasses import dataclass
from .wallet_tracker import WhaleTrade
from tradoshka_strategy import Signal, SignalDirection


@dataclass
class CopySignalConfig:
    max_price_deviation: float = 0.05   # Skip if price moved >5% since whale traded
    min_whale_size: float = 100.0       # Min shares in whale's trade
    position_scale: float = 0.05        # 5% of whale's size
    min_position: float = 10.0          # Minimum $10
    max_position: float = 500.0         # Maximum $500
    max_staleness_secs: int = 300       # Skip trades older than 5 minutes


class SignalExtractor:
    """Convert whale trades into copy trading signals."""

    def __init__(self, config: CopySignalConfig | None = None):
        self.config = config or CopySignalConfig()

    def extract(self, trade: WhaleTrade, current_price: float, strategy_id: str = "copy_trading") -> Signal | None:
        # Skip small trades
        if trade.size < self.config.min_whale_size:
            return None

        # Check price deviation
        price_diff = abs(current_price - trade.price)
        if price_diff > self.config.max_price_deviation:
            return None

        # Determine direction
        if trade.side == "BUY":
            direction = SignalDirection.LONG
        elif trade.side == "SELL":
            direction = SignalDirection.SHORT
        else:
            return None

        # Calculate strength based on trade size
        strength = min(1.0, trade.size / 1000.0)  # 1000 shares = max strength

        return Signal(
            strategy_id=strategy_id,
            symbol=trade.token_id,
            direction=direction,
            strength=round(strength, 2),
            metadata={
                "whale_address": trade.address,
                "whale_size": str(trade.size),
                "whale_price": str(trade.price),
                "outcome": trade.outcome,
            },
        )
```

- [ ] **Step 4: Implement copy trading strategy**

```python
from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection
from .wallet_tracker import WalletTracker
from .signal_extractor import SignalExtractor, CopySignalConfig


class CopyTradingStrategy(BaseStrategy):
    """Copy trades from top Polymarket wallets."""

    def __init__(self, config: CopySignalConfig | None = None):
        super().__init__("copy_trading", "polymarket")
        self.tracker = WalletTracker()
        self.extractor = SignalExtractor(config)

    def name(self) -> str:
        return "Polymarket Copy Trading"

    def add_wallet(self, address: str) -> None:
        self.tracker.add_wallet(address)

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        # In real usage, this would check for new whale trades
        # and generate copy signals. For now, returns None.
        return None

    def on_fill(self, fill) -> None:
        pass
```

- [ ] **Step 5: Write tests**

```python
# tests/test_filters.py
from tradoshka_poly.copy_trading.filters import TraderProfile, TraderFilter, filter_traders

def test_filter_passes_good_trader():
    traders = [TraderProfile("0x1", 10000, 0.25, 0.6, 100, 20, 500, 5)]
    assert len(filter_traders(traders)) == 1

def test_filter_rejects_low_pnl():
    traders = [TraderProfile("0x1", 100, 0.25, 0.6, 100, 20, 500, 5)]
    assert len(filter_traders(traders)) == 0

def test_filter_rejects_low_win_rate():
    traders = [TraderProfile("0x1", 10000, 0.25, 0.4, 100, 20, 500, 5)]
    assert len(filter_traders(traders)) == 0

def test_filter_rejects_inactive():
    traders = [TraderProfile("0x1", 10000, 0.25, 0.6, 100, 20, 500, 180)]
    assert len(filter_traders(traders)) == 0
```

```python
# tests/test_wallet_tracker.py
from datetime import datetime, timezone
from tradoshka_poly.copy_trading.wallet_tracker import WalletTracker, WhaleTrade

def test_add_and_track_wallet():
    tracker = WalletTracker()
    tracker.add_wallet("0xABC")
    assert "0xabc" in tracker.tracked_wallets

def test_record_and_retrieve_trade():
    tracker = WalletTracker()
    tracker.add_wallet("0xABC")
    trade = WhaleTrade("0xabc", "mkt1", "tok1", "BUY", "Yes", 100.0, 0.55, datetime.now(timezone.utc))
    tracker.record_trade(trade)
    recent = tracker.get_recent_trades("0xABC")
    assert len(recent) == 1
    assert recent[0].side == "BUY"

def test_ignores_untracked_wallet():
    tracker = WalletTracker()
    trade = WhaleTrade("0xunknown", "mkt1", "tok1", "BUY", "Yes", 100.0, 0.55, datetime.now(timezone.utc))
    tracker.record_trade(trade)
    assert len(tracker.get_recent_trades("0xunknown")) == 0
```

- [ ] **Step 6: Run tests, commit**

```bash
python -m pytest tests/test_filters.py tests/test_wallet_tracker.py -v
git add strategies/polymarket/ && git commit -m "feat(polymarket): add copy trading strategy with wallet tracking"
```

---

### Task 7: Market Making Strategy

**Files:**
- Create: `strategies/polymarket/tradoshka_poly/market_making/spread.py`
- Create: `strategies/polymarket/tradoshka_poly/market_making/inventory.py`
- Create: `strategies/polymarket/tradoshka_poly/market_making/strategy.py`
- Tests: `test_spread.py`, `test_inventory.py`

- [ ] **Step 1: Implement spread calculator**

```python
from dataclasses import dataclass


@dataclass
class QuoteParams:
    fair_value: float          # Our estimate of fair price
    base_spread_bps: int = 200 # 2% base spread
    min_spread_bps: int = 50   # 0.5% minimum spread
    inventory_skew: float = 0.0 # Shift quotes based on inventory


class SpreadCalculator:
    """Calculate bid/ask quotes around a fair value."""

    def calculate_quotes(self, params: QuoteParams) -> tuple[float, float]:
        """Returns (bid_price, ask_price)."""
        half_spread = max(params.base_spread_bps, params.min_spread_bps) / 10000 / 2

        # Skew quotes based on inventory
        # Positive inventory → lower bid (want less), higher ask
        skew = params.inventory_skew * half_spread * 0.5

        bid = params.fair_value - half_spread - skew
        ask = params.fair_value + half_spread - skew

        # Clamp to valid Polymarket range [0.01, 0.99]
        bid = max(0.01, min(0.99, bid))
        ask = max(0.01, min(0.99, ask))

        # Ensure bid < ask
        if bid >= ask:
            mid = (bid + ask) / 2
            bid = mid - 0.01
            ask = mid + 0.01

        return round(bid, 4), round(ask, 4)
```

- [ ] **Step 2: Implement inventory manager**

```python
from dataclasses import dataclass, field


@dataclass
class InventoryState:
    yes_shares: float = 0.0
    no_shares: float = 0.0
    total_cost: float = 0.0
    realized_pnl: float = 0.0

    @property
    def net_exposure(self) -> float:
        """Positive = long yes, negative = long no."""
        return self.yes_shares - self.no_shares

    @property
    def inventory_skew(self) -> float:
        """Normalized skew: -1.0 to 1.0."""
        total = self.yes_shares + self.no_shares
        if total == 0:
            return 0.0
        return self.net_exposure / total


class InventoryManager:
    """Track and manage market making inventory."""

    def __init__(self, max_inventory: float = 1000.0):
        self.state = InventoryState()
        self.max_inventory = max_inventory

    def can_buy_yes(self, size: float) -> bool:
        return self.state.yes_shares + size <= self.max_inventory

    def can_buy_no(self, size: float) -> bool:
        return self.state.no_shares + size <= self.max_inventory

    def record_buy_yes(self, size: float, price: float) -> None:
        self.state.yes_shares += size
        self.state.total_cost += size * price

    def record_buy_no(self, size: float, price: float) -> None:
        self.state.no_shares += size
        self.state.total_cost += size * price

    def record_sell_yes(self, size: float, price: float) -> None:
        self.state.yes_shares -= size
        self.state.realized_pnl += size * price

    def record_sell_no(self, size: float, price: float) -> None:
        self.state.no_shares -= size
        self.state.realized_pnl += size * price
```

- [ ] **Step 3: Implement market making strategy**

```python
from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection
from .spread import SpreadCalculator, QuoteParams
from .inventory import InventoryManager


class MarketMakingStrategy(BaseStrategy):
    def __init__(self, base_spread_bps: int = 200, max_inventory: float = 1000.0):
        super().__init__("market_making", "polymarket")
        self.spread_calc = SpreadCalculator()
        self.inventory = InventoryManager(max_inventory)
        self.base_spread_bps = base_spread_bps

    def name(self) -> str:
        return "Polymarket Market Making"

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        params = QuoteParams(
            fair_value=event.price,
            base_spread_bps=self.base_spread_bps,
            inventory_skew=self.inventory.state.inventory_skew,
        )
        bid, ask = self.spread_calc.calculate_quotes(params)
        # Signal to place bid (if we can take more inventory)
        if self.inventory.can_buy_yes(10):
            return Signal(self.strategy_id, event.symbol, SignalDirection.LONG, 0.3,
                          stop_loss=bid, take_profit=ask, metadata={"bid": str(bid), "ask": str(ask)})
        return None

    def on_fill(self, fill) -> None:
        if fill.side == "BUY":
            self.inventory.record_buy_yes(fill.quantity, fill.price)
        else:
            self.inventory.record_sell_yes(fill.quantity, fill.price)
```

- [ ] **Step 4: Write tests, run, commit**

```python
# tests/test_spread.py
from tradoshka_poly.market_making.spread import SpreadCalculator, QuoteParams

def test_symmetric_spread():
    calc = SpreadCalculator()
    bid, ask = calc.calculate_quotes(QuoteParams(fair_value=0.5))
    assert bid < 0.5 < ask
    assert abs((ask - bid) - 0.02) < 0.001  # ~2% spread

def test_inventory_skew_shifts_quotes():
    calc = SpreadCalculator()
    bid_neutral, ask_neutral = calc.calculate_quotes(QuoteParams(fair_value=0.5, inventory_skew=0.0))
    bid_long, ask_long = calc.calculate_quotes(QuoteParams(fair_value=0.5, inventory_skew=0.5))
    assert bid_long < bid_neutral  # Long inventory → lower bid
```

```python
# tests/test_inventory.py
from tradoshka_poly.market_making.inventory import InventoryManager

def test_initial_state():
    mgr = InventoryManager()
    assert mgr.state.net_exposure == 0.0
    assert mgr.state.inventory_skew == 0.0

def test_buy_yes_increases_exposure():
    mgr = InventoryManager()
    mgr.record_buy_yes(100, 0.5)
    assert mgr.state.yes_shares == 100
    assert mgr.state.net_exposure == 100

def test_max_inventory_check():
    mgr = InventoryManager(max_inventory=50)
    assert mgr.can_buy_yes(50) == True
    assert mgr.can_buy_yes(51) == False
```

```bash
python -m pytest tests/test_spread.py tests/test_inventory.py -v
git add strategies/polymarket/ && git commit -m "feat(polymarket): add market making strategy with spread and inventory"
```

---

### Task 8: Arbitrage Strategy

**Files:**
- Create: `strategies/polymarket/tradoshka_poly/arbitrage/mispricing.py`
- Create: `strategies/polymarket/tradoshka_poly/arbitrage/strategy.py`
- Test: `test_mispricing.py`

- [ ] **Step 1: Implement mispricing detector**

```python
from dataclasses import dataclass


@dataclass
class MispricingSignal:
    market_id: str
    yes_token_id: str
    no_token_id: str
    yes_price: float
    no_price: float
    total: float
    deviation: float        # |total - 1.0|
    direction: str          # "BUY_YES" or "BUY_NO" (buy the underpriced side)
    expected_profit_pct: float


class MispricingDetector:
    """Detect arbitrage opportunities from Yes+No price deviations."""

    def __init__(self, min_deviation: float = 0.02, min_profit_pct: float = 0.005):
        self.min_deviation = min_deviation
        self.min_profit_pct = min_profit_pct

    def detect(self, yes_price: float, no_price: float,
               yes_token_id: str = "", no_token_id: str = "",
               market_id: str = "") -> MispricingSignal | None:
        total = yes_price + no_price
        deviation = total - 1.0  # Positive = overpriced, negative = underpriced

        if abs(deviation) < self.min_deviation:
            return None

        if total > 1.0:
            # Both sides are overpriced — sell both (merge for $1)
            # Or find the more overpriced side and sell it
            # For simplicity: buy the cheaper side
            if yes_price < no_price:
                direction = "BUY_YES"
                profit_pct = (1.0 - yes_price - (1.0 - no_price)) / yes_price
            else:
                direction = "BUY_NO"
                profit_pct = (1.0 - no_price - (1.0 - yes_price)) / no_price
        else:
            # Both sides are underpriced — buy both (split $1)
            # Buy both, guaranteed $1 payout
            cost = yes_price + no_price
            profit_pct = (1.0 - cost) / cost

            direction = "BUY_YES" if yes_price < no_price else "BUY_NO"

        if profit_pct < self.min_profit_pct:
            return None

        return MispricingSignal(
            market_id=market_id,
            yes_token_id=yes_token_id,
            no_token_id=no_token_id,
            yes_price=yes_price,
            no_price=no_price,
            total=round(total, 4),
            deviation=round(abs(deviation), 4),
            direction=direction,
            expected_profit_pct=round(profit_pct, 4),
        )
```

- [ ] **Step 2: Implement arbitrage strategy**

```python
from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection
from .mispricing import MispricingDetector


class ArbitrageStrategy(BaseStrategy):
    def __init__(self, min_deviation: float = 0.02):
        super().__init__("arbitrage", "polymarket")
        self.detector = MispricingDetector(min_deviation=min_deviation)

    def name(self) -> str:
        return "Polymarket Arbitrage"

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        # Arbitrage works on pairs — needs both Yes and No prices
        # In practice, would receive both from the scanner
        return None

    def on_fill(self, fill) -> None:
        pass
```

- [ ] **Step 3: Write tests**

```python
# tests/test_mispricing.py
from tradoshka_poly.arbitrage.mispricing import MispricingDetector

def test_no_mispricing_at_fair():
    det = MispricingDetector(min_deviation=0.02)
    result = det.detect(0.50, 0.50)
    assert result is None

def test_detects_overpriced():
    det = MispricingDetector(min_deviation=0.02)
    result = det.detect(0.55, 0.50)
    assert result is not None
    assert result.total > 1.0
    assert result.deviation > 0.02

def test_detects_underpriced():
    det = MispricingDetector(min_deviation=0.02)
    result = det.detect(0.45, 0.50)
    assert result is not None
    assert result.total < 1.0

def test_ignores_small_deviation():
    det = MispricingDetector(min_deviation=0.05)
    result = det.detect(0.51, 0.50)
    assert result is None  # deviation = 0.01 < 0.05
```

```bash
python -m pytest tests/test_mispricing.py -v
git add strategies/polymarket/ && git commit -m "feat(polymarket): add arbitrage strategy with mispricing detection"
```

---

### Task 9: Ensemble Combiner

**Files:**
- Create: `strategies/polymarket/tradoshka_poly/ensemble/combiner.py`
- Test: `strategies/polymarket/tests/test_combiner.py`

- [ ] **Step 1: Implement ensemble combiner**

```python
from dataclasses import dataclass, field
from tradoshka_strategy import Signal, SignalDirection


@dataclass
class StrategyWeight:
    strategy_id: str
    weight: float           # 0.0 to 1.0
    recent_accuracy: float = 0.5  # Tracks how accurate this strategy has been


class EnsembleCombiner:
    """Combine signals from multiple strategies using weighted voting."""

    def __init__(self, weights: dict[str, float] | None = None):
        self.weights: dict[str, StrategyWeight] = {}
        if weights:
            for sid, w in weights.items():
                self.weights[sid] = StrategyWeight(strategy_id=sid, weight=w)

    def set_weight(self, strategy_id: str, weight: float) -> None:
        if strategy_id in self.weights:
            self.weights[strategy_id].weight = weight
        else:
            self.weights[strategy_id] = StrategyWeight(strategy_id=strategy_id, weight=weight)

    def combine(self, signals: list[Signal], symbol: str) -> Signal | None:
        """Combine multiple signals into a single ensemble signal."""
        if not signals:
            return None

        # Filter to actionable signals for this symbol
        relevant = [s for s in signals if s.symbol == symbol and s.is_actionable]
        if not relevant:
            return None

        # Weighted vote
        long_score = 0.0
        short_score = 0.0
        close_score = 0.0
        total_weight = 0.0

        for signal in relevant:
            weight = self.weights.get(signal.strategy_id, StrategyWeight(signal.strategy_id, 1.0)).weight
            weighted_strength = signal.strength * weight

            if signal.direction == SignalDirection.LONG:
                long_score += weighted_strength
            elif signal.direction == SignalDirection.SHORT:
                short_score += weighted_strength
            elif signal.direction == SignalDirection.CLOSE:
                close_score += weighted_strength

            total_weight += weight

        if total_weight == 0:
            return None

        # Normalize
        long_score /= total_weight
        short_score /= total_weight
        close_score /= total_weight

        # Pick the dominant direction
        scores = {"long": long_score, "short": short_score, "close": close_score}
        best = max(scores, key=scores.get)

        if best == "long" and long_score > 0.1:
            return Signal("ensemble", symbol, SignalDirection.LONG, round(long_score, 2))
        elif best == "short" and short_score > 0.1:
            return Signal("ensemble", symbol, SignalDirection.SHORT, round(short_score, 2))
        elif best == "close" and close_score > 0.1:
            return Signal("ensemble", symbol, SignalDirection.CLOSE, round(close_score, 2))

        return None

    def update_accuracy(self, strategy_id: str, was_correct: bool) -> None:
        """Update strategy accuracy for adaptive weighting."""
        if strategy_id in self.weights:
            sw = self.weights[strategy_id]
            # Exponential moving average of accuracy
            alpha = 0.1
            sw.recent_accuracy = alpha * (1.0 if was_correct else 0.0) + (1 - alpha) * sw.recent_accuracy
```

- [ ] **Step 2: Write tests**

```python
# tests/test_combiner.py
from tradoshka_poly.ensemble.combiner import EnsembleCombiner
from tradoshka_strategy import Signal, SignalDirection

def test_combine_unanimous_long():
    combiner = EnsembleCombiner({"ai": 0.35, "copy": 0.25, "arb": 0.20})
    signals = [
        Signal("ai", "BTC_YES", SignalDirection.LONG, 0.8),
        Signal("copy", "BTC_YES", SignalDirection.LONG, 0.6),
        Signal("arb", "BTC_YES", SignalDirection.LONG, 0.4),
    ]
    result = combiner.combine(signals, "BTC_YES")
    assert result is not None
    assert result.direction == SignalDirection.LONG
    assert result.strength > 0.3

def test_combine_conflicting_signals():
    combiner = EnsembleCombiner({"ai": 0.5, "copy": 0.5})
    signals = [
        Signal("ai", "ETH_YES", SignalDirection.LONG, 0.8),
        Signal("copy", "ETH_YES", SignalDirection.SHORT, 0.8),
    ]
    result = combiner.combine(signals, "ETH_YES")
    # Both cancel out — result might be LONG or SHORT with low strength
    if result:
        assert result.strength < 0.5

def test_combine_empty():
    combiner = EnsembleCombiner()
    assert combiner.combine([], "X") is None

def test_combine_ignores_other_symbols():
    combiner = EnsembleCombiner({"ai": 1.0})
    signals = [Signal("ai", "OTHER", SignalDirection.LONG, 0.9)]
    assert combiner.combine(signals, "BTC_YES") is None

def test_weighted_favors_higher_weight():
    combiner = EnsembleCombiner({"strong": 0.9, "weak": 0.1})
    signals = [
        Signal("strong", "X", SignalDirection.LONG, 0.8),
        Signal("weak", "X", SignalDirection.SHORT, 0.8),
    ]
    result = combiner.combine(signals, "X")
    assert result is not None
    assert result.direction == SignalDirection.LONG  # Strong outweighs weak
```

- [ ] **Step 3: Run tests, commit**

```bash
python -m pytest tests/test_combiner.py -v
git add strategies/polymarket/ && git commit -m "feat(polymarket): add ensemble combiner with weighted signal fusion"
```

---

### Task 10: Full Verification + Integration

- [ ] **Step 1: Run all Python tests**

```bash
cd strategies/polymarket && python -m pytest tests/ -v
cd ../shared && python -m pytest tests/ -v
```

- [ ] **Step 2: Run all Rust tests**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --workspace
```

- [ ] **Step 3: Update __init__.py exports**

Ensure `tradoshka_poly/__init__.py` correctly exports all strategies.

- [ ] **Step 4: Merge to dev**

```bash
git checkout dev
git merge feature/polymarket-strategies
```

- [ ] **Step 5: Update GOALS.md**

```bash
git add docs/GOALS.md
git commit -m "docs: update goals — Phase 1B Polymarket strategies complete"
```

---

## Summary

| Task | Component | Tests |
|------|-----------|-------|
| 1 | Package setup | — |
| 2 | LLM client | 2 |
| 3 | Agent factory | 5 |
| 4 | Prediction scorer | 6 |
| 5 | World builder + simulation + AI strategy | — |
| 6 | Copy trading (filters, tracker, signals) | 7 |
| 7 | Market making (spread, inventory) | 5 |
| 8 | Arbitrage (mispricing detection) | 4 |
| 9 | Ensemble combiner | 5 |
| 10 | Full verification | All tests |

**Total: 10 tasks, ~34 new Python tests**

Next plan: **Phase 1C — Dashboard v1** (Next.js public performance page)
