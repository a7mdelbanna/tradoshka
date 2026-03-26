from __future__ import annotations

import math
from collections import deque
from typing import Optional


class SMA:
    """Simple Moving Average (stateful, update-based)."""

    def __init__(self, period: int) -> None:
        self._period = period
        self._values: deque[float] = deque(maxlen=period)

    def update(self, value: float) -> Optional[float]:
        self._values.append(value)
        if self.ready:
            return self.value
        return None

    @property
    def ready(self) -> bool:
        return len(self._values) == self._period

    @property
    def value(self) -> Optional[float]:
        if not self.ready:
            return None
        return sum(self._values) / self._period


class EMA:
    """Exponential Moving Average (stateful, update-based).

    Uses multiplier = 2 / (period + 1).
    """

    def __init__(self, period: int) -> None:
        self._period = period
        self._multiplier = 2.0 / (period + 1)
        self._value: Optional[float] = None
        self._count: int = 0

    def update(self, value: float) -> Optional[float]:
        self._count += 1
        if self._value is None:
            # Seed with first value
            self._value = value
        else:
            self._value = (value - self._value) * self._multiplier + self._value
        if self.ready:
            return self._value
        return None

    @property
    def ready(self) -> bool:
        return self._count >= self._period

    @property
    def value(self) -> Optional[float]:
        if not self.ready:
            return None
        return self._value


class RSI:
    """Relative Strength Index (stateful, update-based).

    Returns values in the range [0, 100].
    """

    def __init__(self, period: int = 14) -> None:
        self._period = period
        self._prev_value: Optional[float] = None
        self._gains: deque[float] = deque(maxlen=period)
        self._losses: deque[float] = deque(maxlen=period)
        self._count: int = 0

    def update(self, value: float) -> Optional[float]:
        if self._prev_value is not None:
            change = value - self._prev_value
            gain = max(change, 0.0)
            loss = max(-change, 0.0)
            self._gains.append(gain)
            self._losses.append(loss)
            self._count += 1

        self._prev_value = value

        if not self.ready:
            return None

        avg_gain = sum(self._gains) / self._period
        avg_loss = sum(self._losses) / self._period

        if avg_loss == 0.0:
            return 100.0
        if avg_gain == 0.0:
            return 0.0

        rs = avg_gain / avg_loss
        return 100.0 - (100.0 / (1.0 + rs))

    @property
    def ready(self) -> bool:
        return self._count >= self._period


class ATR:
    """Average True Range (stateful, update-based).

    True range = max(high-low, |high-prev_close|, |low-prev_close|).
    """

    def __init__(self, period: int = 14) -> None:
        self._period = period
        self._prev_close: Optional[float] = None
        self._true_ranges: deque[float] = deque(maxlen=period)

    def update(self, high: float, low: float, close: float) -> Optional[float]:
        if self._prev_close is None:
            tr = high - low
        else:
            tr = max(
                high - low,
                abs(high - self._prev_close),
                abs(low - self._prev_close),
            )

        self._true_ranges.append(tr)
        self._prev_close = close

        if self.ready:
            return self.value
        return None

    @property
    def ready(self) -> bool:
        return len(self._true_ranges) == self._period

    @property
    def value(self) -> Optional[float]:
        if not self.ready:
            return None
        return sum(self._true_ranges) / self._period


class BollingerBands:
    """Bollinger Bands (stateful, update-based).

    Returns (upper, middle, lower) tuple when ready.
    """

    def __init__(self, period: int = 20, std_dev_mult: float = 2.0) -> None:
        self._period = period
        self._std_dev_mult = std_dev_mult
        self._values: deque[float] = deque(maxlen=period)

    def update(self, value: float) -> Optional[tuple[float, float, float]]:
        self._values.append(value)
        if not self.ready:
            return None

        middle = sum(self._values) / self._period
        variance = sum((v - middle) ** 2 for v in self._values) / self._period
        std_dev = math.sqrt(variance)

        upper = middle + self._std_dev_mult * std_dev
        lower = middle - self._std_dev_mult * std_dev
        return (upper, middle, lower)

    @property
    def ready(self) -> bool:
        return len(self._values) == self._period
