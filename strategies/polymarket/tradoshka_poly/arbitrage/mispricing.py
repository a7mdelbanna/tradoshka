from dataclasses import dataclass

@dataclass
class MispricingSignal:
    market_id: str
    yes_token_id: str
    no_token_id: str
    yes_price: float
    no_price: float
    total: float
    deviation: float
    direction: str
    expected_profit_pct: float

class MispricingDetector:
    def __init__(self, min_deviation: float = 0.02, min_profit_pct: float = 0.005):
        self.min_deviation = min_deviation
        self.min_profit_pct = min_profit_pct

    def detect(self, yes_price: float, no_price: float, yes_token_id: str = "",
               no_token_id: str = "", market_id: str = "") -> MispricingSignal | None:
        total = yes_price + no_price
        deviation = total - 1.0
        if abs(deviation) < self.min_deviation:
            return None
        if total > 1.0:
            direction = "BUY_YES" if yes_price < no_price else "BUY_NO"
            cheaper = min(yes_price, no_price)
            profit_pct = (abs(deviation)) / cheaper if cheaper > 0 else 0
        else:
            cost = yes_price + no_price
            profit_pct = (1.0 - cost) / cost if cost > 0 else 0
            direction = "BUY_YES" if yes_price < no_price else "BUY_NO"
        if profit_pct < self.min_profit_pct:
            return None
        return MispricingSignal(market_id, yes_token_id, no_token_id, yes_price, no_price,
                                round(total, 4), round(abs(deviation), 4), direction, round(profit_pct, 4))
