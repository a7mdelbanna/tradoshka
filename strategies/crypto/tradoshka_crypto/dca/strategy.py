from __future__ import annotations

from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection, RSI


class DCAStrategy(BaseStrategy):
    """Smart DCA strategy with RSI-weighted buy sizing and reverse DCA on strength."""

    def __init__(
        self,
        buy_interval: int = 12,
        rsi_oversold: float = 30.0,
        rsi_period: int = 14,
    ):
        super().__init__("dca", "crypto")
        self.buy_interval = buy_interval
        self.rsi = RSI(rsi_period)
        self.rsi_oversold = rsi_oversold
        self.ticks = 0

    @property
    def name(self) -> str:
        return "Smart DCA"

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        self.ticks += 1
        rsi_val = self.rsi.update(event.price)
        # Buy harder when oversold
        if self.ticks % self.buy_interval == 0:
            strength = 0.3
            if rsi_val is not None and rsi_val < self.rsi_oversold:
                strength = min(1.0, 0.3 + (self.rsi_oversold - rsi_val) / 100)
            return Signal(
                self.strategy_id,
                event.symbol,
                SignalDirection.LONG,
                strength,
            )
        return None

    def on_fill(self, fill) -> None:
        pass
