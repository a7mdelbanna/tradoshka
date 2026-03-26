from tradoshka_crypto.market_making.strategy import CryptoMarketMakingStrategy
from tradoshka_strategy import MarketEvent, Fill


def test_mm_quotes_symmetric():
    s = CryptoMarketMakingStrategy(spread_bps=20)
    bid, ask = s.calculate_quotes(100.0)
    assert bid < 100.0 < ask
    spread = ask - bid
    assert abs(spread - 0.20) < 0.01  # ~20 bps


def test_mm_inventory_skew():
    s = CryptoMarketMakingStrategy(spread_bps=20, max_inventory=100)
    s.inventory = 50  # Long bias
    bid1, ask1 = s.calculate_quotes(100.0)
    s.inventory = 0
    bid2, ask2 = s.calculate_quotes(100.0)
    assert bid1 < bid2  # Long inventory → lower bid (skew down)


def test_mm_signal_when_inventory_ok():
    s = CryptoMarketMakingStrategy(spread_bps=20, max_inventory=1000)
    result = s.on_market_event(MarketEvent("BTC", 100.0, 1000, 0))
    assert result is not None


def test_mm_no_signal_at_max_inventory():
    s = CryptoMarketMakingStrategy(spread_bps=20, max_inventory=100)
    s.inventory = 100.0  # At max
    result = s.on_market_event(MarketEvent("BTC", 100.0, 1000, 0))
    assert result is None


def test_mm_fill_updates_inventory():
    s = CryptoMarketMakingStrategy(spread_bps=20, max_inventory=1000)
    fill = Fill("BTC", "BUY", 100.0, 10.0, 0.01)
    s.on_fill(fill)
    assert s.inventory == 10.0


def test_mm_sell_fill_reduces_inventory():
    s = CryptoMarketMakingStrategy(spread_bps=20, max_inventory=1000)
    s.inventory = 20.0
    fill = Fill("BTC", "SELL", 100.0, 5.0, 0.01)
    s.on_fill(fill)
    assert s.inventory == 15.0


def test_mm_disabled():
    s = CryptoMarketMakingStrategy()
    s.disable()
    result = s.on_market_event(MarketEvent("BTC", 100.0, 1000, 0))
    assert result is None
