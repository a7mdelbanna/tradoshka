"use client";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { useTradingWs } from "@/hooks/useTradingWs";
import { ModeBadge } from "@/components/trading/ModeBadge";
import { PortfolioPanel } from "@/components/trading/PortfolioPanel";
import { TradeFeed } from "@/components/trading/TradeFeed";
import { ReadinessScore } from "@/components/trading/ReadinessScore";
import { StrategyHealth } from "@/components/trading/StrategyHealth";
import { EquityCurve } from "@/components/charts/EquityCurve";
import { PLHeatmap } from "@/components/charts/PLHeatmap";
import Link from "next/link";

export default function TradingPage() {
  const { wallet, trades, connected } = useTradingWs();
  const { data: equityData } = useQuery({ queryKey: ["equity-curve"], queryFn: api.equityCurve });
  const { data: pnlData } = useQuery({ queryKey: ["daily-pnl"], queryFn: api.dailyPnl });
  const { data: orchStatus } = useQuery({ queryKey: ["orchestrator"], queryFn: api.orchestratorStatus, refetchInterval: 5000 });

  const handleScan = async () => { try { await api.triggerScan(); } catch {} };
  const handleCycle = async () => { try { await api.triggerCycle(); } catch {} };

  return (
    <div className="min-h-screen bg-slate-950">
      {/* Top Bar */}
      <nav className="border-b border-slate-800/50 bg-slate-950/90 backdrop-blur-xl sticky top-0 z-50">
        <div className="max-w-[1600px] mx-auto px-6 h-14 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link href="/" className="flex items-center gap-2">
              <div className="w-7 h-7 rounded-lg bg-gradient-to-br from-emerald-400 to-emerald-600 flex items-center justify-center">
                <span className="text-slate-950 font-black text-xs">T</span>
              </div>
              <span className="text-lg font-bold text-white">Tradoshka</span>
            </Link>
            <div className="h-4 w-px bg-slate-800" />
            <Link href="/performance" className="text-xs text-slate-400 hover:text-slate-200 transition-colors">Performance</Link>
            <span className="text-xs text-white font-medium">Trading</span>
          </div>
          <div className="flex items-center gap-3">
            <ModeBadge mode={wallet?.mode || "Dry"} />
            {connected && (
              <div className="flex items-center gap-1.5 text-[10px] text-emerald-400">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                Connected
              </div>
            )}
          </div>
        </div>
      </nav>

      <div className="max-w-[1600px] mx-auto px-6 py-6">
        {/* Controls */}
        <div className="flex items-center gap-3 mb-6">
          <button onClick={handleScan}
            className="px-4 py-2 text-xs font-medium bg-slate-800/50 hover:bg-slate-700/50 border border-slate-700/50 hover:border-slate-600/50 text-slate-300 rounded-lg transition-all">
            Scan Markets
          </button>
          <button onClick={handleCycle}
            className="px-4 py-2 text-xs font-medium bg-emerald-500/10 hover:bg-emerald-500/20 border border-emerald-500/20 hover:border-emerald-500/30 text-emerald-400 rounded-lg transition-all">
            Run Cycle
          </button>
          {orchStatus && (
            <div className="flex gap-4 text-[10px] text-slate-500 ml-auto">
              <span>Cycles: {orchStatus.cycle_count}</span>
              <span>Markets: {orchStatus.tracked_markets}</span>
              <span>Positions: {orchStatus.open_positions}</span>
            </div>
          )}
        </div>

        {/* Split Screen */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          {/* Left Panel -- Portfolio */}
          <div className="lg:col-span-4 space-y-6">
            <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-5 backdrop-blur-sm">
              <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-4">Portfolio</h2>
              <PortfolioPanel wallet={wallet} />
            </div>

            <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-5 backdrop-blur-sm">
              <StrategyHealth />
            </div>

            <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-5 backdrop-blur-sm">
              <ReadinessScore />
            </div>
          </div>

          {/* Right Panel -- Trade Feed */}
          <div className="lg:col-span-8">
            <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-5 backdrop-blur-sm mb-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400">Live Trade Feed</h2>
                <span className="text-[10px] text-slate-600">{trades.length} trades</span>
              </div>
              <TradeFeed trades={trades} />
            </div>

            {/* Charts */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-5 backdrop-blur-sm">
                <EquityCurve data={equityData ?? []} />
              </div>
              <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-5 backdrop-blur-sm">
                <PLHeatmap data={pnlData ?? []} />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
