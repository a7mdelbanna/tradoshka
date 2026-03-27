"use client";

import { CryptoSubTabs } from "@/components/trading/CryptoSubTabs";

interface MarketTabsProps {
  selected: string;
  onSelect: (market: string) => void;
  polymarketCount: number;
  cryptoCount: number;
  memecoinsCount?: number;
  cryptoSubTab?: "spot" | "perps";
  onCryptoSubTabSelect?: (tab: "spot" | "perps") => void;
  spotCount?: number;
  perpsCount?: number;
  mcSubTab?: "all" | "ed" | "tr" | "wc";
  onMcSubTabSelect?: (tab: "all" | "ed" | "tr" | "wc") => void;
  mcEdCount?: number;
  mcTrCount?: number;
  mcWcCount?: number;
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
  memecoins: {
    accent: "text-pink-400",
    glow: "shadow-[0_0_20px_rgba(244,114,182,0.15)]",
    bg: "bg-gradient-to-r from-pink-500/15 to-pink-400/5",
    border: "border-pink-500/30",
    badge: "bg-pink-400/20 text-pink-400",
  },
};

export function MarketTabs({
  selected,
  onSelect,
  polymarketCount,
  cryptoCount,
  memecoinsCount = 0,
  cryptoSubTab = "spot",
  onCryptoSubTabSelect,
  spotCount = 0,
  perpsCount = 0,
  mcSubTab = "all",
  onMcSubTabSelect,
  mcEdCount = 0,
  mcTrCount = 0,
  mcWcCount = 0,
}: MarketTabsProps) {
  const tabs = [
    { id: "all", label: "All Markets", count: polymarketCount + cryptoCount + memecoinsCount, icon: "\u25C9" },
    { id: "polymarket", label: "Polymarket", count: polymarketCount, icon: "\u2B21" },
    { id: "crypto", label: "Crypto", count: cryptoCount, icon: "\u25C8" },
    { id: "memecoins", label: "Meme Coins", count: memecoinsCount, icon: "\uD83D\uDE80" },
  ];

  return (
    <div className="flex items-center gap-2 mb-6 flex-wrap">
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
                      : tab.id === "crypto"
                        ? "bg-amber-400"
                        : "bg-pink-400"
                }`}
              />
            )}
          </button>
        );
      })}
      {selected === "crypto" && onCryptoSubTabSelect && (
        <CryptoSubTabs
          selected={cryptoSubTab}
          onSelect={onCryptoSubTabSelect}
          spotCount={spotCount}
          perpsCount={perpsCount}
        />
      )}
      {selected === "memecoins" && onMcSubTabSelect && (
        <div className="flex items-center gap-1 ml-4">
          <button
            onClick={() => onMcSubTabSelect("all")}
            className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
              mcSubTab === "all"
                ? "bg-pink-400/15 text-pink-300 border border-pink-400/30"
                : "text-slate-500 hover:text-slate-300"
            }`}
          >
            All <span className="ml-1 text-[10px] opacity-60">{mcEdCount + mcTrCount + mcWcCount}</span>
          </button>
          <button
            onClick={() => onMcSubTabSelect("ed")}
            className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
              mcSubTab === "ed"
                ? "bg-rose-400/15 text-rose-300 border border-rose-400/30"
                : "text-slate-500 hover:text-slate-300"
            }`}
          >
            Early Detection <span className="ml-1 text-[10px] opacity-60">{mcEdCount}</span>
          </button>
          <button
            onClick={() => onMcSubTabSelect("tr")}
            className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
              mcSubTab === "tr"
                ? "bg-orange-400/15 text-orange-300 border border-orange-400/30"
                : "text-slate-500 hover:text-slate-300"
            }`}
          >
            Trend Riding <span className="ml-1 text-[10px] opacity-60">{mcTrCount}</span>
          </button>
          <button
            onClick={() => onMcSubTabSelect("wc")}
            className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
              mcSubTab === "wc"
                ? "bg-purple-400/15 text-purple-300 border border-purple-400/30"
                : "text-slate-500 hover:text-slate-300"
            }`}
          >
            Whale Copy <span className="ml-1 text-[10px] opacity-60">{mcWcCount}</span>
          </button>
        </div>
      )}
    </div>
  );
}
