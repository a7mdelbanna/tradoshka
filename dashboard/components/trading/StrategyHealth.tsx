"use client";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

const STRATEGY_COLORS: Record<string, string> = {
  "AI Predictor": "emerald",
  "Copy Trading": "blue",
  "Market Making": "purple",
  "Arbitrage": "amber",
};

export function StrategyHealth() {
  const { data: strategies } = useQuery({ queryKey: ["strategies"], queryFn: api.strategies, refetchInterval: 30000 });

  if (!strategies || strategies.length === 0) return null;

  return (
    <div>
      <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-3">Strategies</h3>
      <div className="space-y-2">
        {strategies.map((s) => {
          const color = STRATEGY_COLORS[s.name] || "slate";
          const isPositive = s.return_pct >= 0;
          const health = s.return_pct > 5 ? "emerald" : s.return_pct >= 0 ? "amber" : "red";
          return (
            <div key={s.name} className="flex items-center justify-between py-1.5">
              <div className="flex items-center gap-2">
                <span className={`w-1.5 h-1.5 rounded-full bg-${color}-400`} />
                <span className="text-xs text-slate-300">{s.name}</span>
              </div>
              <div className="flex items-center gap-2">
                <span className={`text-xs font-semibold ${isPositive ? "text-emerald-400" : "text-red-400"}`}>
                  {isPositive ? "+" : ""}{s.return_pct.toFixed(1)}%
                </span>
                <span className={`w-2 h-2 rounded-full bg-${health}-400`} />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
