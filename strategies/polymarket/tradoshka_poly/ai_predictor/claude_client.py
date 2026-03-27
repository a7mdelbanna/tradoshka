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

    # JSON schema for enforced structured output
    VOTE_SCHEMA = json.dumps({
        "type": "object",
        "properties": {
            "votes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "agent_id": {"type": "string"},
                        "probability_yes": {"type": "number", "minimum": 0.01, "maximum": 0.99},
                        "confidence": {"type": "number", "minimum": 0.0, "maximum": 1.0},
                        "reasoning": {"type": "string"},
                    },
                    "required": ["agent_id", "probability_yes", "confidence", "reasoning"],
                },
            },
        },
        "required": ["votes"],
    })

    def is_available(self) -> bool:
        try:
            result = subprocess.run(
                ["claude", "--version"],
                capture_output=True, timeout=10,
            )
            return result.returncode == 0
        except (FileNotFoundError, subprocess.TimeoutExpired):
            return False

    def predict(self, system_prompt: str, user_message: str) -> dict:
        cmd = [
            "claude", "-p", user_message,
            "--system-prompt", system_prompt,
            "--model", self.config.model,
            "--output-format", "json",
            "--json-schema", self.VOTE_SCHEMA,
            "--max-turns", "2",
        ]
        try:
            proc = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=self.config.timeout,
            )
        except subprocess.TimeoutExpired:
            raise RuntimeError(f"Claude CLI timed out after {self.config.timeout}s")

        if proc.returncode != 0:
            raise RuntimeError(f"Claude CLI error (code {proc.returncode}): {proc.stderr}")

        response = json.loads(proc.stdout)
        # --json-schema puts structured output in "structured_output", not "result"
        if "structured_output" in response:
            return response["structured_output"]
        # Fallback: try parsing from "result" field
        result_text = response.get("result", "")
        if isinstance(result_text, str) and result_text.strip():
            return json.loads(result_text)
        raise RuntimeError(f"Claude CLI returned no structured output: {proc.stdout[:200]}")
