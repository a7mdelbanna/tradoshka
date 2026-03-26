"use client";
import { useMemo } from "react";

interface DailyPnl { date: string; pnl: number; }

function getColor(pnl: number, max: number): string {
  if (pnl === 0) return "bg-slate-800/50";
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

const DAYS = ["", "Mon", "", "Wed", "", "Fri", ""];

export function PLHeatmap({ data, year = new Date().getFullYear() }: { data: DailyPnl[]; year?: number }) {
  const { weeks, maxAbs, totalProfit, totalLoss } = useMemo(() => {
    const map = new Map(data.map((d) => [d.date, d.pnl]));
    const maxAbs = Math.max(...data.map((d) => Math.abs(d.pnl)), 1);
    const totalProfit = data.filter(d => d.pnl > 0).reduce((s, d) => s + d.pnl, 0);
    const totalLoss = data.filter(d => d.pnl < 0).reduce((s, d) => s + d.pnl, 0);
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
    return { weeks, maxAbs, totalProfit, totalLoss };
  }, [data, year]);

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-sm font-semibold text-white">Daily P&L</h3>
          <p className="text-xs text-slate-500 mt-0.5">{year} trading calendar</p>
        </div>
        <div className="flex gap-3 text-xs">
          <span className="text-emerald-400">+${totalProfit.toFixed(0)}</span>
          <span className="text-red-400">${totalLoss.toFixed(0)}</span>
        </div>
      </div>
      <div className="flex gap-0.5">
        <div className="flex flex-col gap-0.5 pr-1.5 pt-0">
          {DAYS.map((d, i) => (
            <div key={i} className="h-[13px] flex items-center">
              <span className="text-[9px] text-slate-600 leading-none">{d}</span>
            </div>
          ))}
        </div>
        <div className="overflow-x-auto flex-1">
          <div className="flex gap-[3px]">
            {weeks.map((w, wi) => (
              <div key={wi} className="flex flex-col gap-[3px]">
                {w.map((day, di) => (
                  <div key={di}
                    className={`w-[13px] h-[13px] rounded-[2px] transition-all duration-150 hover:scale-150 hover:z-10 cursor-pointer ${day ? getColor(day.pnl, maxAbs) : "bg-transparent"}`}
                    title={day ? `${day.date}: $${day.pnl.toFixed(2)}` : ""} />
                ))}
              </div>
            ))}
          </div>
        </div>
      </div>
      <div className="flex items-center justify-end gap-1.5 mt-3 text-[10px] text-slate-600">
        <span>Loss</span>
        {["bg-red-400", "bg-red-600/60", "bg-slate-800/50", "bg-emerald-600/60", "bg-emerald-400"].map((c, i) => (
          <div key={i} className={`w-[10px] h-[10px] rounded-[2px] ${c}`} />
        ))}
        <span>Profit</span>
      </div>
    </div>
  );
}
