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
        self._seed = seed
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
        rng = random.Random(self._seed)
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
        except RuntimeError as e:
            logger.warning("Claude prediction failed (%s), falling back to statistical", e)
            return self._simulate_statistical(agents, world)

    def _build_batch_prompt(self, agents: list[AgentPersona], world: SimulationWorld) -> str:
        context = "\n".join(f"- {c}" for c in world.context_data[:10]) if world.context_data else "No additional context."

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
