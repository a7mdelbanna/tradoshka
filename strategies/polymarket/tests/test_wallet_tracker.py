from datetime import datetime, timezone
from tradoshka_poly.copy_trading.wallet_tracker import WalletTracker, WhaleTrade

def test_add_and_track():
    t = WalletTracker()
    t.add_wallet("0xABC")
    assert "0xabc" in t.tracked_wallets

def test_record_and_retrieve():
    t = WalletTracker()
    t.add_wallet("0xABC")
    trade = WhaleTrade("0xabc", "mkt", "tok", "BUY", "Yes", 100.0, 0.55, datetime.now(timezone.utc))
    t.record_trade(trade)
    assert len(t.get_recent_trades("0xABC")) == 1

def test_ignores_untracked():
    t = WalletTracker()
    trade = WhaleTrade("0xother", "mkt", "tok", "BUY", "Yes", 100.0, 0.55, datetime.now(timezone.utc))
    t.record_trade(trade)
    assert len(t.get_recent_trades("0xother")) == 0
