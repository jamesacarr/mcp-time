# Architecture

> Last mapped: 2026-02-25T00:06:14Z

## Directory Structure

```
mcp-time/
  .github/workflows/ci.yml  # CI pipeline (fmt, clippy, test)
  src/
    main.rs                  # Binary entry point -- stdio transport bootstrap
    lib.rs                   # Library crate root -- re-exports `server` module
    server.rs                # Core logic -- TimeServer, tool handlers, helpers, unit tests
  tests/
    integration.rs           # Integration tests -- protocol-level behavior
  Cargo.toml                 # Package manifest (lib + bin targets)
  Cargo.lock                 # Pinned dependency versions
  Makefile                   # Dev workflow targets (build, test, lint, fmt)
  rust-toolchain.toml        # Pins Rust stable channel
```

## Module Boundaries

The crate exposes both a library (`mcp_time`) and a binary (`mcp-time`):

| Target | Path | Responsibility |
|--------|------|---------------|
| Library (`mcp_time`) | `src/lib.rs` | Public API -- exposes `server` module for use by tests and the binary |
| Binary (`mcp-time`) | `src/main.rs` | Bootstrap only -- initializes tracing, creates `TimeServer`, binds stdio transport, awaits shutdown |
| Server module | `src/server.rs` | All domain logic -- `TimeServer` struct, MCP tool handlers (`get_current_time`, `convert_time`), input validation helpers, response serialization, unit tests |
| Integration tests | `tests/integration.rs` | Exercises tools through the public library API at MCP protocol level |

There is a single flat module (`server`). No sub-modules, no further nesting.

## Data Flow

```
stdin (JSON-RPC) --> rmcp stdio transport --> rmcp ToolRouter --> TimeServer handler methods
                                                                       |
                                                             jiff (timezone ops)
                                                                       |
                                                             serde_json serialize
                                                                       |
stdout (JSON-RPC) <-- rmcp stdio transport <-- CallToolResult <--------+
```

1. **Entry** (`src/main.rs:6-14`): `main()` initializes tracing to stderr, creates a `TimeServer`, binds it to `rmcp::transport::stdio()`, and awaits the service.
2. **Routing** (`src/server.rs:84-85`): The `#[tool_router]` proc macro on `impl TimeServer` auto-generates a `ToolRouter` that dispatches incoming MCP `tools/call` requests to the matching method by tool name.
3. **Tool execution** (`src/server.rs:91-128`, `src/server.rs:135-219`): Each tool method deserializes params via `Parameters<T>`, validates input, performs timezone operations using `jiff`, serializes the response to JSON, and returns a `CallToolResult`.
4. **Error path** (`src/server.rs:243-245`): Input validation errors return `CallToolResult::error()` with `is_error: true` -- they do not propagate as Rust errors. Only internal failures (serialization, date construction) use `rmcp::ErrorData`.
5. **Server info** (`src/server.rs:223-238`): The `ServerHandler` impl provides MCP server metadata (name, version, capabilities) via `get_info()`.

## Key Patterns

### Single-module flat architecture
All server logic lives in `src/server.rs`. There are no layers (no repository, no service layer, no middleware). The module contains the struct, handlers, helpers, response types, and unit tests in one file.

### rmcp proc-macro-driven tool registration
Tools are registered declaratively using `#[tool_router]` and `#[tool(...)]` attributes on `impl TimeServer` (`src/server.rs:84-88`). The `#[tool_handler]` attribute on the `ServerHandler` impl (`src/server.rs:222`) wires the router into the MCP protocol. No manual dispatch code is needed.

### Validation-first, error-as-value
Tool methods validate all inputs before performing any operations. Invalid input returns `CallToolResult::error()` (a successful MCP response with `is_error: true`), not a Rust `Err`. This matches MCP convention -- the protocol call succeeds but the tool reports failure. See `tool_error()` helper at `src/server.rs:243-245` and `parse_timezone()` at `src/server.rs:252-288`.

### Library-first testing
The binary (`main.rs`) is a thin bootstrap. All testable logic is in the library crate, allowing both unit tests (inline in `src/server.rs:311-718`) and integration tests (`tests/integration.rs`) to exercise handlers directly without spinning up the stdio transport.

### Typed parameter schemas
Tool parameters use dedicated structs (`GetCurrentTimeParams`, `ConvertTimeParams`) with `serde::Deserialize` and `schemars::JsonSchema` derives (`src/server.rs:26-42`). The `schemars` derive auto-generates JSON Schema for MCP tool discovery.

## Prescriptive Guidance

- **New tools**: Add a new async method to the `#[tool_router] impl TimeServer` block in `src/server.rs`, annotated with `#[tool(name = "...", description = "...")]`. Create a dedicated params struct with `Deserialize` + `JsonSchema` derives. Follow the existing pattern of validate-then-operate-then-serialize.
- **New modules**: If the codebase grows beyond a single domain (e.g., adding date arithmetic or calendar tools), extract into a new module under `src/` and re-export from `src/lib.rs`. Keep the flat structure as long as the file stays under ~500 lines.
- **New response types**: Define private response structs with `Serialize` in `src/server.rs` (see `CurrentTimeResponse`, `ConvertTimeResponse`). Serialize to JSON with `serde_json::to_string_pretty()` and wrap in `Content::text()`.
- **Error handling**: Use `tool_error()` for user-facing input validation errors. Use `rmcp::ErrorData::internal_error()` only for unexpected internal failures. Never panic in tool handlers.
- **Transport**: The binary currently supports only stdio transport. To add HTTP/SSE, modify `src/main.rs` to conditionally select transport -- do not change `src/server.rs`.
