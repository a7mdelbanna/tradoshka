"use client";
import { useCallback, useState } from "react";
import { useWebSocket } from "./useWebSocket";
import type { TradeInfo } from "@/lib/types";

export interface WalletUpdate {
  mode: string;
  balance: string;
  equity: string;
  unrealized_pnl: string;
  realized_pnl: string;
  drawdown_pct: string;
  open_positions: number;
  market?: string;
}

export function useTradingWs() {
  const [wallets, setWallets] = useState<Record<string, WalletUpdate>>({});
  const [trades, setTrades] = useState<TradeInfo[]>([]);
  const [connected, setConnected] = useState(false);

  const WS_URL = process.env.NEXT_PUBLIC_WS_URL
    ? process.env.NEXT_PUBLIC_WS_URL.replace("/ws", "/ws/trading")
    : "ws://localhost:3001/ws/trading";

  const handleMessage = useCallback((data: unknown) => {
    const msg = data as { event?: string; data?: Record<string, unknown> };
    if (!msg.event) return;
    setConnected(true);

    if (msg.event === "wallet_update" && msg.data) {
      const update = msg.data as unknown as WalletUpdate;
      const market = update.market || (msg.data.market as string) || "all";
      setWallets(prev => ({ ...prev, [market]: update }));
    } else if (msg.event === "new_trade" && msg.data) {
      setTrades(prev => [msg.data as unknown as TradeInfo, ...prev].slice(0, 100));
    }
  }, []);

  useWebSocket(WS_URL, handleMessage);

  // Provide a combined wallet for backward compatibility
  const wallet = wallets["all"] || wallets["polymarket"] || wallets["crypto"] || null;

  return { wallet, wallets, trades, connected };
}
