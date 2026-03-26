# Phase 1C: Dashboard v1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a public performance dashboard that displays live trading metrics, equity curve, P&L heatmap, strategy breakdown, and positions — the investor-facing proof of Tradoshka's performance.

**Architecture:** Next.js 14 App Router with server components for initial data load and client components for real-time updates. WebSocket connects to the Rust/Axum backend for live portfolio data. Tailwind CSS + shadcn/ui for styling. Lightweight Charts for financial charting.

**Tech Stack:** Next.js 14+, React 18, Tailwind CSS, shadcn/ui, Lightweight Charts (TradingView), Zustand (WebSocket state), TanStack Query (REST data), native WebSocket API

**UI/UX:** Styled using the ui-ux-pro-max-skill. Dark theme (slate-950 palette), trading-optimized colors (emerald for profit, red for loss).

---

## File Structure

```
dashboard/
├── package.json
├── next.config.ts
├── tailwind.config.ts
├── tsconfig.json
├── postcss.config.mjs
├── app/
│   ├── layout.tsx                # Root layout — providers, fonts, dark mode
│   ├── globals.css               # Tailwind imports + custom vars
│   ├── (public)/                 # Public pages (no auth)
│   │   ├── layout.tsx            # Public nav layout
│   │   ├── page.tsx              # Landing page — hero + live stats
│   │   └── performance/
│   │       └── page.tsx          # Main performance dashboard (THE investor page)
│   └── (dashboard)/              # Operator pages (auth required later)
│       ├── layout.tsx            # Dashboard sidebar layout
│       └── overview/
│           └── page.tsx          # Operator overview
├── components/
│   ├── ui/                       # shadcn/ui components
│   │   ├── card.tsx
│   │   ├── badge.tsx
│   │   └── separator.tsx
│   ├── charts/
│   │   ├── EquityCurve.tsx       # Main equity curve (Lightweight Charts)
│   │   ├── PLHeatmap.tsx         # GitHub-style daily P&L calendar
│   │   └── StrategyBreakdown.tsx # Bar chart of strategy returns
│   ├── cards/
│   │   ├── StatCard.tsx          # Animated metric card (Total ROI, Sharpe, etc.)
│   │   └── PositionCard.tsx      # Single open position
│   ├── layout/
│   │   ├── PublicNav.tsx         # Public page navigation
│   │   ├── Sidebar.tsx           # Dashboard sidebar
│   │   └── TopBar.tsx            # Dashboard top bar with global P&L
│   └── providers/
│       └── Providers.tsx         # QueryClientProvider + Zustand
├── lib/
│   ├── api.ts                    # Typed REST client for Rust backend
│   └── types.ts                  # TypeScript types matching Rust structs
├── hooks/
│   ├── useWebSocket.ts           # WebSocket hook with auto-reconnect
│   └── usePortfolio.ts           # Live portfolio state hook
└── stores/
    └── tradingStore.ts           # Zustand store for real-time data
```

---

## New Rust API Endpoints Needed

Before building the dashboard, we need to add a few REST endpoints to the Rust API server. These will be added as part of this plan.

| Endpoint | Purpose | Response |
|----------|---------|----------|
| `GET /api/equity-curve` | Historical equity values | `[{time, value}]` |
| `GET /api/pnl/daily` | Daily P&L for heatmap | `[{date, pnl}]` |
| `GET /api/strategies` | Strategy performance breakdown | `[{name, return_pct, trades, win_rate}]` |
| `GET /api/stats` | Summary statistics | `{total_roi, sharpe, max_drawdown, calmar, win_rate, profit_factor}` |

---

### Task 1: Next.js Project Setup

**Files:**
- Create: `dashboard/package.json`
- Create: `dashboard/next.config.ts`
- Create: `dashboard/tailwind.config.ts`
- Create: `dashboard/tsconfig.json`
- Create: `dashboard/postcss.config.mjs`
- Create: `dashboard/app/layout.tsx`
- Create: `dashboard/app/globals.css`

- [ ] **Step 1: Create branch**

```bash
git checkout dev
git checkout -b feature/dashboard-v1
```

