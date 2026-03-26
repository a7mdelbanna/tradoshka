"use client";
import { createChart, ColorType, AreaSeries, type IChartApi } from "lightweight-charts";
import { useEffect, useRef } from "react";

interface Props { data: { time: string; value: number }[]; }

export function EquityCurve({ data }: Props) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!ref.current || data.length === 0) return;
    const chart = createChart(ref.current, {
      layout: { background: { type: ColorType.Solid, color: "transparent" }, textColor: "#94a3b8" },
      grid: { vertLines: { color: "#1e293b" }, horzLines: { color: "#1e293b" } },
      width: ref.current.clientWidth, height: 350,
      rightPriceScale: { borderColor: "#334155" },
      timeScale: { borderColor: "#334155" },
    });
    const series = chart.addSeries(AreaSeries, {
      lineColor: "#22c55e", topColor: "rgba(34,197,94,0.3)",
      bottomColor: "rgba(34,197,94,0.0)", lineWidth: 2,
    });
    series.setData(data);
    chart.timeScale().fitContent();
    const resize = () => { if (ref.current) chart.applyOptions({ width: ref.current.clientWidth }); };
    window.addEventListener("resize", resize);
    return () => { window.removeEventListener("resize", resize); chart.remove(); };
  }, [data]);

  return (
    <div className="w-full">
      <h3 className="text-sm font-medium text-slate-400 mb-3">Equity Curve</h3>
      <div ref={ref} className="w-full" />
    </div>
  );
}
