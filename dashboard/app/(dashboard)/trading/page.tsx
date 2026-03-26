"use client";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { useTradingWs } from "@/hooks/useTradingWs";
import { ModeBadge } from "@/components/trading/ModeBadge";
import { PortfolioPanel } from "@/components/trading/PortfolioPanel";
import { TradeFeed } from "@/components/trading/TradeFeed";
import { ReadinessScore } from "@/components/trading/ReadinessScore";
import { StrategyHealth } from "@/components/trading/StrategyHealth";
import { CryptoPriceBar } from "@/components/trading/CryptoPriceBar";
import { MarketTabs } from "@/components/trading/MarketTabs";
import { EquityCurve } from "@/components/charts/EquityCurve";
import { PLHeatmap } from "@/components/charts/PLHeatmap";
import Link from "next/link";

const MARKET_LABELS: Record<string, string> = {
  polymarket: "Polymarket",
  crypto: "Crypto",
};

export default function TradingPage() {
  const [selectedMarket, setSelectedMarket] = useState<string>("all");
  const { wallet, wallets, trades, connected } = useTradingWs();
  const { data: equityData } = useQuery({ queryKey: ["equity-curve"], queryFn: api.equityCurve });
  const { data: pnlData } = useQuery({ queryKey: ["daily-pnl"], queryFn: api.dailyPnl });
  const { data: orchStatus } = useQuery({ queryKey: ["orchestrator"], queryFn: api.orchestratorStatus, refetchInterval: 5000 });
  const { data: cryptoData } = useQuery({ queryKey: ["crypto-assets"], queryFn: api.cryptoAssets, refetchInterval: 10000 });

  // Fetch per-market wallet data when a specific market is selected
  const { data: marketWallet } = useQuery({
    queryKey: ["wallet-market", selectedMarket],
    queryFn: () => api.walletByMarket(selectedMarket),
    enabled: selectedMarket !== "all",
    refetchInterval: 5000,
  });

  // Fetch per-market trades
  const { data: marketTrades } = useQuery({
    queryKey: ["trades-market", selectedMarket],
    queryFn: () => api.tradesByMarket(selectedMarket),
    enabled: selectedMarket !== "all",
    refetchInterval: 5000,
  });

  const handleScan = async () => { try { await api.triggerScan(); } catch {} };
  const handleCycle = async () => { try { await api.triggerCycle(); } catch {} };

  // Determine which wallet to show
  const activeWallet = selectedMarket !== "all"
    ? marketWallet || wallets[selectedMarket] || null
    : wallet;

  // Determine which trades to show
  const activeTrades = selectedMarket !== "all" && marketTrades?.trades
    ? marketTrades.trades
    : trades;

  // Count trades by market for tab badges
  const allTrades = [...trades, ...(marketTrades?.trades || [])];
  const polymarketCount = trades.filter((t) => {
    const m = t.market?.toLowerCase();
    if (m) return m === "polymarket";
    return t.question?.includes("?");
  }).length;
  const cryptoCount = trades.filter((t) => {
    const m = t.market?.toLowerCase();
    if (m) return m === "crypto";
    return t.symbol?.endsWith("USDT");
  }).length;

  const marketLabel = MARKET_LABELS[selectedMarket] || undefined;

  return (
    <div className="min-h-screen bg-slate-950">
      {/* Background gradient accent */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute top-0 left-1/4 w-96 h-96 bg-emerald-500/3 rounded-full blur-3xl" />
        <div className="absolute bottom-0 right-1/4 w-96 h-96 bg-blue-500/3 rounded-full blur-3xl" />
      </div>

      {/* Top Bar */}
      <nav className="relative border-b border-slate-800/40 bg-slate-950/80 backdrop-blur-2xl sticky top-0 z-50">
        <div className="max-w-[1600px] mx-auto px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-5">
            <Link href="/" className="flex items-center gap-2.5 group">
              <div className="w-8 h-8 rounded-xl bg-gradient-to-br from-emerald-400 to-emerald-600 flex items-center justify-center shadow-[0_0_15px_rgba(52,211,153,0.3)] transition-all duration-300 group-hover:shadow-[0_0_25px_rgba(52,211,153,0.4)] group-hover:scale-105">
                <span className="text-slate-950 font-black text-sm">T</span>
              </div>
              <span className="text-lg font-black text-white tracking-tight">Tradoshka</span>
            </Link>
            <div className="h-5 w-px bg-slate-800/60" />
            <Link
              href="/performance"
              className="text-xs text-slate-500 hover:text-slate-200 transition-colors duration-300 font-medium"
            >
              Performance
            </Link>
            <span className="text-xs text-white font-bold relative">
              Trading
              <span className="absolute -bottom-1 left-0 w-full h-0.5 bg-emerald-400 rounded-full" />
            </span>
          </div>
          <div className="flex items-center gap-4">
            <ModeBadge mode={wallet?.mode || "Dry"} />
            <div
              className={`flex items-center gap-2 px-3 py-1.5 rounded-full transition-all duration-300 ${
                connected
                  ? "bg-emerald-400/8 border border-emerald-400/20"
                  : "bg-slate-800/30 border border-slate-700/30"
              }`}
            >
              <span
                className={`w-2 h-2 rounded-full transition-all duration-300 ${
                  connected
                    ? "bg-emerald-400 animate-pulse shadow-[0_0_8px_rgba(52,211,153,0.6)]"
                    : "bg-slate-600"
                }`}
              />
              <span className={`text-[10px] font-semibold ${connected ? "text-emerald-400" : "text-slate-600"}`}>
                {connected ? "Live" : "Offline"}
              </span>
            </div>
          </div>
        </div>
      </nav>

      <div className="relative max-w-[1600px] mx-auto px-6 py-6">
        {/* Controls */}
        <div className="flex items-center gap-3 mb-5">
          <button
            onClick={handleScan}
            className="px-5 py-2.5 text-xs font-semibold glass-panel rounded-xl hover:bg-slate-700/40 text-slate-300 transition-all duration-300 hover:-translate-y-0.5 hover:shadow-lg"
          >
            Scan Markets
          </button>
          <button
            onClick={handleCycle}
            className="px-5 py-2.5 text-xs font-semibold bg-gradient-to-r from-emerald-500/15 to-emerald-400/5 border border-emerald-500/25 hover:border-emerald-500/40 text-emerald-400 rounded-xl transition-all duration-300 hover:-translate-y-0.5 hover:shadow-[0_0_20px_rgba(52,211,153,0.1)]"
          >
            Run Cycle
          </button>
          {orchStatus && (
            <div className="flex gap-5 text-[10px] text-slate-500 ml-auto font-mono">
              <span>
                Cycles{" "}
                <span className="text-slate-300 font-bold">{orchStatus.cycle_count}</span>
              </span>
              <span>
                Markets{" "}
                <span className="text-slate-300 font-bold">{orchStatus.tracked_markets}</span>
              </span>
              <span>
                Positions{" "}
                <span className="text-slate-300 font-bold">{orchStatus.open_positions}</span>
              </span>
            </div>
          )}
        </div>

        {/* Crypto Price Bar */}
        <CryptoPriceBar />

        {/* Market Tabs */}
        <MarketTabs
          selected={selectedMarket}
          onSelect={setSelectedMarket}
          polymarketCount={polymarketCount}
          cryptoCount={cryptoCount}
        />

        {/* Split Screen */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          {/* Left Panel -- Portfolio */}
          <div className="lg:col-span-4 space-y-6">
            <div className="glass-panel rounded-2xl p-6 shadow-[0_0_20px_rgba(52,211,153,0.08)]">
              <h2 className="text-[10px] font-bold uppercase tracking-widest text-slate-500 mb-5 flex items-center gap-2">
                <span className="w-1 h-4 rounded-full bg-gradient-to-b from-emerald-400 to-emerald-600" />
                Portfolio
              </h2>
              <PortfolioPanel wallet={activeWallet} marketLabel={marketLabel} />
            </div>

            <div className="glass-panel rounded-2xl p-6">
              <StrategyHealth />
            </div>

            <div className="glass-panel rounded-2xl p-6">
              <ReadinessScore />
            </div>
          </div>

          {/* Right Panel -- Trade Feed */}
          <div className="lg:col-span-8">
            <div className="glass-panel rounded-2xl p-6 mb-6">
              <div className="flex items-center justify-between mb-5">
                <h2 className="text-[10px] font-bold uppercase tracking-widest text-slate-500 flex items-center gap-2">
                  <span className="w-1 h-4 rounded-full bg-gradient-to-b from-blue-400 to-purple-500" />
                  Live Trade Feed
                </h2>
                <div className="flex items-center gap-2">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                  <span className="text-[10px] text-slate-600 font-mono">
                    {activeTrades.length} trades
                  </span>
                </div>
              </div>
              <TradeFeed trades={activeTrades} marketFilter={selectedMarket} />
            </div>

            {/* Charts */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div className="glass-panel rounded-2xl p-6">
                <EquityCurve data={equityData ?? []} />
              </div>
              <div className="glass-panel rounded-2xl p-6">
                <PLHeatmap data={pnlData ?? []} />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
