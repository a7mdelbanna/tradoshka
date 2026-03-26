from dataclasses import dataclass
from datetime import datetime, timezone

@dataclass
class WhaleTrade:
    address: str
    market_id: str
    token_id: str
    side: str
    outcome: str
    size: float
    price: float
    timestamp: datetime
    title: str = ""

class WalletTracker:
    def __init__(self):
        self._tracked: set[str] = set()
        self._history: dict[str, list[WhaleTrade]] = {}

    def add_wallet(self, address: str) -> None:
        self._tracked.add(address.lower())
        self._history.setdefault(address.lower(), [])

    def remove_wallet(self, address: str) -> None:
        self._tracked.discard(address.lower())

    @property
    def tracked_wallets(self) -> set[str]:
        return self._tracked.copy()

    def record_trade(self, trade: WhaleTrade) -> None:
        addr = trade.address.lower()
        if addr in self._tracked:
            self._history.setdefault(addr, []).append(trade)

    def get_recent_trades(self, address: str, limit: int = 10) -> list[WhaleTrade]:
        trades = self._history.get(address.lower(), [])
        return sorted(trades, key=lambda t: t.timestamp, reverse=True)[:limit]

    def get_new_trades_since(self, address: str, since: datetime) -> list[WhaleTrade]:
        return [t for t in self._history.get(address.lower(), []) if t.timestamp > since]
