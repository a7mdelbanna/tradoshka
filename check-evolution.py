"""Check evolution status — run this anytime to see current state."""
import json
import urllib.request
import sys
from datetime import datetime

API = "http://localhost:3001"

def fetch(path):
    try:
        with urllib.request.urlopen(f"{API}{path}", timeout=5) as r:
            return json.loads(r.read())
    except Exception as e:
        print(f"ERROR: {e}")
        return None

def main():
    print("=" * 90)
    print(f"  TRADOSHKA EVOLUTION CHECK — {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 90)

    # Stats
    stats = fetch("/api/evolution/stats")
    if not stats:
        print("Server not running!")
        return

    print(f"\n  Alive: {stats.get('alive', 0)} | Dead: {stats.get('dead', 0)} | "
          f"Hour: {stats.get('current_hour', 0)} | "
          f"Capital: ${stats.get('total_capital_deployed', '0')} | "
          f"Avg Sharpe: {stats.get('avg_sharpe', 0):.2f}")

    # Leaderboard
    lb = fetch("/api/evolution/leaderboard")
    if lb:
        print(f"\n{'#':>2} {'STRATEGY':30s} {'MARKET':15s} {'TRADES':>6} {'PNL':>10} {'SHARPE':>8} {'WR':>5} {'GEN':>4}")
        print("-" * 85)
        for s in lb.get('strategies', []):
            pnl = f"${s['pnl']:.2f}"
            gen = f"v{s['generation']}" if s['generation'] > 1 else "v1"
            print(f"{s['rank']:>2} {s['name']:30s} {s['market']:15s} {s['trades']:>6} {pnl:>10} {s['sharpe']:>8.2f} {s['win_rate']:>5} {gen:>4}")

        trading = sum(1 for s in lb['strategies'] if s['trades'] > 0)
        total_pnl = sum(s['pnl'] for s in lb['strategies'])
        print(f"\n  Trading: {trading}/{len(lb['strategies'])} | Total PnL: ${total_pnl:.2f}")

    # Timeline
    tl = fetch("/api/evolution/timeline")
    if tl and tl.get('events'):
        print(f"\n  EVOLUTION EVENTS ({len(tl['events'])} total):")
        for e in tl['events'][:10]:
            action = e.get('type', e.get('action', '?'))
            ts = e.get('timestamp', '')[:19]
            print(f"    [{ts}] {action:8s} {e.get('strategy_name', e.get('strategy', '?')):30s} {e.get('details', '')[:60]}")
    else:
        print("\n  No evolution events yet (first cycle hasn't run)")

    # Graveyard
    gy = fetch("/api/evolution/graveyard")
    if gy and gy.get('strategies') and len(gy['strategies']) > 0:
        print(f"\n  GRAVEYARD ({len(gy['strategies'])} dead):")
        for s in gy['strategies']:
            print(f"    {s['name']:30s} lived {s.get('lifetime_hours', '?')}h | trades={s.get('trades', 0)} | pnl=${s.get('pnl', 0)} | cause: {s.get('cause_of_death', '?')}")
    else:
        print("\n  Graveyard: empty (no strategies killed yet)")

    print("\n" + "=" * 90)

if __name__ == "__main__":
    main()
