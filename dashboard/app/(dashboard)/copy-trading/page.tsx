"use client";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export default function CopyTradingPage() {
  const { data: status } = useQuery({
    queryKey: ["copy-trading-status"],
    queryFn: api.copyTradingStatus,
    refetchInterval: 10000,
  });

  return (
    <div className="p-8 max-w-[1400px]">
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-white mb-1">Copy Trading Engine</h1>
        <p className="text-sm text-slate-400">Wallet discovery, basket consensus, AI verification</p>
      </div>

      {/* Stats Bar */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-8">
        {[
          { label: "Tracked Wallets", value: status?.tracked_wallets ?? 0, color: "text-white" },
          { label: "Qualified (A/B)", value: status?.qualified_wallets ?? 0, color: "text-emerald-400" },
          { label: "Baskets", value: status?.baskets ?? 0, color: "text-blue-400" },
          { label: "Positions Tracked", value: status?.recent_positions ?? 0, color: "text-amber-400" },
          { label: "Circuit Breaker", value: status?.circuit_breaker_halted ? "HALTED" : "CLEAR", color: status?.circuit_breaker_halted ? "text-red-400" : "text-emerald-400" },
          { label: "Consec. Losses", value: status?.consecutive_losses ?? 0, color: (status?.consecutive_losses ?? 0) >= 3 ? "text-red-400" : "text-white" },
        ].map(stat => (
          <div key={stat.label} className="bg-slate-900/60 border border-slate-800/50 rounded-xl p-4 backdrop-blur-sm">
            <p className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">{stat.label}</p>
            <p className={`text-xl font-bold ${stat.color}`}>{stat.value}</p>
          </div>
        ))}
      </div>

      {/* Main Content */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* How It Works */}
        <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-6 backdrop-blur-sm">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-4">How Copy Trading Works</h2>
          <div className="space-y-4 text-sm">
            <div className="flex gap-3">
              <div className="w-8 h-8 rounded-lg bg-blue-400/10 flex items-center justify-center text-blue-400 font-bold text-xs shrink-0">1</div>
              <div>
                <p className="text-slate-200 font-medium">Wallet Discovery</p>
                <p className="text-slate-500 text-xs mt-0.5">Score top Polymarket traders by win rate, ROI, consistency. Grade A+ through F.</p>
              </div>
            </div>
            <div className="flex gap-3">
              <div className="w-8 h-8 rounded-lg bg-emerald-400/10 flex items-center justify-center text-emerald-400 font-bold text-xs shrink-0">2</div>
              <div>
                <p className="text-slate-200 font-medium">Basket Consensus (80%)</p>
                <p className="text-slate-500 text-xs mt-0.5">Group wallets into topic baskets. Trade only when 80%+ agree on the same side.</p>
              </div>
            </div>
            <div className="flex gap-3">
              <div className="w-8 h-8 rounded-lg bg-purple-400/10 flex items-center justify-center text-purple-400 font-bold text-xs shrink-0">3</div>
              <div>
                <p className="text-slate-200 font-medium">AI Verification</p>
                <p className="text-slate-500 text-xs mt-0.5">MiroFish AI confirms edge before copying. Confirms = full size, Neutral = half, Disagrees = skip.</p>
              </div>
            </div>
            <div className="flex gap-3">
              <div className="w-8 h-8 rounded-lg bg-red-400/10 flex items-center justify-center text-red-400 font-bold text-xs shrink-0">4</div>
              <div>
                <p className="text-slate-200 font-medium">4-Layer Circuit Breaker</p>
                <p className="text-slate-500 text-xs mt-0.5">Size filter, sequence detection, depth check, daily loss halt. Price band: $0.20-$0.80 only.</p>
              </div>
            </div>
          </div>
        </div>

        {/* Live Status */}
        <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl p-6 backdrop-blur-sm">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-4">Live Engine Status</h2>
          <div className="space-y-3">
            <div className="flex items-center justify-between py-2 border-b border-slate-800/30">
              <span className="text-sm text-slate-300">Wallets Tracked</span>
              <span className="text-sm font-mono text-white">{status?.tracked_wallets ?? 0}</span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-slate-800/30">
              <span className="text-sm text-slate-300">Qualified (Grade A/B+)</span>
              <span className="text-sm font-mono text-emerald-400">{status?.qualified_wallets ?? 0}</span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-slate-800/30">
              <span className="text-sm text-slate-300">Active Baskets</span>
              <span className="text-sm font-mono text-blue-400">{status?.baskets ?? 0}</span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-slate-800/30">
              <span className="text-sm text-slate-300">Wallet Positions Tracked</span>
              <span className="text-sm font-mono text-amber-400">{status?.recent_positions ?? 0}</span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-slate-800/30">
              <span className="text-sm text-slate-300">Consensus Threshold</span>
              <span className="text-sm font-mono text-white">80%</span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-slate-800/30">
              <span className="text-sm text-slate-300">Price Band</span>
              <span className="text-sm font-mono text-white">$0.20 - $0.80</span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-slate-800/30">
              <span className="text-sm text-slate-300">Circuit Breaker</span>
              <span className={`text-sm font-mono ${status?.circuit_breaker_halted ? "text-red-400" : "text-emerald-400"}`}>
                {status?.circuit_breaker_halted ? "HALTED" : "CLEAR"}
              </span>
            </div>
            <div className="flex items-center justify-between py-2">
              <span className="text-sm text-slate-300">Consecutive Losses</span>
              <span className={`text-sm font-mono ${(status?.consecutive_losses ?? 0) >= 3 ? "text-red-400" : "text-white"}`}>
                {status?.consecutive_losses ?? 0}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Basket Info */}
      <div className="mt-6 bg-slate-900/60 border border-slate-800/50 rounded-2xl p-6 backdrop-blur-sm">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-4">Topic Baskets</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {["Politics", "Sports", "Crypto", "General"].map(niche => (
            <div key={niche} className="bg-slate-800/30 border border-slate-700/30 rounded-xl p-4">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm font-medium text-slate-200">{niche}</span>
                <span className="text-[10px] px-2 py-0.5 rounded-full bg-blue-400/10 text-blue-400 border border-blue-400/20">
                  {niche === "General" ? "10+" : "3-5"} wallets
                </span>
              </div>
              <p className="text-[10px] text-slate-500">
                Top traders with &gt;60% WR on {niche.toLowerCase()} markets. 80% consensus required to trigger.
              </p>
              <div className="mt-2 flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                <span className="text-[10px] text-emerald-400">Active</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
