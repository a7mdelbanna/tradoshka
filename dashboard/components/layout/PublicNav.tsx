import Link from "next/link";

export function PublicNav() {
  return (
    <nav className="border-b border-slate-800/50 bg-slate-950/90 backdrop-blur-xl sticky top-0 z-50">
      <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
        <Link href="/" className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-emerald-400 to-emerald-600 flex items-center justify-center">
            <span className="text-slate-950 font-black text-sm">T</span>
          </div>
          <span className="text-xl font-bold tracking-tight text-white">Tradoshka</span>
        </Link>
        <div className="flex items-center gap-1">
          <Link href="/performance"
            className="px-4 py-2 text-sm text-slate-300 hover:text-white hover:bg-slate-800/50 rounded-lg transition-all duration-200">
            Performance
          </Link>
          <a href="https://github.com/a7mdelbanna/tradoshka" target="_blank" rel="noopener"
            className="px-4 py-2 text-sm text-slate-300 hover:text-white hover:bg-slate-800/50 rounded-lg transition-all duration-200">
            GitHub
          </a>
          <div className="ml-2 flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-emerald-400/10 border border-emerald-400/20">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
            <span className="text-xs font-medium text-emerald-400">Live</span>
          </div>
        </div>
      </div>
    </nav>
  );
}
