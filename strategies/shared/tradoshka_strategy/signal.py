from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Any, Optional


class SignalDirection(Enum):
    LONG = "LONG"
    SHORT = "SHORT"
    CLOSE = "CLOSE"
    HOLD = "HOLD"


@dataclass
class Signal:
    strategy_id: str
    symbol: str
    direction: SignalDirection
    strength: float
    stop_loss: Optional[float] = None
    take_profit: Optional[float] = None
    timestamp: datetime = field(default_factory=datetime.utcnow)
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        # Clamp strength to [0.0, 1.0]
        self.strength = max(0.0, min(1.0, self.strength))

    @property
    def is_actionable(self) -> bool:
        """True for LONG, SHORT, and CLOSE directions."""
        return self.direction in (
            SignalDirection.LONG,
            SignalDirection.SHORT,
            SignalDirection.CLOSE,
        )
