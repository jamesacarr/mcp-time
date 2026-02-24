# Approach Research

> Task: Create a new Rust-based MCP server with two tools: (1) fetch current time in a specific timezone (defaulting to UTC), and (2) convert time between timezones. Reference Python implementation: https://github.com/modelcontextprotocol/servers/tree/main/src/time
> Last researched: 2026-02-24T19:26:41Z

## Reference Implementation Analysis

The [Python MCP time server](https://github.com/modelcontextprotocol/servers/blob/main/src/time/src/mcp_server_time/server.py) defines two tools:

**`get_current_time`** -- Input: `timezone` (string, IANA format). Returns: timezone name, ISO-formatted datetime, day of week, DST status.

**`convert_time`** -- Inputs: `source_timezone`, `time` (HH:MM 24hr format), `target_timezone`. Returns: source/target TimeResult objects plus formatted hour offset string (handles fractional offsets like Nepal's +5:45).

Key behaviours: validates timezone names (returns `INVALID_PARAMS` error for bad ones), validates time format (expects HH:MM), anchors conversion to today's date in source timezone, calculates UTC offset difference between zones.

## MCP SDK Decision: rmcp

There is effectively one viable Rust MCP SDK: **[rmcp](https://github.com/modelcontextprotocol/rust-sdk)** (the official Rust SDK, now at v0.16.0, published 2026-02-17). No competing crates have meaningful adoption. This is not a choice point -- rmcp is the correct answer.

Key rmcp patterns for this server:
- `#[tool(description = "...")]` macro on async methods to define tools
- `#[tool_router]` on the impl block to auto-generate tool routing
- `#[tool_handler]` on the `ServerHandler` impl for metadata
- `Parameters<T>` wrapper or `#[tool(param)]` / `#[tool(aggr)]` for input deserialization
- `CallToolResult::success(vec![Content::text(...)])` for return values
- `McpError` with `ErrorCode` for structured errors
- `.serve(stdio()).await?.waiting().await?` for stdio transport lifecycle

**Sources:** [rmcp crates.io](https://crates.io/crates/rmcp), [rmcp README](https://github.com/modelcontextprotocol/rust-sdk/blob/main/crates/rmcp/README.md), [Shuttle guide](https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust), [HackMD guide](https://hackmd.io/@Hamze/S1tlKZP0kx)

## Viable Approaches

### Approach 1: rmcp + chrono + chrono-tz

- **What:** Use `chrono` for datetime operations and `chrono-tz` for IANA timezone support. This is what the existing planning docs (`STACK.md`, `CONCERNS.md`, `INTEGRATIONS.md`) already suggest.
- **How:** `chrono::Utc::now().with_timezone(&tz)` for current time; parse HH:MM with `NaiveTime::parse_from_str`, combine with today's date via `NaiveDate`, localize with `chrono_tz::Tz`, then convert with `.with_timezone()`. DST detection via `offset().fix().local_minus_utc()` comparison. `chrono-tz` embeds the full IANA timezone database at compile time.
- **Pros:**
  - Mature, battle-tested ecosystem (chrono has 200M+ downloads)
  - `chrono-tz` provides `Tz` enum with compile-time validation of timezone names
  - `schemars` 1.0 supports chrono types natively via `chrono04` feature flag
  - Extensive documentation and community examples
  - `Tz` implements `FromStr` for easy string-to-timezone parsing
- **Cons:**
  - Timezone database is baked in at compile time -- requires crate update for IANA DB changes
  - DST gap/fold handling is limited -- `MappedLocalTime::None` for gaps gives minimal info
  - `chrono-tz` adds compile time (code generation from IANA DB)
  - Separate crate needed for timezone support (chrono alone only has `FixedOffset`)
- **Best when:** You want maximum ecosystem compatibility and a proven approach with no surprises.
- **Sources:** [chrono-tz GitHub](https://github.com/chronotope/chrono-tz), [schemars features](https://docs.rs/schemars/latest/schemars/)

### Approach 2: rmcp + jiff

- **What:** Use `jiff` (by BurntSushi, author of ripgrep) as a modern all-in-one datetime + timezone library. No separate timezone crate needed.
- **How:** `Zoned::now().with_time_zone(tz)` for current time; `jiff::civil::Time::strptime("%H:%M", input)` to parse, combine with today's date, attach timezone, then `.in_tz("Target/Zone")` to convert. DST is handled automatically with "compatible" strategy (RFC 5545). Uses system IANA timezone database on Unix (`/usr/share/zoneinfo`), embeds on Windows.
- **Pros:**
  - Correct-by-default DST handling (gap/fold resolution built in)
  - Uses system timezone database -- always up to date without recompilation
  - Single crate replaces both `chrono` and `chrono-tz`
  - Lossless timezone serialization (RFC 9557)
  - Unified `Span` type for duration arithmetic
  - `schemars` 1.0 supports jiff types via `jiff02` feature flag
  - Cleaner API for timezone conversions: `zdt.in_tz("America/New_York")?`
- **Cons:**
  - Newer library (released mid-2024) -- less community precedent for MCP servers
  - `Zoned` is not `Copy` (embeds timezone data), though this is irrelevant for this server
  - System timezone DB dependency means behaviour varies by host OS (mitigated by `jiff-tzdb` feature to embed)
  - Fewer Stack Overflow answers and tutorials vs chrono
- **Best when:** You want the most correct timezone handling with a modern, ergonomic API.
- **Sources:** [jiff GitHub](https://github.com/BurntSushi/jiff), [jiff comparison docs](https://docs.rs/jiff/latest/jiff/_documentation/comparison/index.html), [jiff Zoned docs](https://docs.rs/jiff/latest/jiff/struct.Zoned.html)

### Approach 3: rmcp + chrono + chrono-tz (flat) vs. modular structure

This is an orthogonal structural decision rather than a library choice. Both Approach 1 and 2 can be built with either structure:

**Flat (single `server.rs`):** All tool logic lives in one file alongside the `ServerHandler` impl. Matches the reference Python implementation.

**Modular (`tools/` directory):** Each tool in its own module (`tools/current_time.rs`, `tools/convert.rs`), with the server struct importing and delegating.

- The existing `ARCHITECTURE.md` suggests flat-first with optional modules if complexity grows. Given this server has only two simple tools, flat is appropriate.

## Recommendation

**Use Approach 2: rmcp + jiff.** Rationale:

1. **Correctness matters for a timezone tool.** Jiff's automatic DST gap/fold handling and system timezone database make it the safer choice for a tool whose entire purpose is timezone operations. Chrono-tz's compile-time database and weaker DST handling are fine for most apps but suboptimal for a dedicated time server.

2. **Simpler dependency graph.** One crate (`jiff`) vs two (`chrono` + `chrono-tz`), with a cleaner API for the exact operations this server needs (parse time, attach timezone, convert).

3. **System TZ database.** The server will always use the most current timezone data without needing a crate update. For safety on minimal containers, enable the `jiff-tzdb` feature as a fallback.

4. **schemars compatibility confirmed.** `schemars ^1.0` (required by rmcp 0.16) supports jiff via the `jiff02` feature flag, so tool input schemas work seamlessly.

5. **The existing planning docs suggest chrono** (`STACK.md`, `INTEGRATIONS.md`), but those were generated before this analysis. The recommendation here supersedes them -- the Planner should update those docs.

**Concrete dependency set:**

```toml
[dependencies]
rmcp = { version = "0.16", features = ["server", "transport-io"] }
jiff = { version = "0.2", features = ["serde"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "io-std"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = { version = "1", features = ["jiff02"] }
anyhow = "1"
```

**Structure:** Flat layout. `src/main.rs` for entry point, `src/server.rs` for `TimeServer` struct with `#[tool_router]` impl and `#[tool_handler]` `ServerHandler` impl. No `tools/` directory unless the Planner decides to add more tools later.

**Transport:** stdio only. This is the standard for local MCP servers and matches the reference implementation.

## Open Questions

1. **rmcp macro API at v0.16:** The examples found reference various rmcp versions (0.1, 0.3, 0.12). The macro names (`#[tool_router]` vs `#[tool_box]`) may have changed between versions. The Shuttle guide (July 2025) uses `#[tool_router]` + `Parameters<T>`; the HackMD guide uses `#[tool_box]` + `#[tool(aggr)]`. The Planner should verify the exact macro API against rmcp 0.16 docs or by running `cargo doc` after initial setup.

2. **`convert_time` -- single vs multiple target timezones:** The Python reference accepts a single `target_timezone` string. Some versions of the Python server may accept a list. The Planner should decide whether to match the single-target behaviour or support multiple targets.

3. **Error format for invalid timezones:** The Python server uses `McpError(INVALID_PARAMS, message)`. rmcp's Rust `McpError` has a different shape (`ErrorCode`, `message: Cow<str>`, `data: Option`). Need to confirm the exact construction at v0.16.

4. **tracing/logging:** rmcp depends on `tracing`. Should the server configure a tracing subscriber (e.g., `tracing-subscriber` with stderr output) for debugging, or omit it for simplicity? Stdio transport means stdout is reserved for MCP protocol messages -- logs must go to stderr.
