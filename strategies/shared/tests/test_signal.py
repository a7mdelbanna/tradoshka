from tradoshka_strategy import Signal, SignalDirection


def test_signal_clamps_strength():
    s_high = Signal(
        strategy_id="test",
        symbol="BTC/USDT",
        direction=SignalDirection.LONG,
        strength=1.5,
    )
    assert s_high.strength == 1.0

    s_low = Signal(
        strategy_id="test",
        symbol="BTC/USDT",
        direction=SignalDirection.SHORT,
        strength=-0.5,
    )
    assert s_low.strength == 0.0


def test_signal_is_actionable():
    def make(direction: SignalDirection) -> Signal:
        return Signal(
            strategy_id="test",
            symbol="BTC/USDT",
            direction=direction,
            strength=0.5,
        )

    assert make(SignalDirection.LONG).is_actionable is True
    assert make(SignalDirection.SHORT).is_actionable is True
    assert make(SignalDirection.CLOSE).is_actionable is True
    assert make(SignalDirection.HOLD).is_actionable is False
