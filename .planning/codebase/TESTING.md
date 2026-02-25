# Testing

> Last mapped: 2026-02-25T00:06:14Z

## Test Framework

- Runner: `cargo test` (built-in Rust test harness) with `tokio::test` for async tests
- Async runtime: `#[tokio::test]` attribute -- used throughout `src/server.rs` and `tests/integration.rs`
- Run command: `make test` (or `cargo test`)
- CI command: `cargo test --all-features` -- see `.github/workflows/ci.yml:25`

## Test Organisation

- Unit tests: `src/server.rs:311-718` -- inline `#[cfg(test)] mod tests` block
- Integration tests: `tests/integration.rs` -- tests MCP protocol-level behavior (tool discovery, response structure)
- No E2E tests (no process-level stdio transport tests)

## Test Patterns

### Setup
- Direct struct instantiation: `TimeServer::new()` -- no fixtures, factories, or builders needed
- Parameters wrapped in `Parameters(params)` to match the MCP handler signature
- Example: `src/server.rs:384-386`

### Assertions
- Standard library `assert!`, `assert_eq!` macros
- Custom assertion messages with format strings for debugging context -- e.g., `src/server.rs:461-463`
- JSON response parsing via `serde_json::Value` for structural assertions -- e.g., `src/server.rs:388-389`
- Pattern: assert `is_error` field first, then parse and validate response content

### Mocking
- No mocking framework -- tests call tool methods directly on `TimeServer` instances
- Time-dependent tests use range assertions to handle DST variability -- e.g., `src/server.rs:460-463` checks for either `07:00` or `08:00`

### Helper Functions
- `extract_text()` -- defined in both `src/server.rs:712-717` (unit tests) and `tests/integration.rs:5-9` (integration tests), extracts text content from `CallToolResult`

## Test Categories

| Category | Count | File | Description |
|----------|-------|------|-------------|
| `parse_timezone` | 4 | `src/server.rs:316-339` | Timezone parsing: valid IANA, invalid, abbreviation, offset |
| `format_utc_offset` | 4 | `src/server.rs:342-364` | Offset formatting: positive, negative, fractional, zero |
| `tool_error` | 2 | `src/server.rs:367-380` | Error helper: flag and content |
| `get_current_time` | 8 | `src/server.rs:383-598` | Tool behavior: defaults, valid, invalid, DST, offsets, abbreviations |
| `convert_time` | 10 | `src/server.rs:445-709` | Tool behavior: conversion, validation, edge cases, formats |
| Integration | 4 | `tests/integration.rs:13-93` | Protocol-level: tool discovery, success/error responses |

## Coverage

- No coverage tool configured
- No coverage threshold set

## Prescriptive Guidance

- New unit tests: add to the `#[cfg(test)] mod tests` block at the bottom of the relevant source file. Follow the pattern in `src/server.rs:316` -- one function per scenario, descriptive name like `<function>_<scenario>`.
- New integration tests: add to `tests/integration.rs`. These should test protocol-level concerns (tool metadata, response shape), not business logic.
- Async tests: use `#[tokio::test]` for any test that calls `async` tool methods.
- Assertion pattern: always check `result.is_error` first, then use `extract_text()` + `serde_json::from_str` to validate JSON response structure.
- Time-sensitive tests: use range-based assertions to account for DST (e.g., check for two possible values). Do not hardcode offsets.
- Example to copy: `src/server.rs:405-418` (`get_current_time_returns_valid_response_for_known_timezone`) -- follows the standard arrange/act/assert pattern with JSON validation.
