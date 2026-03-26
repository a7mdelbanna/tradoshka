import random
from .agent_factory import AgentFactory, AgentPersona
from .scorer import AgentVote, PredictionScorer, PredictionResult
from .world_builder import SimulationWorld
from .llm_client import LLMClient, LLMConfig


class PredictionSimulation:
    def __init__(self, llm_config: LLMConfig | None = None, agent_count: int = 20, seed: int | None = None):
        self.llm = LLMClient(llm_config) if llm_config and llm_config.api_key else None
        self.factory = AgentFactory(seed=seed)
        self.agent_count = agent_count

    def run(self, world: SimulationWorld) -> PredictionResult:
        agents = self.factory.create_agents(self.agent_count, topic_tags=["prediction_markets"])
        if self.llm is None:
            votes = self._simulate_statistical(agents, world)
        else:
            votes = self._simulate_with_llm(agents, world)
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

    def _simulate_with_llm(self, agents: list[AgentPersona], world: SimulationWorld) -> list[AgentVote]:
        votes = []
        for agent in agents:
            try:
                vote = self._get_agent_vote(agent, world)
                votes.append(vote)
            except Exception:
                continue
        return votes

    def _get_agent_vote(self, agent: AgentPersona, world: SimulationWorld) -> AgentVote:
        system = (f"You are {agent.name}, a {agent.archetype} with expertise in {', '.join(agent.expertise)}. "
                  f"Risk tolerance: {agent.risk_tolerance}/1.0. Information access: {agent.information_access}. "
                  f"Natural bias: {agent.bias}. Contrarian tendency: {agent.contrarian_factor}/1.0.")
        context = "\n".join(f"- {c}" for c in world.context_data[:10]) if world.context_data else "No additional context."
        end_line = f"End Date: {world.end_date}" if world.end_date else ""
        user = (f"Market: {world.market_question}\n{world.market_description}\n"
                f"Current: YES={world.current_yes_price:.0%}, NO={world.current_no_price:.0%}\n"
                f"Volume: ${world.volume_24h:,.0f}\n{end_line}\nContext:\n{context}\n\n"
                f'Respond JSON only: {{"probability_yes": 0.XX, "confidence": 0.XX, "reasoning": "brief"}}')
        result = self.llm.chat_json(system, user, temperature=0.8)
        return AgentVote(agent.agent_id,
                         max(0.01, min(0.99, float(result.get("probability_yes", 0.5)))),
                         max(0.0, min(1.0, float(result.get("confidence", 0.5)))),
                         result.get("reasoning", ""))
