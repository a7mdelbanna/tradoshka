from __future__ import annotations

import math

from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection


class RegimeType:
    """Market regime labels used by the ensemble combiner."""

    TRENDING = "trending"
    RANGING = "ranging"
    VOLATILE = "volatile"
    CRASH = "crash"


class AIsentimentStrategy(BaseStrategy):
    """Volatility-based regime detector.

    Classifies the market into one of four regimes (trending / ranging /
    volatile / crash) and exposes the result for the ensemble combiner.
    """

    def __init__(self, volatility_window: int = 20) -> None:
        super().__init__("ai_sentiment", "crypto")
        self.price_history: list[float] = []
        self.volatility_window = volatility_window
        self.current_regime: str = RegimeType.RANGING

    @property
    def name(self) -> str:
        return "AI / Sentiment"

    def detect_regime(self, prices: list[float]) -> str:
        """Classify market regime from recent price history."""
        if len(prices) < self.volatility_window:
            return RegimeType.RANGING
        recent = prices[-self.volatility_window :]
        returns = [
            (recent[i] - recent[i - 1]) / recent[i - 1]
            for i in range(1, len(recent))
        ]
        volatility = (
            math.sqrt(sum(r ** 2 for r in returns) / len(returns)) if returns else 0
        )
        avg_return = sum(returns) / len(returns) if returns else 0
        if volatility > 0.05:
            return RegimeType.CRASH if avg_return < -0.02 else RegimeType.VOLATILE
        if abs(avg_return) > 0.005:
            return RegimeType.TRENDING
        return RegimeType.RANGING

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        self.price_history.append(event.price)
        self.current_regime = self.detect_regime(self.price_history)
        return None  # Regime info consumed by ensemble, not surfaced as direct signals

    def on_fill(self, fill) -> None:
        pass
