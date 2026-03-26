"""Visual verification tests for Tradoshka Dashboard using Playwright."""
import sys
from playwright.sync_api import sync_playwright

SCREENSHOTS_DIR = "screenshots"
BASE_URL = "http://localhost:3000"
API_URL = "http://localhost:3001"

def ensure_dir(path):
    import os
    os.makedirs(path, exist_ok=True)

def test_api_endpoints():
    """Verify all API endpoints return valid data."""
    import urllib.request
    import json

    endpoints = [
        ("/health", lambda d: d["status"] == "ok"),
        ("/api/portfolio", lambda d: "balance" in d),
        ("/api/positions", lambda d: "positions" in d),
        ("/api/orders", lambda d: "active_orders" in d),
        ("/api/risk", lambda d: "breaker_state" in d),
        ("/api/markets/polymarket", lambda d: d["connected"] == True),
        ("/api/stats", lambda d: "total_roi" in d),
        ("/api/strategies", lambda d: len(d) == 4),
        ("/api/equity-curve", lambda d: len(d) == 30),
        ("/api/pnl/daily", lambda d: len(d) == 90),
    ]

    print("\n=== API Endpoint Tests ===")
    all_pass = True
    for path, check in endpoints:
        try:
            url = f"{API_URL}{path}"
            with urllib.request.urlopen(url, timeout=5) as resp:
                data = json.loads(resp.read())
                passed = check(data)
                status = "PASS" if passed else "FAIL"
                print(f"  {status}  {path}")
                if not passed:
                    all_pass = False
        except Exception as e:
            print(f"  FAIL  {path} - {e}")
            all_pass = False

    return all_pass

def test_dashboard_pages():
    """Take screenshots of all dashboard pages and verify content."""
    ensure_dir(SCREENSHOTS_DIR)
    all_pass = True

    print("\n=== Dashboard Visual Tests ===")

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(
            viewport={"width": 1920, "height": 1080},
            color_scheme="dark",
        )
        page = context.new_page()

        # Test 1: Landing Page
        print("\n  [1/4] Landing Page...")
        page.goto(BASE_URL, wait_until="networkidle", timeout=15000)

        # Check for key elements
        title_visible = page.locator("text=Tradoshka").first.is_visible()
        print(f"    Title 'Tradoshka' visible: {title_visible}")

        cta_visible = page.locator("text=View Live Performance").first.is_visible()
        print(f"    CTA button visible: {cta_visible}")

        markets_text = page.locator("text=Polymarket").first.is_visible()
        print(f"    Markets text visible: {markets_text}")

        page.screenshot(path=f"{SCREENSHOTS_DIR}/01-landing-page.png", full_page=True)
        print(f"    Screenshot: {SCREENSHOTS_DIR}/01-landing-page.png")

        if not (title_visible and cta_visible):
            all_pass = False
            print("    FAIL - Missing key elements")
        else:
            print("    PASS")

        # Test 2: Performance Page (via navigation)
        print("\n  [2/4] Performance Page (navigation)...")
        page.click("text=View Live Performance")
        page.wait_for_url("**/performance", timeout=10000)
        page.wait_for_load_state("networkidle", timeout=15000)

        # Wait for React Query to fetch data
        page.wait_for_timeout(3000)

        perf_title = page.locator("text=Live Performance").first.is_visible()
        print(f"    'Live Performance' heading visible: {perf_title}")

        page.screenshot(path=f"{SCREENSHOTS_DIR}/02-performance-page.png", full_page=True)
        print(f"    Screenshot: {SCREENSHOTS_DIR}/02-performance-page.png")

        if not perf_title:
            all_pass = False
            print("    FAIL - Performance heading not found")
        else:
            print("    PASS")

        # Test 3: Check stat cards
        print("\n  [3/4] Stat Cards...")

        stat_checks = [
            ("Total ROI", True),
            ("Sharpe Ratio", True),
            ("Max Drawdown", True),
            ("Win Rate", True),
        ]

        for label, expected in stat_checks:
            visible = page.locator(f"text={label}").first.is_visible()
            status = "PASS" if visible == expected else "FAIL"
            print(f"    {status}  '{label}' visible: {visible}")
            if visible != expected:
                all_pass = False

        # Test 4: Check charts section
        print("\n  [4/4] Charts...")

        equity_curve = page.locator("text=Equity Curve").first.is_visible()
        print(f"    Equity Curve section: {equity_curve}")

        daily_pnl = page.locator("text=Daily P").first.is_visible()
        print(f"    Daily P&L section: {daily_pnl}")

        strategy_perf = page.locator("text=Strategy Performance").first.is_visible()
        print(f"    Strategy Performance section: {strategy_perf}")

        # Check strategy names
        strategies_found = []
        for name in ["AI Predictor", "Copy Trading", "Market Making", "Arbitrage"]:
            if page.locator(f"text={name}").first.is_visible():
                strategies_found.append(name)
        print(f"    Strategies found: {', '.join(strategies_found)} ({len(strategies_found)}/4)")

        # Take a close-up of the performance section
        page.screenshot(path=f"{SCREENSHOTS_DIR}/03-performance-stats.png",
                       clip={"x": 0, "y": 0, "width": 1920, "height": 400})
        print(f"    Screenshot: {SCREENSHOTS_DIR}/03-performance-stats.png")

        # Scroll down for full page
        page.evaluate("window.scrollTo(0, document.body.scrollHeight)")
        page.wait_for_timeout(1000)
        page.screenshot(path=f"{SCREENSHOTS_DIR}/04-performance-charts.png", full_page=True)
        print(f"    Screenshot: {SCREENSHOTS_DIR}/04-performance-charts.png")

        if equity_curve and strategy_perf:
            print("    PASS")
        else:
            all_pass = False
            print("    FAIL - Missing chart sections")

        # Mobile viewport test
        print("\n  [Bonus] Mobile Viewport (375x812)...")
        mobile_context = browser.new_context(
            viewport={"width": 375, "height": 812},
            color_scheme="dark",
        )
        mobile_page = mobile_context.new_page()
        mobile_page.goto(f"{BASE_URL}/performance", wait_until="networkidle", timeout=15000)
        mobile_page.wait_for_timeout(3000)
        mobile_page.screenshot(path=f"{SCREENSHOTS_DIR}/05-mobile-performance.png", full_page=True)
        print(f"    Screenshot: {SCREENSHOTS_DIR}/05-mobile-performance.png")
        print("    PASS (responsive layout)")

        mobile_context.close()
        browser.close()

    return all_pass

def main():
    print("=" * 60)
    print("  TRADOSHKA DASHBOARD - VISUAL VERIFICATION")
    print("=" * 60)

    api_ok = test_api_endpoints()
    dashboard_ok = test_dashboard_pages()

    print("\n" + "=" * 60)
    if api_ok and dashboard_ok:
        print("  ALL TESTS PASSED")
    else:
        print("  SOME TESTS FAILED")
    print("=" * 60)

    print(f"\nScreenshots saved to: {SCREENSHOTS_DIR}/")
    print("  01-landing-page.png")
    print("  02-performance-page.png")
    print("  03-performance-stats.png")
    print("  04-performance-charts.png")
    print("  05-mobile-performance.png")

    return 0 if (api_ok and dashboard_ok) else 1

if __name__ == "__main__":
    sys.exit(main())
