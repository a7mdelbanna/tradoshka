"use client";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

const STRATEGY_COLORS: Record<string, { dot: string; bg: string; border: string }> = {
  "AI Predictor": {
    dot: "bg-emerald-400 text-emerald-400",
    bg: "from-emerald-400/8 to-transparent",
    border: "border-emerald-400/15",
  },
  "Copy Trading": {
    dot: "bg-blue-400 text-blue-400",
    bg: "from-blue-400/8 to-transparent",
    border: "border-blue-400/15",
  },
  "Market Making": {
    dot: "bg-purple-400 text-purple-400",
    bg: "from-purple-400/8 to-transparent",
    border: "border-purple-400/15",
  },
  Arbitrage: {
    dot: "bg-amber-400 text-amber-400",
    bg: "from-amber-400/8 to-transparent",
    border: "border-amber-400/15",
  },
};

const DEFAULT_COLORS = {
  dot: "bg-slate-400 text-slate-400",
  bg: "from-slate-400/8 to-transparent",
  border: "border-slate-400/15",
};

export function StrategyHealth() {
  const { data: strategies } = useQuery({
    queryKey: ["strategies"],
    queryFn: api.strategies,
    refetchInterval: 30000,
  });

  if (!strategies || strategies.length === 0) return null;

  return (
    <div>
      <h3 className="text-xs font-bold uppercase tracking-widest text-slate-400 mb-4">
        Strategies
      </h3>
      <div className="space-y-2.5">
        {strategies.map((s) => {
          const colors = STRATEGY_COLORS[s.name] || DEFAULT_COLORS;
          const isPositive = s.return_pct >= 0;
          const isStrong = s.return_pct > 5;

          return (
            <div
              key={s.name}
              className={`flex items-center justify-between py-3 px-3.5 rounded-xl border bg-gradient-to-r ${colors.bg} ${colors.border} transition-all duration-300 hover:scale-[1.01]`}
            >
              <div className="flex items-center gap-3">
                <span
                  className={`w-2.5 h-2.5 rounded-full ${colors.dot} ${isStrong ? "animate-pulse-glow" : ""}`}
                />
                <div>
                  <span className="text-xs font-semibold text-slate-200">{s.name}</span>
                  <div className="flex gap-2 mt-0.5 text-[10px] text-slate-600">
                    <span>{s.trades} trades</span>
                    <span>{(s.win_rate * 100).toFixed(0)}% win</span>
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <span
                  className={`text-sm font-bold ${
                    isPositive ? "text-emerald-400" : "text-red-400"
                  } ${isPositive && isStrong ? "glow-emerald" : ""}`}
                >
                  {isPositive ? "+" : ""}
                  {s.return_pct.toFixed(1)}%
                </span>
                <div
                  className={`w-3 h-3 rounded-full ${
                    isStrong
                      ? "bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]"
                      : isPositive
                        ? "bg-amber-400 shadow-[0_0_8px_rgba(251,191,36,0.3)]"
                        : "bg-red-400 shadow-[0_0_8px_rgba(248,113,113,0.3)]"
                  } ${isStrong ? "animate-pulse-glow" : ""}`}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
