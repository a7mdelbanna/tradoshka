from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime
from typing import Optional

from .signal import Signal


@dataclass
class MarketEvent:
    symbol: str
    price: float
    volume: float
    timestamp: datetime = field(default_factory=datetime.utcnow)


@dataclass
class Fill:
    symbol: str
    side: str
    price: float
    quantity: float
    fee: float


class BaseStrategy(ABC):
    def __init__(self, strategy_id: str, market: str) -> None:
        self.strategy_id = strategy_id
        self.market = market
        self._enabled: bool = True

    @property
    @abstractmethod
    def name(self) -> str:
        """Human-readable name for this strategy."""

    @abstractmethod
    def on_market_event(self, event: MarketEvent) -> Optional[Signal]:
        """Process a market event and optionally return a signal."""

    def on_fill(self, fill: Fill) -> None:
        """Called when an order fill is received. Override to handle fills."""

    @property
    def enabled(self) -> bool:
        return self._enabled

    def enable(self) -> None:
        self._enabled = True

    def disable(self) -> None:
        self._enabled = False
