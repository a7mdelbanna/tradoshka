import { test } from "@playwright/test";

/* ── Mock Data ──────────────────────────────────────────────────────── */

const evolutionStats = {
  alive_count: 8,
  dead_count: 3,
  total_capital: 24750.0,
  avg_sharpe: 1.38,
  best_strategy: "MomentumV3",
  hours_running: 47.5,
};

const evolutionLeaderboard = [
  { rank: 1, id: "s1", name: "MomentumV3",   generation: 3, market: "crypto_spot",  sharpe: 2.41, pnl: 1248.5,  win_rate: 0.74, trades: 89,  age_hours: 38.2 },
  { rank: 2, id: "s2", name: "AlphaHunter",  generation: 2, market: "polymarket",   sharpe: 1.97, pnl: 875.0,   win_rate: 0.68, trades: 54,  age_hours: 45.1 },
  { rank: 3, id: "s3", name: "GridBot",      generation: 1, market: "crypto_perps", sharpe: 1.72, pnl: 620.25,  win_rate: 0.63, trades: 132, age_hours: 47.5 },
  { rank: 4, id: "s4", name: "EdgeFinder",   generation: 2, market: "polymarket",   sharpe: 1.54, pnl: 440.0,   win_rate: 0.61, trades: 41,  age_hours: 30.0 },
  { rank: 5, id: "s5", name: "TrendRider",   generation: 1, market: "crypto_spot",  sharpe: 1.38, pnl: 310.75,  win_rate: 0.59, trades: 78,  age_hours: 22.8 },
  { rank: 6, id: "s6", name: "ArbitrageX",   generation: 1, market: "crypto_perps", sharpe: 0.95, pnl: 125.0,   win_rate: 0.55, trades: 210, age_hours: 47.5 },
  { rank: 7, id: "s7", name: "MeanRevV2",    generation: 2, market: "crypto_spot",  sharpe: 0.51, pnl: -45.0,   win_rate: 0.50, trades: 33,  age_hours: 12.5 },
  { rank: 8, id: "s8", name: "NoisePicker",  generation: 1, market: "polymarket",   sharpe: -0.32, pnl: -210.5, win_rate: 0.43, trades: 67,  age_hours: 47.5 },
];

const evolutionTimeline = [
  { hour: 47, type: "KILLED",  strategy_name: "RandomWalk",  details: "Sharpe dropped below -0.5 threshold after 67 trades" },
  { hour: 46, type: "SPAWNED", strategy_name: "MomentumV3",  details: "Bred from MomentumV2 × AlphaHunter; mutation rate 0.12" },
  { hour: 42, type: "KILLED",  strategy_name: "OverfitBot",  details: "Win rate collapsed to 31% after 120 trades" },
  { hour: 38, type: "SPAWNED", strategy_name: "MeanRevV2",   details: "Evolved from MeanRevV1; doubled lookback window" },
  { hour: 30, type: "SPAWNED", strategy_name: "EdgeFinder",  details: "Cross-bred AlphaHunter × GridBot; new signal threshold" },
  { hour: 24, type: "KILLED",  strategy_name: "SlowMover",   details: "Max drawdown exceeded 25% — circuit breaker triggered" },
  { hour: 18, type: "SPAWNED", strategy_name: "AlphaHunter", details: "Generation 2 evolved from AlphaV1; sharpe improved 0.4" },
  { hour: 12, type: "SPAWNED", strategy_name: "TrendRider",  details: "Spawned from genesis pool with momentum parameters" },
];

const evolutionGraveyard = [
  { name: "RandomWalk",  market: "polymarket",   lifetime_hours: 47.5, trades: 67,  final_pnl: -210.5, win_rate: 0.43, sharpe: -0.32, cause_of_death: "Sharpe < -0.5 for 6 consecutive hours" },
  { name: "OverfitBot",  market: "crypto_spot",  lifetime_hours: 42.0, trades: 120, final_pnl: -180.0, win_rate: 0.31, sharpe: -1.21, cause_of_death: "Win rate collapse — overfit to stale data" },
  { name: "SlowMover",   market: "crypto_perps", lifetime_hours: 24.0, trades: 18,  final_pnl: -95.0,  win_rate: 0.44, sharpe: -0.67, cause_of_death: "Max drawdown 25% — risk circuit breaker" },
];

/* ── Route Handler ──────────────────────────────────────────────────── */

async function mockAPIs(page: import("@playwright/test").Page) {
  await page.route("**/api/**", async (route) => {
    const url = route.request().url();

    if (url.includes("/api/evolution/stats"))       return route.fulfill({ json: evolutionStats });
    if (url.includes("/api/evolution/leaderboard")) return route.fulfill({ json: evolutionLeaderboard });
    if (url.includes("/api/evolution/timeline"))    return route.fulfill({ json: evolutionTimeline });
    if (url.includes("/api/evolution/graveyard"))   return route.fulfill({ json: evolutionGraveyard });
    if (url.includes("/api/evolution/trigger"))     return route.fulfill({ json: { ok: true } });

    return route.fulfill({ json: {} });
  });
}

/* ── Tests ──────────────────────────────────────────────────────────── */

test.describe("Evolution Dashboard Screenshots", () => {
  test("evolution page full view", async ({ page }) => {
    await mockAPIs(page);
    await page.goto("/evolution");
    await page.waitForTimeout(1500);

    await page.screenshot({
      path: "e2e/screenshots/evolution-full.png",
      fullPage: true,
    });
  });
});
