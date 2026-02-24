# Architecture

> Generated: 2026-02-24T19:09:23Z

## Structure

Single-binary MCP server. Flat module layout appropriate for a focused tool server.

```
mcp-time/
  Cargo.toml
  src/
    main.rs          # Entry point: tokio runtime, stdio transport, server init
    server.rs        # ServerHandler impl, tool router, tool definitions
    tools/           # (optional) Separate modules per tool if complexity grows
      mod.rs
      current_time.rs
      convert.rs
      ...
  tests/             # Integration tests
```

## Key Patterns

### Server Definition (rmcp)

```rust
#[tool_router]
impl TimeServer {
    #[tool(description = "Get the current time")]
    async fn get_current_time(&self, #[tool(param)] timezone: Option<String>) -> Result<CallToolResult, McpError> {
        // ...
    }
}

#[tool_handler]
impl ServerHandler for TimeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "mcp-time".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            ..Default::default()
        }
    }
}
```

### Entry Point

```rust
#[tokio::main]
async fn main() -> Result<()> {
    TimeServer::new()
        .serve(rmcp::transport::stdio())
        .await?;
    Ok(())
}
```

## Design Decisions

- **Single binary** — no library crate needed; this is a standalone server
- **stdio transport** — standard for local MCP servers, no HTTP needed initially
- **Flat structure** — tools defined directly on the server struct unless complexity warrants separate modules
- **No state** — time queries are stateless; no database or persistence needed
