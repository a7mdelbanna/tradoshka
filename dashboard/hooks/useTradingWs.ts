"use client";
import { useCallback, useState } from "react";
import { useWebSocket } from "./useWebSocket";
import type { TradeInfo } from "@/lib/types";

interface WalletUpdate {
  mode: string;
  balance: string;
  equity: string;
  unrealized_pnl: string;
  realized_pnl: string;
  drawdown_pct: string;
  open_positions: number;
}

export function useTradingWs() {
  const [wallet, setWallet] = useState<WalletUpdate | null>(null);
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
      setWallet(msg.data as unknown as WalletUpdate);
    } else if (msg.event === "new_trade" && msg.data) {
      setTrades(prev => [msg.data as unknown as TradeInfo, ...prev].slice(0, 100));
    }
  }, []);

  useWebSocket(WS_URL, handleMessage);

  return { wallet, trades, connected };
}
