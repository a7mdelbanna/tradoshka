"use client";
import { useMemo } from "react";

interface DailyPnl { date: string; pnl: number; }

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

export function PLHeatmap({ data, year = new Date().getFullYear() }: { data: DailyPnl[]; year?: number }) {
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
                <div key={di} className={`w-3 h-3 rounded-sm ${day ? getColor(day.pnl, maxAbs) : "bg-transparent"}`}
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