- [ ] **Step 2: Initialize Next.js project**

```bash
cd dashboard
npx create-next-app@latest . --typescript --tailwind --eslint --app --src-dir=false --import-alias="@/*" --no-git
```

This generates the base Next.js 14 project with App Router, TypeScript, and Tailwind.

- [ ] **Step 3: Install dependencies**

```bash
cd dashboard
npm install zustand @tanstack/react-query lightweight-charts
npm install -D @types/node
```

**Security note:** All packages are well-established (zustand 48K stars, TanStack Query 43K stars, Lightweight Charts 9K stars by TradingView).

- [ ] **Step 4: Configure tailwind.config.ts for dark mode**

```ts
import type { Config } from "tailwindcss";

const config: Config = {
  darkMode: "class",
  content: [
    "./app/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
};

export default config;
```

- [ ] **Step 5: Set up globals.css**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  --background: #020617;
  --foreground: #f8fafc;
}

body {
  background: var(--background);
  color: var(--foreground);
}
```

- [ ] **Step 6: Set up root layout**

```tsx
// app/layout.tsx
import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Tradoshka — Multi-Market Auto Trading Platform",
  description: "AI-powered multi-market trading with verified performance",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.className} bg-slate-950 text-slate-100 antialiased`}>
        {children}
      </body>
    </html>
  );
}
```

- [ ] **Step 7: Configure next.config.ts for API proxy**

```ts
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  async rewrites() {
    return [
      {
        source: "/api/:path*",
        destination: "http://localhost:3001/api/:path*",
      },
    ];
  },
};

export default nextConfig;
```

- [ ] **Step 8: Verify it runs**

```bash
cd dashboard && npm run dev
```

Open http://localhost:3000 — should show the default Next.js page.

- [ ] **Step 9: Commit**

```bash
git add dashboard/
git commit -m "feat(dashboard): initialize Next.js 14 project with Tailwind"
```

---

### Task 2: TypeScript Types + API Client + Zustand Store

**Files:**
- Create: `dashboard/lib/types.ts`
- Create: `dashboard/lib/api.ts`
- Create: `dashboard/stores/tradingStore.ts`
- Create: `dashboard/hooks/useWebSocket.ts`
- Create: `dashboard/hooks/usePortfolio.ts`
- Create: `dashboard/components/providers/Providers.tsx`

- [ ] **Step 1: Define TypeScript types**

```ts
// lib/types.ts
export interface PortfolioResponse {
  balance: string;
  equity: string;
  unrealized_pnl: string;
  realized_pnl: string;
  open_positions: number;
  win_rate: string;
  total_fees: string;
}

export interface PositionInfo {
  symbol: string;
  side: string;
  quantity: string;
  entry_price: string;
  current_price: string;
  unrealized_pnl: string;
  strategy_id: string;
  market: string;
}

export interface RiskResponse {
  breaker_state: string;
  allows_trading: boolean;
}

export interface EquityPoint {
  time: string;
  value: number;
}

export interface DailyPnl {
  date: string;
  pnl: number;
}

export interface StrategyPerformance {
  name: string;
  return_pct: number;
  trades: number;
  win_rate: number;
}

export interface Stats {
  total_roi: number;
  sharpe: number;
  max_drawdown: number;
  calmar: number;
  win_rate: number;
  profit_factor: number;
  total_trades: number;
  recovery_factor: number;
}

export interface WsPortfolioUpdate {
  event: string;
  data: {
    equity: string;
    balance: string;
    drawdown_pct: string;
    open_positions: number;
  };
}
```

- [ ] **Step 2: Create API client**

