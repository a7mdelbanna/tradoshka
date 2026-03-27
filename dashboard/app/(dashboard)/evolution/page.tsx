"use client";

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import Link from "next/link";

// ─── Types ────────────────────────────────────────────────────────────────────

interface EvolutionStats {
  alive_count: number;
  dead_count: number;
  total_capital: number;
  avg_sharpe: number;
  best_strategy: string;
  hours_running: number;
}

interface LeaderboardEntry {
  rank: number;
  id: string;
  name: string;
  generation: number;
  market: string;
  sharpe: number;
  pnl: number;
  total_pnl: number;
  unrealized_pnl: number;
  realized_pnl: number;
  equity: number;
  balance: number;
  open_positions: number;
  fees: number;
  win_rate: number;
  trades: number;
  age_hours: number;
}

interface TimelineEvent {
  hour: number;
  type: "KILLED" | "SPAWNED" | "MUTATED";
  strategy_name: string;
  details: string;
}

interface GraveyardEntry {
  name: string;
  market: string;
  lifetime_hours: number;
  trades: number;
  final_pnl: number;
  win_rate: number;
  sharpe: number;
  cause_of_death: string;
}

// ─── Fallback / placeholder data ──────────────────────────────────────────────

const PLACEHOLDER_STATS: EvolutionStats = {
  alive_count: 0,
  dead_count: 0,
  total_capital: 0,
  avg_sharpe: 0,
  best_strategy: "—",
  hours_running: 0,
};

const PLACEHOLDER_LEADERBOARD: LeaderboardEntry[] = [];
const PLACEHOLDER_TIMELINE: TimelineEvent[] = [];
const PLACEHOLDER_GRAVEYARD: GraveyardEntry[] = [];

// ─── Market label helper ───────────────────────────────────────────────────────

function marketLabel(m: string) {
  if (!m) return "—";
  const lower = m.toLowerCase();
  if (lower.includes("poly")) return "Polymarket";
  if (lower.includes("perp")) return "Crypto Perps";
  if (lower.includes("spot")) return "Crypto Spot";
  if (lower.includes("crypto")) return "Crypto";
  return m;
}

// ─── Sort helper ──────────────────────────────────────────────────────────────

type SortKey = keyof LeaderboardEntry;

// ─── Sub-components ───────────────────────────────────────────────────────────

function StatCard({
  label,
  value,
  accent,
  sub,
}: {
  label: string;
  value: string | number;
  accent?: "emerald" | "red" | "blue" | "default";
  sub?: string;
}) {
  const accentClass =
    accent === "emerald"
      ? "text-emerald-400"
      : accent === "red"
      ? "text-red-400"
      : accent === "blue"
      ? "text-blue-400"
      : "text-white";

  return (
    <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl backdrop-blur-sm px-5 py-4 flex flex-col gap-1 min-w-0">
      <span className="text-[10px] font-bold uppercase tracking-widest text-slate-500">
        {label}
      </span>
      <span className={`text-2xl font-black truncate ${accentClass}`}>
        {value}
      </span>
      {sub && <span className="text-[10px] text-slate-600 truncate">{sub}</span>}
    </div>
  );
}

