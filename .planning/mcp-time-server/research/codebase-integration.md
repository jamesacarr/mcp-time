# Codebase Integration Research

> Task: Create a new Rust-based MCP server with two tools: (1) fetch current time in a specific timezone (defaulting to UTC), and (2) convert time between timezones. This is a greenfield project -- the repo is currently empty aside from `.planning/codebase/` docs.
> Last researched: 2026-02-24T19:26:57Z

## Current Repo State

The repository is empty aside from `.git/` and `.planning/codebase/` documentation. There is no `Cargo.toml`, no `src/` directory, and no existing Rust code. Everything must be created from scratch.

Existing planning docs establish conventions at:
- `.planning/codebase/STACK.md` -- defines Rust, rmcp, tokio, chrono, serde, schemars
- `.planning/codebase/ARCHITECTURE.md` -- defines module layout and key patterns
- `.planning/codebase/CONVENTIONS.md` -- defines formatting, linting, naming, error handling
- `.planning/codebase/TESTING.md` -- defines test organization and patterns
- `.planning/codebase/INTEGRATIONS.md` -- defines MCP protocol usage and data flow
- `.planning/codebase/CONCERNS.md` -- notes on timezone handling and rmcp maturity

**Important version discrepancy:** `STACK.md` references `rmcp = { version = "0.1" ... }` but the current latest is **0.16.0** (released 2026-02-17). The planning docs need updating.

## Affected Code

| File/Module | Role | Change Type |
|------------|------|-------------|
| `Cargo.toml` | Package manifest, dependencies, binary target | create |
| `src/main.rs` | Entry point: tokio runtime, stdio transport, server init | create |
| `src/server.rs` | TimeServer struct, ToolRouter, ServerHandler impl | create |
| `src/tools/mod.rs` | Tool module declarations | create |
| `src/tools/current_time.rs` | `get_current_time` tool implementation | create |
| `src/tools/convert.rs` | `convert_time` tool implementation | create |
| `.gitignore` | Ignore `target/`, editor files | create |
| `Makefile` | Common tasks (build, test, fmt, clippy, run) | create |

**Note:** `ARCHITECTURE.md` suggests the `tools/` submodule is optional ("if complexity grows"). Given two distinct tools with different parameter types and logic, a `tools/` module is justified and keeps `server.rs` focused on routing/handler concerns.

## Entry Points

The new code has a single entry point:

- **`src/main.rs`** -- Creates a `TimeServer` instance and serves it over stdio transport. Pattern from the official examples and `ARCHITECTURE.md`:

