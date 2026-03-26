interface PositionCardProps {
  symbol: string; side: string; quantity: string; entryPrice: string;
  currentPrice: string; pnl: string; strategy: string;
}

export function PositionCard({ symbol, side, quantity, entryPrice, currentPrice, pnl, strategy }: PositionCardProps) {
  const pnlNum = parseFloat(pnl);
  const pnlColor = pnlNum >= 0 ? "text-emerald-400" : "text-red-400";
  const sideColor = side === "Buy" ? "bg-emerald-400/10 text-emerald-400" : "bg-red-400/10 text-red-400";
  return (
    <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-4 flex items-center justify-between">
      <div>
        <div className="flex items-center gap-2">
          <span className="font-semibold text-sm">{symbol}</span>
          <span className={`text-xs px-2 py-0.5 rounded-full ${sideColor}`}>{side}</span>
        </div>
        <p className="text-xs text-slate-500 mt-1">{strategy} · {quantity} @ {entryPrice}</p>
      </div>
      <div className="text-right">
        <p className={`font-semibold ${pnlColor}`}>${pnl}</p>
        <p className="text-xs text-slate-500">{currentPrice}</p>
      </div>
    </div>
  );
}
