"use client";
import { createChart, ColorType, type IChartApi, AreaSeries } from "lightweight-charts";
import { useEffect, useRef } from "react";

interface Props { data: { time: string; value: number }[]; }

export function EquityCurve({ data }: Props) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!ref.current || data.length === 0) return;
    const chart = createChart(ref.current, {
      layout: { background: { type: ColorType.Solid, color: "transparent" }, textColor: "#64748b", fontSize: 11 },
      grid: { vertLines: { color: "rgba(30,41,59,0.5)" }, horzLines: { color: "rgba(30,41,59,0.5)" } },
      width: ref.current.clientWidth, height: 380,
      rightPriceScale: { borderColor: "rgba(51,65,85,0.3)", scaleMargins: { top: 0.1, bottom: 0.1 } },
      timeScale: { borderColor: "rgba(51,65,85,0.3)", timeVisible: false },
      crosshair: {
        vertLine: { color: "rgba(52,211,153,0.3)", width: 1, style: 2, labelBackgroundColor: "#059669" },
        horzLine: { color: "rgba(52,211,153,0.3)", width: 1, style: 2, labelBackgroundColor: "#059669" },
      },
      handleScroll: { vertTouchDrag: false },
    });

    const series = chart.addSeries(AreaSeries, {
      lineColor: "#34d399",
      topColor: "rgba(52,211,153,0.25)",
      bottomColor: "rgba(52,211,153,0.0)",
      lineWidth: 2,
      crosshairMarkerBackgroundColor: "#34d399",
      crosshairMarkerBorderColor: "#059669",
      crosshairMarkerRadius: 5,
    });
    series.setData(data);
    chart.timeScale().fitContent();

    const resize = () => { if (ref.current) chart.applyOptions({ width: ref.current.clientWidth }); };
    window.addEventListener("resize", resize);
    return () => { window.removeEventListener("resize", resize); chart.remove(); };
  }, [data]);

  if (data.length === 0) {
    return (
      <div className="w-full">
        <div className="mb-4">
          <h3 className="text-sm font-semibold text-white">Equity Curve</h3>
          <p className="text-xs text-slate-500 mt-0.5">Portfolio value over time</p>
        </div>
        <div className="flex items-center justify-center h-[380px] text-slate-500 text-sm">
          No data yet
        </div>
      </div>
    );
  }

  return (
    <div className="w-full">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-sm font-semibold text-white">Equity Curve</h3>
          <p className="text-xs text-slate-500 mt-0.5">Portfolio value over time</p>
        </div>
        <div className="flex gap-1">
          {["1W", "1M", "3M", "ALL"].map(p => (
            <button key={p} className={`px-3 py-1 text-xs rounded-md transition-colors ${p === "1M" ? "bg-emerald-500/10 text-emerald-400 border border-emerald-500/20" : "text-slate-500 hover:text-slate-300 hover:bg-slate-800/50"}`}>
              {p}
            </button>
          ))}
        </div>
      </div>
      <div ref={ref} className="w-full rounded-lg overflow-hidden" />
    </div>
  );
}
