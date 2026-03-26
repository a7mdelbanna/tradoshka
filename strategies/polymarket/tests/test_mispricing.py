from tradoshka_poly.arbitrage.mispricing import MispricingDetector

def test_no_mispricing_at_fair():
    assert MispricingDetector(min_deviation=0.02).detect(0.50, 0.50) is None

def test_detects_overpriced():
    r = MispricingDetector(min_deviation=0.02).detect(0.55, 0.50)
    assert r is not None
    assert r.total > 1.0

def test_detects_underpriced():
    r = MispricingDetector(min_deviation=0.02).detect(0.45, 0.50)
    assert r is not None
    assert r.total < 1.0

def test_ignores_small_deviation():
    assert MispricingDetector(min_deviation=0.05).detect(0.51, 0.50) is None
