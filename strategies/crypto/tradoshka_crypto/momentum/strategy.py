from __future__ import annotations

from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection, EMA, RSI


class MomentumStrategy(BaseStrategy):
    """EMA crossover + RSI confirmation momentum / breakout trend-following strategy."""

    def __init__(
        self,
        fast_period: int = 9,
        slow_period: int = 21,
        rsi_period: int = 14,
    ):
        super().__init__("momentum", "crypto")
        self.fast_ema = EMA(fast_period)
        self.slow_ema = EMA(slow_period)
        self.rsi = RSI(rsi_period)

    @property
    def name(self) -> str:
        return "Momentum / Trend Following"

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        fast = self.fast_ema.update(event.price)
        slow = self.slow_ema.update(event.price)
        rsi_val = self.rsi.update(event.price)
        if fast is None or slow is None:
            return None
        # EMA crossover + RSI confirmation
        if fast > slow and (rsi_val is None or rsi_val > 50):
            strength = min(1.0, (fast - slow) / slow * 20)
            return Signal(
                self.strategy_id,
                event.symbol,
                SignalDirection.LONG,
                max(0.1, strength),
            )
        elif fast < slow and (rsi_val is None or rsi_val < 50):
            strength = min(1.0, (slow - fast) / slow * 20)
            return Signal(
                self.strategy_id,
                event.symbol,
                SignalDirection.SHORT,
                max(0.1, strength),
            )
        return None

    def on_fill(self, fill) -> None:
        pass
