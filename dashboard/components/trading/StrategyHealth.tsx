"use client";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

/* ─── Tier config ─────────────────────────────────────────────────── */
/** Thresholds (cumulative trades) to advance tier */
const TIER_THRESHOLDS = { Unproven: 20, Tested: 60, Proven: Infinity } as const;
type Tier = keyof typeof TIER_THRESHOLDS;

const TIER_META: Record<Tier, { emoji: string; label: string; next?: Tier; nextLabel?: string }> = {
  Unproven: { emoji: "🔵", label: "Unproven",  next: "Tested",  nextLabel: "Tested" },
  Tested:   { emoji: "🟡", label: "Tested",    next: "Proven",  nextLabel: "Proven"  },
  Proven:   { emoji: "🟢", label: "Proven" },
};

const TIER_COLORS: Record<Tier, { dot: string; bg: string; border: string; badge: string }> = {
  Unproven: {
    dot:    "bg-blue-400",
    bg:     "from-blue-400/8 to-transparent",
    border: "border-blue-400/15",
    badge:  "bg-blue-400/10 text-blue-400 border-blue-400/20",
  },
  Tested: {
    dot:    "bg-amber-400",
    bg:     "from-amber-400/8 to-transparent",
    border: "border-amber-400/15",
    badge:  "bg-amber-400/10 text-amber-400 border-amber-400/20",
  },
  Proven: {
    dot:    "bg-emerald-400",
    bg:     "from-emerald-400/8 to-transparent",
    border: "border-emerald-400/15",
    badge:  "bg-emerald-400/10 text-emerald-400 border-emerald-400/20",
  },
};

const STRATEGY_COLORS: Record<string, { dot: string; bg: string; border: string; badge: string }> = {
  "AI Predictor": {
    dot: "bg-emerald-400",
    bg: "from-emerald-400/8 to-transparent",
    border: "border-emerald-400/15",
    badge: "bg-emerald-400/10 text-emerald-400 border-emerald-400/20",
  },
  "Copy Trading": {
    dot: "bg-blue-400",
    bg: "from-blue-400/8 to-transparent",
    border: "border-blue-400/15",
    badge: "bg-blue-400/10 text-blue-400 border-blue-400/20",
  },
  "Market Making": {
    dot: "bg-purple-400",
    bg: "from-purple-400/8 to-transparent",
    border: "border-purple-400/15",
    badge: "bg-purple-400/10 text-purple-400 border-purple-400/20",
  },
  Arbitrage: {
    dot: "bg-amber-400",
    bg: "from-amber-400/8 to-transparent",
    border: "border-amber-400/15",
    badge: "bg-amber-400/10 text-amber-400 border-amber-400/20",
  },
};

const DEFAULT_COLORS = {
  dot: "bg-slate-400",
  bg: "from-slate-400/8 to-transparent",
  border: "border-slate-400/15",
  badge: "bg-slate-400/10 text-slate-400 border-slate-400/20",
};

/** Derive a static tier from trade count until the API exposes it */
function inferTier(trades: number): Tier {
  if (trades < 20) return "Unproven";
  if (trades < 60) return "Tested";
  return "Proven";
}

/* ─── Progress bar ────────────────────────────────────────────────── */
function TierProgress({ trades, tier }: { trades: number; tier: Tier }) {
  const meta = TIER_META[tier];
  if (!meta.next) {
    return (
      <p className="text-[9px] text-emerald-400 font-semibold">Fully proven strategy</p>
    );
  }

  const start = tier === "Unproven" ? 0 : tier === "Tested" ? 20 : 60;
  const end   = TIER_THRESHOLDS[tier];
  const done  = trades - start;
  const total = end - start;
  const pct   = Math.min(Math.max((done / total) * 100, 0), 100);
  const remaining = Math.max(total - done, 0);

  const barColor =
    tier === "Unproven" ? "bg-blue-400" :
    tier === "Tested"   ? "bg-amber-400" : "bg-emerald-400";

  return (
    <div className="space-y-1">
      <div className="h-1.5 bg-slate-800/60 rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-700 ease-out ${barColor}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <p className="text-[9px] text-slate-600">
        {remaining > 0
          ? `${remaining} more trade${remaining !== 1 ? "s" : ""} to ${meta.nextLabel}`
          : `Ready to advance to ${meta.nextLabel}`}
      </p>
    </div>
  );
}

/* ─── Main component ──────────────────────────────────────────────── */
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
          const tier      = inferTier(s.trades);
          const tierMeta  = TIER_META[tier];
          const tierColor = TIER_COLORS[tier];
          const baseColor = STRATEGY_COLORS[s.name] ?? DEFAULT_COLORS;

          const isPositive = s.return_pct >= 0;
          const isStrong   = s.return_pct > 5;

          /* tier index 1-based for display (Unproven=1, Tested=2, Proven=3) */
          const tierIndex = tier === "Unproven" ? 1 : tier === "Tested" ? 2 : 3;

          return (
            <div
              key={s.name}
              className={`py-3 px-3.5 rounded-xl border bg-gradient-to-r ${baseColor.bg} ${baseColor.border} transition-all duration-300 hover:scale-[1.01] space-y-2.5`}
            >
              {/* Row 1: dot · name · tier badge · return · health dot */}
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2.5">
                  <span
                    className={`w-2.5 h-2.5 rounded-full shrink-0 ${baseColor.dot} ${isStrong ? "animate-pulse-glow" : ""}`}
                  />
                  <div>
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-xs font-semibold text-slate-200">{s.name}</span>
                      <span className={`text-[9px] font-bold px-1.5 py-0.5 rounded border ${tierColor.badge}`}>
                        {tierMeta.emoji} {tierMeta.label}
                      </span>
                    </div>
                    <div className="flex gap-2 mt-0.5 text-[10px] text-slate-600">
                      <span>{s.trades} trades</span>
                      <span>{(s.win_rate * 100).toFixed(0)}% win</span>
                      <span>Tier {tierIndex}/3</span>
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2.5 shrink-0">
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

              {/* Row 2: tier progress bar */}
              <TierProgress trades={s.trades} tier={tier} />

              {/* Row 3: static risk params */}
              <div className="flex gap-3 text-[9px] text-slate-600 border-t border-slate-700/30 pt-1.5">
                <span>Risk: <span className="text-slate-400">1%/trade</span></span>
                <span>Max: <span className="text-slate-400">3 positions</span></span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
