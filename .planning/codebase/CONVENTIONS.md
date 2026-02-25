# Conventions

> Last mapped: 2026-02-25T00:06:14Z

## Naming

- Files: `snake_case.rs` -- example: `src/server.rs`
- Functions/methods: `snake_case` -- example: `parse_timezone`, `format_utc_offset` in `src/server.rs`
- Structs/types: `PascalCase` -- example: `TimeServer`, `GetCurrentTimeParams` in `src/server.rs`
- Constants: `SCREAMING_SNAKE_CASE` -- example: `ERR_INVALID_TIMEZONE` at `src/server.rs:11`
- MCP tool names: `snake_case` -- `get_current_time`, `convert_time` (declared via `#[tool(name = "...")]`)
- Crate name uses hyphens (`mcp-time`), lib name uses underscores (`mcp_time`) -- configured in `Cargo.toml:2,8`

## File Organisation

- Source root: `src/`
- Library entry point: `src/lib.rs` (re-exports `pub mod server`)
- Binary entry point: `src/main.rs`
- Unit tests: inline `#[cfg(test)] mod tests` at bottom of `src/server.rs:311`
- Integration tests: `tests/integration.rs`

## Imports

- Style: absolute paths from external crates, `use super::*` within test modules
- Ordering: external crate imports first, then internal (`use mcp_time::...`), enforced by `cargo fmt`
- Grouping: imports from the same crate are consolidated with nested braces -- see `src/server.rs:1-8`

## Error Handling

- **Tool-level errors**: returned as `CallToolResult` with `is_error: true` via the `tool_error()` helper at `src/server.rs:243-245`. User-facing validation errors are not panics -- they produce error content.
- **Internal errors**: `rmcp::ErrorData::internal_error()` for serialization failures -- see `src/server.rs:123-125`
- **Application errors**: `anyhow::Result` for the binary entry point at `src/main.rs:6`
- **Parse errors**: `Result<T, String>` for internal validation helpers like `parse_timezone` at `src/server.rs:252`
- Pattern: validation errors are always returned as successful MCP responses with the `is_error` flag set, never as `Err` variants. Only truly unexpected failures (serialization, date construction) use `Err`.

## Code Style

- Formatter: `cargo fmt` (default rustfmt settings, no `rustfmt.toml` present)
- Linter: `cargo clippy --all-targets --all-features -- -D warnings` (no `clippy.toml` present, all warnings treated as errors)
- CI enforcement: both formatting and linting checked in `.github/workflows/ci.yml`
- Makefile targets: `make fmt`, `make lint`, `make check` -- see `Makefile`
- Doc comments: `///` on all public structs, methods, and helper functions -- consistent throughout `src/server.rs`
- Inline comments: used sparingly for non-obvious logic (DST detection, format validation)

## Prescriptive Guidance

- New files: add as `snake_case.rs` under `src/`, register in `src/lib.rs` with `pub mod <name>`
- New structs: use `PascalCase`, derive `Debug` at minimum; derive `Deserialize` + `JsonSchema` for input params, `Serialize` for responses
- New functions: use `snake_case`, document with `///`, return `Result` types for fallible operations
- Error handling: for MCP tool errors visible to users, use the `tool_error()` helper pattern. For internal failures, use `rmcp::ErrorData::internal_error()`. Never panic on user input.
- Imports: let `cargo fmt` handle ordering. Group external crate imports, then internal modules.
- Before committing: run `make check` (or `cargo fmt --check` + `cargo clippy --all-targets --all-features -- -D warnings`)
