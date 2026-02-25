# Integrations

> Last mapped: 2026-02-25T00:06:18Z

## External APIs

This project has no external HTTP API clients, databases, or third-party service integrations. It is a self-contained MCP server that uses the system's IANA timezone database (via `jiff`) and communicates over stdio.

## MCP Protocol (stdio transport)

| Component | Library | Config | Used in |
|-----------|---------|--------|---------|
| MCP server | rmcp (server + transport-io features) | `Cargo.toml` | `src/main.rs`, `src/server.rs` |

The server exposes two MCP tools over stdin/stdout JSON-RPC:
- `get_current_time` -- returns current time in a given IANA timezone
- `convert_time` -- converts a time between two IANA timezones

## Environment Variables

| Variable | Purpose | Referenced in |
|----------|---------|--------------|
| RUST_LOG | Controls tracing log level via `EnvFilter::from_default_env()` | `src/main.rs:8` |

No `.env` files or secrets are used. The only runtime input is the IANA timezone database bundled by `jiff`.

### Prescriptive Guidance
- If adding a new MCP tool, implement it as a method on `TimeServer` in `src/server.rs` using the `#[tool]` attribute macro from rmcp. The `#[tool_router]` and `#[tool_handler]` macros on the impl blocks handle registration automatically.
- No external network calls exist. If a future tool needs HTTP access, add a client dependency and document the env vars for configuration here.
- Logging is controlled entirely by `RUST_LOG`. Set `RUST_LOG=debug` or `RUST_LOG=mcp_time=debug` for development output on stderr.
