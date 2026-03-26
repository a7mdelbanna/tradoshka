from dataclasses import dataclass


@dataclass
class AgentVote:
    agent_id: str
    probability_yes: float  # 0.0 to 1.0
    confidence: float       # 0.0 to 1.0
    reasoning: str = ""


@dataclass
class PredictionResult:
    probability_yes: float
    probability_no: float
    confidence: float
    num_agents: int
    bull_count: int
    bear_count: int
    std_dev: float
    edge_vs_market: float


class PredictionScorer:
    def __init__(self, market_price: float = 0.5):
        self.market_price = market_price

    def aggregate(self, votes: list[AgentVote]) -> PredictionResult:
        if not votes:
            return PredictionResult(0.5, 0.5, 0.0, 0, 0, 0, 0.0, 0.0)

        total_weight = sum(v.confidence for v in votes)
        if total_weight == 0:
            total_weight = len(votes)
            weighted_sum = sum(v.probability_yes for v in votes)
        else:
            weighted_sum = sum(v.probability_yes * v.confidence for v in votes)

        prob_yes = max(0.01, min(0.99, weighted_sum / total_weight))

        variance = sum((v.probability_yes - prob_yes) ** 2 for v in votes) / len(votes)
        std_dev = variance ** 0.5

        agreement_confidence = max(0.0, 1.0 - (std_dev / 0.5))
        avg_confidence = sum(v.confidence for v in votes) / len(votes)
        confidence = (agreement_confidence * 0.6) + (avg_confidence * 0.4)

        return PredictionResult(
            probability_yes=round(prob_yes, 4),
            probability_no=round(1.0 - prob_yes, 4),
            confidence=round(confidence, 4),
            num_agents=len(votes),
            bull_count=sum(1 for v in votes if v.probability_yes > 0.5),
            bear_count=sum(1 for v in votes if v.probability_yes < 0.5),
            std_dev=round(std_dev, 4),
            edge_vs_market=round(prob_yes - self.market_price, 4),
        )

    def should_trade(self, result: PredictionResult, min_edge: float = 0.05, min_confidence: float = 0.3) -> tuple[str, float] | None:
        if result.confidence < min_confidence:
            return None
        if result.edge_vs_market > min_edge:
            return ("YES", round(min(1.0, result.edge_vs_market / 0.20), 2))
        elif result.edge_vs_market < -min_edge:
            return ("NO", round(min(1.0, abs(result.edge_vs_market) / 0.20), 2))
        return None
