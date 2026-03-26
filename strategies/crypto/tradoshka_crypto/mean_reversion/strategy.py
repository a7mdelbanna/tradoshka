from __future__ import annotations

from tradoshka_strategy import (
    BaseStrategy,
    MarketEvent,
    Signal,
    SignalDirection,
    BollingerBands,
    RSI,
)


class MeanReversionStrategy(BaseStrategy):
    """Bollinger Band bounce strategy with RSI confirmation for mean reversion entries."""

    def __init__(
        self,
        bb_period: int = 20,
        bb_std: float = 2.0,
        rsi_period: int = 14,
    ):
        super().__init__("mean_reversion", "crypto")
        self.bb = BollingerBands(bb_period, bb_std)
        self.rsi = RSI(rsi_period)

    @property
    def name(self) -> str:
        return "Mean Reversion"

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        bb_result = self.bb.update(event.price)
        rsi_val = self.rsi.update(event.price)
        if bb_result is None:
            return None
        upper, middle, lower = bb_result
        band_width = upper - lower
        # Use strict inequalities so that price exactly at the band edge (e.g.
        # when std dev = 0 and upper == lower == price) does not trigger.
        if event.price < lower and (rsi_val is None or rsi_val < 30):
            strength = (
                min(1.0, (lower - event.price) / band_width * 2)
                if band_width > 0
                else 0.5
            )
            return Signal(
                self.strategy_id,
                event.symbol,
                SignalDirection.LONG,
                max(0.2, strength),
            )
        elif event.price > upper and (rsi_val is None or rsi_val > 70):
            strength = (
                min(1.0, (event.price - upper) / band_width * 2)
                if band_width > 0
                else 0.5
            )
            return Signal(
                self.strategy_id,
                event.symbol,
                SignalDirection.SHORT,
                max(0.2, strength),
            )
        return None

    def on_fill(self, fill) -> None:
        pass
