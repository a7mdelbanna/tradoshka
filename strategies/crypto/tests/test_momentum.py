from tradoshka_crypto.momentum.strategy import MomentumStrategy
from tradoshka_strategy import MarketEvent


def test_momentum_uptrend():
    s = MomentumStrategy(fast_period=3, slow_period=5)
    prices = [100, 101, 102, 103, 105, 108, 112, 115, 120, 125]
    result = None
    for i, p in enumerate(prices):
        result = s.on_market_event(MarketEvent("BTC", p, 1000, i))
    assert result is not None
    assert result.direction.value == "LONG"


def test_momentum_downtrend():
    s = MomentumStrategy(fast_period=3, slow_period=5)
    prices = [100, 98, 95, 92, 88, 85, 80, 75, 70, 65]
    result = None
    for i, p in enumerate(prices):
        result = s.on_market_event(MarketEvent("BTC", p, 1000, i))
    assert result is not None
    assert result.direction.value == "SHORT"


def test_momentum_no_signal_before_ready():
    s = MomentumStrategy(fast_period=5, slow_period=10)
    # Not enough data for slow EMA
    for i in range(4):
        result = s.on_market_event(MarketEvent("BTC", 100 + i, 1000, i))
    assert result is None


def test_momentum_strength_positive():
    s = MomentumStrategy(fast_period=3, slow_period=5)
    prices = [100, 101, 102, 103, 105, 108, 112, 115, 120, 125]
    result = None
    for i, p in enumerate(prices):
        result = s.on_market_event(MarketEvent("BTC", p, 1000, i))
    if result is not None:
        assert result.strength > 0


def test_momentum_disabled():
    s = MomentumStrategy(fast_period=3, slow_period=5)
    s.disable()
    prices = [100, 101, 102, 103, 105]
    for i, p in enumerate(prices):
        result = s.on_market_event(MarketEvent("BTC", p, 1000, i))
    assert result is None


def test_momentum_name():
    s = MomentumStrategy()
    assert s.name == "Momentum / Trend Following"
