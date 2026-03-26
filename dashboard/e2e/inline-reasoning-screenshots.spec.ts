import { test } from "@playwright/test";

/* ── Rich mock trades with ALL new fields ─────────────────────────── */

const richTrades = [
  {
    id: "rt1",
    timestamp: new Date().toISOString(),
    symbol: "DOGEUSDT",
    question: "DOGEUSDT Spot",
    direction: "Long",
    side: "Buy",
    shares: "20",
    price: "0.0913",
    fee: "0.01",
    strategy: "research",
    strength: 0.65,
    edge: 0.12,
    pnl: null,
    closed: false,
    market: "crypto",
    thesis_reasoning:
      "EMA9 above EMA21 (bullish trend). RSI above 50 confirming bullish momentum. 2/3 signals. Conf: 65%",
    stop_loss: "0.0876",
    trailing_stop: "0.0885",
    take_profit: "0.0986",
    time_stop_hours: 24,
    risk_amount: "1.00",
    reward_risk_ratio: 2.0,
    strategy_tier: "Unproven",
    close_reason: null,
  },
  {
    id: "rt2",
    timestamp: new Date().toISOString(),
    symbol: "BTCUSDT",
    question: "BTCUSDT",
    direction: "Long",
    side: "Buy",
    shares: "0.002",
    price: "87500",
    fee: "0.15",
    strategy: "AI Predictor",
    strength: 0.9,
    edge: 0.12,
    pnl: "+5.00",
    closed: false,
    market: "crypto",
    thesis_reasoning:
      "Strong breakout above $87k resistance. Volume confirms. MACD crossover bullish. Conf: 90%",
    stop_loss: "85200",
    trailing_stop: "86000",
    take_profit: "91000",
    time_stop_hours: 48,
    risk_amount: "4.60",
    reward_risk_ratio: 1.52,
    strategy_tier: "Tested",
    close_reason: null,
  },
  {
    id: "rt3",
    timestamp: new Date().toISOString(),
    symbol: "ETHUSDT",
    question: "ETHUSDT",
    direction: "Long",
    side: "Buy",
    shares: "0.05",
    price: "2050",
    fee: "0.08",
    strategy: "Arbitrage",
    strength: 0.85,
    edge: 0.1,
    pnl: "-2.10",
    closed: true,
    market: "crypto",
    thesis_reasoning:
      "Spread between spot and perp exceeded 0.3%. Expected mean reversion within 12h.",
    stop_loss: "1980",
    trailing_stop: null,
    take_profit: "2120",
    time_stop_hours: 12,
    risk_amount: "3.50",
    reward_risk_ratio: 1.0,
    strategy_tier: "Proven",
    close_reason: "Hard stop hit at $1980",
  },
  {
    id: "rt4",
    timestamp: new Date().toISOString(),
    symbol: "",
    question: "Will Bitcoin hit $100k by 2026?",
    direction: "Yes",
    side: "Buy",
    shares: "50",
    price: "0.65",
    fee: "0.02",
    strategy: "AI Predictor",
    strength: 0.82,
    edge: 0.15,
    pnl: "+3.25",
    closed: false,
    market: "polymarket",
    thesis_reasoning:
      "Historical cycles suggest BTC peaks at end of halving year. Current momentum strong. 3 signals aligned.",
    stop_loss: null,
    trailing_stop: null,
    take_profit: null,
    time_stop_hours: 168,
    risk_amount: "5.00",
    reward_risk_ratio: 3.5,
    strategy_tier: "Tested",
    close_reason: null,
  },
];

const allWallet = {
  mode: "Dry",
  balance: "175.00",
  equity: "198.00",
  unrealized_pnl: "23.00",
  realized_pnl: "8.75",
  drawdown_pct: "0.018",
  total_fees: "2.00",
  open_positions: 4,
  positions: [],
};

const strategies = [
  { name: "research",      return_pct: 0.0,  trades: 6,   win_rate: 0.0 },
  { name: "AI Predictor",  return_pct: 12.5, trades: 45,  win_rate: 0.68 },
  { name: "Arbitrage",     return_pct: 8.1,  trades: 67,  win_rate: 0.74 },
  { name: "Market Making", return_pct: -1.8, trades: 120, win_rate: 0.52 },
];

