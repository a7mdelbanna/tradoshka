from tradoshka_poly.ai_predictor.agent_factory import AgentFactory

def test_create_agents_count():
    factory = AgentFactory(seed=42)
    assert len(factory.create_agents(100)) == 100

def test_create_agents_unique_ids():
    factory = AgentFactory(seed=42)
    agents = factory.create_agents(50)
    assert len(set(a.agent_id for a in agents)) == 50

def test_create_agents_diverse_archetypes():
    factory = AgentFactory(seed=42)
    dist = factory.archetype_distribution(factory.create_agents(100))
    assert len(dist) >= 4

def test_agent_properties_in_range():
    for a in AgentFactory(seed=42).create_agents(50):
        assert 0.0 <= a.risk_tolerance <= 1.0
        assert -1.0 <= a.bias <= 1.0
        assert 0.0 <= a.confidence <= 1.0
        assert 0.0 <= a.contrarian_factor <= 1.0

def test_deterministic_with_seed():
    a1 = AgentFactory(seed=123).create_agents(10)
    a2 = AgentFactory(seed=123).create_agents(10)
    assert [a.bias for a in a1] == [a.bias for a in a2]
