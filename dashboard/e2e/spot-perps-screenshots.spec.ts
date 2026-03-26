import { test } from "@playwright/test";

test("Crypto > Spot tab screenshot", async ({ page }) => {
  await page.goto("/trading");
  // Click Crypto tab
  await page.getByRole("button", { name: /Crypto/i }).first().click();
  // Spot is the default sub-tab, just wait for content
  await page.waitForTimeout(1000);
  await page.screenshot({ path: "e2e/screenshots/crypto-spot.png", fullPage: false });
});

test("Crypto > Perpetuals tab screenshot", async ({ page }) => {
  await page.goto("/trading");
  // Click Crypto tab
  await page.getByRole("button", { name: /Crypto/i }).first().click();
  // Click Perpetuals sub-tab
  await page.getByRole("button", { name: /Perpetuals/i }).click();
  await page.waitForTimeout(1000);
  await page.screenshot({ path: "e2e/screenshots/crypto-perps.png", fullPage: false });
});
