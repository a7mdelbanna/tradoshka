"use client";

import type { PerpPositionInfo } from "@/lib/types";

interface WalletData {
  balance: string;
  equity: string;
  unrealized_pnl: string;
  realized_pnl: string;
  drawdown_pct: string;
  total_fees?: string;
  open_positions: number;
  // perps-specific optional fields
  used_margin?: string;
  available_margin?: string;
  default_leverage?: number;
  total_funding?: string;
  positions?: PerpPositionInfo[] | object[];
}

interface PortfolioPanelProps {
  wallet: WalletData | null;
  marketLabel?: string;
  marketType?: "polymarket" | "spot" | "perps";
}

export function PortfolioPanel({ wallet, marketLabel, marketType }: PortfolioPanelProps) {
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

  const isPerps = marketType === "perps";
  const usedMargin = wallet.used_margin ? parseFloat(wallet.used_margin) : null;
  const availableMargin = wallet.available_margin ? parseFloat(wallet.available_margin) : null;
  const defaultLeverage = wallet.default_leverage ?? null;
  const totalFunding = wallet.total_funding ? parseFloat(wallet.total_funding) : null;

  const perpPositions: PerpPositionInfo[] = isPerps && Array.isArray(wallet.positions)
    ? (wallet.positions as PerpPositionInfo[])
    : [];

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
        <div className="flex items-center gap-3 flex-wrap">
          <p className={`text-5xl font-black tracking-tight text-gradient-emerald ${isProfit ? "glow-emerald" : ""}`}>
            ${equity.toFixed(2)}
          </p>
          {isPerps && defaultLeverage != null && (
            <span className="px-2.5 py-1 text-[10px] font-black tracking-wider rounded-lg bg-purple-400/15 text-purple-300 border border-purple-400/30 uppercase">
              {defaultLeverage}x LEVERAGE
            </span>
          )}
        </div>
        <div className={`mt-2 inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold ${
          isProfit
            ? "bg-emerald-400/10 text-emerald-400"
            : "bg-red-400/10 text-red-400"
        }`}>
          <span>{isProfit ? "▲" : "▼"}</span>
          <span>{isProfit ? "+" : ""}${unrealized.toFixed(2)} unrealized</span>
        </div>
      </div>

      {/* Stats grid */}
      <div className="grid grid-cols-2 gap-3">
        {isPerps && usedMargin != null ? (
          <>
            <StatBox label="Used Margin" value={`$${usedMargin.toFixed(2)}`} />
            <StatBox label="Avail. Margin" value={`$${availableMargin != null ? availableMargin.toFixed(2) : "—"}`} />
          </>
        ) : (
          <>
            <StatBox label="Balance" value={`$${balance.toFixed(2)}`} />
            <StatBox label="Positions" value={String(wallet.open_positions)} />
          </>
        )}
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
        {isPerps && (
          <>
            <StatBox label="Positions" value={String(wallet.open_positions)} />
            {totalFunding != null && (
              <StatBox
                label="Funding"
                value={`${totalFunding >= 0 ? "+" : ""}$${totalFunding.toFixed(4)}`}
                color={totalFunding >= 0 ? "emerald" : "red"}
              />
            )}
          </>
        )}
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

      {/* Perps position cards */}
      {isPerps && perpPositions.length > 0 && (
        <div className="space-y-2">
          <p className="text-[10px] text-slate-500 uppercase tracking-widest font-semibold">Open Positions</p>
          {perpPositions.map((pos, i) => (
            <PerpPositionCard key={i} position={pos} />
          ))}
        </div>
      )}
    </div>
  );
}

function PerpPositionCard({ position }: { position: PerpPositionInfo }) {
  const roe = parseFloat(position.roe_pct);
  const funding = parseFloat(position.funding);
  const isLong = position.side?.toUpperCase() === "LONG";
  const roePositive = roe >= 0;

  return (
    <div className="glass-panel rounded-xl p-3.5 space-y-2">
      {/* Title row */}
      <div className="flex items-center justify-between gap-2 flex-wrap">
        <div className="flex items-center gap-2">
          <span className="text-xs font-bold text-white">{position.symbol}</span>
          <span className={`text-[10px] font-bold px-1.5 py-0.5 rounded ${
            isLong ? "bg-emerald-400/15 text-emerald-400" : "bg-red-400/15 text-red-400"
          }`}>
            {position.side?.toUpperCase()} {position.leverage}x
          </span>
          {position.timeframe && (
            <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-slate-700/60 text-slate-400 border border-slate-600/40">
              {position.timeframe}
            </span>
          )}
        </div>
        <span className={`text-[10px] font-bold ${roePositive ? "text-emerald-400" : "text-red-400"}`}>
          ROE {roePositive ? "+" : ""}{roe.toFixed(2)}%
        </span>
      </div>
      {/* Details row */}
      <div className="grid grid-cols-3 gap-x-3 gap-y-1 text-[10px] text-slate-400">
        <span>Size <span className="text-slate-200">{position.size}</span></span>
        <span>Entry <span className="text-slate-200">${parseFloat(position.entry_price).toLocaleString()}</span></span>
        <span>Mark <span className="text-slate-200">${parseFloat(position.mark_price).toLocaleString()}</span></span>
        <span>Liq <span className="text-red-400">${parseFloat(position.liquidation_price).toLocaleString()}</span></span>
        <span>Funding <span className={funding >= 0 ? "text-emerald-400" : "text-red-400"}>{funding >= 0 ? "+" : ""}${funding.toFixed(4)}</span></span>
        {position.strategy && (
          <span className="text-slate-500 truncate">{position.strategy}</span>
        )}
      </div>

      {/* Stop management note */}
      <div className="mt-2 flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-slate-700/30 border border-slate-600/20">
        <span className="text-[9px] text-slate-500">
          🛡 Stops managed by position monitor — see trade cards for levels
        </span>
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
