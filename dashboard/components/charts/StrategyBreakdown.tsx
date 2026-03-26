"use client";

interface Strategy { name: string; return_pct: number; trades: number; win_rate: number; }

const STRATEGY_COLORS: Record<string, { bar: string; text: string; bg: string }> = {
  "AI Predictor": { bar: "bg-emerald-400", text: "text-emerald-400", bg: "bg-emerald-400/10" },
  "Copy Trading": { bar: "bg-blue-400", text: "text-blue-400", bg: "bg-blue-400/10" },
  "Market Making": { bar: "bg-purple-400", text: "text-purple-400", bg: "bg-purple-400/10" },
  "Arbitrage": { bar: "bg-amber-400", text: "text-amber-400", bg: "bg-amber-400/10" },
};

const DEFAULT_COLOR = { bar: "bg-slate-400", text: "text-slate-400", bg: "bg-slate-400/10" };

export function StrategyBreakdown({ strategies }: { strategies: Strategy[] }) {
  const maxReturn = Math.max(...strategies.map((s) => Math.abs(s.return_pct)), 1);

  return (
    <div>
      <div className="mb-4">
        <h3 className="text-sm font-semibold text-white">Strategy Performance</h3>
        <p className="text-xs text-slate-500 mt-0.5">Returns by strategy</p>
      </div>
      <div className="space-y-4">
        {strategies.map((s) => {
          const pct = Math.abs(s.return_pct) / maxReturn * 100;
          const colors = STRATEGY_COLORS[s.name] || DEFAULT_COLOR;
          return (
            <div key={s.name} className="group">
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <div className={`w-2 h-2 rounded-full ${colors.bar}`} />
                  <span className="text-sm font-medium text-slate-200">{s.name}</span>
                </div>
                <span className={`text-sm font-bold ${colors.text}`}>
                  {s.return_pct > 0 ? "+" : ""}{s.return_pct.toFixed(1)}%
                </span>
              </div>
              <div className="h-2 bg-slate-800/80 rounded-full overflow-hidden">
                <div className={`h-full rounded-full ${colors.bar} transition-all duration-700 ease-out`}
                  style={{ width: `${pct}%` }} />
              </div>
              <div className="flex gap-4 mt-1.5">
                <span className="text-[11px] text-slate-500">{s.trades} trades</span>
                <span className="text-[11px] text-slate-500">{(s.win_rate * 100).toFixed(0)}% win rate</span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
