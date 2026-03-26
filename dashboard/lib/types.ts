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

export interface WalletResponse {
  mode: string;
  balance: string;
  equity: string;
  unrealized_pnl: string;
  realized_pnl: string;
  drawdown_pct: string;
  total_fees: string;
  open_positions: number;
  total_trades?: number;
  positions: WalletPositionInfo[];
}

export interface WalletPositionInfo {
  token_id: string;
  question: string;
  outcome: string;
  side: string;
  shares: string;
  avg_price: string;
  current_price: string;
  unrealized_pnl: string;
  strategy: string;
}

export interface TradeInfo {
  id: string;
  timestamp: string;
  symbol: string;
  question: string;
  direction: string;
  side: string;
  shares: string;
  price: string;
  fee: string;
  strategy: string;
  strength: number;
  edge: number;
  pnl: string | null;
  closed: boolean;
  market?: string;
  thesis_reasoning?: string;
  stop_loss?: string;
  trailing_stop?: string;
  take_profit?: string;
  time_stop_hours?: number;
  risk_amount?: string;
  reward_risk_ratio?: number;
  strategy_tier?: string;
  close_reason?: string | null;
}

export interface ReadinessResponse {
  criteria: ReadinessCriterion[];
  passed: number;
  total: number;
  is_ready: boolean;
}

export interface ReadinessCriterion {
  name: string;
  threshold: string;
  current_value: string;
  passed: boolean;
}

export interface OrchestratorStatus {
  cycle_count: number;
  last_cycle_at: string | null;
  wallet_mode: string;
  tracked_markets: number;
  open_positions: number;
}

export interface CycleResult {
  cycle: number;
  markets_evaluated: number;
  signals_generated: number;
  trades_executed: number;
  trades: object[];
}

export interface CryptoAsset {
  symbol: string;
  price: string;
  volume_24h: number;
}

export interface PerpPositionInfo {
  symbol: string;
  side: string;
  size: string;
  entry_price: string;
  mark_price: string;
  leverage: number;
  margin: string;
  unrealized_pnl: string;
  roe_pct: string;
  liquidation_price: string;
  funding: string;
  strategy: string;
  timeframe: string;
}

export interface PerpWalletResponse extends WalletResponse {
  used_margin?: string;
  available_margin?: string;
  default_leverage?: number;
  total_funding?: string;
}
