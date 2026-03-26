# Core Rust Rules

## Dependencies (workspace-level)
All deps are pinned in root Cargo.toml. Never add deps without:
1. Justification (why we need it)
2. Security audit (check for known vulns)
3. Approval in PR

## Patterns
- Use `thiserror` for library errors, `anyhow` for binary errors
- All async code uses `tokio`
- Prefer `rust_decimal::Decimal` over f64 for money values
- Use `chrono::DateTime<Utc>` for all timestamps
- Traits in `common`, implementations in their respective crates

## Testing
- Unit tests inline with `#[cfg(test)]`
- Integration tests in `tests/` directory
- Use `rust_decimal_macros::dec!()` for test values