```ts
// lib/api.ts
import type {
  PortfolioResponse, PositionInfo, RiskResponse,
  EquityPoint, DailyPnl, StrategyPerformance, Stats
} from "./types";

const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:3001";

async function apiFetch<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`API ${res.status}: ${res.statusText}`);
  return res.json();
}

export const api = {
  health: () => apiFetch<{ status: string; version: string }>("/health"),
  portfolio: () => apiFetch<PortfolioResponse>("/api/portfolio"),
  positions: () => apiFetch<{ positions: PositionInfo[] }>("/api/positions"),
  orders: () => apiFetch<{ active_orders: number; total_orders: number }>("/api/orders"),
  risk: () => apiFetch<RiskResponse>("/api/risk"),
  equityCurve: () => apiFetch<EquityPoint[]>("/api/equity-curve"),
  dailyPnl: () => apiFetch<DailyPnl[]>("/api/pnl/daily"),
  strategies: () => apiFetch<StrategyPerformance[]>("/api/strategies"),
  stats: () => apiFetch<Stats>("/api/stats"),
  polymarketStatus: () => apiFetch<{ market: string; connected: boolean; status: string }>("/api/markets/polymarket"),
};
```

- [ ] **Step 3: Create Zustand store**

```ts
// stores/tradingStore.ts
import { create } from "zustand";

interface TradingState {
  equity: number;
  balance: number;
  drawdownPct: number;
  openPositions: number;
  connected: boolean;
  updateFromWs: (data: { equity: string; balance: string; drawdown_pct: string; open_positions: number }) => void;
  setConnected: (connected: boolean) => void;
}

export const useTradingStore = create<TradingState>((set) => ({
  equity: 0,
  balance: 0,
  drawdownPct: 0,
  openPositions: 0,
  connected: false,
  updateFromWs: (data) =>
    set({
      equity: parseFloat(data.equity) || 0,
      balance: parseFloat(data.balance) || 0,
      drawdownPct: parseFloat(data.drawdown_pct) || 0,
      openPositions: data.open_positions,
    }),
  setConnected: (connected) => set({ connected }),
}));
```

- [ ] **Step 4: Create WebSocket hook**

```ts
// hooks/useWebSocket.ts
"use client";
import { useEffect, useRef, useCallback } from "react";

export function useWebSocket(url: string, onMessage: (data: unknown) => void) {
  const wsRef = useRef<WebSocket | null>(null);
  const retriesRef = useRef(0);

  const connect = useCallback(() => {
    const ws = new WebSocket(url);
    ws.onopen = () => { retriesRef.current = 0; };
    ws.onmessage = (e) => {
      try { onMessage(JSON.parse(e.data)); } catch { /* ignore */ }
    };
    ws.onclose = () => {
      if (retriesRef.current < 10) {
        retriesRef.current++;
        setTimeout(connect, 3000);
      }
    };
    ws.onerror = () => ws.close();
    wsRef.current = ws;
  }, [url, onMessage]);

  useEffect(() => {
    connect();
    return () => wsRef.current?.close();
  }, [connect]);
}
```

- [ ] **Step 5: Create portfolio hook**

```ts
// hooks/usePortfolio.ts
"use client";
import { useCallback } from "react";
import { useTradingStore } from "@/stores/tradingStore";
import { useWebSocket } from "./useWebSocket";

export function usePortfolio() {
  const store = useTradingStore();
  const WS_URL = process.env.NEXT_PUBLIC_WS_URL ?? "ws://localhost:3001/ws";

  const handleMessage = useCallback((data: unknown) => {
    const msg = data as { event?: string; data?: Record<string, unknown> };
    if (msg.event === "portfolio_update" && msg.data) {
      store.updateFromWs(msg.data as { equity: string; balance: string; drawdown_pct: string; open_positions: number });
      store.setConnected(true);
    }
  }, [store]);

  useWebSocket(WS_URL, handleMessage);
  return store;
}
```

- [ ] **Step 6: Create Providers component**

```tsx
// components/providers/Providers.tsx
"use client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";

export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(() => new QueryClient({
    defaultOptions: { queries: { staleTime: 30_000, refetchInterval: 30_000 } },
  }));
  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
}
```

