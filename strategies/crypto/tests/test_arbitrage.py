from tradoshka_crypto.arbitrage.strategy import ArbitrageStrategy, FundingArbSignal
from tradoshka_strategy import MarketEvent


def test_arb_detects_high_funding():
    s = ArbitrageStrategy(min_funding_rate=0.0003)
    result = s.evaluate_funding("BTCUSDT", 0.001)
    assert result is not None
    assert result.direction == "long_spot_short_perp"


def test_arb_ignores_low_funding():
    s = ArbitrageStrategy(min_funding_rate=0.0003)
    assert s.evaluate_funding("BTCUSDT", 0.0001) is None


def test_arb_negative_funding():
    s = ArbitrageStrategy(min_funding_rate=0.0003)
    result = s.evaluate_funding("ETHUSDT", -0.0005)
    assert result is not None
    assert result.direction == "short_spot_long_perp"


def test_arb_annualized_calculation():
    s = ArbitrageStrategy(min_funding_rate=0.0001)
    result = s.evaluate_funding("BTCUSDT", 0.001)
    assert result is not None
    # 0.001 * 3 * 365 = 1.095
    assert abs(result.annualized - 1.095) < 0.001


def test_arb_below_threshold_ignored():
    s = ArbitrageStrategy(min_funding_rate=0.0003)
    # Strictly below threshold → ignored
    assert s.evaluate_funding("BTCUSDT", 0.00029) is None


def test_arb_at_threshold_triggers():
    s = ArbitrageStrategy(min_funding_rate=0.0003)
    # abs(rate) == min → NOT less-than → signal returned
    result = s.evaluate_funding("BTCUSDT", 0.0003)
    assert result is not None


def test_arb_on_market_event_returns_none():
    s = ArbitrageStrategy()
    result = s.on_market_event(MarketEvent("BTCUSDT", 50000, 1000, 0))
    assert result is None


def test_arb_dataclass_fields():
    sig = FundingArbSignal("BTCUSDT", 0.001, 1.095, "long_spot_short_perp")
    assert sig.symbol == "BTCUSDT"
    assert sig.funding_rate == 0.001
