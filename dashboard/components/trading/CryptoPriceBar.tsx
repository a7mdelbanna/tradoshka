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
    <div className="flex items-center gap-4 overflow-x-auto py-2 px-1 mb-4">
      {data.assets.map((a) => {
        const price = parseFloat(a.price);
        const formatted =
          price >= 1000
            ? `$${price.toLocaleString(undefined, { maximumFractionDigits: 2 })}`
            : price >= 1
              ? `$${price.toFixed(2)}`
              : `$${price.toFixed(4)}`;
        return (
          <div
            key={a.symbol}
            className="flex items-center gap-2 px-3 py-1.5 bg-slate-800/30 rounded-lg border border-slate-700/30 whitespace-nowrap"
          >
            <span className="text-xs font-semibold text-slate-200">
              {a.symbol.replace("USDT", "")}
            </span>
            <span className="text-xs font-mono text-emerald-400">{formatted}</span>
          </div>
        );
      })}
    </div>
  );
}
