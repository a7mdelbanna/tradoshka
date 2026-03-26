import Link from "next/link";

export default function HomePage() {
  return (
    <div className="flex flex-col items-center justify-center min-h-[70vh] text-center">
      <h1 className="text-5xl md:text-7xl font-bold tracking-tight mb-6">
        <span className="text-emerald-400">Tradoshka</span>
      </h1>
      <p className="text-xl text-slate-400 mb-4 max-w-2xl">
        Multi-market auto trading platform powered by multi-agent AI simulation,
        institutional-grade risk management, and verified performance.
      </p>
      <p className="text-slate-500 mb-8">Polymarket · Crypto · Forex · Stocks</p>
      <div className="flex gap-4">
        <Link href="/performance" className="px-8 py-3 bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-semibold rounded-lg transition-colors">
          View Live Performance
        </Link>
        <a href="https://github.com/a7mdelbanna/tradoshka" target="_blank" rel="noopener"
          className="px-8 py-3 border border-slate-700 hover:border-slate-500 text-slate-300 rounded-lg transition-colors">
          GitHub
        </a>
      </div>
    </div>
  );
}
