"use client";
import type { TradeInfo } from "@/lib/types";

function inferMarket(trade: TradeInfo): string {
  if (trade.market) return trade.market.toLowerCase();
  if (trade.question && trade.question.includes("?")) return "polymarket";
  if (trade.symbol && trade.symbol.endsWith("USDT")) return "crypto";
  return "unknown";
}

function MarketBadge({ market }: { market: string }) {
  if (market === "polymarket") {
    return (
      <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-blue-400/10 text-blue-400 border border-blue-400/20">
        Polymarket
      </span>
    );
  }
  if (market === "crypto") {
    return (
      <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-amber-400/10 text-amber-400 border border-amber-400/20">
        Crypto
      </span>
    );
  }
  return (
    <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-slate-400/10 text-slate-400 border border-slate-400/20">
      {market}
    </span>
  );
}

interface TradeFeedProps {
  trades: TradeInfo[];
  marketFilter?: string;
}

export function TradeFeed({ trades, marketFilter }: TradeFeedProps) {
  const filtered =
    marketFilter && marketFilter !== "all"
      ? trades.filter((t) => inferMarket(t) === marketFilter)
      : trades;

  if (filtered.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500">
        <div className="text-center py-16">
          <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-slate-800/50 flex items-center justify-center">
            <span className="text-2xl opacity-30">{"\u{1F4CA}"}</span>
          </div>
          <p className="text-base font-semibold text-slate-400 mb-1">No trades yet</p>
          <p className="text-xs text-slate-600">
            Trades will appear here when strategies fire signals
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-2.5 overflow-y-auto max-h-[600px] pr-1 premium-scrollbar">
      {filtered.map((trade, i) => {
        const isBuy = trade.side === "Buy";
        const time = new Date(trade.timestamp).toLocaleTimeString();
        const pnl = trade.pnl ? parseFloat(trade.pnl) : null;
        const market = inferMarket(trade);

        return (
          <div
            key={trade.id || i}
            className="animate-fade-in-right relative flex glass-panel rounded-xl overflow-hidden hover:-translate-y-0.5 hover:shadow-lg transition-all duration-300 group"
            style={{ animationDelay: `${i * 50}ms` }}
          >
            {/* Left accent bar */}
            <div
              className={`w-1 shrink-0 ${
                isBuy
                  ? "bg-gradient-to-b from-emerald-400 to-emerald-600"
                  : "bg-gradient-to-b from-red-400 to-red-600"
              }`}
            />

            <div className="flex-1 p-3.5">
              <div className="flex items-start justify-between mb-2">
                <div className="flex items-center gap-2">
                  <span
                    className={`text-[10px] font-black px-2 py-0.5 rounded-md ${
                      isBuy
                        ? "bg-emerald-400/15 text-emerald-400 border border-emerald-400/20"
                        : "bg-red-400/15 text-red-400 border border-red-400/20"
                    }`}
                  >
                    {trade.side.toUpperCase()}
                  </span>
                  <span
                    className={`text-[10px] font-semibold px-2 py-0.5 rounded-md ${
                      trade.direction === "Yes"
                        ? "bg-blue-400/10 text-blue-400"
                        : "bg-purple-400/10 text-purple-400"
                    }`}
                  >
                    {trade.direction}
                  </span>
                  {/* Strategy pill */}
                  {trade.strategy && (
                    <span className="text-[10px] font-medium px-2 py-0.5 rounded-md bg-slate-700/40 text-slate-400 border border-slate-600/30">
                      {trade.strategy}
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-2.5">
                  <MarketBadge market={market} />
                  <span className="text-[10px] text-slate-600 font-mono">{time}</span>
                </div>
              </div>

              <p className="text-xs text-slate-200 mb-2.5 leading-relaxed line-clamp-2 group-hover:text-white transition-colors duration-300">
                {trade.question || trade.symbol}
              </p>

              <div className="flex items-center justify-between">
                <div className="flex gap-3 text-[10px] text-slate-500">
                  <span className="font-mono">{trade.shares} shares</span>
                  <span className="font-mono">@ ${trade.price}</span>
                </div>
                {pnl !== null && (
                  <span
                    className={`text-xs font-bold px-2 py-0.5 rounded-md ${
                      pnl >= 0
                        ? "text-emerald-400 bg-emerald-400/10 glow-emerald"
                        : "text-red-400 bg-red-400/10 glow-red"
                    }`}
                  >
                    {pnl >= 0 ? "+" : ""}${pnl.toFixed(2)}
                  </span>
                )}
              </div>

              {trade.edge > 0 && (
                <div className="mt-2 flex gap-3 text-[10px]">
                  <span className="text-slate-500">
                    Edge: <span className="text-emerald-400/70 font-mono">{(trade.edge * 100).toFixed(1)}%</span>
                  </span>
                  <span className="text-slate-500">
                    Strength: <span className="text-blue-400/70 font-mono">{(trade.strength * 100).toFixed(0)}%</span>
                  </span>
                </div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
