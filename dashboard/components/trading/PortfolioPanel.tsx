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

interface PortfolioPanelProps {
  wallet: WalletData | null;
  marketLabel?: string;
}

export function PortfolioPanel({ wallet, marketLabel }: PortfolioPanelProps) {
  if (!wallet) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-slate-500">
        <div className="w-10 h-10 rounded-full border-2 border-slate-700 border-t-emerald-400 animate-spin mb-3" />
        <span className="text-sm">Connecting...</span>
      </div>
    );
  }

  const equity = parseFloat(wallet.equity);
  const unrealized = parseFloat(wallet.unrealized_pnl);
  const realized = parseFloat(wallet.realized_pnl);
  const balance = parseFloat(wallet.balance);
  const dd = parseFloat(wallet.drawdown_pct) * 100;

  const isProfit = unrealized >= 0;
  const ddColor = dd > 10 ? "red" : dd > 5 ? "amber" : "emerald";

  return (
    <div className="space-y-5">
      {/* Header with market label */}
      {marketLabel && (
        <div className="flex items-center gap-2">
          <div className={`w-1.5 h-1.5 rounded-full ${
            marketLabel === "Polymarket" ? "bg-blue-400" : marketLabel === "Crypto" ? "bg-amber-400" : "bg-emerald-400"
          }`} />
          <span className="text-[10px] uppercase tracking-widest text-slate-500 font-semibold">{marketLabel} Wallet</span>
        </div>
      )}

      {/* Giant equity display */}
      <div className="relative">
        <p className="text-[10px] text-slate-500 uppercase tracking-widest mb-2 font-medium">Total Equity</p>
        <p className={`text-5xl font-black tracking-tight text-gradient-emerald ${isProfit ? "glow-emerald" : ""}`}>
          ${equity.toFixed(2)}
        </p>
        <div className={`mt-2 inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold ${
          isProfit
            ? "bg-emerald-400/10 text-emerald-400"
            : "bg-red-400/10 text-red-400"
        }`}>
          <span>{isProfit ? "\u25B2" : "\u25BC"}</span>
          <span>{isProfit ? "+" : ""}${unrealized.toFixed(2)} unrealized</span>
        </div>
      </div>

      {/* Stats grid with glowing borders */}
      <div className="grid grid-cols-2 gap-3">
        <StatBox label="Balance" value={`$${balance.toFixed(2)}`} />
        <StatBox label="Positions" value={String(wallet.open_positions)} />
        <StatBox
          label="Unrealized"
          value={`${unrealized >= 0 ? "+" : ""}$${unrealized.toFixed(2)}`}
          color={unrealized >= 0 ? "emerald" : "red"}
          glow={unrealized >= 0}
        />
        <StatBox
          label="Realized"
          value={`${realized >= 0 ? "+" : ""}$${realized.toFixed(2)}`}
          color={realized >= 0 ? "emerald" : "red"}
          glow={realized >= 0}
        />
      </div>

      {/* Animated drawdown bar */}
      <div className="glass-panel rounded-xl p-4">
        <div className="flex justify-between items-center mb-3">
          <p className="text-[10px] text-slate-400 uppercase tracking-wider font-semibold">Max Drawdown</p>
          <p className={`text-sm font-bold ${
            ddColor === "red" ? "text-red-400 glow-red" : ddColor === "amber" ? "text-amber-400" : "text-slate-300"
          }`}>
            -{dd.toFixed(2)}%
          </p>
        </div>
        <div className="h-2 bg-slate-800/60 rounded-full overflow-hidden">
          <div
            className={`h-full rounded-full transition-all duration-700 ease-out ${
              ddColor === "red"
                ? "bg-gradient-to-r from-red-500 to-red-400"
                : ddColor === "amber"
                  ? "bg-gradient-to-r from-amber-500 to-amber-400"
                  : "bg-gradient-to-r from-emerald-500 to-emerald-400"
            }`}
            style={{ width: `${Math.min(dd / 20 * 100, 100)}%` }}
          />
        </div>
        <div className="flex justify-between mt-1.5 text-[10px] text-slate-600">
          <span>0%</span>
          <span>Safe &lt; 5%</span>
          <span>20%</span>
        </div>
      </div>
    </div>
  );
}

function StatBox({
  label,
  value,
  color,
  glow,
}: {
  label: string;
  value: string;
  color?: "emerald" | "red";
  glow?: boolean;
}) {
  return (
    <div
      className={`relative rounded-xl p-3.5 transition-all duration-300 hover:scale-[1.02] glass-panel ${
        glow && color === "emerald"
          ? "shadow-[0_0_20px_rgba(52,211,153,0.08)]"
          : ""
      }`}
    >
      <p className="text-[10px] text-slate-500 uppercase tracking-wider font-medium mb-1">{label}</p>
      <p
        className={`text-sm font-bold ${
          color === "emerald"
            ? "text-emerald-400"
            : color === "red"
              ? "text-red-400"
              : "text-white"
        }`}
      >
        {value}
      </p>
    </div>
  );
}