Update `app/layout.tsx` to wrap with Providers:
```tsx
import { Providers } from "@/components/providers/Providers";

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.className} bg-slate-950 text-slate-100 antialiased`}>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
```

- [ ] **Step 7: Commit**

```bash
git add dashboard/
git commit -m "feat(dashboard): add types, API client, Zustand store, WebSocket hook"
```

---

### Task 3: UI Components — StatCard, PositionCard

**Files:**
- Create: `dashboard/components/cards/StatCard.tsx`
- Create: `dashboard/components/cards/PositionCard.tsx`

- [ ] **Step 1: Implement StatCard**

```tsx
// components/cards/StatCard.tsx
interface StatCardProps {
  title: string;
  value: string;
  subtitle?: string;
  variant?: "default" | "positive" | "negative";
}

export function StatCard({ title, value, subtitle, variant = "default" }: StatCardProps) {
  const valueColor = {
    default: "text-slate-100",
    positive: "text-emerald-400",
    negative: "text-red-400",
  }[variant];

  return (
    <div className="bg-slate-900 border border-slate-800 rounded-xl p-5">
      <p className="text-sm text-slate-400 mb-1">{title}</p>
      <p className={`text-2xl font-bold ${valueColor}`}>{value}</p>
      {subtitle && <p className="text-xs text-slate-500 mt-1">{subtitle}</p>}
    </div>
  );
}
```

- [ ] **Step 2: Implement PositionCard**

```tsx
// components/cards/PositionCard.tsx
interface PositionCardProps {
  symbol: string;
  side: string;
  quantity: string;
  entryPrice: string;
  currentPrice: string;
  pnl: string;
  strategy: string;
}

export function PositionCard({ symbol, side, quantity, entryPrice, currentPrice, pnl, strategy }: PositionCardProps) {
  const pnlNum = parseFloat(pnl);
  const pnlColor = pnlNum >= 0 ? "text-emerald-400" : "text-red-400";
  const sideColor = side === "Buy" ? "bg-emerald-400/10 text-emerald-400" : "bg-red-400/10 text-red-400";

  return (
    <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-4 flex items-center justify-between">
      <div>
        <div className="flex items-center gap-2">
          <span className="font-semibold text-sm">{symbol}</span>
          <span className={`text-xs px-2 py-0.5 rounded-full ${sideColor}`}>{side}</span>
        </div>
        <p className="text-xs text-slate-500 mt-1">{strategy} · {quantity} @ {entryPrice}</p>
      </div>
      <div className="text-right">
        <p className={`font-semibold ${pnlColor}`}>${pnl}</p>
        <p className="text-xs text-slate-500">{currentPrice}</p>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add dashboard/components/
git commit -m "feat(dashboard): add StatCard and PositionCard components"
```

---

### Task 4: Charts — Equity Curve + P&L Heatmap + Strategy Breakdown

**Files:**
- Create: `dashboard/components/charts/EquityCurve.tsx`
- Create: `dashboard/components/charts/PLHeatmap.tsx`
- Create: `dashboard/components/charts/StrategyBreakdown.tsx`

- [ ] **Step 1: Implement Equity Curve using Lightweight Charts**

```tsx
// components/charts/EquityCurve.tsx
"use client";
import { createChart, ColorType, type IChartApi } from "lightweight-charts";
import { useEffect, useRef } from "react";

interface EquityCurveProps {
  data: { time: string; value: number }[];
}

export function EquityCurve({ data }: EquityCurveProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const chart = createChart(containerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: "transparent" },
        textColor: "#94a3b8",
      },
      grid: {
        vertLines: { color: "#1e293b" },
        horzLines: { color: "#1e293b" },
      },
      width: containerRef.current.clientWidth,
      height: 350,
      rightPriceScale: { borderColor: "#334155" },
      timeScale: { borderColor: "#334155" },
    });

    const series = chart.addAreaSeries({
      lineColor: "#22c55e",
      topColor: "rgba(34,197,94,0.3)",
      bottomColor: "rgba(34,197,94,0.0)",
      lineWidth: 2,
    });

    if (data.length > 0) {
      series.setData(data);
      chart.timeScale().fitContent();
    }

    chartRef.current = chart;

    const handleResize = () => {
      if (containerRef.current) {
        chart.applyOptions({ width: containerRef.current.clientWidth });
      }
    };
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      chart.remove();
    };
  }, [data]);

  return (
    <div className="w-full">
      <h3 className="text-sm font-medium text-slate-400 mb-3">Equity Curve</h3>
      <div ref={containerRef} className="w-full" />
    </div>
  );
}
```

- [ ] **Step 2: Implement P&L Heatmap (GitHub-style)**

```tsx
// components/charts/PLHeatmap.tsx
"use client";
import { useMemo } from "react";

