"use client";
import type { TradeInfo } from "@/lib/types";

function inferMarket(trade: TradeInfo): string {
  if (trade.market) return trade.market.toLowerCase();
  if (trade.question && trade.question.includes("?")) return "polymarket";
  if (trade.symbol && trade.symbol.endsWith("USDT")) return "crypto";
  return "unknown";
}

function MarketBadge({ market }: { market: string }) {
  if (market === "polymarket") {
    return (
      <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-blue-400/10 text-blue-400 border border-blue-400/20">
        Polymarket
      </span>
    );
  }
  if (market === "crypto") {
    return (
      <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-amber-400/10 text-amber-400 border border-amber-400/20">
        Crypto
      </span>
    );
  }
  return (
    <span className="text-[10px] font-bold px-2 py-0.5 rounded-full bg-slate-400/10 text-slate-400 border border-slate-400/20">
      {market}
    </span>
  );
}

/** Map strategy_tier to a colour + emoji badge */
function TierBadge({ tier }: { tier?: string }) {
  if (!tier) return null;

  const map: Record<string, { emoji: string; cls: string }> = {
    Unproven: { emoji: "🔵", cls: "bg-blue-400/10 text-blue-400 border-blue-400/20" },
    Tested:   { emoji: "🟡", cls: "bg-amber-400/10 text-amber-400 border-amber-400/20" },
    Proven:   { emoji: "🟢", cls: "bg-emerald-400/10 text-emerald-400 border-emerald-400/20" },
  };
  const style = map[tier] ?? { emoji: "⚪", cls: "bg-slate-400/10 text-slate-400 border-slate-400/20" };

  return (
    <span className={`text-[10px] font-semibold px-2 py-0.5 rounded-md border ${style.cls}`}>
      {style.emoji}{tier}
    </span>
  );
}

/** Compute percentage change between two price strings */
function pricePct(entry: string, target: string): string {
  const e = parseFloat(entry);
  const t = parseFloat(target);
  if (!e || isNaN(t)) return "";
  const pct = ((t - e) / e) * 100;
  return `${pct >= 0 ? "+" : ""}${pct.toFixed(1)}%`;
}

interface TradeFeedProps {
  trades: TradeInfo[];
  marketFilter?: string;
}

