import sys
import os
# Add shared strategies to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..', '..', 'shared'))

from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection
from .simulation import PredictionSimulation
from .world_builder import WorldBuilder
from .scorer import PredictionScorer
from .llm_client import LLMConfig


class AIPredictorStrategy(BaseStrategy):
    def __init__(self, llm_config: LLMConfig | None = None, agent_count: int = 20):
        super().__init__("ai_predictor", "polymarket")
        self.sim = PredictionSimulation(llm_config=llm_config, agent_count=agent_count)
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
