from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection
from .wallet_tracker import WalletTracker
from .signal_extractor import SignalExtractor, CopySignalConfig

class CopyTradingStrategy(BaseStrategy):
    def __init__(self, config: CopySignalConfig | None = None):
        super().__init__("copy_trading", "polymarket")
        self.tracker = WalletTracker()
        self.extractor = SignalExtractor(config)

    @property
    def name(self) -> str:
        return "Polymarket Copy Trading"

    def add_wallet(self, address: str) -> None:
        self.tracker.add_wallet(address)

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        return None

    def on_fill(self, fill) -> None:
        pass
