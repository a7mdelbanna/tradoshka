"use client";
import { useEffect, useRef, useCallback } from "react";

export function useWebSocket(url: string, onMessage: (data: unknown) => void) {
  const wsRef = useRef<WebSocket | null>(null);
  const retriesRef = useRef(0);

  const connect = useCallback(() => {
    const ws = new WebSocket(url);
    ws.onopen = () => { retriesRef.current = 0; };
    ws.onmessage = (e) => { try { onMessage(JSON.parse(e.data)); } catch {} };
    ws.onclose = () => {
      if (retriesRef.current < 10) { retriesRef.current++; setTimeout(connect, 3000); }
    };
    ws.onerror = () => ws.close();
    wsRef.current = ws;
  }, [url, onMessage]);

  useEffect(() => { connect(); return () => wsRef.current?.close(); }, [connect]);
}
