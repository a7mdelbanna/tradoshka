import type { PortfolioResponse, EquityPoint, DailyPnl, StrategyPerformance, Stats, PositionInfo, RiskResponse, WalletResponse, TradeInfo, ReadinessResponse, OrchestratorStatus, CycleResult, CryptoAsset } from "./types";

const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:3001";

async function apiFetch<T>(path: string, method: string = "GET"): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, { method });
  if (!res.ok) throw new Error(`API ${res.status}: ${res.statusText}`);
  return res.json();
}

export const api = {
  health: () => apiFetch<{ status: string; version: string }>("/health"),
  portfolio: () => apiFetch<PortfolioResponse>("/api/portfolio"),
  positions: () => apiFetch<{ positions: PositionInfo[] }>("/api/positions"),
  orders: () => apiFetch<{ active_orders: number; total_orders: number }>("/api/orders"),
  risk: () => apiFetch<RiskResponse>("/api/risk"),
  equityCurve: () => apiFetch<EquityPoint[]>("/api/equity-curve"),
  dailyPnl: () => apiFetch<DailyPnl[]>("/api/pnl/daily"),
  strategies: () => apiFetch<StrategyPerformance[]>("/api/strategies"),
  stats: () => apiFetch<Stats>("/api/stats"),
  wallet: () => apiFetch<WalletResponse>("/api/wallet"),
  walletByMarket: (market: string) => apiFetch<WalletResponse>(`/api/wallet/${market}`),
  tradesLive: () => apiFetch<{ trades: TradeInfo[]; total: number }>("/api/trades/live"),
  tradesByMarket: (market: string) => apiFetch<{ trades: TradeInfo[]; total: number }>(`/api/trades/${market}`),
  readiness: () => apiFetch<ReadinessResponse>("/api/readiness"),
  trackedMarkets: () => apiFetch<{ count: number; markets: object[] }>("/api/markets/tracked"),
  orchestratorStatus: () => apiFetch<OrchestratorStatus>("/api/orchestrator"),
  triggerScan: () => apiFetch<object>("/api/orchestrator/scan", "POST"),
  triggerCycle: () => apiFetch<CycleResult>("/api/orchestrator/cycle", "POST"),
  cryptoAssets: () => apiFetch<{ count: number; assets: CryptoAsset[] }>("/api/crypto/assets"),
  cryptoScan: () => apiFetch<{ scanned: number; assets: object[] }>("/api/crypto/scan", "POST"),
  evolutionStats: () => apiFetch<any>("/api/evolution/stats"),
  evolutionLeaderboard: () => apiFetch<any>("/api/evolution/leaderboard"),
  evolutionTimeline: () => apiFetch<any>("/api/evolution/timeline"),
  evolutionGraveyard: () => apiFetch<any>("/api/evolution/graveyard"),
  evolutionTrigger: () => apiFetch<any>("/api/evolution/trigger", "POST"),
  evolutionWallet: (name: string) => apiFetch<any>(`/api/evolution/wallet/${name}`),
  copyTradingStatus: () => apiFetch<any>("/api/copy-trading/status"),
};
