from tradoshka_poly.ai_predictor.scorer import PredictionScorer, AgentVote

def test_aggregate_unanimous_bullish():
    scorer = PredictionScorer(market_price=0.5)
    votes = [AgentVote("a1", 0.8, 0.9), AgentVote("a2", 0.85, 0.8), AgentVote("a3", 0.75, 0.7)]
    result = scorer.aggregate(votes)
    assert result.probability_yes > 0.75
    assert result.confidence > 0.5
    assert result.bull_count == 3
    assert result.bear_count == 0

def test_aggregate_mixed_opinions():
    scorer = PredictionScorer(market_price=0.5)
    votes = [AgentVote("a1", 0.8, 0.9), AgentVote("a2", 0.2, 0.9), AgentVote("a3", 0.5, 0.5)]
    result = scorer.aggregate(votes)
    assert 0.3 < result.probability_yes < 0.7
    assert result.std_dev > 0.1

def test_aggregate_empty():
    result = PredictionScorer(0.5).aggregate([])
    assert result.probability_yes == 0.5
    assert result.confidence == 0.0

def test_should_trade_strong_edge():
    scorer = PredictionScorer(market_price=0.5)
    votes = [AgentVote(f"a{i}", 0.75, 0.8) for i in range(10)]
    result = scorer.aggregate(votes)
    trade = scorer.should_trade(result, min_edge=0.05)
    assert trade is not None
    assert trade[0] == "YES"

def test_should_trade_no_edge():
    scorer = PredictionScorer(market_price=0.5)
    votes = [AgentVote(f"a{i}", 0.51, 0.5) for i in range(10)]
    result = scorer.aggregate(votes)
    assert scorer.should_trade(result, min_edge=0.05) is None

def test_should_trade_low_confidence():
    scorer = PredictionScorer(market_price=0.5)
    votes = [AgentVote("a1", 0.9, 0.1), AgentVote("a2", 0.1, 0.1)]
    result = scorer.aggregate(votes)
    assert scorer.should_trade(result, min_confidence=0.5) is None
