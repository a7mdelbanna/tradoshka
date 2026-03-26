from dataclasses import dataclass, field


@dataclass
class SimulationWorld:
    market_question: str
    market_description: str
    current_yes_price: float
    current_no_price: float
    volume_24h: float
    end_date: str | None
    context_data: list[str] = field(default_factory=list)

    @property
    def market_implied_probability(self) -> float:
        return self.current_yes_price


class WorldBuilder:
    def build(self, question: str, description: str = "", yes_price: float = 0.5,
              no_price: float = 0.5, volume: float = 0.0, end_date: str | None = None,
              context: list[str] | None = None) -> SimulationWorld:
        return SimulationWorld(
            market_question=question, market_description=description,
            current_yes_price=yes_price, current_no_price=no_price,
            volume_24h=volume, end_date=end_date,
            context_data=context or [],
        )
