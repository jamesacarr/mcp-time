# Stack

> Last mapped: 2026-02-25T00:06:18Z

## Languages
- Rust (edition 2024, MSRV 1.85) -- configured in `Cargo.toml`
- Stable toolchain -- configured in `rust-toolchain.toml`

## Frameworks
- **rmcp** 0.16 -- MCP (Model Context Protocol) server SDK; entry point: `src/main.rs`
- **tokio** 1.49 -- async runtime with `macros`, `rt-multi-thread`, `io-std` features; entry point: `src/main.rs`

## Package Manager
- Cargo -- lockfile: `Cargo.lock`

## Key Dependencies

| Dependency | Version (locked) | Purpose | Used in |
|-----------|-----------------|---------|---------|
| rmcp | 0.16.0 | MCP server framework (server + stdio transport) | `src/server.rs`, `src/main.rs` |
| jiff | 0.2.21 | Timezone-aware datetime library (IANA tz database) | `src/server.rs` |
| tokio | 1.49.0 | Async runtime | `src/main.rs` |
| serde | 1.0.228 | Serialization/deserialization (derive) | `src/server.rs` |
| serde_json | 1.0.149 | JSON serialization for tool responses | `src/server.rs` |
| schemars | 1.2.1 | JSON Schema generation for tool parameters (jiff02 feature) | `src/server.rs` |
| anyhow | 1.0.102 | Error handling in main | `src/main.rs` |
| tracing-subscriber | 0.3.22 | Logging to stderr via RUST_LOG env filter | `src/main.rs` |

## Build & Dev Tools
- **Makefile**: `Makefile` -- primary task runner (`make help` for targets)
- **cargo fmt**: formatting (`make fmt`)
- **cargo clippy**: linting with `-D warnings` (`make lint`)
- **Release profile**: LTO + symbol stripping enabled in `Cargo.toml` (`[profile.release]`)

## Build Targets
- Library: `mcp_time` -- `src/lib.rs`
- Binary: `mcp-time` -- `src/main.rs`

### Prescriptive Guidance
- Use Makefile targets (`make build`, `make test`, `make lint`, `make fmt`) rather than invoking cargo directly.
- When adding dependencies, pin to major version (e.g., `version = "1"`) consistent with existing `Cargo.toml` style.
- The project uses Rust edition 2024; new code should use current stable Rust idioms and the 2024 edition import rules.
- All clippy warnings are treated as errors (`-D warnings`); fix all clippy issues before committing.
- The binary uses stdio transport -- it reads/writes JSON-RPC over stdin/stdout. Logging goes to stderr.
