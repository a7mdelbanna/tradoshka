"use client";
import type { TradeInfo } from "@/lib/types";

export function TradeFeed({ trades }: { trades: TradeInfo[] }) {
  if (trades.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500 text-sm">
        <div className="text-center">
          <p className="text-lg mb-1">No trades yet</p>
          <p className="text-xs text-slate-600">Trades will appear here when strategies fire signals</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-2 overflow-y-auto max-h-[600px] pr-1">
      {trades.map((trade, i) => {
        const isBuy = trade.side === "Buy";
        const time = new Date(trade.timestamp).toLocaleTimeString();
        const pnl = trade.pnl ? parseFloat(trade.pnl) : null;

        return (
          <div key={trade.id || i}
            className="bg-slate-800/30 border border-slate-700/30 rounded-lg p-3 hover:border-slate-600/50 transition-colors">
            <div className="flex items-start justify-between mb-1.5">
              <div className="flex items-center gap-2">
                <span className={`text-[10px] font-bold px-1.5 py-0.5 rounded ${
                  isBuy ? "bg-emerald-400/10 text-emerald-400" : "bg-red-400/10 text-red-400"
                }`}>
                  {trade.side.toUpperCase()}
                </span>
                <span className={`text-[10px] font-medium px-1.5 py-0.5 rounded ${
                  trade.direction === "Yes"
                    ? "bg-blue-400/10 text-blue-400"
                    : "bg-purple-400/10 text-purple-400"
                }`}>
                  {trade.direction}
                </span>
              </div>
              <span className="text-[10px] text-slate-600">{time}</span>
            </div>
            <p className="text-xs text-slate-200 mb-2 leading-relaxed line-clamp-2">{trade.question}</p>
            <div className="flex items-center justify-between text-[10px]">
              <div className="flex gap-3 text-slate-500">
                <span>{trade.shares} shares</span>
                <span>@ ${trade.price}</span>
                <span className="text-slate-600">{trade.strategy}</span>
              </div>
              {pnl !== null && (
                <span className={`font-semibold ${pnl >= 0 ? "text-emerald-400" : "text-red-400"}`}>
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
