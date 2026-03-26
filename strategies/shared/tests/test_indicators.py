from tradoshka_strategy import SMA, EMA, RSI, ATR, BollingerBands


def test_sma():
    sma = SMA(period=3)
    result = None
    for v in [10, 20, 30]:
        result = sma.update(v)
    assert result == 20.0


def test_sma_rolling():
    sma = SMA(period=3)
    result = None
    for v in [10, 20, 30, 40]:
        result = sma.update(v)
    assert result == 30.0


def test_ema():
    ema = EMA(period=3)
    result = None
    for v in [10, 20, 30]:
        result = ema.update(v)
    # After 3 updates, EMA should be ready and greater than initial seed of 10
    assert result is not None
    assert result > 20


def test_rsi_overbought():
    rsi = RSI(period=3)
    result = None
    # All gains (prices only go up) -> RSI = 100
    for v in [10, 20, 30, 40]:
        result = rsi.update(v)
    assert result == 100.0


def test_rsi_oversold():
    rsi = RSI(period=3)
    result = None
    # All losses (prices only go down) -> RSI = 0
    for v in [40, 30, 20, 10]:
        result = rsi.update(v)
    assert result == 0.0


def test_atr():
    atr = ATR(period=3)
    result = None
    candles = [
        (15, 10, 12),
        (14, 9, 11),
        (13, 8, 10),
        (12, 7, 9),
    ]
    for high, low, close in candles:
        result = atr.update(high, low, close)
    assert result is not None
    assert result > 0


def test_bollinger_bands():
    bb = BollingerBands(period=3, std_dev_mult=2.0)
    result = None
    # All constant values -> std_dev = 0, so upper == middle == lower
    for v in [10, 10, 10]:
        result = bb.update(v)
    assert result is not None
    upper, middle, lower = result
    assert upper == middle == lower == 10.0
