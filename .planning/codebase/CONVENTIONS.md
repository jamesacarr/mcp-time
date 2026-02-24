# Conventions

> Generated: 2026-02-24T19:09:23Z

## Formatting

- **rustfmt** — default Rust formatter, no custom config needed
- Run: `cargo fmt`
- Check: `cargo fmt --check`

## Linting

- **Clippy** — standard Rust linter
- Run: `cargo clippy -- -D warnings`
- Treat warnings as errors in CI

## Naming

- Rust standard conventions:
  - `snake_case` for functions, variables, modules
  - `CamelCase` for types, traits, enums
  - `SCREAMING_SNAKE_CASE` for constants
- Tool names exposed via MCP: `snake_case` (e.g., `get_current_time`, `convert_timezone`)

## Error Handling

- Use `McpError` for tool-level errors returned to MCP clients
- Use `anyhow::Result` or custom error types for internal errors
- No `unwrap()` in production paths

## Dependencies

- Minimize dependency count — this is a small, focused server
- Prefer well-maintained crates from the Rust ecosystem
- Pin versions in `Cargo.toml` with `=` or use `Cargo.lock`

## Documentation

- Doc comments (`///`) on public types and tool functions
- Tool descriptions via `#[tool(description = "...")]` macro attribute
