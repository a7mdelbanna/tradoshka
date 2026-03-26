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
    <div className="relative">
      {/* Background gradient */}
      <div className="absolute top-0 right-0 w-[500px] h-[500px] bg-emerald-500/3 rounded-full blur-[150px] pointer-events-none" />

      <div className="relative z-10">
        {/* Header */}
        <div className="mb-10">
          <div className="flex items-center gap-3 mb-3">
            <h1 className="text-3xl font-bold text-white">Live Performance</h1>
            {portfolio.connected && (
              <div className="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-emerald-400/10 border border-emerald-400/20">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                <span className="text-xs font-medium text-emerald-400">Connected</span>
              </div>
            )}
          </div>
          <p className="text-slate-400">Real-time verified trading results across all markets</p>
        </div>

        {/* Stat Cards */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          <StatCard title="Total ROI"
            value={`${(stats?.total_roi ?? 0) > 0 ? "+" : ""}${(stats?.total_roi ?? 0).toFixed(1)}%`}
            subtitle="All time" variant={(stats?.total_roi ?? 0) >= 0 ? "positive" : "negative"} />
          <StatCard title="Sharpe Ratio" value={(stats?.sharpe ?? 0).toFixed(2)} subtitle="Risk-adjusted" />
          <StatCard title="Max Drawdown"
            value={`-${(stats?.max_drawdown ?? 0).toFixed(1)}%`}
            subtitle="Peak to trough" variant="negative" />
          <StatCard title="Win Rate"
            value={`${((stats?.win_rate ?? 0) * 100).toFixed(1)}%`}
            subtitle={`${stats?.total_trades ?? 0} trades`} />
        </div>

        {/* Secondary stats row */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          <StatCard title="Profit Factor" value={(stats?.profit_factor ?? 0).toFixed(2)} subtitle="Gross profit / loss" />
          <StatCard title="Calmar Ratio" value={(stats?.calmar ?? 0).toFixed(2)} subtitle="Return / drawdown" />
          <StatCard title="Recovery Factor" value={(stats?.recovery_factor ?? 0).toFixed(1)} subtitle="Net profit / max DD" />
          <StatCard title="Total Trades" value={`${stats?.total_trades ?? 0}`} subtitle="All markets" />
        </div>

        {/* Equity Curve */}
        <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-6 mb-6 backdrop-blur-sm">
          <EquityCurve data={equityData ?? []} />
        </div>

        {/* P&L Heatmap + Strategy Breakdown */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
          <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-6 backdrop-blur-sm">
            <PLHeatmap data={pnlData ?? []} />
          </div>
          <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-6 backdrop-blur-sm">
            <StrategyBreakdown strategies={strategies ?? []} />
          </div>
        </div>

        {/* Live Status */}
        {portfolio.connected && (
          <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-6 mb-8 backdrop-blur-sm">
            <h3 className="text-sm font-semibold text-white mb-4">Live Status</h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
              <div>
                <p className="text-xs text-slate-500 mb-1">Equity</p>
                <p className="text-xl font-bold text-white">${portfolio.equity.toFixed(2)}</p>
              </div>
              <div>
                <p className="text-xs text-slate-500 mb-1">Balance</p>
                <p className="text-xl font-bold text-white">${portfolio.balance.toFixed(2)}</p>
              </div>
              <div>
                <p className="text-xs text-slate-500 mb-1">Drawdown</p>
                <p className="text-xl font-bold text-red-400">{(portfolio.drawdownPct * 100).toFixed(2)}%</p>
              </div>
              <div>
                <p className="text-xs text-slate-500 mb-1">Open Positions</p>
                <p className="text-xl font-bold text-white">{portfolio.openPositions}</p>
              </div>
            </div>
          </div>
        )}

        {/* Markets Status */}
        <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-6 mb-8 backdrop-blur-sm">
          <h3 className="text-sm font-semibold text-white mb-4">Markets</h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {[
              { name: "Polymarket", status: "Active", color: "emerald" },
              { name: "Crypto", status: "Coming Soon", color: "slate" },
              { name: "Forex", status: "Coming Soon", color: "slate" },
              { name: "Stocks", status: "Coming Soon", color: "slate" },
            ].map(m => (
              <div key={m.name} className="flex items-center justify-between p-3 rounded-xl bg-slate-800/30 border border-slate-700/30">
                <span className="text-sm font-medium text-slate-200">{m.name}</span>
                <span className={`text-xs px-2 py-0.5 rounded-full ${m.color === "emerald" ? "bg-emerald-400/10 text-emerald-400 border border-emerald-400/20" : "bg-slate-700/50 text-slate-500 border border-slate-600/30"}`}>
                  {m.status}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Footer */}
        <div className="text-center py-6 border-t border-slate-800/50">
          <p className="text-xs text-slate-600">
            Verified by on-chain settlement + broker API audit logs · All returns are net of fees
          </p>
        </div>
      </div>
    </div>
  );
}
