from dataclasses import dataclass

@dataclass
class InventoryState:
    yes_shares: float = 0.0
    no_shares: float = 0.0
    total_cost: float = 0.0
    realized_pnl: float = 0.0

    @property
    def net_exposure(self) -> float:
        return self.yes_shares - self.no_shares

    @property
    def inventory_skew(self) -> float:
        total = self.yes_shares + self.no_shares
        return self.net_exposure / total if total else 0.0

class InventoryManager:
    def __init__(self, max_inventory: float = 1000.0):
        self.state = InventoryState()
        self.max_inventory = max_inventory

    def can_buy_yes(self, size: float) -> bool:
        return self.state.yes_shares + size <= self.max_inventory

    def can_buy_no(self, size: float) -> bool:
        return self.state.no_shares + size <= self.max_inventory

    def record_buy_yes(self, size: float, price: float) -> None:
        self.state.yes_shares += size
        self.state.total_cost += size * price

    def record_buy_no(self, size: float, price: float) -> None:
        self.state.no_shares += size
        self.state.total_cost += size * price

    def record_sell_yes(self, size: float, price: float) -> None:
        self.state.yes_shares -= size
        self.state.realized_pnl += size * price

    def record_sell_no(self, size: float, price: float) -> None:
        self.state.no_shares -= size
        self.state.realized_pnl += size * price
