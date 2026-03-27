import json
import subprocess
from dataclasses import dataclass


@dataclass
class ClaudeConfig:
    model: str = "sonnet"
    max_tokens: int = 16384
    temperature: float = 0.7
    timeout: int = 120


class ClaudeClient:
    """Claude Code CLI wrapper. Uses claude -p for inference via Max subscription."""

    def __init__(self, config: ClaudeConfig | None = None):
        self.config = config or ClaudeConfig()

    def is_available(self) -> bool:
        try:
            result = subprocess.run(
                ["claude", "--version"],
                capture_output=True, timeout=10,
            )
            return result.returncode == 0
        except (FileNotFoundError, subprocess.TimeoutExpired):
            return False
