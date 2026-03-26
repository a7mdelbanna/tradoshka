import Link from "next/link";

export default function HomePage() {
  return (
    <div className="flex flex-col items-center justify-center min-h-[80vh] text-center relative">
      {/* Gradient orb background */}
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-emerald-500/5 rounded-full blur-[120px] pointer-events-none" />

      <div className="relative z-10">
        <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-emerald-500/20 bg-emerald-500/5 mb-8">
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
          <span className="text-xs font-medium text-emerald-400">Live Trading Active</span>
        </div>

        <h1 className="text-6xl md:text-8xl font-black tracking-tight mb-6">
          <span className="bg-gradient-to-r from-white via-slate-200 to-slate-400 bg-clip-text text-transparent">Trade</span>
          <span className="bg-gradient-to-r from-emerald-300 to-emerald-500 bg-clip-text text-transparent">oshka</span>
        </h1>

        <p className="text-lg md:text-xl text-slate-400 mb-4 max-w-2xl leading-relaxed">
          Multi-market auto trading powered by multi-agent AI simulation,
          institutional-grade risk management, and <span className="text-white font-medium">verified performance</span>.
        </p>

        <div className="flex items-center justify-center gap-3 text-sm text-slate-500 mb-10">
          {["Polymarket", "Crypto", "Forex", "Stocks"].map((m, i) => (
            <span key={m} className="flex items-center gap-3">
              {i > 0 && <span className="w-1 h-1 rounded-full bg-slate-700" />}
              <span>{m}</span>
            </span>
          ))}
        </div>

        <div className="flex flex-col sm:flex-row gap-3 justify-center">
          <Link href="/performance"
            className="px-8 py-3.5 bg-gradient-to-r from-emerald-500 to-emerald-600 hover:from-emerald-400 hover:to-emerald-500 text-white font-semibold rounded-xl transition-all duration-300 shadow-[0_0_20px_rgba(52,211,153,0.15)] hover:shadow-[0_0_30px_rgba(52,211,153,0.25)]">
            View Live Performance
          </Link>
          <a href="https://github.com/a7mdelbanna/tradoshka" target="_blank" rel="noopener"
            className="px-8 py-3.5 border border-slate-700 hover:border-slate-500 text-slate-300 hover:text-white font-semibold rounded-xl transition-all duration-300 hover:bg-slate-800/30">
            View on GitHub
          </a>
        </div>
      </div>
    </div>
  );
}
