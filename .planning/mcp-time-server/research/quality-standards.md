# Quality & Standards Research

> Task: Create a new Rust-based MCP server with two tools: (1) fetch current time in a specific timezone (defaulting to UTC), and (2) convert time between timezones.
> Last researched: 2026-02-24T19:25:15Z

## Security

### Input Validation
- **Timezone strings**: All user-supplied timezone names must be validated against the IANA database via `chrono-tz`. Invalid timezone strings should return a structured `McpError` with `INVALID_PARAMS` code, not panic or produce undefined behaviour.
- **Time format strings**: The `convert_time` tool accepts a time string (e.g., `"14:30"`). Parse with `NaiveTime::parse_from_str` and return a clear error on malformed input. Never pass raw user input to format strings.
- **No `unwrap()` on user input paths**: All parsing must use `?` or explicit `match`/`map_err` to convert to `McpError`.

### Transport Security
- stdio transport is inherently local -- no network exposure, no authentication needed.
- If HTTP/SSE transport is added later, TLS and auth become mandatory concerns. Not in scope now.

### Dependency Audit
- Run `cargo audit` in CI to catch known vulnerabilities in dependencies.
- Keep `chrono`, `chrono-tz`, and `rmcp` pinned and updated.

**Sources**: [OWASP Input Validation](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html), `.planning/codebase/CONCERNS.md`, `.planning/codebase/CONVENTIONS.md`

## Performance

### Expected Profile
This is a low-throughput, low-latency tool server. MCP clients send occasional tool calls (seconds to minutes apart). Performance is not a primary concern, but there are still sensible defaults to follow.

### Timezone Operations
- `chrono-tz` compiles the full IANA database into the binary at build time. Lookups are O(1) hash/match operations -- negligible cost.
- If binary size matters, `chrono-tz` supports `filter-by-regex` feature to include only needed timezones. Not recommended unless binary size becomes a problem.

### Async Runtime
- Tokio runtime is required by `rmcp`. Time operations are CPU-bound and trivially fast, so `spawn_blocking` is unnecessary.
- Avoid holding locks across `.await` points (relevant if any shared state is introduced).

### Build Performance
- `serde`, `schemars`, and `rmcp` proc macros add compile time. Acceptable for a small project.
- Use `cargo build --release` with LTO for the distributed binary.

