from __future__ import annotations

from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection


class CryptoMarketMakingStrategy(BaseStrategy):
    """Inventory-aware spread quoting strategy for crypto markets."""

    def __init__(self, spread_bps: int = 20, max_inventory: float = 1000.0):
        super().__init__("market_making", "crypto")
        self.spread_bps = spread_bps
        self.max_inventory = max_inventory
        self.inventory = 0.0

    @property
    def name(self) -> str:
        return "Crypto Market Making"

    def calculate_quotes(self, mid_price: float) -> tuple[float, float]:
        """Return (bid, ask) skewed by current inventory."""
        half_spread = mid_price * (self.spread_bps / 10000 / 2)
        skew = (
            (self.inventory / self.max_inventory) * half_spread * 0.5
            if self.max_inventory
            else 0
        )
        bid = mid_price - half_spread - skew
        ask = mid_price + half_spread - skew
        return round(bid, 8), round(ask, 8)

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        bid, ask = self.calculate_quotes(event.price)
        if abs(self.inventory) < self.max_inventory:
            return Signal(
                self.strategy_id,
                event.symbol,
                SignalDirection.LONG,
                0.3,
                stop_loss=bid,
                take_profit=ask,
            )
        return None

    def on_fill(self, fill) -> None:
        if fill.side == "BUY":
            self.inventory += fill.quantity
        else:
            self.inventory -= fill.quantity
