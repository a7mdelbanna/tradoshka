import json
import os
from dataclasses import dataclass
from urllib.request import Request, urlopen
from urllib.error import HTTPError


@dataclass
class LLMConfig:
    api_url: str = "https://api.openai.com/v1/chat/completions"
    api_key: str = ""
    model: str = "gpt-4o-mini"
    temperature: float = 0.7
    max_tokens: int = 4096

    @classmethod
    def from_env(cls) -> "LLMConfig":
        return cls(
            api_url=os.getenv("LLM_API_URL", "https://api.openai.com/v1/chat/completions"),
            api_key=os.getenv("LLM_API_KEY", ""),
            model=os.getenv("LLM_MODEL", "gpt-4o-mini"),
        )


class LLMClient:
    """Minimal LLM client using raw HTTP. No SDK dependencies."""

    def __init__(self, config: LLMConfig | None = None):
        self.config = config or LLMConfig.from_env()

    def chat(self, system_prompt: str, user_message: str, temperature: float | None = None) -> str:
        payload = {
            "model": self.config.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_message},
            ],
            "temperature": temperature or self.config.temperature,
            "max_tokens": self.config.max_tokens,
        }
        data = json.dumps(payload).encode("utf-8")
        req = Request(
            self.config.api_url,
            data=data,
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {self.config.api_key}",
            },
            method="POST",
        )
        try:
            with urlopen(req, timeout=60) as resp:
                result = json.loads(resp.read())
                return result["choices"][0]["message"]["content"]
        except HTTPError as e:
            error_body = e.read().decode() if e.fp else str(e)
            raise RuntimeError(f"LLM API error {e.code}: {error_body}")

    def chat_json(self, system_prompt: str, user_message: str, temperature: float | None = None) -> dict:
        response = self.chat(system_prompt, user_message, temperature)
        text = response.strip()
        if text.startswith("```json"):
            text = text[7:]
        if text.startswith("```"):
            text = text[3:]
        if text.endswith("```"):
            text = text[:-3]
        return json.loads(text.strip())