**Sources**: [chrono-tz docs](https://docs.rs/chrono-tz), `.planning/codebase/CONCERNS.md`

## Accessibility

Not applicable (no UI changes). This is a CLI server communicating via stdio with MCP clients. The MCP client (Claude Desktop, Claude Code, etc.) owns the user-facing interface.

## Testing Strategy

### Test Types Needed

| Type | Scope | Framework |
|------|-------|-----------|
| Unit tests | Timezone parsing, time formatting, conversion logic | `#[test]`, `#[tokio::test]` |
| Integration tests | Tool routing, MCP protocol compliance, server lifecycle | `#[tokio::test]` in `tests/` |

### Key Test Cases

**`get_current_time` tool:**
1. Default (no timezone) returns UTC time in ISO 8601 format
2. Valid IANA timezone (e.g., `"America/New_York"`) returns correctly offset time
3. Invalid timezone string returns `McpError` with descriptive message
4. Result includes `is_dst` indicator (per reference Python implementation)
5. Result includes timezone name and UTC offset

**`convert_time` tool:**
1. Convert between two valid timezones produces correct result
2. Convert across DST boundary (e.g., `"America/New_York"` in March) handles offset change
3. Invalid source timezone returns error
4. Invalid target timezone returns error
5. Malformed time string (e.g., `"25:99"`, `"abc"`) returns error
6. Edge times: midnight (`"00:00"`), end of day (`"23:59"`)
7. Fractional-offset timezones (e.g., `"Asia/Kathmandu"` at UTC+5:45)

**Timezone parsing (internal):**
1. Case sensitivity of IANA timezone names
2. Empty string input
3. Common aliases that are NOT valid IANA names (e.g., `"EST"`, `"PST"`) -- decide whether to support or reject

**Server lifecycle:**
1. Server lists both tools via `list_tools`
2. Tool metadata includes correct descriptions and parameter schemas
3. Calling a non-existent tool returns appropriate error

### Mocking Approach

- **System clock**: For `get_current_time`, the current time changes every call. Two strategies:
  - (Preferred) Accept that the time will differ and assert on format/structure rather than exact value. Verify the result parses as valid ISO 8601 and the timezone offset matches expectations.
  - (If needed) Inject a clock trait for deterministic testing. Only worth it if time-dependent edge cases arise.
- **No external services to mock**: All operations use `chrono`/`chrono-tz` with no network calls.

### Edge Cases to Cover

- DST "spring forward" gap: 2:00 AM doesn't exist on transition day. The `convert_time` tool should handle or document this.
- DST "fall back" overlap: 1:30 AM occurs twice. `chrono-tz` resolves this deterministically, but test to verify.
- Date rollover during conversion: converting `"23:30"` from UTC to `"Asia/Tokyo"` (UTC+9) yields the next day.
- Leap seconds: `chrono` does not model leap seconds -- this is fine, but document it.

### Existing Test Patterns

Per `.planning/codebase/TESTING.md`:
- Unit tests co-located with source in `#[cfg(test)] mod tests { }` blocks
- Async tests use `#[tokio::test]`
- Integration tests in `tests/` directory
- Run all with `cargo test`

## Error Handling Standards

### Pattern: `thiserror` for domain errors, `McpError` for protocol errors

```rust
// Internal domain errors (thiserror)
#[derive(Debug, thiserror::Error)]
enum TimeError {
    #[error("Invalid timezone: {0}")]
    InvalidTimezone(String),
    #[error("Invalid time format: {0}. Expected HH:MM")]
    InvalidTimeFormat(String),
}

// Convert to McpError at the tool boundary
impl From<TimeError> for McpError {
    fn from(e: TimeError) -> Self {
        McpError::new(ErrorCode::INVALID_PARAMS, e.to_string(), None)
    }
}
```

This keeps domain logic testable without MCP dependency, while tool handlers convert at the boundary.

**Sources**: [thiserror vs anyhow guide](https://momori.dev/posts/rust-error-handling-thiserror-anyhow/), [rmcp docs](https://docs.rs/rmcp/latest/rmcp/), `.planning/codebase/CONVENTIONS.md`

## Linting & Formatting

| Tool | Command | CI Enforcement |
|------|---------|---------------|
| rustfmt | `cargo fmt --check` | Fail build on violations |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | Treat all warnings as errors |
| cargo audit | `cargo audit` | Fail on known vulnerabilities |

No custom `rustfmt.toml` needed -- use Rust defaults per `.planning/codebase/CONVENTIONS.md`.

## CI/CD (GitHub Actions)

### Recommended Workflow

```yaml
name: CI
on: [push, pull_request]
env:
  CARGO_TERM_COLOR: always
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --all-features -- -D warnings
      - run: cargo test --all-features
      - run: cargo audit
```

Key decisions:
- `dtolnay/rust-toolchain@stable` over `hecrj/setup-rust-action` (more actively maintained).
- `Swatinem/rust-cache@v2` caches `~/.cargo` and `target/` for faster builds.
- Single job is fine for a small project; split into parallel jobs if build time exceeds 5 minutes.
- `cargo audit` requires `cargo install cargo-audit` or use `actions-rs/audit-check`.

**Sources**: [Rust CI with GitHub Actions](https://dev.to/bampeers/rust-ci-with-github-actions-1ne9), [GitHub Actions workflow gist](https://gist.github.com/domnikl/ccb8d0b82056fbe5cf7f4f145ac7f44b)

## Documentation Standards

| What | How | Required |
|------|-----|----------|
| Public types and functions | `///` rustdoc comments | Yes |
| Tool descriptions | `#[tool(description = "...")]` macro | Yes |
| Parameter descriptions | `#[schemars(description = "...")]` on fields | Yes |
| README | Usage, installation, MCP client config example | Yes |
| Inline comments | Only for non-obvious logic | As needed |

Generate docs with `cargo doc --no-deps --open` to verify rustdoc renders correctly.

## Standards Checklist

1. All user-supplied timezone strings are validated against `chrono-tz` before use
2. All user-supplied time strings are parsed with explicit format and return errors on failure
3. No `unwrap()` or `expect()` on any path reachable from user input
4. `cargo fmt --check` passes with zero violations
5. `cargo clippy --all-targets --all-features -- -D warnings` passes with zero warnings
6. `cargo test --all-features` passes with zero failures
7. Unit tests exist for: valid timezone parsing, invalid timezone rejection, time format parsing, time format rejection, UTC default, DST-aware conversion, fractional-offset timezones
8. Integration tests exist for: tool listing, `get_current_time` invocation, `convert_time` invocation, error responses for invalid parameters
9. All public types and tool functions have rustdoc comments
10. All tool parameters have `schemars` descriptions for JSON Schema generation
11. Error messages returned to MCP clients are human-readable and actionable
12. CI workflow runs fmt, clippy, test, and audit on every push and PR
13. `Cargo.lock` is committed (binary project, not a library)
14. No secrets, credentials, or `.env` files committed
