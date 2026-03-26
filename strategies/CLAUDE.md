# Python Strategy Rules

## Structure
Every strategy must:
1. Extend `BaseStrategy`
2. Implement `name()` and `on_market_event()`
3. Return `Signal` objects (never raw values)
4. Have tests in the corresponding `tests/` directory

## Indicators
- Use indicators from `tradoshka_strategy.indicators`
- Never use external TA libraries (ta-lib, etc.) — write our own
- All indicators must be stateful (update-based, not batch)

## Testing
- Test with known input/output sequences
- Test edge cases (empty data, single data point, NaN)
- Test signal generation logic separately from indicator logic
