from tradoshka_crypto.ensemble.combiner import CryptoEnsemble, REGIME_WEIGHTS
from tradoshka_strategy import Signal, SignalDirection


def test_regime_weights_exist():
    for regime in ["trending", "ranging", "volatile", "crash"]:
        assert regime in REGIME_WEIGHTS


def test_trending_favors_momentum():
    w = REGIME_WEIGHTS["trending"]
    assert w["momentum"] > w["grid_trading"]


def test_ranging_favors_grid():
    w = REGIME_WEIGHTS["ranging"]
    assert w["grid_trading"] > w["momentum"]


def test_volatile_favors_arbitrage():
    w = REGIME_WEIGHTS["volatile"]
    assert w["arbitrage"] > w["momentum"]


def test_crash_favors_dca():
    w = REGIME_WEIGHTS["crash"]
    assert w["dca"] > w["momentum"]


def test_combine_signals():
    e = CryptoEnsemble()
    e.set_regime("trending")
    signals = [
        Signal("momentum", "BTCUSDT", SignalDirection.LONG, 0.8),
        Signal("dca", "BTCUSDT", SignalDirection.LONG, 0.5),
    ]
    result = e.combine(signals, "BTCUSDT")
    assert result is not None
    assert result.direction == SignalDirection.LONG


def test_combine_empty():
    assert CryptoEnsemble().combine([], "X") is None


def test_combine_wrong_symbol_ignored():
    e = CryptoEnsemble()
    signals = [
        Signal("momentum", "ETHUSDT", SignalDirection.LONG, 0.8),
    ]
    result = e.combine(signals, "BTCUSDT")
    assert result is None


def test_combine_conflicting_signals_net_long():
    e = CryptoEnsemble()
    e.set_regime("trending")
    signals = [
        Signal("momentum", "BTCUSDT", SignalDirection.LONG, 0.9),
        Signal("mean_reversion", "BTCUSDT", SignalDirection.SHORT, 0.1),
    ]
    result = e.combine(signals, "BTCUSDT")
    assert result is not None
    assert result.direction == SignalDirection.LONG


def test_combine_strength_between_0_and_1():
    e = CryptoEnsemble()
    signals = [Signal("momentum", "BTC", SignalDirection.LONG, 0.7)]
    result = e.combine(signals, "BTC")
    if result:
        assert 0.0 <= result.strength <= 1.0


def test_set_regime_changes_weights():
    e = CryptoEnsemble()
    e.set_regime("volatile")
    w = e.get_weights()
    assert w["arbitrage"] == REGIME_WEIGHTS["volatile"]["arbitrage"]


def test_unknown_regime_falls_back_to_ranging():
    e = CryptoEnsemble()
    e.set_regime("unknown_regime")
    w = e.get_weights()
    assert w == REGIME_WEIGHTS["ranging"]
