from dataclasses import dataclass

@dataclass
class QuoteParams:
    fair_value: float
    base_spread_bps: int = 200
    min_spread_bps: int = 50
    inventory_skew: float = 0.0

class SpreadCalculator:
    def calculate_quotes(self, params: QuoteParams) -> tuple[float, float]:
        half_spread = max(params.base_spread_bps, params.min_spread_bps) / 10000 / 2
        skew = params.inventory_skew * half_spread * 0.5
        bid = max(0.01, min(0.99, params.fair_value - half_spread - skew))
        ask = max(0.01, min(0.99, params.fair_value + half_spread - skew))
        if bid >= ask:
            mid = (bid + ask) / 2
            bid, ask = mid - 0.01, mid + 0.01
        return round(bid, 4), round(ask, 4)
