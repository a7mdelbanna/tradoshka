"use client";

interface MarketTabsProps {
  selected: string;
  onSelect: (market: string) => void;
  polymarketCount: number;
  cryptoCount: number;
}

export function MarketTabs({
  selected,
  onSelect,
  polymarketCount,
  cryptoCount,
}: MarketTabsProps) {
  const tabs = [
    { id: "all", label: "All Markets", count: polymarketCount + cryptoCount },
    { id: "polymarket", label: "Polymarket", count: polymarketCount, color: "blue" },
    { id: "crypto", label: "Crypto", count: cryptoCount, color: "amber" },
  ];

  return (
    <div className="flex items-center gap-1 mb-6">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onSelect(tab.id)}
          className={`px-4 py-2 text-xs font-medium rounded-lg transition-all ${
            selected === tab.id
              ? "bg-slate-700/50 text-white border border-slate-600/50"
              : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/30"
          }`}
        >
          {tab.label}
          {tab.count > 0 && (
            <span
              className={`ml-1.5 px-1.5 py-0.5 rounded-full text-[10px] ${
                selected === tab.id
                  ? "bg-emerald-400/20 text-emerald-400"
                  : "bg-slate-700/50 text-slate-500"
              }`}
            >
              {tab.count}
            </span>
          )}
        </button>
      ))}
    </div>
  );
}
