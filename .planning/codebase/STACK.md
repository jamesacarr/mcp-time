# Stack

> Generated: 2026-02-24T19:09:23Z

## Language

- **Rust** (stable toolchain)
- Edition: 2021+

## Core Dependencies

| Crate | Purpose |
|-------|---------|
| `rmcp` | Official Rust MCP SDK — server handler, tool routing, transport |
| `tokio` | Async runtime (required by rmcp) |
| `serde` / `serde_json` | Serialization/deserialization |
| `schemars` | JSON Schema generation for tool parameters |
| `chrono` | Date/time operations (core domain) |

## rmcp Feature Flags

```toml
[dependencies]
rmcp = { version = "0.1", features = ["server", "macros", "transport-io", "schemars"] }
```

- `server` — server-side handler and tool routing
- `macros` — `#[tool]`, `#[tool_router]`, `#[tool_handler]` derive macros
- `transport-io` — stdio transport for CLI-based MCP servers
- `schemars` — automatic JSON Schema from Rust types

## Build System

- **Cargo** — standard Rust build tool
- `cargo build` / `cargo run` / `cargo test`

## Runtime

- **Tokio** async runtime
- stdio transport (MCP standard for local servers)
