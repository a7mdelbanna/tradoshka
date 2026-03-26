from tradoshka_poly.market_making.spread import SpreadCalculator, QuoteParams

def test_symmetric_spread():
    bid, ask = SpreadCalculator().calculate_quotes(QuoteParams(fair_value=0.5))
    assert bid < 0.5 < ask
    spread = ask - bid
    assert abs(spread - 0.02) < 0.001

def test_inventory_skew_shifts_quotes():
    calc = SpreadCalculator()
    bid_n, _ = calc.calculate_quotes(QuoteParams(fair_value=0.5, inventory_skew=0.0))
    bid_l, _ = calc.calculate_quotes(QuoteParams(fair_value=0.5, inventory_skew=0.5))
    assert bid_l < bid_n

def test_clamped_to_valid_range():
    bid, ask = SpreadCalculator().calculate_quotes(QuoteParams(fair_value=0.01))
    assert bid >= 0.01
    assert ask <= 0.99
