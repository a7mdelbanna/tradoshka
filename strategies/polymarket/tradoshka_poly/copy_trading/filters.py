from dataclasses import dataclass

@dataclass
class TraderProfile:
    address: str
    total_pnl: float
    roi: float
    win_rate: float
    total_trades: int
    markets_traded: int
    avg_position_size: float
    last_active_days_ago: int

@dataclass
class TraderFilter:
    min_pnl: float = 5000.0
    min_roi: float = 0.15
    min_win_rate: float = 0.55
    min_trades: int = 50
    min_markets: int = 10
    min_avg_position: float = 100.0
    max_inactive_days: int = 90

def filter_traders(traders: list[TraderProfile], criteria: TraderFilter | None = None) -> list[TraderProfile]:
    c = criteria or TraderFilter()
    return [t for t in traders if (
        t.total_pnl >= c.min_pnl and t.roi >= c.min_roi and
        t.win_rate >= c.min_win_rate and t.total_trades >= c.min_trades and
        t.markets_traded >= c.min_markets and t.avg_position_size >= c.min_avg_position and
        t.last_active_days_ago <= c.max_inactive_days
    )]
