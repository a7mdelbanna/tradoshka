from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection
from .mispricing import MispricingDetector

class ArbitrageStrategy(BaseStrategy):
    def __init__(self, min_deviation: float = 0.02):
        super().__init__("arbitrage", "polymarket")
        self.detector = MispricingDetector(min_deviation=min_deviation)

    @property
    def name(self) -> str:
        return "Polymarket Arbitrage"

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        return None

    def on_fill(self, fill) -> None:
        pass
