"use client";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

function CircularScore({ passed, total }: { passed: number; total: number }) {
  const pct = total > 0 ? (passed / total) * 100 : 0;
  const radius = 32;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (pct / 100) * circumference;
  const isReady = passed === total;

  return (
    <div className="relative w-20 h-20 flex items-center justify-center">
      <svg width="80" height="80" className="circular-progress">
        {/* Background circle */}
        <circle
          cx="40"
          cy="40"
          r={radius}
          fill="none"
          stroke="rgba(51, 65, 85, 0.4)"
          strokeWidth="4"
        />
        {/* Progress circle */}
        <circle
          cx="40"
          cy="40"
          r={radius}
          fill="none"
          stroke={isReady ? "#34d399" : "#fbbf24"}
          strokeWidth="4"
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          className="transition-all duration-700 ease-out"
          style={{
            filter: isReady
              ? "drop-shadow(0 0 6px rgba(52,211,153,0.5))"
              : "drop-shadow(0 0 6px rgba(251,191,36,0.3))",
          }}
        />
      </svg>
      <div className="absolute inset-0 flex items-center justify-center">
        <span className={`text-lg font-black ${isReady ? "text-emerald-400" : "text-amber-400"}`}>
          {passed}/{total}
        </span>
      </div>
    </div>
  );
}

export function ReadinessScore() {
  const { data } = useQuery({
    queryKey: ["readiness"],
    queryFn: api.readiness,
    refetchInterval: 10000,
  });

  if (!data) return null;

  return (
    <div>
      <div className="flex items-center justify-between mb-5">
        <h3 className="text-xs font-bold uppercase tracking-widest text-slate-400">
          Production Readiness
        </h3>
        <CircularScore passed={data.passed} total={data.total} />
      </div>

      <div className="space-y-2">
        {data.criteria.map((c) => (
          <div
            key={c.name}
            className={`flex items-center justify-between text-xs py-2.5 px-3 rounded-lg transition-all duration-300 ${
              c.passed
                ? "bg-emerald-400/5 hover:bg-emerald-400/10"
                : "bg-slate-800/20 opacity-60 hover:opacity-80"
            }`}
          >
            <div className="flex items-center gap-2.5">
              <span
                className={`w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold transition-all duration-300 ${
                  c.passed
                    ? "bg-emerald-400/20 text-emerald-400 shadow-[0_0_10px_rgba(52,211,153,0.3)]"
                    : "bg-slate-800 text-slate-600"
                }`}
              >
                {c.passed ? "\u2713" : "\u2715"}
              </span>
              <span className={`font-medium ${c.passed ? "text-slate-200" : "text-slate-500"}`}>
                {c.name}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <span
                className={`font-mono font-semibold ${
                  c.passed ? "text-emerald-400" : "text-slate-600"
                }`}
              >
                {c.current_value}
              </span>
              <span className="text-slate-700">/</span>
              <span className="text-slate-600 font-mono">{c.threshold}</span>
            </div>
          </div>
        ))}
      </div>

      {data.is_ready && (
        <div className="mt-5 p-4 rounded-xl bg-gradient-to-r from-emerald-400/8 to-emerald-400/3 border border-emerald-400/20 text-center shadow-[0_0_30px_rgba(52,211,153,0.08)]">
          <p className="text-emerald-400 text-sm font-bold mb-2.5">
            {"\u2728"} Strategies Validated
          </p>
          <button className="px-6 py-2 bg-gradient-to-r from-emerald-500 to-emerald-400 hover:from-emerald-400 hover:to-emerald-300 text-slate-950 text-xs font-black rounded-lg transition-all duration-300 hover:shadow-[0_0_20px_rgba(52,211,153,0.3)] hover:-translate-y-0.5">
            Go Live
          </button>
        </div>
      )}
    </div>
  );
}
