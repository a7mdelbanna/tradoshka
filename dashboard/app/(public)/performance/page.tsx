"use client";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { StatCard } from "@/components/cards/StatCard";
import { EquityCurve } from "@/components/charts/EquityCurve";
import { PLHeatmap } from "@/components/charts/PLHeatmap";
import { StrategyBreakdown } from "@/components/charts/StrategyBreakdown";
import { usePortfolio } from "@/hooks/usePortfolio";

export default function PerformancePage() {
  const portfolio = usePortfolio();

  const { data: stats } = useQuery({ queryKey: ["stats"], queryFn: api.stats });
  const { data: equityData } = useQuery({ queryKey: ["equity-curve"], queryFn: api.equityCurve });
  const { data: pnlData } = useQuery({ queryKey: ["daily-pnl"], queryFn: api.dailyPnl });
  const { data: strategies } = useQuery({ queryKey: ["strategies"], queryFn: api.strategies });

  return (
    <div>
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Live Performance</h1>
        <p className="text-slate-400">
          Real-time verified trading results across all markets
          {portfolio.connected && (
            <span className="ml-2 inline-flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              <span className="text-emerald-400 text-xs">Live</span>
            </span>
          )}
        </p>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
        <StatCard title="Total ROI" value={`${(stats?.total_roi ?? 0) > 0 ? "+" : ""}${(stats?.total_roi ?? 0).toFixed(1)}%`}
          subtitle="All time" variant={(stats?.total_roi ?? 0) >= 0 ? "positive" : "negative"} />
        <StatCard title="Sharpe Ratio" value={(stats?.sharpe ?? 0).toFixed(2)} subtitle="Risk-adjusted" />
        <StatCard title="Max Drawdown" value={`-${(stats?.max_drawdown ?? 0).toFixed(1)}%`} subtitle="Peak to trough" variant="negative" />
        <StatCard title="Win Rate" value={`${((stats?.win_rate ?? 0) * 100).toFixed(1)}%`} subtitle={`${stats?.total_trades ?? 0} trades`} />
      </div>

      <div className="bg-slate-900 border border-slate-800 rounded-xl p-6 mb-6">
        <EquityCurve data={equityData ?? []} />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-6">
          <PLHeatmap data={pnlData ?? []} />
        </div>
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-6">
          <StrategyBreakdown strategies={strategies ?? []} />
        </div>
      </div>

      {portfolio.connected && (
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-6">
          <h3 className="text-sm font-medium text-slate-400 mb-3">Live Status</h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div><p className="text-xs text-slate-500">Equity</p><p className="text-lg font-semibold">${portfolio.equity.toFixed(2)}</p></div>
            <div><p className="text-xs text-slate-500">Balance</p><p className="text-lg font-semibold">${portfolio.balance.toFixed(2)}</p></div>
            <div><p className="text-xs text-slate-500">Drawdown</p><p className="text-lg font-semibold text-red-400">{(portfolio.drawdownPct * 100).toFixed(2)}%</p></div>
            <div><p className="text-xs text-slate-500">Open Positions</p><p className="text-lg font-semibold">{portfolio.openPositions}</p></div>
          </div>
        </div>
      )}

      <div className="text-center text-xs text-slate-600 mt-8 py-4 border-t border-slate-800">
        Verified by on-chain settlement + broker API audit logs · All returns are net of fees
      </div>
    </div>
  );
}
