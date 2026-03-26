"use client";

interface WalletData {
  balance: string;
  equity: string;
  unrealized_pnl: string;
  realized_pnl: string;
  drawdown_pct: string;
  total_fees?: string;
  open_positions: number;
}

export function PortfolioPanel({ wallet }: { wallet: WalletData | null }) {
  if (!wallet) return <div className="text-slate-500 text-sm">Connecting...</div>;

  const equity = parseFloat(wallet.equity);
  const unrealized = parseFloat(wallet.unrealized_pnl);
  const realized = parseFloat(wallet.realized_pnl);
  const dd = parseFloat(wallet.drawdown_pct) * 100;

  return (
    <div className="space-y-4">
      <div>
        <p className="text-xs text-slate-500 uppercase tracking-wider mb-1">Equity</p>
        <p className="text-3xl font-bold text-white">${equity.toFixed(2)}</p>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <div className="bg-slate-800/30 rounded-lg p-3">
          <p className="text-[10px] text-slate-500 uppercase">Balance</p>
          <p className="text-sm font-semibold text-white">${parseFloat(wallet.balance).toFixed(2)}</p>
        </div>
        <div className="bg-slate-800/30 rounded-lg p-3">
          <p className="text-[10px] text-slate-500 uppercase">Positions</p>
          <p className="text-sm font-semibold text-white">{wallet.open_positions}</p>
        </div>
        <div className="bg-slate-800/30 rounded-lg p-3">
          <p className="text-[10px] text-slate-500 uppercase">Unrealized</p>
          <p className={`text-sm font-semibold ${unrealized >= 0 ? "text-emerald-400" : "text-red-400"}`}>
            {unrealized >= 0 ? "+" : ""}${unrealized.toFixed(2)}
          </p>
        </div>
        <div className="bg-slate-800/30 rounded-lg p-3">
          <p className="text-[10px] text-slate-500 uppercase">Realized</p>
          <p className={`text-sm font-semibold ${realized >= 0 ? "text-emerald-400" : "text-red-400"}`}>
            {realized >= 0 ? "+" : ""}${realized.toFixed(2)}
          </p>
        </div>
      </div>
      <div className="bg-slate-800/30 rounded-lg p-3">
        <div className="flex justify-between items-center">
          <p className="text-[10px] text-slate-500 uppercase">Drawdown</p>
          <p className={`text-sm font-semibold ${dd > 10 ? "text-red-400" : dd > 5 ? "text-amber-400" : "text-slate-300"}`}>
            -{dd.toFixed(2)}%
          </p>
        </div>
        <div className="h-1.5 bg-slate-700 rounded-full mt-2 overflow-hidden">
          <div className={`h-full rounded-full transition-all ${dd > 10 ? "bg-red-400" : dd > 5 ? "bg-amber-400" : "bg-emerald-400"}`}
            style={{ width: `${Math.min(dd / 20 * 100, 100)}%` }} />
        </div>
      </div>
    </div>
  );
}