const readiness = {
  criteria: [
    { name: "Win Rate > 55%",      threshold: "55%", current_value: "68%", passed: true },
    { name: "Profit Factor > 1.2", threshold: "1.2", current_value: "1.85", passed: true },
    { name: "Max Drawdown < 15%",  threshold: "15%", current_value: "1.8%", passed: true },
    { name: "Min 30 Trades",       threshold: "30",  current_value: "45",   passed: true },
    { name: "Sharpe > 1.0",        threshold: "1.0", current_value: "1.42", passed: true },
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
  open_positions: 4,
};

async function mockAPIs(page: import("@playwright/test").Page) {
  await page.route("**/api/**", async (route) => {
    const url = route.request().url();

    if (url.includes("/api/wallet/polymarket")) return route.fulfill({ json: allWallet });
    if (url.includes("/api/wallet/crypto"))     return route.fulfill({ json: allWallet });
    if (url.includes("/api/wallet"))            return route.fulfill({ json: allWallet });

    if (url.includes("/api/trades/polymarket")) return route.fulfill({ json: { trades: richTrades.filter(t => t.market === "polymarket"), total: 1 } });
    if (url.includes("/api/trades/crypto"))     return route.fulfill({ json: { trades: richTrades.filter(t => t.market === "crypto"),     total: 3 } });
    if (url.includes("/api/trades/live"))       return route.fulfill({ json: { trades: richTrades, total: richTrades.length } });

    if (url.includes("/api/crypto/assets"))  return route.fulfill({ json: { count: 0, assets: [] } });
    if (url.includes("/api/strategies"))     return route.fulfill({ json: strategies });
    if (url.includes("/api/readiness"))      return route.fulfill({ json: readiness });
    if (url.includes("/api/orchestrator"))   return route.fulfill({ json: orchestratorStatus });
    if (url.includes("/api/equity-curve"))   return route.fulfill({ json: [] });
    if (url.includes("/api/pnl/daily"))      return route.fulfill({ json: [] });

    return route.fulfill({ json: {} });
  });

  await page.addInitScript((trades) => {
    class MockWS extends EventTarget {
      readyState = 1;
      url: string;
      onopen:    ((ev: Event) => void) | null = null;
      onmessage: ((ev: MessageEvent) => void) | null = null;
      onclose:   ((ev: Event) => void) | null = null;
      onerror:   ((ev: Event) => void) | null = null;
      constructor(url: string) {
        super();
        this.url = url;
        setTimeout(() => {
          if (this.onopen) this.onopen(new Event("open"));
          const walletMsg = { event: "wallet_update", data: { mode: "Dry", balance: "175.00", equity: "198.00", unrealized_pnl: "23.00", realized_pnl: "8.75", drawdown_pct: "0.018", open_positions: 4, market: "all" } };
          if (this.onmessage) {
            this.onmessage(new MessageEvent("message", { data: JSON.stringify(walletMsg) }));
            for (const t of trades) {
              this.onmessage(new MessageEvent("message", { data: JSON.stringify({ event: "new_trade", data: t }) }));
            }
          }
        }, 100);
      }
      close() { this.readyState = 3; }
      send()  {}
    }
    // @ts-ignore
    window.WebSocket = MockWS;
  }, richTrades);
}

/* ── Tests ─────────────────────────────────────────────────────────── */

test.describe("Inline Reasoning Screenshots", () => {
  test("All trades with reasoning visible", async ({ page }) => {
    await mockAPIs(page);
    await page.goto("/trading");
    await page.waitForTimeout(2000);

    await page.screenshot({
      path: "e2e/screenshots/inline-reasoning-all.png",
      fullPage: true,
    });
  });

  test("Crypto tab – stops and tiers", async ({ page }) => {
    await mockAPIs(page);
    await page.goto("/trading");
    await page.waitForTimeout(1500);

    await page.getByText("Crypto").first().click();
    await page.waitForTimeout(800);

    await page.screenshot({
      path: "e2e/screenshots/inline-reasoning-crypto.png",
      fullPage: true,
    });
  });

  test("Polymarket tab – risk info", async ({ page }) => {
    await mockAPIs(page);
    await page.goto("/trading");
    await page.waitForTimeout(1500);

    await page.getByText("Polymarket").first().click();
    await page.waitForTimeout(800);

    await page.screenshot({
      path: "e2e/screenshots/inline-reasoning-polymarket.png",
      fullPage: true,
    });
  });
});
