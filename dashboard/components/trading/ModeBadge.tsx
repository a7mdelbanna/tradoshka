"use client";

export function ModeBadge({ mode }: { mode: string }) {
  const isDry = mode === "Dry";
  return (
    <div
      className={`relative flex items-center gap-2 px-4 py-1.5 rounded-full border transition-all duration-300 ${
        isDry
          ? "bg-amber-400/10 border-amber-400/20 hover:border-amber-400/40 shadow-[0_0_15px_rgba(251,191,36,0.08)]"
          : "bg-emerald-400/10 border-emerald-400/20 hover:border-emerald-400/40 shadow-[0_0_15px_rgba(52,211,153,0.08)]"
      }`}
    >
      <span
        className={`w-2 h-2 rounded-full animate-pulse ${
          isDry ? "bg-amber-400 shadow-[0_0_8px_rgba(251,191,36,0.6)]" : "bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.6)]"
        }`}
      />
      <span
        className={`text-xs font-bold tracking-wider ${
          isDry ? "text-amber-400" : "text-emerald-400"
        }`}
      >
        {isDry ? "DRY MODE" : "LIVE MODE"}
      </span>
    </div>
  );
}
