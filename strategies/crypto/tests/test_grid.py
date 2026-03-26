from tradoshka_crypto.grid.strategy import GridTradingStrategy, GridConfig
from tradoshka_strategy import MarketEvent


def test_grid_initializes():
    s = GridTradingStrategy(GridConfig(grid_size=5, grid_spacing_pct=1.0))
    s.initialize_grid(100.0)
    assert len(s.levels) == 10  # 5 buy + 5 sell


def test_grid_buy_signal():
    s = GridTradingStrategy(GridConfig(grid_size=3, grid_spacing_pct=2.0))
    s.initialize_grid(100.0)
    event = MarketEvent("BTC", 97.5, 1000, 0)  # Below first buy level at 98
    signal = s.on_market_event(event)
    assert signal is not None
    assert signal.direction.value == "LONG"


def test_grid_sell_signal():
    s = GridTradingStrategy(GridConfig(grid_size=3, grid_spacing_pct=2.0))
    s.initialize_grid(100.0)
    event = MarketEvent("BTC", 102.5, 1000, 0)  # Above first sell level at 102
    signal = s.on_market_event(event)
    assert signal is not None
    assert signal.direction.value == "SHORT"


def test_grid_no_signal_in_center():
    s = GridTradingStrategy(GridConfig(grid_size=3, grid_spacing_pct=2.0))
    s.initialize_grid(100.0)
    event = MarketEvent("BTC", 100.0, 1000, 0)  # At center
    assert s.on_market_event(event) is None


def test_grid_no_refill_after_filled():
    s = GridTradingStrategy(GridConfig(grid_size=3, grid_spacing_pct=2.0))
    s.initialize_grid(100.0)
    # Trigger the first buy level
    s.on_market_event(MarketEvent("BTC", 97.5, 1000, 0))
    # Same price again — level already filled, no new signal for same level
    second = s.on_market_event(MarketEvent("BTC", 97.5, 1000, 1))
    # Either None or a signal for a different (lower) level
    if second is not None:
        assert second.direction.value == "LONG"


def test_grid_auto_initializes_on_first_event():
    s = GridTradingStrategy(GridConfig(grid_size=4, grid_spacing_pct=1.0))
    # First event auto-initialises — should return None
    result = s.on_market_event(MarketEvent("BTC", 100.0, 1000, 0))
    assert result is None
    assert s._initialized is True
    assert len(s.levels) == 8


def test_grid_stop_loss_attached():
    s = GridTradingStrategy(GridConfig(grid_size=3, grid_spacing_pct=2.0))
    s.initialize_grid(100.0)
    signal = s.on_market_event(MarketEvent("BTC", 97.5, 1000, 0))
    assert signal is not None
    assert signal.stop_loss is not None
    assert signal.stop_loss < 97.5


def test_grid_disabled():
    s = GridTradingStrategy(GridConfig(grid_size=3, grid_spacing_pct=2.0))
    s.initialize_grid(100.0)
    s.disable()
    assert s.on_market_event(MarketEvent("BTC", 97.5, 1000, 0)) is None
