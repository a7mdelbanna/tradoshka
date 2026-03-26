"use client";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function ReadinessScore() {
  const { data } = useQuery({ queryKey: ["readiness"], queryFn: api.readiness, refetchInterval: 10000 });

  if (!data) return null;

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-400">Production Readiness</h3>
        <span className={`text-xs font-bold px-2 py-0.5 rounded-full ${
          data.is_ready
            ? "bg-emerald-400/10 text-emerald-400 border border-emerald-400/20"
            : "bg-amber-400/10 text-amber-400 border border-amber-400/20"
        }`}>
          {data.passed}/{data.total}
        </span>
      </div>
      <div className="space-y-2">
        {data.criteria.map((c) => (
          <div key={c.name} className="flex items-center justify-between text-xs">
            <div className="flex items-center gap-2">
              <span className={`w-4 h-4 rounded flex items-center justify-center text-[10px] ${
                c.passed ? "bg-emerald-400/10 text-emerald-400" : "bg-slate-800 text-slate-500"
              }`}>
                {c.passed ? "\u2713" : "\u25CB"}
              </span>
              <span className="text-slate-300">{c.name}</span>
            </div>
            <div className="flex items-center gap-2">
              <span className={`font-mono ${c.passed ? "text-emerald-400" : "text-slate-500"}`}>{c.current_value}</span>
              <span className="text-slate-600">/ {c.threshold}</span>
            </div>
          </div>
        ))}
      </div>
      {data.is_ready && (
        <div className="mt-4 p-3 rounded-lg bg-emerald-400/5 border border-emerald-400/20 text-center">
          <p className="text-emerald-400 text-sm font-semibold">Strategies Validated — Ready for Live Trading</p>
          <button className="mt-2 px-4 py-1.5 bg-emerald-500 hover:bg-emerald-400 text-slate-950 text-xs font-bold rounded-lg transition-colors">
            Go Live
          </button>
        </div>
      )}
    </div>
  );
}
