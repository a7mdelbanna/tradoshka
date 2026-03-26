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
