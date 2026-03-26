import type { PortfolioResponse, EquityPoint, DailyPnl, StrategyPerformance, Stats, PositionInfo, RiskResponse } from "./types";

const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:3001";

async function apiFetch<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
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
};
