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
