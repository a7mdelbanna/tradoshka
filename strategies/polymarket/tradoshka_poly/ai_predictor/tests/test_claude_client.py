import json
import subprocess
from unittest.mock import patch, MagicMock
import pytest


def test_claude_config_defaults():
    from ai_predictor.claude_client import ClaudeConfig
    config = ClaudeConfig()
    assert config.model == "sonnet"
    assert config.max_tokens == 16384
    assert config.temperature == 0.7
    assert config.timeout == 120


def test_claude_client_is_available_when_cli_exists():
    from ai_predictor.claude_client import ClaudeClient
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0)
        client = ClaudeClient()
        assert client.is_available() is True
        mock_run.assert_called_once_with(
            ["claude", "--version"],
            capture_output=True, timeout=10,
        )


def test_claude_client_is_available_when_cli_missing():
    from ai_predictor.claude_client import ClaudeClient
    with patch("subprocess.run", side_effect=FileNotFoundError):
        client = ClaudeClient()
        assert client.is_available() is False


def test_predict_returns_parsed_json():
    from ai_predictor.claude_client import ClaudeClient, ClaudeConfig
    config = ClaudeConfig(model="sonnet", timeout=30)
    client = ClaudeClient(config)

    mock_response = json.dumps({
        "result": '{"votes": [{"agent_id": "agent_0000", "probability_yes": 0.72, "confidence": 0.85, "reasoning": "test"}]}',
        "session_id": "test-session",
    })

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout=mock_response,
        )
        result = client.predict("system prompt", "user message")
        assert result == {
            "votes": [{
                "agent_id": "agent_0000",
                "probability_yes": 0.72,
                "confidence": 0.85,
                "reasoning": "test",
            }]
        }

        call_args = mock_run.call_args
        cmd = call_args[0][0]
        assert cmd[0] == "claude"
        assert "-p" in cmd
        assert "--model" in cmd
        assert "--output-format" in cmd
        assert "--bare" in cmd
        assert "--max-turns" in cmd


def test_predict_raises_on_cli_failure():
    from ai_predictor.claude_client import ClaudeClient
    client = ClaudeClient()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(
            returncode=1,
            stdout="",
            stderr="auth expired",
        )
        with pytest.raises(RuntimeError, match="Claude CLI error"):
            client.predict("system", "user")


def test_predict_raises_on_timeout():
    from ai_predictor.claude_client import ClaudeClient
    client = ClaudeClient()

    with patch("subprocess.run", side_effect=subprocess.TimeoutExpired(cmd="claude", timeout=120)):
        with pytest.raises(RuntimeError, match="timed out"):
            client.predict("system", "user")
