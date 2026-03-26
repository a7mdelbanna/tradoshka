from tradoshka_poly.ensemble.combiner import EnsembleCombiner
from tradoshka_strategy import Signal, SignalDirection

def test_combine_unanimous_long():
    combiner = EnsembleCombiner({"ai": 0.35, "copy": 0.25, "arb": 0.20})
    signals = [
        Signal("ai", "BTC_YES", SignalDirection.LONG, 0.8),
        Signal("copy", "BTC_YES", SignalDirection.LONG, 0.6),
        Signal("arb", "BTC_YES", SignalDirection.LONG, 0.4),
    ]
    result = combiner.combine(signals, "BTC_YES")
    assert result is not None
    assert result.direction == SignalDirection.LONG
    assert result.strength > 0.3

def test_combine_conflicting():
    combiner = EnsembleCombiner({"ai": 0.5, "copy": 0.5})
    signals = [
        Signal("ai", "ETH", SignalDirection.LONG, 0.8),
        Signal("copy", "ETH", SignalDirection.SHORT, 0.8),
    ]
    result = combiner.combine(signals, "ETH")
    # Both cancel — weak signal either direction
    if result:
        assert result.strength < 0.5

def test_combine_empty():
    assert EnsembleCombiner().combine([], "X") is None

def test_combine_ignores_other_symbols():
    combiner = EnsembleCombiner({"ai": 1.0})
    signals = [Signal("ai", "OTHER", SignalDirection.LONG, 0.9)]
    assert combiner.combine(signals, "BTC") is None

def test_weighted_favors_higher():
    combiner = EnsembleCombiner({"strong": 0.9, "weak": 0.1})
    signals = [
        Signal("strong", "X", SignalDirection.LONG, 0.8),
        Signal("weak", "X", SignalDirection.SHORT, 0.8),
    ]
    result = combiner.combine(signals, "X")
    assert result is not None
    assert result.direction == SignalDirection.LONG
