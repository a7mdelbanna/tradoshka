from dataclasses import dataclass
import random


@dataclass
class AgentPersona:
    agent_id: str
    name: str
    archetype: str
    expertise: list[str]
    risk_tolerance: float   # 0.0-1.0
    bias: float             # -1.0 to 1.0
    confidence: float       # 0.0-1.0
    information_access: str # "high"/"medium"/"low"
    contrarian_factor: float # 0.0-1.0


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
    def __init__(self, seed: int | None = None):
        self.rng = random.Random(seed)

    def _rand_range(self, r: tuple[float, float]) -> float:
        return self.rng.uniform(r[0], r[1])

    def create_agents(self, count: int, topic_tags: list[str] | None = None) -> list[AgentPersona]:
        agents = []
        for i in range(count):
            archetype = self.rng.choice(ARCHETYPES)
            expertise = topic_tags[:3] if topic_tags else ["general"]
            agents.append(AgentPersona(
                agent_id=f"agent_{i:04d}",
                name=f"{archetype['name']} #{i}",
                archetype=archetype["archetype"],
                expertise=expertise,
                risk_tolerance=round(self._rand_range(archetype["risk_tolerance"]), 2),
                bias=round(self._rand_range(archetype["bias"]), 2),
                confidence=round(self._rand_range(archetype["confidence"]), 2),
                information_access=archetype["information_access"],
                contrarian_factor=round(self._rand_range(archetype["contrarian_factor"]), 2),
            ))
        return agents

    def archetype_distribution(self, agents: list[AgentPersona]) -> dict[str, int]:
        dist: dict[str, int] = {}
        for a in agents:
            dist[a.archetype] = dist.get(a.archetype, 0) + 1
        return dist
