"use client";

interface Strategy { name: string; return_pct: number; trades: number; win_rate: number; }

export function StrategyBreakdown({ strategies }: { strategies: Strategy[] }) {
  const maxReturn = Math.max(...strategies.map((s) => Math.abs(s.return_pct)), 1);
  return (
    <div>
      <h3 className="text-sm font-medium text-slate-400 mb-3">Strategy Performance</h3>
      <div className="space-y-3">
        {strategies.map((s) => {
          const pct = Math.abs(s.return_pct) / maxReturn * 100;
          const color = s.return_pct >= 0 ? "bg-emerald-400" : "bg-red-400";
          const tc = s.return_pct >= 0 ? "text-emerald-400" : "text-red-400";
          return (
            <div key={s.name}>
              <div className="flex justify-between text-sm mb-1">
                <span className="text-slate-300">{s.name}</span>
                <span className={tc}>{s.return_pct > 0 ? "+" : ""}{s.return_pct.toFixed(1)}%</span>
              </div>
              <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                <div className={`h-full rounded-full ${color}`} style={{ width: `${pct}%` }} />
              </div>
              <div className="flex gap-3 text-xs text-slate-500 mt-1">
                <span>{s.trades} trades</span>
                <span>{(s.win_rate * 100).toFixed(0)}% win</span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
