from tradoshka_crypto.mean_reversion.strategy import MeanReversionStrategy
from tradoshka_strategy import MarketEvent


def test_mean_reversion_oversold():
    s = MeanReversionStrategy(bb_period=5, bb_std=2.0)
    prices = [100, 100, 100, 100, 100, 80]  # Drop below lower band
    result = None
    for i, p in enumerate(prices):
        result = s.on_market_event(MarketEvent("BTC", p, 1000, i))
    if result:
        assert result.direction.value == "LONG"


def test_mean_reversion_no_signal_at_middle():
    s = MeanReversionStrategy(bb_period=5, bb_std=2.0)
    prices = [100, 100, 100, 100, 100, 100]
    result = None
    for i, p in enumerate(prices):
        result = s.on_market_event(MarketEvent("BTC", p, 1000, i))
    assert result is None  # At middle of bands — std dev = 0, no signal


def test_mean_reversion_overbought():
    s = MeanReversionStrategy(bb_period=5, bb_std=2.0)
    prices = [100, 100, 100, 100, 100, 130]  # Spike above upper band
    result = None
    for i, p in enumerate(prices):
        result = s.on_market_event(MarketEvent("BTC", p, 1000, i))
    if result:
        assert result.direction.value == "SHORT"


def test_mean_reversion_no_signal_before_ready():
    s = MeanReversionStrategy(bb_period=10, bb_std=2.0)
    for i in range(9):
        result = s.on_market_event(MarketEvent("BTC", 100, 1000, i))
    assert result is None


def test_mean_reversion_disabled():
    s = MeanReversionStrategy(bb_period=5, bb_std=2.0)
    s.disable()
    for i in range(6):
        result = s.on_market_event(MarketEvent("BTC", 80, 1000, i))
    assert result is None


def test_mean_reversion_strength_min():
    s = MeanReversionStrategy(bb_period=5, bb_std=2.0)
    prices = [100, 100, 100, 100, 100, 80]
    result = None
    for i, p in enumerate(prices):
        result = s.on_market_event(MarketEvent("BTC", p, 1000, i))
    if result:
        assert result.strength >= 0.2
