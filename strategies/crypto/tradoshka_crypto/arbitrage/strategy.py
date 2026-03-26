from __future__ import annotations

from dataclasses import dataclass

from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection


@dataclass
class FundingArbSignal:
    """Represents a detected funding rate arbitrage opportunity."""

    symbol: str
    funding_rate: float
    annualized: float
    direction: str  # "long_spot_short_perp" or "short_spot_long_perp"


class ArbitrageStrategy(BaseStrategy):
    """Funding rate arbitrage strategy: captures carry between spot and perp markets."""

    def __init__(self, min_funding_rate: float = 0.0003):
        super().__init__("arbitrage", "crypto")
        self.min_funding_rate = min_funding_rate

    @property
    def name(self) -> str:
        return "Funding Rate Arbitrage"

    def evaluate_funding(
        self, symbol: str, funding_rate: float
    ) -> FundingArbSignal | None:
        """Evaluate a funding rate and return an arb signal if above threshold."""
        if abs(funding_rate) < self.min_funding_rate:
            return None
        annualized = funding_rate * 3 * 365  # 3 funding periods/day * 365 days
        direction = (
            "long_spot_short_perp" if funding_rate > 0 else "short_spot_long_perp"
        )
        return FundingArbSignal(symbol, funding_rate, annualized, direction)

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        return None  # Funding arb needs a separate data feed

    def on_fill(self, fill) -> None:
        pass
