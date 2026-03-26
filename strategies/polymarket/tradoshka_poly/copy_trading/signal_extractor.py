from dataclasses import dataclass
from .wallet_tracker import WhaleTrade
from tradoshka_strategy import Signal, SignalDirection

@dataclass
class CopySignalConfig:
    max_price_deviation: float = 0.05
    min_whale_size: float = 100.0
    position_scale: float = 0.05
    min_position: float = 10.0
    max_position: float = 500.0

class SignalExtractor:
    def __init__(self, config: CopySignalConfig | None = None):
        self.config = config or CopySignalConfig()

    def extract(self, trade: WhaleTrade, current_price: float, strategy_id: str = "copy_trading") -> Signal | None:
        if trade.size < self.config.min_whale_size:
            return None
        if abs(current_price - trade.price) > self.config.max_price_deviation:
            return None
        direction = SignalDirection.LONG if trade.side == "BUY" else SignalDirection.SHORT
        strength = min(1.0, trade.size / 1000.0)
        return Signal(strategy_id=strategy_id, symbol=trade.token_id, direction=direction,
                      strength=round(strength, 2), metadata={"whale": trade.address, "size": str(trade.size)})
