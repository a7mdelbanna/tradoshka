import json
from unittest.mock import patch, MagicMock
import pytest


def test_simulation_uses_statistical_when_claude_unavailable():
    from ai_predictor.simulation import PredictionSimulation
    from ai_predictor.world_builder import SimulationWorld

    sim = PredictionSimulation(claude_config=None, agent_count=5, seed=42)
    world = SimulationWorld(
        market_question="Will X happen?",
        market_description="Test market",
        current_yes_price=0.6,
        current_no_price=0.4,
        volume_24h=10000.0,
        end_date="2026-04-01",
    )
    result = sim.run(world)
    assert result.num_agents == 5
    assert 0.01 <= result.probability_yes <= 0.99


def test_simulation_builds_batch_prompt_with_all_agents():
    from ai_predictor.simulation import PredictionSimulation
    from ai_predictor.claude_client import ClaudeConfig
    from ai_predictor.world_builder import SimulationWorld

    config = ClaudeConfig()

    mock_votes = {
        "votes": [
            {"agent_id": f"agent_{i:04d}", "probability_yes": 0.5 + (i * 0.02),
             "confidence": 0.7, "reasoning": f"Agent {i} reasoning"}
            for i in range(5)
        ]
    }

    with patch("ai_predictor.simulation.ClaudeClient") as MockClient:
        mock_instance = MagicMock()
        mock_instance.is_available.return_value = True
        mock_instance.predict.return_value = mock_votes
        MockClient.return_value = mock_instance

        sim = PredictionSimulation(claude_config=config, agent_count=5, seed=42)
        world = SimulationWorld(
            market_question="Will X happen?",
            market_description="Test market",
            current_yes_price=0.6,
            current_no_price=0.4,
            volume_24h=10000.0,
            end_date="2026-04-01",
        )
        result = sim.run(world)

        assert result.num_agents == 5
        mock_instance.predict.assert_called_once()
        call_args = mock_instance.predict.call_args
        system_prompt = call_args[0][0]
        user_message = call_args[0][1]
        assert "prediction market simulation engine" in system_prompt.lower()
        assert "Will X happen?" in user_message
        assert "agent_0000" in user_message or "[1]" in user_message


def test_simulation_falls_back_on_claude_error():
    from ai_predictor.simulation import PredictionSimulation
    from ai_predictor.claude_client import ClaudeConfig
    from ai_predictor.world_builder import SimulationWorld

    config = ClaudeConfig()

    with patch("ai_predictor.simulation.ClaudeClient") as MockClient:
        mock_instance = MagicMock()
        mock_instance.is_available.return_value = True
        mock_instance.predict.side_effect = RuntimeError("CLI crashed")
        MockClient.return_value = mock_instance

        sim = PredictionSimulation(claude_config=config, agent_count=5, seed=42)
        world = SimulationWorld(
            market_question="Will X happen?",
            market_description="Test market",
            current_yes_price=0.6,
            current_no_price=0.4,
            volume_24h=10000.0,
            end_date="2026-04-01",
        )
        result = sim.run(world)
        # Should fall back to statistical, not crash
        assert result.num_agents == 5
        assert 0.01 <= result.probability_yes <= 0.99
