import { test } from "@playwright/test";

/* ── Mock Data ──────────────────────────────────────────────────────── */

const polymarketWallet = {
  mode: "Dry",
  balance: "85.00",
  equity: "98.00",
  unrealized_pnl: "13.00",
  realized_pnl: "5.50",
  drawdown_pct: "0.02",
  total_fees: "1.20",
  open_positions: 19,
  positions: [],
};

const cryptoWallet = {
  mode: "Dry",
  balance: "90.00",
  equity: "100.00",
  unrealized_pnl: "10.00",
  realized_pnl: "3.25",
  drawdown_pct: "0.015",
  total_fees: "0.80",
  open_positions: 10,
  positions: [],
};

const allWallet = {
  mode: "Dry",
  balance: "175.00",
  equity: "198.00",
  unrealized_pnl: "23.00",
  realized_pnl: "8.75",
  drawdown_pct: "0.018",
  total_fees: "2.00",
  open_positions: 29,
  positions: [],
};

const polyTrades = {
  trades: [
    { id: "pt1", timestamp: new Date().toISOString(), symbol: "", question: "Will Bitcoin hit $100k by 2026?", direction: "Yes", side: "Buy", shares: "50", price: "0.65", fee: "0.02", strategy: "AI Predictor", strength: 0.82, edge: 0.15, pnl: "+3.25", closed: false, market: "polymarket" },
    { id: "pt2", timestamp: new Date().toISOString(), symbol: "", question: "Will the Fed cut rates in March?", direction: "No", side: "Sell", shares: "30", price: "0.42", fee: "0.01", strategy: "Copy Trading", strength: 0.71, edge: 0.08, pnl: "-1.10", closed: false, market: "polymarket" },
  ],
  total: 2,
};

const cryptoTrades = {
  trades: [
    { id: "ct1", timestamp: new Date().toISOString(), symbol: "BTCUSDT", question: "BTCUSDT", direction: "Long", side: "Buy", shares: "0.002", price: "87500", fee: "0.15", strategy: "Arbitrage", strength: 0.9, edge: 0.12, pnl: "+5.00", closed: false, market: "crypto" },
    { id: "ct2", timestamp: new Date().toISOString(), symbol: "ETHUSDT", question: "ETHUSDT", direction: "Long", side: "Buy", shares: "0.05", price: "2050", fee: "0.08", strategy: "AI Predictor", strength: 0.85, edge: 0.1, pnl: "+2.30", closed: false, market: "crypto" },
  ],
  total: 2,
};

const cryptoAssets = {
  count: 5,
  assets: [
    { symbol: "BTCUSDT", price: "87543.21", volume_24h: 1200000000 },
    { symbol: "ETHUSDT", price: "2054.78", volume_24h: 800000000 },
    { symbol: "SOLUSDT", price: "142.35", volume_24h: 500000000 },
    { symbol: "AVAXUSDT", price: "23.67", volume_24h: 150000000 },
    { symbol: "DOGEUSDT", price: "0.0821", volume_24h: 300000000 },
  ],
};

const strategies = [
  { name: "AI Predictor", return_pct: 12.5, trades: 45, win_rate: 0.68 },
  { name: "Copy Trading", return_pct: 3.2, trades: 22, win_rate: 0.59 },
  { name: "Market Making", return_pct: -1.8, trades: 120, win_rate: 0.52 },
  { name: "Arbitrage", return_pct: 8.1, trades: 67, win_rate: 0.74 },
];

const readiness = {
  criteria: [
    { name: "Win Rate > 55%", threshold: "55%", current_value: "68%", passed: true },
    { name: "Profit Factor > 1.2", threshold: "1.2", current_value: "1.85", passed: true },
    { name: "Max Drawdown < 15%", threshold: "15%", current_value: "1.8%", passed: true },
    { name: "Min 30 Trades", threshold: "30", current_value: "45", passed: true },
    { name: "Sharpe > 1.0", threshold: "1.0", current_value: "1.42", passed: true },
  ],
  passed: 5,
  total: 5,
  is_ready: true,
};

const orchestratorStatus = {
  cycle_count: 142,
  last_cycle_at: new Date().toISOString(),
  wallet_mode: "Dry",
  tracked_markets: 34,
  open_positions: 29,
};

/* ── Route Handler ──────────────────────────────────────────────────── */

