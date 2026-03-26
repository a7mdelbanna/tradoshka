from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection
from .spread import SpreadCalculator, QuoteParams
from .inventory import InventoryManager

class MarketMakingStrategy(BaseStrategy):
    def __init__(self, base_spread_bps: int = 200, max_inventory: float = 1000.0):
        super().__init__("market_making", "polymarket")
        self.spread_calc = SpreadCalculator()
        self.inventory = InventoryManager(max_inventory)
        self.base_spread_bps = base_spread_bps

    @property
    def name(self) -> str:
        return "Polymarket Market Making"

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        params = QuoteParams(fair_value=event.price, base_spread_bps=self.base_spread_bps,
                            inventory_skew=self.inventory.state.inventory_skew)
        bid, ask = self.spread_calc.calculate_quotes(params)
        if self.inventory.can_buy_yes(10):
            return Signal(self.strategy_id, event.symbol, SignalDirection.LONG, 0.3,
                         stop_loss=bid, take_profit=ask, metadata={"bid": str(bid), "ask": str(ask)})
        return None

    def on_fill(self, fill) -> None:
        if fill.side == "BUY":
            self.inventory.record_buy_yes(fill.quantity, fill.price)
        else:
            self.inventory.record_sell_yes(fill.quantity, fill.price)
