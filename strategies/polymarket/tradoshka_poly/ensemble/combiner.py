from dataclasses import dataclass
from tradoshka_strategy import Signal, SignalDirection


@dataclass
class StrategyWeight:
    strategy_id: str
    weight: float
    recent_accuracy: float = 0.5


class EnsembleCombiner:
    """Combine signals from multiple strategies using weighted voting."""

    def __init__(self, weights: dict[str, float] | None = None):
        self.weights: dict[str, StrategyWeight] = {}
        if weights:
            for sid, w in weights.items():
                self.weights[sid] = StrategyWeight(strategy_id=sid, weight=w)

    def set_weight(self, strategy_id: str, weight: float) -> None:
        if strategy_id in self.weights:
            self.weights[strategy_id].weight = weight
        else:
            self.weights[strategy_id] = StrategyWeight(strategy_id=strategy_id, weight=weight)

    def combine(self, signals: list[Signal], symbol: str) -> Signal | None:
        if not signals:
            return None
        relevant = [s for s in signals if s.symbol == symbol and s.is_actionable]
        if not relevant:
            return None

        long_score = 0.0
        short_score = 0.0
        close_score = 0.0
        total_weight = 0.0

        for signal in relevant:
            w = self.weights.get(signal.strategy_id, StrategyWeight(signal.strategy_id, 1.0)).weight
            ws = signal.strength * w
            if signal.direction == SignalDirection.LONG:
                long_score += ws
            elif signal.direction == SignalDirection.SHORT:
                short_score += ws
            elif signal.direction == SignalDirection.CLOSE:
                close_score += ws
            total_weight += w

        if total_weight == 0:
            return None

        long_score /= total_weight
        short_score /= total_weight
        close_score /= total_weight

        scores = {"long": long_score, "short": short_score, "close": close_score}
        best = max(scores, key=scores.get)

        if best == "long" and long_score > 0.1:
            return Signal("ensemble", symbol, SignalDirection.LONG, round(long_score, 2))
        elif best == "short" and short_score > 0.1:
            return Signal("ensemble", symbol, SignalDirection.SHORT, round(short_score, 2))
        elif best == "close" and close_score > 0.1:
            return Signal("ensemble", symbol, SignalDirection.CLOSE, round(close_score, 2))
        return None

    def update_accuracy(self, strategy_id: str, was_correct: bool) -> None:
        if strategy_id in self.weights:
            sw = self.weights[strategy_id]
            alpha = 0.1
            sw.recent_accuracy = alpha * (1.0 if was_correct else 0.0) + (1 - alpha) * sw.recent_accuracy