async function mockAPIs(page: import("@playwright/test").Page) {
  await page.route("**/api/**", async (route) => {
    const url = route.request().url();

    if (url.includes("/api/wallet/polymarket")) return route.fulfill({ json: polymarketWallet });
    if (url.includes("/api/wallet/crypto")) return route.fulfill({ json: cryptoWallet });
    if (url.includes("/api/wallet")) return route.fulfill({ json: allWallet });

    if (url.includes("/api/trades/polymarket")) return route.fulfill({ json: polyTrades });
    if (url.includes("/api/trades/crypto")) return route.fulfill({ json: cryptoTrades });
    if (url.includes("/api/trades/live")) return route.fulfill({ json: { trades: [...polyTrades.trades, ...cryptoTrades.trades], total: 4 } });

    if (url.includes("/api/crypto/assets")) return route.fulfill({ json: cryptoAssets });
    if (url.includes("/api/strategies")) return route.fulfill({ json: strategies });
    if (url.includes("/api/readiness")) return route.fulfill({ json: readiness });
    if (url.includes("/api/orchestrator")) return route.fulfill({ json: orchestratorStatus });
    if (url.includes("/api/equity-curve")) return route.fulfill({ json: [] });
    if (url.includes("/api/pnl/daily")) return route.fulfill({ json: [] });

    return route.fulfill({ json: {} });
  });

  // Mock WebSocket so the page shows as connected and receives wallet data
  await page.addInitScript(() => {
    const origWS = window.WebSocket;
    class MockWS extends EventTarget {
      readyState = 1;
      url: string;
      onopen: ((ev: Event) => void) | null = null;
      onmessage: ((ev: MessageEvent) => void) | null = null;
      onclose: ((ev: Event) => void) | null = null;
      onerror: ((ev: Event) => void) | null = null;
      constructor(url: string) {
        super();
        this.url = url;
        setTimeout(() => {
          if (this.onopen) this.onopen(new Event("open"));
          // Send wallet updates for each market
          const walletAll = { event: "wallet_update", data: { mode: "Dry", balance: "175.00", equity: "198.00", unrealized_pnl: "23.00", realized_pnl: "8.75", drawdown_pct: "0.018", open_positions: 29, market: "all" } };
          const walletPoly = { event: "wallet_update", data: { mode: "Dry", balance: "85.00", equity: "98.00", unrealized_pnl: "13.00", realized_pnl: "5.50", drawdown_pct: "0.02", open_positions: 19, market: "polymarket" } };
          const walletCrypto = { event: "wallet_update", data: { mode: "Dry", balance: "90.00", equity: "100.00", unrealized_pnl: "10.00", realized_pnl: "3.25", drawdown_pct: "0.015", open_positions: 10, market: "crypto" } };
          if (this.onmessage) {
            this.onmessage(new MessageEvent("message", { data: JSON.stringify(walletAll) }));
            this.onmessage(new MessageEvent("message", { data: JSON.stringify(walletPoly) }));
            this.onmessage(new MessageEvent("message", { data: JSON.stringify(walletCrypto) }));

            // Send some trades too
            const trades = [
              { event: "new_trade", data: { id: "pt1", timestamp: new Date().toISOString(), symbol: "", question: "Will Bitcoin hit $100k by 2026?", direction: "Yes", side: "Buy", shares: "50", price: "0.65", fee: "0.02", strategy: "AI Predictor", strength: 0.82, edge: 0.15, pnl: "+3.25", closed: false, market: "polymarket" } },
              { event: "new_trade", data: { id: "pt2", timestamp: new Date().toISOString(), symbol: "", question: "Will the Fed cut rates in March?", direction: "No", side: "Sell", shares: "30", price: "0.42", fee: "0.01", strategy: "Copy Trading", strength: 0.71, edge: 0.08, pnl: "-1.10", closed: false, market: "polymarket" } },
              { event: "new_trade", data: { id: "ct1", timestamp: new Date().toISOString(), symbol: "BTCUSDT", question: "BTCUSDT", direction: "Long", side: "Buy", shares: "0.002", price: "87500", fee: "0.15", strategy: "Arbitrage", strength: 0.9, edge: 0.12, pnl: "+5.00", closed: false, market: "crypto" } },
              { event: "new_trade", data: { id: "ct2", timestamp: new Date().toISOString(), symbol: "ETHUSDT", question: "ETHUSDT", direction: "Long", side: "Buy", shares: "0.05", price: "2050", fee: "0.08", strategy: "AI Predictor", strength: 0.85, edge: 0.1, pnl: "+2.30", closed: false, market: "crypto" } },
            ];
            for (const t of trades) {
              this.onmessage(new MessageEvent("message", { data: JSON.stringify(t) }));
            }
          }
        }, 100);
      }
      close() { this.readyState = 3; }
      send() {}
    }
    // @ts-ignore
    window.WebSocket = MockWS as any;
  });
}

/* ── Tests ──────────────────────────────────────────────────────────── */

test.describe("Trading Dashboard Screenshots", () => {
  test("Polymarket tab selected", async ({ page }) => {
    await mockAPIs(page);
    await page.goto("/trading");
    await page.waitForTimeout(1500);

    // Click the Polymarket tab
    await page.getByText("Polymarket").first().click();
    await page.waitForTimeout(1000);

    await page.screenshot({
      path: "e2e/screenshots/trading-polymarket.png",
      fullPage: true,
    });
  });

  test("Crypto tab selected", async ({ page }) => {
    await mockAPIs(page);
    await page.goto("/trading");
    await page.waitForTimeout(1500);

    // Click the Crypto tab
    await page.getByText("Crypto").first().click();
    await page.waitForTimeout(1000);

    await page.screenshot({
      path: "e2e/screenshots/trading-crypto.png",
      fullPage: true,
    });
  });
});