function LeaderboardTable({
  rows,
  totalAlive,
}: {
  rows: LeaderboardEntry[];
  totalAlive: number;
}) {
  const [sortKey, setSortKey] = useState<SortKey>("rank");
  const [sortAsc, setSortAsc] = useState(true);

  const top10Cutoff = Math.max(1, Math.ceil(totalAlive * 0.1));
  const bottom10Cutoff = Math.max(0, totalAlive - Math.ceil(totalAlive * 0.1));

  const sorted = [...rows].sort((a, b) => {
    const av = a[sortKey];
    const bv = b[sortKey];
    if (typeof av === "number" && typeof bv === "number") {
      return sortAsc ? av - bv : bv - av;
    }
    return sortAsc
      ? String(av).localeCompare(String(bv))
      : String(bv).localeCompare(String(av));
  });

  const toggleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortAsc(!sortAsc);
    } else {
      setSortKey(key);
      setSortAsc(key === "rank" ? true : false);
    }
  };

  const arrow = (key: SortKey) => {
    if (sortKey !== key) return null;
    return <span className="ml-0.5 opacity-60">{sortAsc ? "\u2191" : "\u2193"}</span>;
  };

  const pnlColor = (v: number) => v >= 0 ? "text-emerald-400" : "text-red-400";
  const pnlFmt = (v: number) => `${v >= 0 ? "+" : ""}$${v.toFixed(2)}`;

  const thCls =
    "text-left text-[10px] font-bold uppercase tracking-wider text-slate-500 pb-2 pr-3 cursor-pointer hover:text-slate-300 transition-colors whitespace-nowrap select-none";

  return (
    <div className="overflow-x-auto premium-scrollbar">
      {rows.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-16 text-slate-600">
          <span className="text-4xl mb-3">&#x1F9EC;</span>
          <p className="text-sm font-medium">No active strategies</p>
          <p className="text-xs mt-1">Run an evolution cycle to populate the leaderboard</p>
        </div>
      ) : (
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-800/50">
              <th className={thCls} onClick={() => toggleSort("rank")}>
                # {arrow("rank")}
              </th>
              <th className={thCls} onClick={() => toggleSort("name")}>
                Strategy {arrow("name")}
              </th>
              <th className={thCls} onClick={() => toggleSort("market")}>
                Market {arrow("market")}
              </th>
              <th className={thCls} onClick={() => toggleSort("equity")}>
                Equity {arrow("equity")}
              </th>
              <th className={thCls} onClick={() => toggleSort("unrealized_pnl")}>
                Unreal {arrow("unrealized_pnl")}
              </th>
              <th className={thCls} onClick={() => toggleSort("realized_pnl")}>
                Real {arrow("realized_pnl")}
              </th>
              <th className={thCls} onClick={() => toggleSort("total_pnl")}>
                Total PnL {arrow("total_pnl")}
              </th>
              <th className={thCls} onClick={() => toggleSort("open_positions")}>
                Pos {arrow("open_positions")}
              </th>
              <th className={thCls} onClick={() => toggleSort("sharpe")}>
                Sharpe {arrow("sharpe")}
              </th>
              <th className={thCls} onClick={() => toggleSort("win_rate")}>
                Win % {arrow("win_rate")}
              </th>
              <th className={thCls} onClick={() => toggleSort("trades")}>
                Trades {arrow("trades")}
              </th>
              <th className={thCls} onClick={() => toggleSort("fees")}>
                Fees {arrow("fees")}
              </th>
              <th className={thCls} onClick={() => toggleSort("age_hours")}>
                Age {arrow("age_hours")}
              </th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((row) => {
              const isTop = row.rank <= top10Cutoff;
              const isBottom = row.rank > bottom10Cutoff;
              const rowGlow = isTop
                ? "shadow-[inset_0_0_20px_rgba(52,211,153,0.06)] border-l-2 border-l-emerald-500/40"
                : isBottom
                ? "shadow-[inset_0_0_20px_rgba(248,113,113,0.06)] border-l-2 border-l-red-500/40"
                : "";

              const equity = row.equity ?? 100;
              const unrealPnl = row.unrealized_pnl ?? 0;
              const realPnl = row.realized_pnl ?? 0;
              const totalPnl = row.total_pnl ?? row.pnl ?? 0;
              const openPos = row.open_positions ?? 0;
              const fees = row.fees ?? 0;

              return (
                <tr
                  key={row.id || row.name}
                  className={`border-b border-slate-800/30 hover:bg-slate-800/20 transition-colors ${rowGlow}`}
                >
                  <td className="py-2.5 pr-3 font-mono text-slate-400 font-bold text-xs">
                    #{row.rank}
                  </td>
                  <td className="py-2.5 pr-3 font-medium text-white whitespace-nowrap">
                    <Link
                      href={`/evolution/${encodeURIComponent(row.name)}`}
                      className="mr-1.5 text-emerald-400 hover:text-emerald-300 hover:underline transition-colors"
                    >
                      {row.name}
                    </Link>
                    {row.generation > 1 && (
                      <span className="text-[9px] font-bold px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-400 border border-blue-500/20">
                        v{row.generation}
                      </span>
                    )}
                  </td>
                  <td className="py-2.5 pr-3">
                    <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-slate-800/60 text-slate-400 border border-slate-700/40 whitespace-nowrap">
                      {marketLabel(row.market)}
                    </span>
                  </td>
                  <td className="py-2.5 pr-3 font-mono font-semibold text-white">
                    ${equity.toFixed(2)}
                  </td>
                  <td className={`py-2.5 pr-3 font-mono text-xs ${pnlColor(unrealPnl)}`}>
                    {pnlFmt(unrealPnl)}
                  </td>
                  <td className={`py-2.5 pr-3 font-mono text-xs ${pnlColor(realPnl)}`}>
                    {pnlFmt(realPnl)}
                  </td>
                  <td className={`py-2.5 pr-3 font-mono font-semibold ${pnlColor(totalPnl)}`}>
                    {pnlFmt(totalPnl)}
                  </td>
                  <td className="py-2.5 pr-3 font-mono text-slate-300 text-center">
                    {openPos}
                  </td>
                  <td className={`py-2.5 pr-3 font-mono font-bold ${row.sharpe >= 0 ? "text-emerald-400" : "text-red-400"}`}>
                    {row.sharpe.toFixed(2)}
                  </td>
                  <td className="py-2.5 pr-3 font-mono text-slate-300">
                    {(row.win_rate * 100).toFixed(1)}%
                  </td>
                  <td className="py-2.5 pr-3 font-mono text-slate-300">
                    {row.trades}
                  </td>
                  <td className="py-2.5 pr-3 font-mono text-slate-500 text-xs">
                    ${fees.toFixed(2)}
                  </td>
                  <td className="py-2.5 pr-3 font-mono text-slate-500 text-xs">
                    {row.age_hours.toFixed(1)}h
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

function TimelineFeed({ events }: { events: TimelineEvent[] }) {
  if (events.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-slate-600">
        <span className="text-4xl mb-3">📜</span>
        <p className="text-sm font-medium">No evolution events yet</p>
        <p className="text-xs mt-1">Events appear as strategies evolve</p>
      </div>
    );
  }

  return (
    <div className="space-y-2 overflow-y-auto premium-scrollbar pr-1" style={{ maxHeight: "calc(100vh - 300px)" }}>
      {events.map((ev, i) => {
        const isKilled = ev.type === "KILLED";
        const isSpawned = ev.type === "SPAWNED";

        const icon = isKilled ? "🔴" : isSpawned ? "🟢" : "🔵";
        const textColor = isKilled
          ? "text-red-400"
          : isSpawned
          ? "text-emerald-400"
          : "text-blue-400";
        const bg = isKilled
          ? "bg-red-500/5 border-red-500/20"
          : isSpawned
          ? "bg-emerald-500/5 border-emerald-500/20"
          : "bg-blue-500/5 border-blue-500/20";

        return (
          <div
            key={i}
            className={`flex items-start gap-3 px-3 py-2.5 rounded-xl border ${bg} animate-fade-in-right`}
          >
            <span className="text-base mt-0.5 shrink-0">{icon}</span>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2 flex-wrap">
                <span className={`text-[10px] font-black uppercase tracking-widest ${textColor}`}>
                  {ev.type}
                </span>
                <span className="text-xs font-semibold text-white truncate">
                  {ev.strategy_name}
                </span>
                <span className="text-[10px] text-slate-600 font-mono ml-auto shrink-0">
                  h{ev.hour}
                </span>
              </div>
              <p className="text-[11px] text-slate-500 mt-0.5 leading-relaxed">
                {ev.details}
              </p>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function GraveyardTable({ rows }: { rows: GraveyardEntry[] }) {
  if (rows.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-slate-700">
        <span className="text-4xl mb-3">⚰️</span>
        <p className="text-sm font-medium">Graveyard is empty</p>
        <p className="text-xs mt-1">No strategies have been eliminated yet</p>
      </div>
    );
  }

  return (
    <div className="overflow-x-auto premium-scrollbar">
      <table className="w-full text-sm opacity-60">
        <thead>
          <tr className="border-b border-slate-800/30">
            {["Strategy", "Market", "Lifetime", "Trades", "Final PnL", "Win %", "Sharpe", "Cause of Death"].map((h) => (
              <th
                key={h}
                className="text-left text-[10px] font-bold uppercase tracking-wider text-slate-600 pb-2 pr-3 whitespace-nowrap"
              >
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={i} className="border-b border-slate-800/20 hover:opacity-80 transition-opacity">
              <td className="py-2.5 pr-3 font-medium text-slate-500 line-through">
                {row.name}
              </td>
              <td className="py-2.5 pr-3">
                <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-slate-800/40 text-slate-600 border border-slate-700/30 whitespace-nowrap">
                  {marketLabel(row.market)}
                </span>
              </td>
              <td className="py-2.5 pr-3 font-mono text-slate-600 text-xs">
                {row.lifetime_hours.toFixed(1)}h
              </td>
              <td className="py-2.5 pr-3 font-mono text-slate-600">
                {row.trades}
              </td>
              <td className={`py-2.5 pr-3 font-mono font-semibold ${row.final_pnl >= 0 ? "text-emerald-700" : "text-red-700"}`}>
                {row.final_pnl >= 0 ? "+" : ""}${row.final_pnl.toFixed(2)}
              </td>
              <td className="py-2.5 pr-3 font-mono text-slate-600">
                {(row.win_rate * 100).toFixed(1)}%
              </td>
              <td className="py-2.5 pr-3 font-mono text-slate-600">
                {row.sharpe.toFixed(2)}
              </td>
              <td className="py-2.5 pr-3 text-slate-600 text-xs max-w-xs truncate" title={row.cause_of_death}>
                {row.cause_of_death}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ─── Main page ────────────────────────────────────────────────────────────────

export default function EvolutionPage() {
  const [triggerLoading, setTriggerLoading] = useState(false);
  const [triggerMsg, setTriggerMsg] = useState<string | null>(null);
  const [marketFilter, setMarketFilter] = useState<string>("all");
  const [pmSubFilter, setPmSubFilter] = useState<"all" | "ct" | "ai">("all");
  const [mcSubFilter, setMcSubFilter] = useState<"all" | "ed" | "tr" | "wc">("all");

  const { data: statsRaw } = useQuery({
    queryKey: ["evolution-stats"],
    queryFn: api.evolutionStats,
    refetchInterval: 30_000,
  });

  const { data: leaderboardRaw } = useQuery({
    queryKey: ["evolution-leaderboard"],
    queryFn: api.evolutionLeaderboard,
    refetchInterval: 30_000,
  });

  const { data: timelineRaw } = useQuery({
    queryKey: ["evolution-timeline"],
    queryFn: api.evolutionTimeline,
    refetchInterval: 30_000,
  });

  const { data: graveyardRaw } = useQuery({
    queryKey: ["evolution-graveyard"],
    queryFn: api.evolutionGraveyard,
    refetchInterval: 30_000,
  });

  // Normalize API responses (handle missing data gracefully)
  const stats: EvolutionStats = statsRaw ?? PLACEHOLDER_STATS;
  const leaderboard: LeaderboardEntry[] =
    Array.isArray(leaderboardRaw)
      ? leaderboardRaw
      : Array.isArray(leaderboardRaw?.strategies)
      ? leaderboardRaw.strategies
      : PLACEHOLDER_LEADERBOARD;

  const timeline: TimelineEvent[] =
    Array.isArray(timelineRaw)
      ? timelineRaw
      : Array.isArray(timelineRaw?.events)
      ? timelineRaw.events
      : PLACEHOLDER_TIMELINE;

  const graveyard: GraveyardEntry[] =
    Array.isArray(graveyardRaw)
      ? graveyardRaw
      : Array.isArray(graveyardRaw?.strategies)
      ? graveyardRaw.strategies
      : PLACEHOLDER_GRAVEYARD;

  // PM sub-filter counts
  const pmStrategies = leaderboard.filter((s) => s.market === "polymarket");
  const ctCount = pmStrategies.filter((s) => s.name.includes("-CT-")).length;
  const aiCount = pmStrategies.filter((s) => s.name.includes("-AI-")).length;

  // MC sub-filter counts (filter by name prefix since market field says "crypto")
  const mcStrategies = leaderboard.filter((s) => s.name?.startsWith("MC-"));
  const mcEdCount = mcStrategies.filter((s) => s.name?.includes("-ED-")).length;
  const mcTrCount = mcStrategies.filter((s) => s.name?.includes("-TR-")).length;
  const mcWcCount = mcStrategies.filter((s) => s.name?.includes("-WC-")).length;

  // Apply market filter (+ PM/MC sub-filter) to leaderboard
  const filteredLeaderboard = leaderboard.filter((s) => {
    if (marketFilter === "memecoins") {
      if (!s.name?.startsWith("MC-")) return false;
      if (mcSubFilter !== "all") {
        if (mcSubFilter === "ed" && !s.name.includes("-ED-")) return false;
        if (mcSubFilter === "tr" && !s.name.includes("-TR-")) return false;
        if (mcSubFilter === "wc" && !s.name.includes("-WC-")) return false;
      }
      return true;
    }
    if (marketFilter !== "all" && s.market !== marketFilter) return false;
    if (marketFilter === "polymarket" && pmSubFilter !== "all") {
      if (pmSubFilter === "ct" && !s.name.includes("-CT-")) return false;
      if (pmSubFilter === "ai" && !s.name.includes("-AI-")) return false;
    }
    return true;
  });

  const handleTrigger = async () => {
    setTriggerLoading(true);
    setTriggerMsg(null);
    try {
      await api.evolutionTrigger();
      setTriggerMsg("Evolution cycle triggered!");
    } catch {
      setTriggerMsg("Failed to trigger evolution");
    } finally {
      setTriggerLoading(false);
      setTimeout(() => setTriggerMsg(null), 4000);
    }
  };

  return (
    <div className="min-h-screen bg-slate-950">
      {/* Background ambient gradient */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute top-0 left-1/4 w-96 h-96 bg-emerald-500/3 rounded-full blur-3xl" />
        <div className="absolute top-1/3 right-1/4 w-80 h-80 bg-purple-500/3 rounded-full blur-3xl" />
        <div className="absolute bottom-0 right-1/3 w-96 h-96 bg-blue-500/3 rounded-full blur-3xl" />
      </div>

      <div className="relative max-w-[1600px] mx-auto px-6 py-6 space-y-6">

        {/* ── Page Header ───────────────────────────────────────────────────── */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-black text-white tracking-tight">
              Strategy Evolution
            </h1>
            <p className="text-xs text-slate-500 mt-1">
              Genetic algorithm — strategies compete, adapt, and reproduce
            </p>
          </div>
          <div className="flex items-center gap-3">
            {triggerMsg && (
              <span className="text-xs font-semibold text-emerald-400 animate-fade-in-right">
                {triggerMsg}
              </span>
            )}
            <button
              onClick={handleTrigger}
              disabled={triggerLoading}
              className="px-5 py-2.5 text-xs font-semibold bg-gradient-to-r from-emerald-500/15 to-emerald-400/5 border border-emerald-500/25 hover:border-emerald-500/40 text-emerald-400 rounded-xl transition-all duration-300 hover:-translate-y-0.5 hover:shadow-[0_0_20px_rgba(52,211,153,0.1)] disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:translate-y-0"
            >
              {triggerLoading ? "Running…" : "Trigger Evolution"}
            </button>
          </div>
        </div>

        {/* ── Stats Bar ─────────────────────────────────────────────────────── */}
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
          <StatCard
            label="Alive"
            value={stats.alive_count}
            accent="emerald"
            sub="active strategies"
          />
          <StatCard
            label="Dead"
            value={stats.dead_count}
            accent="red"
            sub="eliminated"
          />
          <StatCard
            label="Total Capital"
            value={`$${(stats.total_capital ?? 0).toLocaleString(undefined, { maximumFractionDigits: 0 })}`}
            accent="blue"
            sub="across all strategies"
          />
          <StatCard
            label="Avg Sharpe"
            value={(stats.avg_sharpe ?? 0).toFixed(2)}
            accent={(stats.avg_sharpe ?? 0) >= 0 ? "emerald" : "red"}
            sub="population average"
          />
          <StatCard
            label="Best Strategy"
            value={stats.best_strategy || "—"}
            accent="default"
            sub="top performer"
          />
          <StatCard
            label="Hours Running"
            value={`${(stats.hours_running ?? 0).toFixed(1)}h`}
            accent="default"
            sub="since genesis"
          />
        </div>

        {/* ── Middle Row: Leaderboard + Timeline ───────────────────────────── */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">

          {/* Leaderboard */}
          <div className="lg:col-span-8 bg-slate-900/60 border border-slate-800/50 rounded-2xl backdrop-blur-sm p-6">
            <div className="flex items-center justify-between mb-5">
              <h2 className="text-[10px] font-bold uppercase tracking-widest text-slate-500 flex items-center gap-2">
                <span className="w-1 h-4 rounded-full bg-gradient-to-b from-emerald-400 to-emerald-600" />
                Leaderboard — Alive Strategies
              </h2>
              <span className="text-[10px] font-mono text-slate-600">
                {filteredLeaderboard.length} strategies
              </span>
            </div>

            {/* Market filter bar */}
            <div className="flex flex-wrap items-center gap-2 mb-4">
              {(["all", "polymarket", "crypto_spot", "crypto_perps", "memecoins"] as const).map((m) => (
                <button
                  key={m}
                  onClick={() => {
                    setMarketFilter(m);
                    if (m !== "polymarket") setPmSubFilter("all");
                    if (m !== "memecoins") setMcSubFilter("all");
                  }}
                  className={`px-3 py-1.5 text-xs font-medium rounded-lg transition-all ${
                    marketFilter === m
                      ? "bg-emerald-400/15 text-emerald-300 border border-emerald-400/30"
                      : "text-slate-500 hover:text-slate-300 bg-slate-800/30"
                  }`}
                >
                  {m === "all"
                    ? "All Markets"
                    : m === "polymarket"
                    ? "Polymarket"
                    : m === "crypto_spot"
                    ? "Crypto Spot"
                    : m === "crypto_perps"
                    ? "Crypto Perps"
                    : "Meme Coins"}
                  <span className="ml-1 opacity-60">
                    {m === "all"
                      ? leaderboard.length
                      : m === "memecoins"
                      ? mcStrategies.length
                      : leaderboard.filter((s) => s.market === m).length}
                  </span>
                </button>
              ))}

              {/* PM sub-tabs — visible only when Polymarket is selected */}
              {marketFilter === "polymarket" && (
                <div className="flex items-center gap-1 ml-4 pl-4 border-l border-slate-700/50">
                  <button
                    onClick={() => setPmSubFilter("all")}
                    className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
                      pmSubFilter === "all"
                        ? "bg-blue-400/15 text-blue-300 border border-blue-400/30"
                        : "text-slate-500 hover:text-slate-300"
                    }`}
                  >
                    All PM <span className="ml-1 text-[10px] opacity-60">{pmStrategies.length}</span>
                  </button>
                  <button
                    onClick={() => setPmSubFilter("ct")}
                    className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
                      pmSubFilter === "ct"
                        ? "bg-emerald-400/15 text-emerald-300 border border-emerald-400/30"
                        : "text-slate-500 hover:text-slate-300"
                    }`}
                  >
                    Copy Trading <span className="ml-1 text-[10px] opacity-60">{ctCount}</span>
                  </button>
                  <button
                    onClick={() => setPmSubFilter("ai")}
                    className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
                      pmSubFilter === "ai"
                        ? "bg-purple-400/15 text-purple-300 border border-purple-400/30"
                        : "text-slate-500 hover:text-slate-300"
                    }`}
                  >
                    AI Niche <span className="ml-1 text-[10px] opacity-60">{aiCount}</span>
                  </button>
                </div>
              )}

              {/* MC sub-tabs — visible only when Meme Coins is selected */}
              {marketFilter === "memecoins" && (
                <div className="flex items-center gap-1 ml-4 pl-4 border-l border-slate-700/50">
                  <button
                    onClick={() => setMcSubFilter("all")}
                    className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
                      mcSubFilter === "all"
                        ? "bg-pink-400/15 text-pink-300 border border-pink-400/30"
                        : "text-slate-500 hover:text-slate-300"
                    }`}
                  >
                    All MC <span className="ml-1 text-[10px] opacity-60">{mcStrategies.length}</span>
                  </button>
                  <button
                    onClick={() => setMcSubFilter("ed")}
                    className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
                      mcSubFilter === "ed"
                        ? "bg-rose-400/15 text-rose-300 border border-rose-400/30"
                        : "text-slate-500 hover:text-slate-300"
                    }`}
                  >
                    Early Detection <span className="ml-1 text-[10px] opacity-60">{mcEdCount}</span>
                  </button>
                  <button
                    onClick={() => setMcSubFilter("tr")}
                    className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
                      mcSubFilter === "tr"
                        ? "bg-orange-400/15 text-orange-300 border border-orange-400/30"
                        : "text-slate-500 hover:text-slate-300"
                    }`}
                  >
                    Trend Riding <span className="ml-1 text-[10px] opacity-60">{mcTrCount}</span>
                  </button>
                  <button
                    onClick={() => setMcSubFilter("wc")}
                    className={`px-3 py-1 text-[11px] font-medium rounded-md transition-all ${
                      mcSubFilter === "wc"
                        ? "bg-purple-400/15 text-purple-300 border border-purple-400/30"
                        : "text-slate-500 hover:text-slate-300"
                    }`}
                  >
                    Whale Copy <span className="ml-1 text-[10px] opacity-60">{mcWcCount}</span>
                  </button>
                </div>
              )}
            </div>

            <LeaderboardTable rows={filteredLeaderboard} totalAlive={stats.alive_count || filteredLeaderboard.length} />
          </div>

          {/* Timeline */}
          <div className="lg:col-span-4 bg-slate-900/60 border border-slate-800/50 rounded-2xl backdrop-blur-sm p-6 lg:sticky lg:top-4 lg:self-start">
            <div className="flex items-center justify-between mb-5">
              <h2 className="text-[10px] font-bold uppercase tracking-widest text-slate-500 flex items-center gap-2">
                <span className="w-1 h-4 rounded-full bg-gradient-to-b from-purple-400 to-blue-500" />
                Evolution Timeline
              </h2>
              <span className="text-[10px] font-mono text-slate-600">
                {timeline.length} events
              </span>
            </div>
            <TimelineFeed events={timeline} />
          </div>
        </div>

        {/* ── Graveyard ─────────────────────────────────────────────────────── */}
        <div className="bg-slate-900/60 border border-slate-800/50 rounded-2xl backdrop-blur-sm p-6">
          <div className="flex items-center justify-between mb-5">
            <h2 className="text-[10px] font-bold uppercase tracking-widest text-slate-500 flex items-center gap-2">
              <span className="w-1 h-4 rounded-full bg-gradient-to-b from-slate-500 to-slate-700" />
              Graveyard — Eliminated Strategies
            </h2>
            <span className="text-[10px] font-mono text-slate-600">
              {graveyard.length} dead
            </span>
          </div>
          <GraveyardTable rows={graveyard} />
        </div>

      </div>
    </div>
  );
}
