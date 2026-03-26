"use client";
import type { TradeInfo } from "@/lib/types";

function inferMarket(trade: TradeInfo): string {
  if (trade.market) return trade.market.toLowerCase();
  // Fallback heuristic: if the question looks like a prediction market question
  if (trade.question && trade.question.includes("?")) return "polymarket";
  // If symbol ends with USDT, likely crypto
  if (trade.symbol && trade.symbol.endsWith("USDT")) return "crypto";
  return "unknown";
}

function MarketBadge({ market }: { market: string }) {
  if (market === "polymarket") {
    return (
      <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-blue-400/10 text-blue-400 border border-blue-400/20">
        Polymarket
      </span>
    );
  }
  if (market === "crypto") {
    return (
      <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-amber-400/10 text-amber-400 border border-amber-400/20">
        Crypto
      </span>
    );
  }
  return (
    <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-slate-400/10 text-slate-400 border border-slate-400/20">
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
      <div className="flex items-center justify-center h-full text-slate-500 text-sm">
        <div className="text-center">
          <p className="text-lg mb-1">No trades yet</p>
          <p className="text-xs text-slate-600">
            Trades will appear here when strategies fire signals
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-2 overflow-y-auto max-h-[600px] pr-1">
      {filtered.map((trade, i) => {
        const isBuy = trade.side === "Buy";
        const time = new Date(trade.timestamp).toLocaleTimeString();
        const pnl = trade.pnl ? parseFloat(trade.pnl) : null;
        const market = inferMarket(trade);

        return (
          <div
            key={trade.id || i}
            className="bg-slate-800/30 border border-slate-700/30 rounded-lg p-3 hover:border-slate-600/50 transition-colors"
          >
            <div className="flex items-start justify-between mb-1.5">
              <div className="flex items-center gap-2">
                <span
                  className={`text-[10px] font-bold px-1.5 py-0.5 rounded ${
                    isBuy
                      ? "bg-emerald-400/10 text-emerald-400"
                      : "bg-red-400/10 text-red-400"
                  }`}
                >
                  {trade.side.toUpperCase()}
                </span>
                <span
                  className={`text-[10px] font-medium px-1.5 py-0.5 rounded ${
                    trade.direction === "Yes"
                      ? "bg-blue-400/10 text-blue-400"
                      : "bg-purple-400/10 text-purple-400"
                  }`}
                >
                  {trade.direction}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <MarketBadge market={market} />
                <span className="text-[10px] text-slate-600">{time}</span>
              </div>
            </div>
            <p className="text-xs text-slate-200 mb-2 leading-relaxed line-clamp-2">
              {trade.question}
            </p>
            <div className="flex items-center justify-between text-[10px]">
              <div className="flex gap-3 text-slate-500">
                <span>{trade.shares} shares</span>
                <span>@ ${trade.price}</span>
                <span className="text-slate-600">{trade.strategy}</span>
              </div>
              {pnl !== null && (
                <span
                  className={`font-semibold ${pnl >= 0 ? "text-emerald-400" : "text-red-400"}`}
                >
                  {pnl >= 0 ? "+" : ""}${pnl.toFixed(2)}
                </span>
              )}
            </div>
            {trade.edge > 0 && (
              <div className="mt-1.5 flex gap-2 text-[10px] text-slate-600">
                <span>Edge: {(trade.edge * 100).toFixed(1)}%</span>
                <span>Strength: {(trade.strength * 100).toFixed(0)}%</span>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