```rust
use rmcp::{transport::stdio, ServiceExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = TimeServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

MCP clients connect via the binary name in their config:
```json
{ "mcpServers": { "time": { "command": "mcp-time", "args": [] } } }
```

## Existing Patterns to Follow

All patterns come from the planning docs since no code exists yet:

- **Tool definition via `#[tool]` macro** -- each tool is an async method on the server struct returning `Result<CallToolResult, McpError>` -- example: `ARCHITECTURE.md` lines 28-34
- **Parameter extraction via `Parameters<T>`** -- request types derive `Deserialize` + `JsonSchema` with `#[schemars(description = "...")]` on fields -- source: [Shuttle MCP guide](https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust)
- **`#[tool_router]` on impl block** -- generates routing; struct must contain a `tool_router: ToolRouter` field initialized via `Self::tool_router()` -- source: [rmcp docs](https://docs.rs/rmcp)
- **`#[tool_handler]` on ServerHandler impl** -- provides `get_info()` returning server name and version -- example: `ARCHITECTURE.md` lines 37-45
- **Error handling** -- `McpError` for tool errors, `anyhow::Result` for internal errors, no `unwrap()` in production -- per `CONVENTIONS.md`
- **Co-located unit tests** -- `#[cfg(test)] mod tests { }` inside each source file -- per `TESTING.md`
- **Integration tests** -- in `tests/` directory using `#[tokio::test]` -- per `TESTING.md`

## Shared Code to Reuse

No existing shared code (greenfield). However, these utilities should be created as reusable within the project:

- **Timezone parsing function** -- validates a timezone string (e.g., "America/New_York") into a `chrono_tz::Tz` value. Used by both `get_current_time` and `convert_time` tools. Should live in a shared location (e.g., a helper in `src/tools/mod.rs` or a `src/timezone.rs` module).

## Dependencies

All dependencies must be added to a new `Cargo.toml`:

| Crate | Version | Features | Purpose |
|-------|---------|----------|---------|
| `rmcp` | `"0.16"` | `["server", "transport-io", "schemars"]` | MCP SDK: server handler, tool routing, stdio transport, JSON Schema |
| `tokio` | `"1"` | `["full"]` | Async runtime (required by rmcp) |
| `chrono` | `"0.4"` | `["serde"]` | Date/time operations |
| `chrono-tz` | `"0.10"` | default | IANA timezone database |
| `serde` | `"1"` | `["derive"]` | Serialization/deserialization |
| `serde_json` | `"1"` | default | JSON handling for tool results |
| `schemars` | `"1.0"` | default | JSON Schema generation for tool parameters |
| `anyhow` | `"1"` | default | Internal error handling |

**Version notes:**
- `rmcp` at `"0.16"` is current as of 2026-02-17. The `macros` feature is included by default (no need to specify). Source: [rmcp crates.io](https://crates.io/crates/rmcp), [docs.rs](https://docs.rs/rmcp)
- `schemars` must be `"1.0"` -- rmcp 0.16 depends on `schemars = { version = "1.0", features = ["chrono04"] }`. Source: [rmcp Cargo.toml on GitHub](https://github.com/modelcontextprotocol/rust-sdk)
- `STACK.md` references `schemars` without a version -- it should be `"1.0"` to match rmcp's requirement

**Dev dependencies:**

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | `"1"` (with `["macros", "rt-multi-thread"]`) | `#[tokio::test]` in tests |

Note: `tokio` with `features = ["full"]` already covers test needs, so no separate dev-dependency entry is required.

## Cargo.toml Structure

```toml
[package]
name = "mcp-time"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "mcp-time"
path = "src/main.rs"

[dependencies]
rmcp = { version = "0.16", features = ["server", "transport-io", "schemars"] }
tokio = { version = "1", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
chrono-tz = "0.10"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1.0"
anyhow = "1"

[profile.release]
strip = true
lto = true
```

**Edition note:** The official rmcp examples use `edition = "2024"`. The planning docs say "2021+", so 2024 is acceptable and preferred for a new project.

## Data Flow

```
MCP Client (Claude Desktop / Claude Code)
    |
    | stdio (JSON-RPC over stdin/stdout)
    |
    v
main.rs  -->  TimeServer::new().serve(stdio())
                |
                v
            server.rs  -->  #[tool_router] dispatches to tools
                |
                +-- tools/current_time.rs
                |     |
                |     +-- Parse optional timezone string -> chrono_tz::Tz
                |     +-- chrono::Utc::now().with_timezone(&tz)
                |     +-- Return formatted ISO 8601 string as CallToolResult
                |
                +-- tools/convert.rs
                      |
                      +-- Parse source time string -> NaiveDateTime
                      +-- Parse source timezone -> chrono_tz::Tz
                      +-- Parse target timezone -> chrono_tz::Tz
                      +-- Convert: source_tz.from_local_datetime(&naive).with_timezone(&target_tz)
                      +-- Return formatted ISO 8601 string as CallToolResult
```

## Build Configuration

- **Makefile** should be created per user preferences (`CLAUDE.md`: "Prefer makefile targets"). Targets:
  - `build` -- `cargo build`
  - `release` -- `cargo build --release`
  - `run` -- `cargo run`
  - `test` -- `cargo test`
  - `fmt` -- `cargo fmt`
  - `lint` -- `cargo clippy -- -D warnings`
  - `check` -- `cargo fmt --check && cargo clippy -- -D warnings`
  - `help` -- list available targets

- **`.gitignore`** should include `/target` and `Cargo.lock` (binary projects should commit `Cargo.lock`, but this is a convention choice -- Rust guidance says to commit it for applications).

## Key Imports Reference

For the server implementation, these are the key imports based on rmcp 0.16:

```rust
// server.rs
use rmcp::{
    ServerHandler, Error as McpError,
    handler::server::tool::ToolRouter,
    model::{ServerInfo, CallToolResult},
    schemars, tool,
};
use rmcp::handler::server::tool::Parameters;

// main.rs
use rmcp::{transport::stdio, ServiceExt};
```

Source: [rmcp docs.rs](https://docs.rs/rmcp), [Shuttle guide](https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust)

## Planning Doc Corrections Needed

The following discrepancies between `.planning/codebase/` docs and current reality should be noted:

1. **`STACK.md`** -- `rmcp = { version = "0.1" ... }` should be `"0.16"`. Feature flags listed are correct.
2. **`STACK.md`** -- Missing `chrono-tz` as a separate dependency (only mentions `chrono`). `chrono-tz` is needed for IANA timezone parsing.
3. **`STACK.md`** -- Missing `anyhow` and `serde_json` from the core dependencies table.
4. **`ARCHITECTURE.md`** -- The `main.rs` pattern should include `.waiting().await?` after `.serve()` -- this is the current rmcp pattern for keeping the server alive.
