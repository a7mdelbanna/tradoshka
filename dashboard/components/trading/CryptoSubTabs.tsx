"use client";

interface CryptoSubTabsProps {
  selected: "spot" | "perps";
  onSelect: (tab: "spot" | "perps") => void;
  spotCount: number;
  perpsCount: number;
}

export function CryptoSubTabs({ selected, onSelect, spotCount, perpsCount }: CryptoSubTabsProps) {
  return (
    <div className="flex items-center gap-1 ml-4">
      <button onClick={() => onSelect("spot")}
        className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
          selected === "spot"
            ? "bg-amber-400/15 text-amber-300 border border-amber-400/30"
            : "text-slate-500 hover:text-slate-300"
        }`}>
        Spot <span className="ml-1 text-[10px] opacity-60">{spotCount}</span>
      </button>
      <button onClick={() => onSelect("perps")}
        className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
          selected === "perps"
            ? "bg-purple-400/15 text-purple-300 border border-purple-400/30"
            : "text-slate-500 hover:text-slate-300"
        }`}>
        Perpetuals <span className="ml-1 text-[10px] opacity-60">{perpsCount}</span>
      </button>
    </div>
  );
}
