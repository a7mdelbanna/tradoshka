from __future__ import annotations

from tradoshka_strategy import BaseStrategy, MarketEvent, Signal, SignalDirection


class CryptoCopyTradingStrategy(BaseStrategy):
    """Mirror Binance lead traders — on-chain position following strategy."""

    def __init__(self) -> None:
        super().__init__("copy_trading", "crypto")
        self.tracked_traders: list[str] = []

    @property
    def name(self) -> str:
        return "Crypto Copy Trading"

    def add_trader(self, address: str) -> None:
        """Register a trader address to follow."""
        if address not in self.tracked_traders:
            self.tracked_traders.append(address)

    def remove_trader(self, address: str) -> None:
        """Unregister a trader address."""
        self.tracked_traders = [t for t in self.tracked_traders if t != address]

    def on_market_event(self, event: MarketEvent) -> Signal | None:
        if not self.enabled:
            return None
        return None  # Needs separate data feed (Binance lead traders API)

    def on_fill(self, fill) -> None:
        pass
