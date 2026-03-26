"use client";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function CryptoPriceBar() {
  const { data } = useQuery({
    queryKey: ["crypto-assets"],
    queryFn: api.cryptoAssets,
    refetchInterval: 10000,
  });

  if (!data || data.assets.length === 0) return null;

  return (
    <div className="flex items-center gap-3 overflow-x-auto py-2 px-1 mb-4 premium-scrollbar">
      {data.assets.map((a) => {
        const price = parseFloat(a.price);
        const formatted =
          price >= 1000
            ? `$${price.toLocaleString(undefined, { maximumFractionDigits: 2 })}`
            : price >= 1
              ? `$${price.toFixed(2)}`
              : `$${price.toFixed(4)}`;

        // For now, show as neutral-positive (since we don't have 24h change data)
        const changePositive = true;

        return (
          <div
            key={a.symbol}
            className={`flex items-center gap-2.5 px-4 py-2 rounded-xl border whitespace-nowrap transition-all duration-300 hover:scale-[1.03] hover:-translate-y-0.5 cursor-default ${
              changePositive
                ? "bg-gradient-to-r from-emerald-500/8 to-emerald-400/3 border-emerald-500/15 hover:border-emerald-500/30"
                : "bg-gradient-to-r from-red-500/8 to-red-400/3 border-red-500/15 hover:border-red-500/30"
            }`}
          >
            <span className="text-xs font-black text-white tracking-wide">
              {a.symbol.replace("USDT", "")}
            </span>
            <span className={`text-xs font-mono font-semibold ${changePositive ? "text-emerald-400" : "text-red-400"}`}>
              {formatted}
            </span>
            <span className={`text-[10px] ${changePositive ? "text-emerald-500" : "text-red-500"}`}>
              {changePositive ? "\u25B2" : "\u25BC"}
            </span>
          </div>
        );
      })}
    </div>
  );
}
