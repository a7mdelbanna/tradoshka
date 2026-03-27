"use client";
import { Suspense, useState, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { useSearchParams } from "next/navigation";
import Link from "next/link";
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

const MARKET_LABELS: Record<string, string> = {
  polymarket: "Polymarket",
  crypto: "Crypto",
};

function TradingContent() {
  const searchParams = useSearchParams();
  const [selectedMarket, setSelectedMarket] = useState<string>("all");
  const [cryptoSubTab, setCryptoSubTab] = useState<"spot" | "perps">("spot");
  const [selectedStrategy, setSelectedStrategy] = useState<string | null>(null);

  // Read market selection from URL query params (set by sidebar)
  useEffect(() => {
    const market = searchParams.get("market");
    const sub = searchParams.get("sub");
    if (market === "polymarket") {
      setSelectedMarket("polymarket");
    } else if (market === "crypto") {
      setSelectedMarket("crypto");
      if (sub === "perps") setCryptoSubTab("perps");
      else setCryptoSubTab("spot");
    }
  }, [searchParams]);

  // Reset strategy picker when market changes
  useEffect(() => {
    setSelectedStrategy(null);
  }, [selectedMarket]);

  const { wallet, wallets, trades, connected } = useTradingWs();
  const { data: equityData } = useQuery({ queryKey: ["equity-curve"], queryFn: api.equityCurve });
  const { data: pnlData } = useQuery({ queryKey: ["daily-pnl"], queryFn: api.dailyPnl });
  const { data: orchStatus } = useQuery({ queryKey: ["orchestrator"], queryFn: api.orchestratorStatus, refetchInterval: 5000 });
  const { data: cryptoData } = useQuery({ queryKey: ["crypto-assets"], queryFn: api.cryptoAssets, refetchInterval: 10000 });

  // When crypto is selected, use the sub-tab to determine which endpoint to call
  const effectiveMarket = selectedMarket === "crypto" ? cryptoSubTab : selectedMarket;

  // Fetch per-market wallet data when a specific market is selected
  const { data: marketWallet } = useQuery({
    queryKey: ["wallet-market", effectiveMarket],
    queryFn: () => api.walletByMarket(effectiveMarket),
    enabled: selectedMarket !== "all",
    refetchInterval: 5000,
  });

  // Fetch per-market trades
  const { data: marketTrades } = useQuery({
    queryKey: ["trades-market", effectiveMarket],
    queryFn: () => api.tradesByMarket(effectiveMarket),
    enabled: selectedMarket !== "all",
    refetchInterval: 5000,
  });

  // Separate queries for tab badge counts
  const { data: spotWallet } = useQuery({
    queryKey: ["wallet-spot-count"],
    queryFn: () => api.walletByMarket("spot"),
    refetchInterval: 10000,
  });
  const { data: perpsWallet } = useQuery({
    queryKey: ["wallet-perps-count"],
    queryFn: () => api.walletByMarket("perps"),
    refetchInterval: 10000,
  });

  // Evolution leaderboard for PM strategy wallets
  const { data: evolutionData } = useQuery({
    queryKey: ["evolution-leaderboard"],
    queryFn: api.evolutionLeaderboard,
    refetchInterval: 30000,
  });

  // Fetch specific strategy wallet when selected
  const { data: strategyWallet } = useQuery({
    queryKey: ["strategy-wallet", selectedStrategy],
    queryFn: () => api.evolutionWallet(selectedStrategy!),
    enabled: !!selectedStrategy,
    refetchInterval: 10000,
  });

  const handleScan = async () => { try { await api.triggerScan(); } catch {} };
  const handleCycle = async () => { try { await api.triggerCycle(); } catch {} };

  // PM strategy wallet aggregation
  const allStrategies: any[] = evolutionData?.strategies ?? evolutionData?.leaderboard ?? [];
  const pmStrategies = allStrategies.filter((s: any) => {
    const m = (s.market ?? "").toLowerCase();
    return m.includes("poly");
  });
  const pmWithTrades = pmStrategies.filter((s: any) => (s.trades ?? 0) > 0);
  const pmTotalEquity = pmStrategies.reduce((sum: number, s: any) => sum + (s.equity ?? 0), 0);
  const pmTotalTrades = pmStrategies.reduce((sum: number, s: any) => sum + (s.trades ?? 0), 0);
  const pmTotalPnl = pmStrategies.reduce((sum: number, s: any) => sum + (s.total_pnl ?? s.pnl ?? 0), 0);

  // Determine which wallet to show
  const isPolymarketSelected = selectedMarket === "polymarket";
  const mainWalletTrades = (marketWallet as any)?.total_trades ?? (marketWallet as any)?.open_positions ?? 0;
  const showPmAggregated = isPolymarketSelected && mainWalletTrades === 0 && pmStrategies.length > 0 && !selectedStrategy;

  // Build a synthetic aggregated wallet for PM when main is empty
  const pmAggregatedWallet = showPmAggregated ? {
    balance: pmStrategies.reduce((sum: number, s: any) => sum + (s.balance ?? 0), 0).toFixed(4),
    equity: pmTotalEquity.toFixed(4),
    unrealized_pnl: pmStrategies.reduce((sum: number, s: any) => sum + (s.unrealized_pnl ?? 0), 0).toFixed(4),
    realized_pnl: pmStrategies.reduce((sum: number, s: any) => sum + (s.realized_pnl ?? 0), 0).toFixed(4),
    drawdown_pct: "0",
    open_positions: pmStrategies.reduce((sum: number, s: any) => sum + (s.open_positions ?? 0), 0),
  } : null;

  // Normalize strategy wallet response to WalletData shape
  const normalizedStrategyWallet = strategyWallet ? {
    balance: String(strategyWallet.balance ?? "0"),
    equity: String(strategyWallet.equity ?? "0"),
    unrealized_pnl: String(strategyWallet.unrealized_pnl ?? "0"),
    realized_pnl: String(strategyWallet.realized_pnl ?? "0"),
    drawdown_pct: String(strategyWallet.drawdown_pct ?? "0"),
    open_positions: strategyWallet.open_positions ?? 0,
  } : null;

  let activeWallet: any;
  if (selectedStrategy && normalizedStrategyWallet) {
    activeWallet = normalizedStrategyWallet;
  } else if (showPmAggregated) {
    activeWallet = pmAggregatedWallet;
  } else if (selectedMarket !== "all") {
    activeWallet = marketWallet || wallets[effectiveMarket] || null;
  } else {
    activeWallet = wallet;
  }

  // Determine which trades to show
  const activeTrades = selectedMarket !== "all" && marketTrades?.trades
    ? marketTrades.trades
    : trades;

  // Count trades by market for tab badges — sourced from API, not WS trades
  const polymarketCount = trades.filter((t) => {
    const m = t.market?.toLowerCase();
    if (m) return m === "polymarket";
    return t.question?.includes("?");
  }).length;
  const spotCount = spotWallet?.total_trades ?? spotWallet?.open_positions ?? 0;
  const perpsCount = perpsWallet?.total_trades ?? perpsWallet?.open_positions ?? 0;
  const cryptoCount = spotCount + perpsCount;

  const marketLabel = selectedMarket === "crypto"
    ? cryptoSubTab === "perps" ? "Perps" : "Spot"
    : MARKET_LABELS[selectedMarket] || undefined;

  // Determine market type for PortfolioPanel
  const marketType: "polymarket" | "spot" | "perps" | undefined =
    selectedMarket === "polymarket" ? "polymarket"
    : selectedMarket === "crypto" ? (cryptoSubTab === "perps" ? "perps" : "spot")
    : undefined;

  return (
    <div className="min-h-screen bg-slate-950">
      {/* Background gradient accent */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute top-0 left-1/4 w-96 h-96 bg-emerald-500/3 rounded-full blur-3xl" />
        <div className="absolute bottom-0 right-1/4 w-96 h-96 bg-blue-500/3 rounded-full blur-3xl" />
      </div>

      <div className="relative max-w-[1600px] mx-auto px-6 py-6">
        {/* Page header with status badges */}
        <div className="flex items-center justify-between mb-5">
          <h1 className="text-2xl font-black text-white tracking-tight">Trading</h1>
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
          cryptoSubTab={cryptoSubTab}
          onCryptoSubTabSelect={setCryptoSubTab}
          spotCount={spotCount}
          perpsCount={perpsCount}
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

              {/* Strategy wallet picker — shown when Polymarket is selected */}
              {isPolymarketSelected && pmStrategies.length > 0 && (
                <div className="mb-4 space-y-3">
                  <div className="flex items-center gap-2 text-[10px] text-slate-500 uppercase tracking-widest font-semibold">
                    <span className="w-1.5 h-1.5 rounded-full bg-blue-400" />
                    {pmStrategies.length} Strategy Wallets
                  </div>
                  <select
                    value={selectedStrategy || ""}
                    onChange={(e) => setSelectedStrategy(e.target.value || null)}
                    className="bg-slate-800 border border-slate-700 text-slate-200 text-xs rounded-lg px-3 py-2 w-full focus:outline-none focus:border-emerald-500/50"
                  >
                    <option value="">All Strategies (aggregated)</option>
                    {pmStrategies.map((s: any) => (
                      <option key={s.name ?? s.id} value={s.name ?? s.id}>
                        {s.name ?? s.id} — ${((s.total_pnl ?? s.pnl ?? 0)).toFixed(2)} ({s.trades ?? 0} trades)
                      </option>
                    ))}
                  </select>

                  {/* PM aggregated summary strip */}
                  {!selectedStrategy && (
                    <div className="grid grid-cols-3 gap-2">
                      <div className="rounded-xl bg-slate-800/40 border border-slate-700/30 p-2.5 text-center">
                        <p className="text-[9px] text-slate-500 uppercase tracking-wider mb-1">Equity</p>
                        <p className="text-xs font-bold text-white">${pmTotalEquity.toFixed(0)}</p>
                      </div>
                      <div className="rounded-xl bg-slate-800/40 border border-slate-700/30 p-2.5 text-center">
                        <p className="text-[9px] text-slate-500 uppercase tracking-wider mb-1">Trades</p>
                        <p className="text-xs font-bold text-white">{pmTotalTrades}</p>
                      </div>
                      <div className="rounded-xl bg-slate-800/40 border border-slate-700/30 p-2.5 text-center">
                        <p className="text-[9px] text-slate-500 uppercase tracking-wider mb-1">PnL</p>
                        <p className={`text-xs font-bold ${pmTotalPnl >= 0 ? "text-emerald-400" : "text-red-400"}`}>
                          {pmTotalPnl >= 0 ? "+" : ""}${pmTotalPnl.toFixed(2)}
                        </p>
                      </div>
                    </div>
                  )}
                </div>
              )}

              <PortfolioPanel
                wallet={activeWallet}
                marketLabel={selectedStrategy ? `${selectedStrategy}` : marketLabel}
                marketType={marketType}
              />
            </div>

            {/* PM top strategies panel — shown when aggregated view active */}
            {isPolymarketSelected && !selectedStrategy && pmWithTrades.length > 0 && (
              <div className="glass-panel rounded-2xl p-6">
                <div className="flex items-center justify-between mb-4">
                  <h2 className="text-[10px] font-bold uppercase tracking-widest text-slate-500 flex items-center gap-2">
                    <span className="w-1 h-4 rounded-full bg-gradient-to-b from-blue-400 to-purple-500" />
                    Top PM Strategies
                  </h2>
                  <Link
                    href="/evolution"
                    className="text-[10px] text-emerald-400 hover:text-emerald-300 transition-colors"
                  >
                    View All →
                  </Link>
                </div>
                <div className="space-y-2">
                  {pmWithTrades.slice(0, 5).map((s: any, i: number) => {
                    const pnl = s.total_pnl ?? s.pnl ?? 0;
                    const pnlPositive = pnl >= 0;
                    return (
                      <button
                        key={s.name ?? s.id}
                        onClick={() => setSelectedStrategy(s.name ?? s.id)}
                        className="w-full flex items-center justify-between p-2.5 rounded-xl bg-slate-800/30 border border-slate-700/30 hover:border-slate-600/50 hover:bg-slate-800/50 transition-all duration-200 text-left"
                      >
                        <div className="flex items-center gap-2 min-w-0">
                          <span className="text-[10px] font-mono text-slate-500 w-4 shrink-0">#{i + 1}</span>
                          <span className="text-xs font-medium text-slate-200 truncate">{s.name ?? s.id}</span>
                        </div>
                        <div className="flex items-center gap-3 shrink-0 ml-2">
                          <span className="text-[10px] text-slate-500">{s.trades ?? 0}t</span>
                          <span className={`text-xs font-bold ${pnlPositive ? "text-emerald-400" : "text-red-400"}`}>
                            {pnlPositive ? "+" : ""}${pnl.toFixed(2)}
                          </span>
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>
            )}

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

export default function TradingPage() {
  return (
    <Suspense>
      <TradingContent />
    </Suspense>
  );
}
