from tradoshka_crypto.dca.strategy import DCAStrategy
from tradoshka_strategy import MarketEvent


def test_dca_buys_on_interval():
    s = DCAStrategy(buy_interval=3)
    for i in range(3):
        result = s.on_market_event(MarketEvent("BTC", 100, 1000, i))
    assert result is not None
    assert result.direction.value == "LONG"


def test_dca_no_signal_between_intervals():
    s = DCAStrategy(buy_interval=5)
    result = s.on_market_event(MarketEvent("BTC", 100, 1000, 0))
    assert result is None


def test_dca_signal_strength_default():
    s = DCAStrategy(buy_interval=1)
    result = s.on_market_event(MarketEvent("BTC", 100, 1000, 0))
    assert result is not None
    assert result.strength == 0.3


def test_dca_oversold_increases_strength():
    s = DCAStrategy(buy_interval=1, rsi_oversold=30.0, rsi_period=2)
    # Feed enough prices to get RSI ready then a sharp drop
    prices = [100.0] * 3 + [60.0]
    result = None
    for i, p in enumerate(prices):
        result = s.on_market_event(MarketEvent("BTC", p, 1000, i))
    assert result is not None
    assert result.strength > 0.3


def test_dca_disabled():
    s = DCAStrategy(buy_interval=1)
    s.disable()
    result = s.on_market_event(MarketEvent("BTC", 100, 1000, 0))
    assert result is None


def test_dca_strength_capped_at_1():
    s = DCAStrategy(buy_interval=1, rsi_oversold=90.0, rsi_period=2)
    # Very oversold scenario — strength must not exceed 1.0
    prices = [100.0, 10.0]
    result = None
    for i, p in enumerate(prices):
        result = s.on_market_event(MarketEvent("BTC", p, 1000, i))
    if result is not None:
        assert result.strength <= 1.0