export function TradeFeed({ trades, marketFilter }: TradeFeedProps) {
  const filtered =
    marketFilter && marketFilter !== "all"
      ? trades.filter((t) => inferMarket(t) === marketFilter)
      : trades;

  if (filtered.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500">
        <div className="text-center py-16">
          <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-slate-800/50 flex items-center justify-center">
            <span className="text-2xl opacity-30">{"📊"}</span>
          </div>
          <p className="text-base font-semibold text-slate-400 mb-1">No trades yet</p>
          <p className="text-xs text-slate-600">
            Trades will appear here when strategies fire signals
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-2.5 overflow-y-auto max-h-[600px] pr-1 premium-scrollbar">
      {filtered.map((trade, i) => {
        const isBuy = trade.side === "Buy";
        const time = new Date(trade.timestamp).toLocaleTimeString();
        const pnl = trade.pnl ? parseFloat(trade.pnl) : null;
        const market = inferMarket(trade);
        const hasRiskData =
          trade.stop_loss || trade.take_profit || trade.risk_amount || trade.reward_risk_ratio;

        return (
          <div
            key={trade.id || i}
            className="animate-fade-in-right relative flex glass-panel rounded-xl overflow-hidden hover:-translate-y-0.5 hover:shadow-lg transition-all duration-300 group"
            style={{ animationDelay: `${i * 50}ms` }}
          >
            {/* Left accent bar */}
            <div
              className={`w-1 shrink-0 ${
                isBuy
                  ? "bg-gradient-to-b from-emerald-400 to-emerald-600"
                  : "bg-gradient-to-b from-red-400 to-red-600"
              }`}
            />

            <div className="flex-1 p-3.5 space-y-2.5">
              {/* ── Row 1: BUY/SELL · direction · strategy · tier ·  market · time ── */}
              <div className="flex items-start justify-between gap-2 flex-wrap">
                <div className="flex items-center gap-1.5 flex-wrap">
                  <span
                    className={`text-[10px] font-black px-2 py-0.5 rounded-md ${
                      isBuy
                        ? "bg-emerald-400/15 text-emerald-400 border border-emerald-400/20"
                        : "bg-red-400/15 text-red-400 border border-red-400/20"
                    }`}
                  >
                    {trade.side.toUpperCase()}
                  </span>
                  <span
                    className={`text-[10px] font-semibold px-2 py-0.5 rounded-md ${
                      trade.direction === "Yes"
                        ? "bg-blue-400/10 text-blue-400"
                        : "bg-purple-400/10 text-purple-400"
                    }`}
                  >
                    {trade.direction}
                  </span>
                  {trade.strategy && (
                    <span className="text-[10px] font-medium px-2 py-0.5 rounded-md bg-slate-700/40 text-slate-400 border border-slate-600/30">
                      {trade.strategy}
                    </span>
                  )}
                  <TierBadge tier={trade.strategy_tier} />
                </div>
                <div className="flex items-center gap-2.5">
                  <MarketBadge market={market} />
                  <span className="text-[10px] text-slate-600 font-mono">{time}</span>
                </div>
              </div>

              {/* ── Row 2: Symbol / Question ── */}
              <p className="text-xs text-slate-200 leading-relaxed group-hover:text-white transition-colors duration-300">
                {trade.question || trade.symbol}
              </p>

              {/* ── Row 3: Shares & price ── */}
              <div className="flex items-center justify-between">
                <div className="flex gap-3 text-[10px] text-slate-500">
                  <span className="font-mono">{trade.shares} shares</span>
                  <span className="font-mono">@ ${trade.price}</span>
                </div>
                {pnl !== null && (
                  <span
                    className={`text-xs font-bold px-2 py-0.5 rounded-md ${
                      pnl >= 0
                        ? "text-emerald-400 bg-emerald-400/10 glow-emerald"
                        : "text-red-400 bg-red-400/10 glow-red"
                    }`}
                  >
                    {pnl >= 0 ? "+" : ""}${pnl.toFixed(2)}
                  </span>
                )}
              </div>

              {/* ── Thesis reasoning ── */}
              {trade.thesis_reasoning && (
                <div className="rounded-lg bg-slate-800/50 border border-slate-700/40 px-3 py-2">
                  <p className="text-[10px] italic text-slate-400 leading-relaxed">
                    &ldquo;{trade.thesis_reasoning}&rdquo;
                  </p>
                </div>
              )}

              {/* ── Close reason banner ── */}
              {trade.close_reason && (
                <div className="rounded-lg bg-red-400/8 border border-red-400/20 px-3 py-1.5">
                  <p className="text-[10px] font-semibold text-red-400">
                    Closed: {trade.close_reason}
                  </p>
                </div>
              )}

              {/* ── Stop / TP levels ── */}
              {hasRiskData && (
                <div className="rounded-lg bg-slate-800/40 border border-slate-700/30 px-3 py-2 space-y-1.5">
                  {/* SL + Trail row */}
                  {(trade.stop_loss || trade.trailing_stop) && (
                    <div className="flex flex-wrap gap-x-4 gap-y-1 text-[10px]">
                      {trade.stop_loss && (
                        <span className="flex items-center gap-1 text-red-400 font-mono">
                          <span>🔴</span>
                          <span className="text-slate-500">SL:</span>
                          <span>${trade.stop_loss}</span>
                          <span className="text-slate-600">({pricePct(trade.price, trade.stop_loss)})</span>
                        </span>
                      )}
                      {trade.trailing_stop && (
                        <span className="flex items-center gap-1 text-amber-400 font-mono">
                          <span>🟡</span>
                          <span className="text-slate-500">Trail:</span>
                          <span>${trade.trailing_stop}</span>
                        </span>
                      )}
                    </div>
                  )}

                  {/* TP + time stop row */}
                  {(trade.take_profit || trade.time_stop_hours != null) && (
                    <div className="flex flex-wrap gap-x-4 gap-y-1 text-[10px]">
                      {trade.take_profit && (
                        <span className="flex items-center gap-1 text-emerald-400 font-mono">
                          <span>🟢</span>
                          <span className="text-slate-500">TP:</span>
                          <span>${trade.take_profit}</span>
                          <span className="text-slate-600">({pricePct(trade.price, trade.take_profit)})</span>
                        </span>
                      )}
                      {trade.time_stop_hours != null && (
                        <span className="flex items-center gap-1 text-slate-400 font-mono">
                          <span>⏱</span>
                          <span>{trade.time_stop_hours}h</span>
                        </span>
                      )}
                    </div>
                  )}

                  {/* Risk + R:R row */}
                  {(trade.risk_amount || trade.reward_risk_ratio != null) && (
                    <div className="flex flex-wrap gap-x-4 gap-y-1 text-[10px] border-t border-slate-700/40 pt-1.5 mt-1">
                      {trade.risk_amount && (
                        <span className="flex items-center gap-1 text-slate-300 font-mono">
                          <span>💰</span>
                          <span className="text-slate-500">Risk:</span>
                          <span>${trade.risk_amount}</span>
                        </span>
                      )}
                      {trade.reward_risk_ratio != null && (
                        <span className="flex items-center gap-1 font-mono font-semibold text-blue-400">
                          <span>R:R</span>
                          <span>{trade.reward_risk_ratio.toFixed(1)}x</span>
                        </span>
                      )}
                    </div>
                  )}
                </div>
              )}

              {/* ── Edge / Strength (only when no risk data, avoids double clutter) ── */}
              {trade.edge > 0 && !hasRiskData && (
                <div className="flex gap-3 text-[10px]">
                  <span className="text-slate-500">
                    Edge: <span className="text-emerald-400/70 font-mono">{(trade.edge * 100).toFixed(1)}%</span>
                  </span>
                  <span className="text-slate-500">
                    Strength: <span className="text-blue-400/70 font-mono">{(trade.strength * 100).toFixed(0)}%</span>
                  </span>
                </div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
