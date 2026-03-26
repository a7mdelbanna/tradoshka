"""Tradoshka Health Check — Run every hour to catch issues automatically."""
import json
import urllib.request
import sys
from datetime import datetime

API = "http://localhost:3001"
ISSUES = []


def fetch(path):
    try:
        with urllib.request.urlopen(f"{API}{path}", timeout=10) as r:
            return json.loads(r.read())
    except Exception as e:
        ISSUES.append(f"API UNREACHABLE: {path} - {e}")
        return None


def check_api_health():
    """Check all critical endpoints respond."""
    endpoints = [
        "/health",
        "/api/status",
        "/api/evolution/stats",
        "/api/evolution/leaderboard",
        "/api/wallet/polymarket",
        "/api/wallet/crypto",
        "/api/wallet/perps",
    ]
    for ep in endpoints:
        data = fetch(ep)
        if data is None:
            ISSUES.append(f"ENDPOINT DOWN: {ep}")
        else:
            print(f"  OK: {ep} responded")


def check_strategies_trading():
    """Check all market types have active strategies."""
    lb = fetch("/api/evolution/leaderboard")
    if not lb:
        return

    by_market = {}
    for s in lb.get("strategies", []):
        m = s.get("market", "unknown")
        by_market.setdefault(m, []).append(s)

    for market in ["polymarket", "crypto_spot", "crypto_perps"]:
        strats = by_market.get(market, [])
        if not strats:
            ISSUES.append(f"NO STRATEGIES for {market}")
            continue
        trading = [s for s in strats if s.get("trades", 0) > 0]
        if not trading:
            ISSUES.append(
                f"ZERO TRADES: {market} has {len(strats)} strategies but NONE are trading"
            )
        else:
            print(f"  OK: {market}: {len(trading)}/{len(strats)} strategies trading")


def check_pnl_differentiation():
    """Check strategies aren't all showing identical PnL."""
    lb = fetch("/api/evolution/leaderboard")
    if not lb:
        return

    trading = [s for s in lb.get("strategies", []) if s.get("trades", 0) > 0]
    if len(trading) < 5:
        print(f"  INFO: Only {len(trading)} trading strategies — skipping PnL differentiation check")
        return

    sharpes = set(round(s.get("sharpe", 0), 2) for s in trading)
    if len(sharpes) < 3:
        ISSUES.append(
            f"IDENTICAL RESULTS: Only {len(sharpes)} unique Sharpe values across "
            f"{len(trading)} trading strategies"
        )
    else:
        print(f"  OK: {len(sharpes)} unique Sharpe values across {len(trading)} strategies")


def check_evolution_running():
    """Check evolution has fired at least once."""
    stats = fetch("/api/evolution/stats")
    if not stats:
        return

    hour = stats.get("current_hour", 0)
    dead = stats.get("dead", stats.get("dead_count", 0))
    if hour == 0:
        print(f"  INFO: Evolution hasn't fired yet (hour 0) — normal on fresh start")
    elif dead == 0 and hour > 0:
        ISSUES.append(
            f"EVOLUTION NOT KILLING: Hour {hour} but 0 dead strategies"
        )
    else:
        print(f"  OK: Evolution hour {hour}, {dead} dead strategies")


def check_wallets_updating():
    """Check that trading wallet equities are changing from initial $100."""
    lb = fetch("/api/evolution/leaderboard")
    if not lb:
        return

    trading = [s for s in lb.get("strategies", []) if s.get("trades", 0) > 0]
    if not trading:
        print("  INFO: No trading strategies yet — skipping wallet update check")
        return

    stuck = [s for s in trading if abs(s.get("equity", 100) - 100.0) < 0.001]
    if len(stuck) > len(trading) * 0.5:
        ISSUES.append(
            f"WALLETS STUCK: {len(stuck)}/{len(trading)} trading strategies "
            f"still at exactly $100"
        )
    else:
        changed = len(trading) - len(stuck)
        print(f"  OK: {changed}/{len(trading)} wallets showing real equity changes")


def check_stop_losses():
    """Check that trades have stop losses set."""
    trades = fetch("/api/trades/crypto")
    if not trades:
        return

    trade_list = trades.get("trades", [])
    if not trade_list:
        print("  INFO: No crypto trades yet — skipping stop loss check")
        return

    no_sl = [
        t for t in trade_list
        if not t.get("stop_loss") or str(t["stop_loss"]) == "0"
    ]
    if no_sl:
        ISSUES.append(
            f"MISSING STOP LOSSES: {len(no_sl)}/{len(trade_list)} trades have no stop loss"
        )
    else:
        print(f"  OK: All {len(trade_list)} trades have stop losses")


def check_pm_strategies_specifically():
    """Extra check: specifically verify PM strategies are trading."""
    lb = fetch("/api/evolution/leaderboard")
    if not lb:
        return

    pm_strats = [
        s for s in lb.get("strategies", [])
        if s.get("name", "").startswith("PM-") or s.get("market", "") == "polymarket"
    ]
    if not pm_strats:
        ISSUES.append("NO PM STRATEGIES: No PM-* strategies found in leaderboard")
        return

    pm_trading = [s for s in pm_strats if s.get("trades", 0) > 0]
    if not pm_trading:
        ISSUES.append(
            f"PM ZERO TRADES: {len(pm_strats)} PM strategies exist but trades=0 on all. "
            f"Check research thresholds or market data."
        )
    else:
        total_pm_trades = sum(s.get("trades", 0) for s in pm_trading)
        print(
            f"  OK: {len(pm_trading)}/{len(pm_strats)} PM strategies trading "
            f"({total_pm_trades} total trades)"
        )


def main():
    print("=" * 70)
    print(f"  TRADOSHKA HEALTH CHECK — {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 70)

    print("\n[1] API Endpoints")
    check_api_health()

    print("\n[2] Strategies Trading by Market")
    check_strategies_trading()

    print("\n[3] PM Strategies (Critical)")
    check_pm_strategies_specifically()

    print("\n[4] PnL Differentiation")
    check_pnl_differentiation()

    print("\n[5] Evolution Engine")
    check_evolution_running()

    print("\n[6] Wallet Equity Updates")
    check_wallets_updating()

    print("\n[7] Stop Losses")
    check_stop_losses()

    print()
    if ISSUES:
        print(f"  ISSUES FOUND: {len(ISSUES)}")
        print("-" * 70)
        for i, issue in enumerate(ISSUES, 1):
            print(f"  {i}. {issue}")
    else:
        print("  ALL CHECKS PASSED")

    print("=" * 70)
    return 1 if ISSUES else 0


if __name__ == "__main__":
    sys.exit(main())
