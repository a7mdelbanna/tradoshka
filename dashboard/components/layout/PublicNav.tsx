import Link from "next/link";

export function PublicNav() {
  return (
    <nav className="border-b border-slate-800 bg-slate-950/80 backdrop-blur-sm sticky top-0 z-50">
      <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
        <Link href="/" className="text-xl font-bold tracking-tight">
          <span className="text-emerald-400">Tradoshka</span>
        </Link>
        <div className="flex items-center gap-6">
          <Link href="/performance" className="text-sm text-slate-400 hover:text-slate-100 transition-colors">Performance</Link>
          <a href="https://github.com/a7mdelbanna/tradoshka" target="_blank" rel="noopener" className="text-sm text-slate-400 hover:text-slate-100 transition-colors">GitHub</a>
        </div>
      </div>
    </nav>
  );
}
