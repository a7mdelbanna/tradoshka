from __future__ import annotations

from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection


class GridConfig:
    def __init__(
        self,
        center_price: float = 0.0,
        grid_size: int = 10,
        grid_spacing_pct: float = 0.5,
        order_size: float = 10.0,
    ):
        self.center_price = center_price
        self.grid_size = grid_size
        self.grid_spacing_pct = grid_spacing_pct
        self.order_size = order_size


class GridLevel:
    def __init__(self, price: float, side: str, filled: bool = False):
        self.price = price
        self.side = side
        self.filled = filled


class GridTradingStrategy(BaseStrategy):
    """Dynamic geometric grid trading strategy."""

    def __init__(self, config: GridConfig | None = None):
        super().__init__("grid_trading", "crypto")
        self.config = config or GridConfig()
        self.levels: list[GridLevel] = []
        self._initialized = False

    @property
    def name(self) -> str:
        return "Grid Trading"

    def initialize_grid(self, center: float) -> None:
        """Build grid levels symmetrically around the center price."""
        self.config.center_price = center
        self.levels = []
        spacing = center * (self.config.grid_spacing_pct / 100)
        for i in range(self.config.grid_size):
            buy_price = center - spacing * (i + 1)
            sell_price = center + spacing * (i + 1)
            self.levels.append(GridLevel(buy_price, "buy"))
            self.levels.append(GridLevel(sell_price, "sell"))
        self._initialized = True

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        if not self._initialized:
            self.initialize_grid(event.price)
            return None
        # Check if price crossed any unfilled grid level
        for level in self.levels:
            if level.filled:
                continue
            if level.side == "buy" and event.price <= level.price:
                level.filled = True
                return Signal(
                    self.strategy_id,
                    event.symbol,
                    SignalDirection.LONG,
                    0.5,
                    stop_loss=level.price * 0.95,
                )
            elif level.side == "sell" and event.price >= level.price:
                level.filled = True
                return Signal(
                    self.strategy_id,
                    event.symbol,
                    SignalDirection.SHORT,
                    0.5,
                    take_profit=level.price * 1.05,
                )
        return None

    def on_fill(self, fill) -> None:
        pass