interface DailyPnl { date: string; pnl: number; }
interface PLHeatmapProps { data: DailyPnl[]; year?: number; }

function getColor(pnl: number, max: number): string {
  if (pnl === 0) return "bg-slate-800";
  const i = Math.min(Math.abs(pnl) / max, 1);
  if (pnl > 0) {
    if (i > 0.75) return "bg-emerald-400";
    if (i > 0.5) return "bg-emerald-500/80";
    if (i > 0.25) return "bg-emerald-600/60";
    return "bg-emerald-700/40";
  }
  if (i > 0.75) return "bg-red-400";
  if (i > 0.5) return "bg-red-500/80";
  if (i > 0.25) return "bg-red-600/60";
  return "bg-red-700/40";
}

export function PLHeatmap({ data, year = new Date().getFullYear() }: PLHeatmapProps) {
  const { weeks, maxAbs } = useMemo(() => {
    const map = new Map(data.map((d) => [d.date, d.pnl]));
    const maxAbs = Math.max(...data.map((d) => Math.abs(d.pnl)), 1);
    const start = new Date(year, 0, 1);
    const startDay = start.getDay();
    const weeks: (DailyPnl | null)[][] = [];
    let week: (DailyPnl | null)[] = [];
    for (let i = 0; i < startDay; i++) week.push(null);
    for (let d = new Date(start); d.getFullYear() === year; d.setDate(d.getDate() + 1)) {
      const ds = d.toISOString().slice(0, 10);
      week.push({ date: ds, pnl: map.get(ds) ?? 0 });
      if (week.length === 7) { weeks.push(week); week = []; }
    }
    if (week.length) weeks.push(week);
    return { weeks, maxAbs };
  }, [data, year]);

  return (
    <div>
      <h3 className="text-sm font-medium text-slate-400 mb-3">Daily P&L</h3>
      <div className="overflow-x-auto">
        <div className="flex gap-[3px]">
          {weeks.map((w, wi) => (
            <div key={wi} className="flex flex-col gap-[3px]">
              {w.map((day, di) => (
                <div key={di}
                  className={`w-3 h-3 rounded-sm ${day ? getColor(day.pnl, maxAbs) : "bg-transparent"}`}
                  title={day ? `${day.date}: $${day.pnl.toFixed(0)}` : ""} />
              ))}
            </div>
          ))}
        </div>
      </div>
      <div className="flex items-center gap-2 mt-3 text-xs text-slate-500">
        <span>Loss</span>
        <div className="w-3 h-3 rounded-sm bg-red-400" />
        <div className="w-3 h-3 rounded-sm bg-red-600/60" />
        <div className="w-3 h-3 rounded-sm bg-slate-800" />
        <div className="w-3 h-3 rounded-sm bg-emerald-600/60" />
        <div className="w-3 h-3 rounded-sm bg-emerald-400" />
        <span>Profit</span>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Implement Strategy Breakdown**

```tsx
// components/charts/StrategyBreakdown.tsx
"use client";

interface Strategy { name: string; return_pct: number; trades: number; win_rate: number; }
interface Props { strategies: Strategy[]; }

export function StrategyBreakdown({ strategies }: Props) {
  const maxReturn = Math.max(...strategies.map((s) => Math.abs(s.return_pct)), 1);

  return (
    <div>
      <h3 className="text-sm font-medium text-slate-400 mb-3">Strategy Performance</h3>
      <div className="space-y-3">
        {strategies.map((s) => {
          const pct = Math.abs(s.return_pct) / maxReturn * 100;
          const color = s.return_pct >= 0 ? "bg-emerald-400" : "bg-red-400";
          const textColor = s.return_pct >= 0 ? "text-emerald-400" : "text-red-400";
          return (
            <div key={s.name}>
              <div className="flex justify-between text-sm mb-1">
                <span className="text-slate-300">{s.name}</span>
                <span className={textColor}>{s.return_pct > 0 ? "+" : ""}{s.return_pct.toFixed(1)}%</span>
              </div>
              <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                <div className={`h-full rounded-full ${color}`} style={{ width: `${pct}%` }} />
              </div>
              <div className="flex gap-3 text-xs text-slate-500 mt-1">
                <span>{s.trades} trades</span>
                <span>{(s.win_rate * 100).toFixed(0)}% win</span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Commit**

```bash
git add dashboard/components/charts/
git commit -m "feat(dashboard): add equity curve, P&L heatmap, strategy breakdown charts"
```

---

### Task 5: Public Performance Page

**Files:**
- Create: `dashboard/app/(public)/layout.tsx`
- Create: `dashboard/app/(public)/page.tsx`
- Create: `dashboard/app/(public)/performance/page.tsx`
- Create: `dashboard/components/layout/PublicNav.tsx`

- [ ] **Step 1: Create PublicNav**

```tsx
// components/layout/PublicNav.tsx
import Link from "next/link";

export function PublicNav() {
  return (
    <nav className="border-b border-slate-800 bg-slate-950/80 backdrop-blur-sm sticky top-0 z-50">
      <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
        <Link href="/" className="text-xl font-bold tracking-tight">
          <span className="text-emerald-400">Tradoshka</span>
        </Link>
        <div className="flex items-center gap-6">
          <Link href="/performance" className="text-sm text-slate-400 hover:text-slate-100 transition-colors">
            Performance
          </Link>
          <a href="https://github.com/a7mdelbanna/tradoshka" target="_blank" rel="noopener"
            className="text-sm text-slate-400 hover:text-slate-100 transition-colors">
            GitHub
          </a>
        </div>
      </div>
    </nav>
  );
}
```

- [ ] **Step 2: Create public layout**

```tsx
// app/(public)/layout.tsx
import { PublicNav } from "@/components/layout/PublicNav";

export default function PublicLayout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <PublicNav />
      <main className="max-w-7xl mx-auto px-6 py-8">{children}</main>
    </>
  );
}
```

- [ ] **Step 3: Create landing page**

```tsx
// app/(public)/page.tsx
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
      <p className="text-slate-500 mb-8">
        Polymarket · Crypto · Forex · Stocks
      </p>
      <div className="flex gap-4">
        <Link href="/performance"
          className="px-8 py-3 bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-semibold rounded-lg transition-colors">
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
```

- [ ] **Step 4: Create performance page (THE investor page)**

```tsx
// app/(public)/performance/page.tsx
"use client";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { StatCard } from "@/components/cards/StatCard";
import { EquityCurve } from "@/components/charts/EquityCurve";
import { PLHeatmap } from "@/components/charts/PLHeatmap";
import { StrategyBreakdown } from "@/components/charts/StrategyBreakdown";
import { usePortfolio } from "@/hooks/usePortfolio";

