"use client";

interface MarketTabsProps {
  selected: string;
  onSelect: (market: string) => void;
  polymarketCount: number;
  cryptoCount: number;
}

const TAB_CONFIG: Record<string, { accent: string; glow: string; bg: string; border: string; badge: string }> = {
  all: {
    accent: "text-emerald-400",
    glow: "shadow-[0_0_20px_rgba(52,211,153,0.15)]",
    bg: "bg-gradient-to-r from-emerald-500/15 to-emerald-400/5",
    border: "border-emerald-500/30",
    badge: "bg-emerald-400/20 text-emerald-400",
  },
  polymarket: {
    accent: "text-blue-400",
    glow: "shadow-[0_0_20px_rgba(59,130,246,0.15)]",
    bg: "bg-gradient-to-r from-blue-500/15 to-blue-400/5",
    border: "border-blue-500/30",
    badge: "bg-blue-400/20 text-blue-400",
  },
  crypto: {
    accent: "text-amber-400",
    glow: "shadow-[0_0_20px_rgba(251,191,36,0.15)]",
    bg: "bg-gradient-to-r from-amber-500/15 to-amber-400/5",
    border: "border-amber-500/30",
    badge: "bg-amber-400/20 text-amber-400",
  },
};

export function MarketTabs({
  selected,
  onSelect,
  polymarketCount,
  cryptoCount,
}: MarketTabsProps) {
  const tabs = [
    { id: "all", label: "All Markets", count: polymarketCount + cryptoCount, icon: "\u25C9" },
    { id: "polymarket", label: "Polymarket", count: polymarketCount, icon: "\u2B21" },
    { id: "crypto", label: "Crypto", count: cryptoCount, icon: "\u25C8" },
  ];

  return (
    <div className="flex items-center gap-2 mb-6">
      {tabs.map((tab) => {
        const config = TAB_CONFIG[tab.id] || TAB_CONFIG.all;
        const isSelected = selected === tab.id;

        return (
          <button
            key={tab.id}
            onClick={() => onSelect(tab.id)}
            className={`relative px-5 py-2.5 text-xs font-semibold rounded-xl transition-all duration-300 ${
              isSelected
                ? `${config.bg} ${config.accent} border ${config.border} ${config.glow}`
                : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/40 border border-transparent"
            }`}
          >
            <div className="flex items-center gap-2">
              <span className="text-sm">{tab.icon}</span>
              <span>{tab.label}</span>
              {tab.count > 0 && (
                <span
                  className={`px-2 py-0.5 rounded-full text-[10px] font-bold transition-all duration-300 ${
                    isSelected ? config.badge : "bg-slate-700/50 text-slate-500"
                  }`}
                >
                  {tab.count}
                </span>
              )}
            </div>
            {isSelected && (
              <div
                className={`absolute bottom-0 left-1/2 -translate-x-1/2 w-2/3 h-0.5 rounded-full ${
                  tab.id === "all"
                    ? "bg-emerald-400"
                    : tab.id === "polymarket"
                      ? "bg-blue-400"
                      : "bg-amber-400"
                }`}
              />
            )}
          </button>
        );
      })}
    </div>
  );
}
