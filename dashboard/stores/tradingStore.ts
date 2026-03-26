import { create } from "zustand";

interface TradingState {
  equity: number;
  balance: number;
  drawdownPct: number;
  openPositions: number;
  connected: boolean;
  updateFromWs: (data: { equity: string; balance: string; drawdown_pct: string; open_positions: number }) => void;
  setConnected: (c: boolean) => void;
}

export const useTradingStore = create<TradingState>((set) => ({
  equity: 0, balance: 0, drawdownPct: 0, openPositions: 0, connected: false,
  updateFromWs: (data) => set({
    equity: parseFloat(data.equity) || 0,
    balance: parseFloat(data.balance) || 0,
    drawdownPct: parseFloat(data.drawdown_pct) || 0,
    openPositions: data.open_positions,
  }),
  setConnected: (connected) => set({ connected }),
}));