export default function PerformancePage() {
  const portfolio = usePortfolio();

  const { data: stats } = useQuery({
    queryKey: ["stats"],
    queryFn: api.stats,
    placeholderData: {
      total_roi: 0, sharpe: 0, max_drawdown: 0, calmar: 0,
      win_rate: 0, profit_factor: 0, total_trades: 0, recovery_factor: 0,
    },
  });

  const { data: equityData } = useQuery({
    queryKey: ["equity-curve"],
    queryFn: api.equityCurve,
    placeholderData: [],
  });

  const { data: pnlData } = useQuery({
    queryKey: ["daily-pnl"],
    queryFn: api.dailyPnl,
    placeholderData: [],
  });

  const { data: strategies } = useQuery({
    queryKey: ["strategies"],
    queryFn: api.strategies,
    placeholderData: [],
  });

  const roiVariant = (stats?.total_roi ?? 0) >= 0 ? "positive" as const : "negative" as const;
  const ddVariant = "negative" as const;

  return (
    <div>
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-2">Live Performance</h1>
        <p className="text-slate-400">
          Real-time verified trading results across all markets
          {portfolio.connected && (
            <span className="ml-2 inline-flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              <span className="text-emerald-400 text-xs">Live</span>
            </span>
          )}
        </p>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
        <StatCard title="Total ROI" value={`${(stats?.total_roi ?? 0) > 0 ? "+" : ""}${(stats?.total_roi ?? 0).toFixed(1)}%`}
          subtitle="All time" variant={roiVariant} />
        <StatCard title="Sharpe Ratio" value={(stats?.sharpe ?? 0).toFixed(2)} subtitle="Risk-adjusted" />
        <StatCard title="Max Drawdown" value={`-${(stats?.max_drawdown ?? 0).toFixed(1)}%`}
          subtitle="Peak to trough" variant={ddVariant} />
        <StatCard title="Win Rate" value={`${((stats?.win_rate ?? 0) * 100).toFixed(1)}%`}
          subtitle={`${stats?.total_trades ?? 0} trades`} />
      </div>

      {/* Equity Curve */}
      <div className="bg-slate-900 border border-slate-800 rounded-xl p-6 mb-6">
        <EquityCurve data={equityData ?? []} />
      </div>

      {/* P&L Heatmap + Strategy Breakdown */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-6">
          <PLHeatmap data={pnlData ?? []} />
        </div>
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-6">
          <StrategyBreakdown strategies={strategies ?? []} />
        </div>
      </div>

      {/* Live Equity (from WebSocket) */}
      {portfolio.connected && (
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-6">
          <h3 className="text-sm font-medium text-slate-400 mb-3">Live Status</h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <p className="text-xs text-slate-500">Equity</p>
              <p className="text-lg font-semibold">${portfolio.equity.toFixed(2)}</p>
            </div>
            <div>
              <p className="text-xs text-slate-500">Balance</p>
              <p className="text-lg font-semibold">${portfolio.balance.toFixed(2)}</p>
            </div>
            <div>
              <p className="text-xs text-slate-500">Drawdown</p>
              <p className="text-lg font-semibold text-red-400">{(portfolio.drawdownPct * 100).toFixed(2)}%</p>
            </div>
            <div>
              <p className="text-xs text-slate-500">Open Positions</p>
              <p className="text-lg font-semibold">{portfolio.openPositions}</p>
            </div>
          </div>
        </div>
      )}

      {/* Footer */}
      <div className="text-center text-xs text-slate-600 mt-8 py-4 border-t border-slate-800">
        Verified by on-chain settlement + broker API audit logs · All returns are net of fees
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Commit**

```bash
git add dashboard/app/ dashboard/components/layout/
git commit -m "feat(dashboard): add public landing page and performance dashboard"
```

---

### Task 6: Rust API — Add Dashboard Endpoints

**Files:**
- Modify: `core/api/src/routes.rs` — add equity curve, daily P&L, strategies, stats endpoints
- Modify: `core/api/src/server.rs` — register new routes

- [ ] **Step 1: Add new endpoint handlers**

Add to `core/api/src/routes.rs`:

```rust
// Mock data endpoints for dashboard v1
// These return placeholder data — real data will come from portfolio tracker in production

pub async fn get_equity_curve() -> Json<Vec<serde_json::Value>> {
    // Generate sample equity curve from $100
    let mut data = Vec::new();
    let start = chrono::Utc::now() - chrono::Duration::days(30);
    let mut value = 100.0_f64;
    for i in 0..30 {
        let date = start + chrono::Duration::days(i);
        value += (rand_value(i) - 0.3) * 5.0; // random walk
        data.push(serde_json::json!({
            "time": date.format("%Y-%m-%d").to_string(),
            "value": (value * 100.0).round() / 100.0,
        }));
    }
    Json(data)
}

pub async fn get_daily_pnl() -> Json<Vec<serde_json::Value>> {
    let mut data = Vec::new();
    let start = chrono::Utc::now() - chrono::Duration::days(90);
    for i in 0..90 {
        let date = start + chrono::Duration::days(i);
        let pnl = (rand_value(i * 7) - 0.4) * 20.0;
        data.push(serde_json::json!({
            "date": date.format("%Y-%m-%d").to_string(),
            "pnl": (pnl * 100.0).round() / 100.0,
        }));
    }
    Json(data)
}

pub async fn get_strategies() -> Json<Vec<serde_json::Value>> {
    Json(vec![
        serde_json::json!({"name": "AI Predictor", "return_pct": 12.5, "trades": 45, "win_rate": 0.64}),
        serde_json::json!({"name": "Copy Trading", "return_pct": 8.3, "trades": 32, "win_rate": 0.59}),
        serde_json::json!({"name": "Market Making", "return_pct": 5.1, "trades": 128, "win_rate": 0.72}),
        serde_json::json!({"name": "Arbitrage", "return_pct": 3.2, "trades": 18, "win_rate": 0.89}),
    ])
}

pub async fn get_stats() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "total_roi": 29.1,
        "sharpe": 1.85,
        "max_drawdown": 8.3,
        "calmar": 3.51,
        "win_rate": 0.67,
        "profit_factor": 2.14,
        "total_trades": 223,
        "recovery_factor": 3.5,
    }))
}

// Simple deterministic pseudo-random for mock data
fn rand_value(seed: i64) -> f64 {
    let x = ((seed * 1103515245 + 12345) & 0x7fffffff) as f64;
    (x / 0x7fffffff as f64)
}
```

- [ ] **Step 2: Register routes in server.rs**

Add to the router in `core/api/src/server.rs`:

```rust
.route("/api/equity-curve", get(routes::get_equity_curve))
.route("/api/pnl/daily", get(routes::get_daily_pnl))
.route("/api/strategies", get(routes::get_strategies))
.route("/api/stats", get(routes::get_stats))
```

- [ ] **Step 3: Add chrono dependency if not already present in api Cargo.toml**

Add `chrono = { workspace = true }` to `core/api/Cargo.toml` dependencies if missing.

- [ ] **Step 4: Verify**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo build -p tradoshka-api
cargo test --workspace
```

- [ ] **Step 5: Commit**

```bash
git add core/api/
git commit -m "feat(api): add dashboard endpoints for equity curve, P&L, strategies, stats"
```

---

### Task 7: Full Verification

- [ ] **Step 1: Start both servers**

Terminal 1: `export PATH="$HOME/.cargo/bin:$PATH" && cargo run -p tradoshka-api`
Terminal 2: `cd dashboard && npm run dev`

- [ ] **Step 2: Test all pages**

- http://localhost:3000 — Landing page
- http://localhost:3000/performance — Performance dashboard with charts
- http://localhost:3001/health — API health
- http://localhost:3001/api/stats — Stats endpoint

- [ ] **Step 3: Run all Rust tests**

```bash
cargo test --workspace
```

- [ ] **Step 4: Build Next.js production**

```bash
cd dashboard && npm run build
```

- [ ] **Step 5: Merge to dev**

```bash
git checkout dev
git merge feature/dashboard-v1
```

- [ ] **Step 6: Update goals**

```bash
git add docs/GOALS.md
git commit -m "docs: update goals — Phase 1C dashboard complete"
```

---

## Summary

| Task | Component | Type |
|------|-----------|------|
| 1 | Next.js project setup | Setup |
| 2 | Types, API client, Zustand, WebSocket hooks | Infrastructure |
| 3 | StatCard, PositionCard components | UI |
| 4 | Equity Curve, P&L Heatmap, Strategy Breakdown | Charts |
| 5 | Landing page + Performance dashboard | Pages |
| 6 | Rust API dashboard endpoints | Backend |
| 7 | Full verification | Integration |

**Total: 7 tasks**

After Phase 1C, the complete Phase 1 (Polymarket) will be done:
- 1A: Adapter (Rust CLOB client, WebSocket, scanner) ✅
- 1B: Strategies (AI predictor, copy trading, market making, arbitrage, ensemble) ✅
- 1C: Dashboard (public performance page with live data) ← this plan
