from __future__ import annotations

from tradoshka_strategy import Signal, SignalDirection

# Regime-adaptive weight tables for each constituent strategy.
# Weights within a regime sum to 1.0.
REGIME_WEIGHTS: dict[str, dict[str, float]] = {
    "trending": {
        "momentum": 0.35,
        "dca": 0.25,
        "grid_trading": 0.05,
        "mean_reversion": 0.05,
        "arbitrage": 0.10,
        "market_making": 0.05,
        "ai_sentiment": 0.15,
    },
    "ranging": {
        "momentum": 0.05,
        "dca": 0.10,
        "grid_trading": 0.30,
        "mean_reversion": 0.25,
        "arbitrage": 0.10,
        "market_making": 0.15,
        "ai_sentiment": 0.05,
    },
    "volatile": {
        "momentum": 0.10,
        "dca": 0.05,
        "grid_trading": 0.05,
        "mean_reversion": 0.10,
        "arbitrage": 0.40,
        "market_making": 0.05,
        "ai_sentiment": 0.25,
    },
    "crash": {
        "momentum": 0.05,
        "dca": 0.30,
        "grid_trading": 0.05,
        "mean_reversion": 0.15,
        "arbitrage": 0.15,
        "market_making": 0.00,
        "ai_sentiment": 0.30,
    },
}


class CryptoEnsemble:
    """Regime-adaptive weighted fusion of all crypto strategy signals."""

    def __init__(self) -> None:
        self.regime: str = "ranging"

    def set_regime(self, regime: str) -> None:
        """Update the active market regime."""
        self.regime = regime

    def get_weights(self) -> dict[str, float]:
        """Return weight table for the current regime."""
        return REGIME_WEIGHTS.get(self.regime, REGIME_WEIGHTS["ranging"])

    def combine(self, signals: list[Signal], symbol: str) -> Signal | None:
        """Fuse actionable signals for *symbol* into a single combined signal.

        Returns None when there is no actionable consensus.
        """
        relevant = [s for s in signals if s.symbol == symbol and s.is_actionable]
        if not relevant:
            return None

        weights = self.get_weights()
        long_score = 0.0
        short_score = 0.0
        total_w = 0.0

        for sig in relevant:
            w = weights.get(sig.strategy_id, 0.1)
            if sig.direction == SignalDirection.LONG:
                long_score += sig.strength * w
            elif sig.direction == SignalDirection.SHORT:
                short_score += sig.strength * w
            total_w += w

        if total_w == 0:
            return None

        long_score /= total_w
        short_score /= total_w

        if long_score > short_score and long_score > 0.1:
            return Signal(
                "crypto_ensemble",
                symbol,
                SignalDirection.LONG,
                round(long_score, 2),
            )
        elif short_score > long_score and short_score > 0.1:
            return Signal(
                "crypto_ensemble",
                symbol,
                SignalDirection.SHORT,
                round(short_score, 2),
            )
        return None
