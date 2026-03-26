export function ModeBadge({ mode }: { mode: string }) {
  const isDry = mode === "Dry";
  return (
    <div className={`flex items-center gap-1.5 px-3 py-1.5 rounded-full border ${
      isDry
        ? "bg-amber-400/10 border-amber-400/20"
        : "bg-emerald-400/10 border-emerald-400/20"
    }`}>
      <span className={`w-1.5 h-1.5 rounded-full ${isDry ? "bg-amber-400" : "bg-emerald-400"} animate-pulse`} />
      <span className={`text-xs font-semibold ${isDry ? "text-amber-400" : "text-emerald-400"}`}>
        {isDry ? "DRY MODE" : "LIVE MODE"}
      </span>
    </div>
  );
}
