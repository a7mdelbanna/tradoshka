"use client";

import { useParams } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import Link from "next/link";

// ─── Market badge helper ───────────────────────────────────────────────────────

function MarketBadge({ market }: { market: string }) {
  const lower = (market ?? "").toLowerCase();
  const cls = lower.includes("spot")
    ? "bg-amber-400/10 text-amber-400 border-amber-400/20"
    : lower.includes("perp")
    ? "bg-purple-400/10 text-purple-400 border-purple-400/20"
    : "bg-blue-400/10 text-blue-400 border-blue-400/20";
  return (
    <span className={`px-2 py-0.5 rounded-full text-xs font-medium border ${cls}`}>
      {market}
    </span>
  );
}

// ─── Stat card ────────────────────────────────────────────────────────────────

function StatCard({
  label,
  value,
  color = "text-white",
}: {
  label: string;
  value: string | number;
  color?: string;
}) {
  return (
    <div className="bg-slate-900/60 border border-slate-800/50 rounded-xl p-4 backdrop-blur-sm">
      <p className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">{label}</p>
      <p className={`text-xl font-bold ${color}`}>{value}</p>
    </div>
  );
}

// ─── Main page ────────────────────────────────────────────────────────────────

export default function StrategyDetailPage() {
  const params = useParams<{ strategy: string }>();
  const name = decodeURIComponent(params.strategy ?? "");

  const { data, isLoading } = useQuery({
    queryKey: ["evolution-wallet", name],
    queryFn: () => api.evolutionWallet(name),
    refetchInterval: 10_000,
  });

  if (isLoading) {
    return (
      <div className="p-8 text-slate-500 text-sm">Loading strategy data…</div>
    );
  }

  if (data?.error) {
    return (
      <div className="p-8 text-red-400 text-sm">
        Strategy not found: <span className="font-mono">{name}</span>
      </div>
    );
  }

  const pnl = parseFloat(data?.pnl ?? "0");
  const equity = parseFloat(data?.equity ?? "100");
  const balance = parseFloat(data?.balance ?? "0");
  const winRate = parseFloat(data?.win_rate ?? "0");

  const positions: any[] = Array.isArray(data?.positions) ? data.positions : [];
  const tradesList: any[] = Array.isArray(data?.trades_list) ? data.trades_list : [];

  return (
    <div className="min-h-screen bg-slate-950">
      {/* Background gradients */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute top-0 left-1/4 w-96 h-96 bg-emerald-500/3 rounded-full blur-3xl" />
        <div className="absolute bottom-0 right-1/3 w-96 h-96 bg-blue-500/3 rounded-full blur-3xl" />
      </div>

      <div className="relative max-w-[1400px] mx-auto px-6 py-6 space-y-6">

        {/* Breadcrumb */}
        <div className="flex items-center gap-2 text-sm text-slate-500">
          <Link
            href="/evolution"
            className="hover:text-slate-300 transition-colors"
          >
            Evolution
          </Link>
          <span>/</span>
          <span className="text-white font-medium">{name}</span>
        </div>

        {/* Header */}
        <div className="flex items-start justify-between flex-wrap gap-4">
          <div>
            <h1 className="text-2xl font-black text-white tracking-tight mb-2">
              {name}
            </h1>
            <div className="flex items-center flex-wrap gap-3 text-sm">
              <MarketBadge market={data?.market ?? ""} />
              <span className="text-slate-500">
                Gen {data?.generation ?? 1}
              </span>
              {data?.parent && (
                <span className="text-slate-600 text-xs">
                  ← {data.parent}
                </span>
              )}
              <span
                className={`px-2 py-0.5 rounded-full text-xs ${
                  data?.status === "Alive"
                    ? "bg-emerald-400/10 text-emerald-400"
                    : "bg-red-400/10 text-red-400"
                }`}
              >
                {data?.status}
              </span>
              {data?.age_hours !== undefined && (
                <span className="text-slate-600 text-xs font-mono">
                  {data.age_hours}h old
                </span>
              )}
            </div>
          </div>
          <Link
            href="/evolution"
            className="text-xs text-slate-500 hover:text-slate-300 transition-colors border border-slate-700/50 hover:border-slate-600/50 rounded-lg px-3 py-2"
          >
            ← Back to leaderboard
          </Link>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4">
          <StatCard label="Equity" value={`$${equity.toFixed(2)}`} />
          <StatCard
            label="P&L"
            value={`${pnl >= 0 ? "+" : ""}$${pnl.toFixed(2)}`}
            color={pnl >= 0 ? "text-emerald-400" : "text-red-400"}
          />
          <StatCard label="Balance" value={`$${balance.toFixed(2)}`} />
          <StatCard
            label="Sharpe"
            value={(data?.sharpe ?? 0).toFixed(2)}
            color={
              (data?.sharpe ?? 0) >= 0 ? "text-emerald-400" : "text-red-400"
            }
          />
          <StatCard label="Win Rate" value={`${winRate}%`} />
          <StatCard label="Trades" value={data?.trades ?? 0} />
        </div>

        {/* Secondary Stats */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <StatCard label="Closed Trades" value={data?.closed_trades ?? 0} />
          <StatCard label="Winning Trades" value={data?.winning_trades ?? 0} />
          <StatCard label="Open Positions" value={data?.open_positions ?? positions.length} />
          <StatCard label="Strategy Type" value={data?.strategy_type ?? "—"} />
        </div>

        {/* Parameters */}
        {data?.params && Object.keys(data.params).length > 0 && (
          <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-5 backdrop-blur-sm">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-3">
              Strategy Parameters
            </h2>
            <div className="flex flex-wrap gap-3">
              {Object.entries(data.params).map(([key, val]) => (
                <div
                  key={key}
                  className="bg-slate-800/50 border border-slate-700/30 rounded-lg px-3 py-2"
                >
                  <span className="text-[10px] text-slate-500 uppercase tracking-wider block mb-0.5">
                    {key}
                  </span>
                  <p className="text-sm font-mono text-white">{String(val)}</p>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Two-column: Positions + Trades */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">

          {/* Open Positions */}
          <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-5 backdrop-blur-sm">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-4">
              Open Positions ({positions.length})
            </h2>
            {positions.length === 0 ? (
              <p className="text-slate-600 text-sm">No open positions</p>
            ) : (
              <div className="space-y-3">
                {positions.map((pos: any, i: number) => {
                  const upnl = parseFloat(pos.unrealized_pnl ?? "0");
                  return (
                    <div
                      key={i}
                      className="bg-slate-800/30 border border-slate-700/30 rounded-lg p-3"
                    >
                      <div className="flex items-center justify-between mb-1.5">
                        <span className="text-sm font-medium text-slate-200 truncate max-w-[60%]" title={pos.question}>
                          {pos.token_id || pos.question}
                        </span>
                        <span
                          className={`text-sm font-semibold ${
                            upnl >= 0 ? "text-emerald-400" : "text-red-400"
                          }`}
                        >
                          {upnl >= 0 ? "+" : ""}${upnl.toFixed(4)}
                        </span>
                      </div>
                      {pos.question && pos.token_id && (
                        <p className="text-[10px] text-slate-500 mb-1.5 line-clamp-1" title={pos.question}>
                          {pos.question}
                        </p>
                      )}
                      <div className="flex flex-wrap gap-3 text-[10px] text-slate-500">
                        <span>
                          <span className={`font-semibold ${pos.side === "Buy" ? "text-emerald-500" : "text-red-500"}`}>{pos.side}</span>{" "}
                          {pos.shares} shares
                        </span>
                        <span>Entry: ${parseFloat(pos.avg_price ?? "0").toFixed(6)}</span>
                        <span>Mark: ${parseFloat(pos.current_price ?? "0").toFixed(6)}</span>
                        {pos.outcome && <span>Outcome: {pos.outcome}</span>}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {/* Recent Trades */}
          <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-5 backdrop-blur-sm">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-4">
              Recent Trades ({data?.trades ?? 0})
            </h2>
            {tradesList.length === 0 ? (
              <p className="text-slate-600 text-sm">No trades recorded</p>
            ) : (
              <div className="space-y-3 max-h-[600px] overflow-y-auto premium-scrollbar pr-1">
                {tradesList.map((t: any, i: number) => {
                  const tradePnl = t.pnl != null ? parseFloat(t.pnl) : null;
                  return (
                    <div
                      key={i}
                      className="bg-slate-800/30 border border-slate-700/30 rounded-lg p-3"
                    >
                      <div className="flex items-center justify-between mb-1">
                        <div className="flex items-center gap-2 min-w-0">
                          <span
                            className={`text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0 ${
                              t.side === "Buy"
                                ? "bg-emerald-400/10 text-emerald-400"
                                : "bg-red-400/10 text-red-400"
                            }`}
                          >
                            {t.side}
                          </span>
                          <span className="text-xs text-slate-300 truncate">
                            {t.symbol || t.question}
                          </span>
                        </div>
                        <div className="flex items-center gap-2 shrink-0 ml-2">
                          {tradePnl != null && (
                            <span
                              className={`text-xs font-semibold ${
                                tradePnl >= 0 ? "text-emerald-400" : "text-red-400"
                              }`}
                            >
                              {tradePnl >= 0 ? "+" : ""}${tradePnl.toFixed(4)}
                            </span>
                          )}
                          {t.closed && (
                            <span className="text-[9px] text-slate-600 bg-slate-800 px-1 py-0.5 rounded">
                              CLOSED
                            </span>
                          )}
                          <span className="text-[10px] text-slate-600 font-mono">
                            {new Date(t.timestamp).toLocaleTimeString()}
                          </span>
                        </div>
                      </div>

                      {t.thesis_reasoning && (
                        <p className="text-[10px] text-slate-500 italic mt-1 line-clamp-2">
                          {t.thesis_reasoning}
                        </p>
                      )}

                      <div className="flex flex-wrap gap-3 text-[10px] text-slate-600 mt-1.5">
                        <span>
                          {t.shares} @ ${parseFloat(t.price ?? "0").toFixed(4)}
                        </span>
                        {t.stop_loss && parseFloat(t.stop_loss) > 0 && (
                          <span>SL: ${parseFloat(t.stop_loss).toFixed(4)}</span>
                        )}
                        {t.take_profit && parseFloat(t.take_profit) > 0 && (
                          <span>TP: ${parseFloat(t.take_profit).toFixed(4)}</span>
                        )}
                        {t.strategy_tier && (
                          <span className="text-slate-700">{t.strategy_tier}</span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
