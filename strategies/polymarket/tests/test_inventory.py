from tradoshka_poly.market_making.inventory import InventoryManager

def test_initial_state():
    mgr = InventoryManager()
    assert mgr.state.net_exposure == 0.0
    assert mgr.state.inventory_skew == 0.0

def test_buy_yes_increases_exposure():
    mgr = InventoryManager()
    mgr.record_buy_yes(100, 0.5)
    assert mgr.state.yes_shares == 100
    assert mgr.state.net_exposure == 100

def test_max_inventory():
    mgr = InventoryManager(max_inventory=50)
    assert mgr.can_buy_yes(50)
    assert not mgr.can_buy_yes(51)
