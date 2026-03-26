from tradoshka_poly.copy_trading.filters import TraderProfile, TraderFilter, filter_traders

def test_filter_passes_good_trader():
    assert len(filter_traders([TraderProfile("0x1", 10000, 0.25, 0.6, 100, 20, 500, 5)])) == 1

def test_filter_rejects_low_pnl():
    assert len(filter_traders([TraderProfile("0x1", 100, 0.25, 0.6, 100, 20, 500, 5)])) == 0

def test_filter_rejects_low_win_rate():
    assert len(filter_traders([TraderProfile("0x1", 10000, 0.25, 0.4, 100, 20, 500, 5)])) == 0

def test_filter_rejects_inactive():
    assert len(filter_traders([TraderProfile("0x1", 10000, 0.25, 0.6, 100, 20, 500, 180)])) == 0
