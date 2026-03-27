"""Save session state for documentation and continuity."""
import json, urllib.request, os
from datetime import datetime

API = "http://localhost:3001"

def fetch(path):
    try:
        with urllib.request.urlopen(f"{API}{path}", timeout=10) as r:
            return json.loads(r.read())
    except:
        return None

os.makedirs("docs/sessions", exist_ok=True)
timestamp = datetime.now().strftime("%Y-%m-%d_%H%M")

# Save full system state
state = {
    "timestamp": datetime.now().isoformat(),
    "status": fetch("/api/status"),
    "evolution_stats": fetch("/api/evolution/stats"),
    "leaderboard": fetch("/api/evolution/leaderboard"),
    "timeline": fetch("/api/evolution/timeline"),
    "graveyard": fetch("/api/evolution/graveyard"),
    "wallets": {
        "polymarket": fetch("/api/wallet/polymarket"),
        "crypto": fetch("/api/wallet/crypto"),
        "perps": fetch("/api/wallet/perps"),
    }
}

path = f"docs/sessions/{timestamp}_session.json"
with open(path, "w") as f:
    json.dump(state, f, indent=2, default=str)
print(f"Session saved to {path}")

# Append to session log
with open("docs/SESSION_LOG.md", "a") as f:
    stats = state.get("evolution_stats") or {}
    f.write(f"\n### Checkpoint {timestamp}\n")
    f.write(f"- Alive: {stats.get('alive', stats.get('alive_count', '?'))}\n")
    f.write(f"- Dead: {stats.get('dead', stats.get('dead_count', '?'))}\n")
    f.write(f"- Capital: ${stats.get('total_capital_deployed', stats.get('total_capital', '?'))}\n")
    f.write(f"- Avg Sharpe: {stats.get('avg_sharpe', '?')}\n")
    f.write(f"- Best: {stats.get('best_strategy', '?')}\n")

print("Session log updated")
