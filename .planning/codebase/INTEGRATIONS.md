# Integrations

> Generated: 2026-02-24T19:09:23Z

## MCP Protocol

- **Protocol**: Model Context Protocol (MCP)
- **Role**: Server — exposes time-related tools to MCP clients
- **Transport**: stdio (primary), potentially streamable HTTP later
- **SDK**: `rmcp` — official Rust MCP SDK

## MCP Client Integration

MCP clients (Claude Desktop, Claude Code, etc.) connect to this server via:

```json
{
  "mcpServers": {
    "time": {
      "command": "mcp-time",
      "args": []
    }
  }
}
```

## External APIs

- None expected — time operations use system clock and `chrono` crate
- No network calls, databases, or third-party services

## Data Flow

```
MCP Client (Claude) <--stdio--> mcp-time server
                                  |
                                  +-- chrono (system clock)
```
