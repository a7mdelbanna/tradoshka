export interface PortfolioResponse {
  balance: string;
  equity: string;
  unrealized_pnl: string;
  realized_pnl: string;
  open_positions: number;
  win_rate: string;
  total_fees: string;
}

export interface PositionInfo {
  symbol: string;
  side: string;
  quantity: string;
  entry_price: string;
  current_price: string;
  unrealized_pnl: string;
  strategy_id: string;
  market: string;
}

export interface RiskResponse {
  breaker_state: string;
  allows_trading: boolean;
}

export interface EquityPoint {
  time: string;
  value: number;
}

export interface DailyPnl {
  date: string;
  pnl: number;
}

export interface StrategyPerformance {
  name: string;
  return_pct: number;
  trades: number;
  win_rate: number;
}

export interface Stats {
  total_roi: number;
  sharpe: number;
  max_drawdown: number;
  calmar: number;
  win_rate: number;
  profit_factor: number;
  total_trades: number;
  recovery_factor: number;
}
