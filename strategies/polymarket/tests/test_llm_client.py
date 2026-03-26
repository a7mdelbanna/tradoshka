from tradoshka_poly.ai_predictor.llm_client import LLMConfig, LLMClient

def test_config_defaults():
    config = LLMConfig()
    assert config.model == "gpt-4o-mini"
    assert config.temperature == 0.7
    assert "openai" in config.api_url

def test_config_from_env(monkeypatch):
    monkeypatch.setenv("LLM_MODEL", "claude-3")
    monkeypatch.setenv("LLM_API_KEY", "test-key")
    config = LLMConfig.from_env()
    assert config.model == "claude-3"
    assert config.api_key == "test-key"
