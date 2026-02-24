# Plan: mcp-time-server

> status: planning
> created: 2026-02-24T19:30:59Z
> updated: 2026-02-24T19:37:19Z

## Goal

A working Rust-based MCP server binary (`mcp-time`) that exposes two tools over stdio transport -- `get_current_time` (returns current time in a given timezone, defaulting to UTC) and `convert_time` (converts a time between two timezones) -- matching the reference Python implementation's behaviour, with full test coverage and CI.

## Success Criteria

1. `cargo build` compiles the `mcp-time` binary without errors or warnings
2. `cargo test` passes all unit and integration tests (zero failures)
3. `cargo fmt --check` reports no formatting violations
4. `cargo clippy --all-targets --all-features -- -D warnings` reports zero warnings
5. The `get_current_time` tool returns ISO 8601 datetime, timezone name, UTC offset, and DST status for any valid IANA timezone
6. The `get_current_time` tool defaults to UTC when no timezone is provided
7. The `convert_time` tool correctly converts a time (HH:MM format) between two valid IANA timezones, including fractional-offset zones (e.g., Asia/Kathmandu at +5:45)
8. Both tools return structured MCP error responses with `isError: true` for invalid timezone strings or malformed time input
9. The server responds to `tools/list` with metadata for both tools including descriptions and JSON Schema for parameters
10. The binary runs as a stdio MCP server and exits cleanly when stdin closes

## Non-Functional Requirements

| Category | Requirement | Testable Criterion |
|----------|------------|-------------------|
| Security | All user-supplied timezone strings validated before use | Unit tests confirm invalid timezone strings return `CallToolResult` with `is_error: true`, not panic |
| Security | All user-supplied time strings parsed with explicit format | Unit tests confirm malformed time strings (e.g., "25:99", "abc", "24:00") return `CallToolResult` with `is_error: true` |
| Security | No `unwrap()` or `expect()` on user-input paths | `cargo clippy` passes; manual review during implementation |
| Performance | Timezone lookups are O(1) via jiff's timezone database | No specific test needed; inherent to jiff's design |
| Correctness | DST gap times (spring forward) produce clear error or documented resolution | Unit test for DST gap time returns explanatory response |
| Correctness | DST fold times (fall back) resolve deterministically | Unit test for ambiguous time resolves consistently |
| Correctness | Fractional UTC offsets (e.g., +5:45) format correctly | Unit test for Asia/Kathmandu shows correct offset |
| Correctness | `is_dst` field accurately reflects DST status | Unit test asserts `is_dst: true` for America/New_York in July and `is_dst: false` in January |
| Robustness | Input validation errors and internal errors use distinct handling paths | Input validation returns `CallToolResult` with `is_error: true`; unexpected internal errors propagate as `Err(McpError)` |

## Dependencies Between Waves

- Wave 1 creates project scaffolding (Cargo.toml, main.rs, .gitignore, Makefile, rust-toolchain.toml) and verifies the rmcp API surface -- all other waves depend on this
- Wave 2 implements the server struct and both tools -- depends on Wave 1 for compilable project and confirmed rmcp API
- Wave 3 adds unit tests -- depends on Wave 2 for the code under test
- Wave 4 adds integration tests and CI -- depends on Waves 2-3 for a testable server

---

## Wave 1: Project Scaffolding

> status: pending

Create the Rust project structure, dependencies, and build tooling. After this wave, `cargo check` should succeed (with stub code), the rmcp API surface should be verified, and `rust-toolchain.toml` should ensure correct toolchain selection.

### Task 1.1: Create Cargo.toml

- **File(s):** `Cargo.toml`
- **Action:** Create `Cargo.toml` at project root with:
  - `[package]` section: `name = "mcp-time"`, `version = "0.1.0"`, `edition = "2024"`, `rust-version = "1.85"`
  - `[[bin]]` section: `name = "mcp-time"`, `path = "src/main.rs"`
  - `[dependencies]`:
    - `rmcp = { version = "0.16", features = ["server", "transport-io"] }` (macros are default, schemars via rmcp re-export)
    - `jiff = { version = "0.2", features = ["serde"] }` (recommended over chrono per approach research for better DST handling and system TZ database)
    - `tokio = { version = "1", features = ["macros", "rt-multi-thread", "io-std"] }`
    - `serde = { version = "1", features = ["derive"] }`
    - `serde_json = "1"`
    - `schemars = { version = "1", features = ["jiff02"] }` (jiff02 feature for JSON Schema generation of jiff types)
    - `anyhow = "1"`
    - `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` (logging to stderr, needed since stdout is reserved for MCP protocol)
  - `[profile.release]`: `strip = true`, `lto = true`
- **Verification:** `cargo check` succeeds (after Task 1.2 provides a minimal main.rs)
- **Done when:** `Cargo.toml` exists with all dependencies listed above and `cargo check` passes

### Task 1.2: Create src/main.rs stub

- **File(s):** `src/main.rs`
- **Action:** Create `src/main.rs` with:
  - A minimal `#[tokio::main] async fn main() -> anyhow::Result<()>` that initializes `tracing_subscriber` with stderr writer and `RUST_LOG` env filter, then prints a placeholder and returns `Ok(())`
  - Add `mod server;` declaration (commented out until server.rs exists in Wave 2)
  - Follow entry point pattern from `ARCHITECTURE.md`: the final version will call `TimeServer::new().serve(stdio()).await?.waiting().await?` but for now just ensure compilation
- **Verification:** `cargo check` succeeds
- **Done when:** `src/main.rs` compiles and `cargo run` exits cleanly

### Task 1.3: Create .gitignore

- **File(s):** `.gitignore`
- **Action:** Create `.gitignore` with:
  - `/target` (build artifacts)
  - `*.swp`, `*.swo`, `*~` (editor temp files)
  - `.DS_Store` (macOS)
  - Do NOT ignore `Cargo.lock` -- this is a binary project, so `Cargo.lock` should be committed per Rust convention
- **Verification:** `cat .gitignore` shows expected entries
- **Done when:** `.gitignore` exists with `/target` and editor file patterns

### Task 1.4: Create Makefile

- **File(s):** `Makefile`
- **Action:** Create `Makefile` per user preference (`CLAUDE.md`: "Prefer makefile targets"). Include targets:
  - `.PHONY` declarations for all targets
  - `help` (default target): list all available targets with descriptions using `@grep -E '^[a-zA-Z_-]+:.*?## .*$$'` self-documenting pattern
  - `build`: `cargo build` -- `## Build debug binary`
  - `release`: `cargo build --release` -- `## Build release binary`
  - `run`: `cargo run` -- `## Run the server`
  - `test`: `cargo test` -- `## Run all tests`
  - `fmt`: `cargo fmt` -- `## Format code`
  - `lint`: `cargo clippy --all-targets --all-features -- -D warnings` -- `## Run clippy linter`
  - `check`: `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings` -- `## Check formatting and linting`
  - `clean`: `cargo clean` -- `## Clean build artifacts`
- **Verification:** `make help` lists all targets with descriptions
- **Done when:** `make help` outputs a formatted list of targets; `make build` triggers `cargo build`

### Task 1.5: Create rust-toolchain.toml

- **File(s):** `rust-toolchain.toml`
- **Action:** Create `rust-toolchain.toml` at project root per risks research recommendation (risk: "rmcp requires Rust Edition 2024... Ensure `rust-toolchain.toml` specifies a minimum of 1.85"). Contents:
  ```toml
  [toolchain]
  channel = "stable"
  ```
  This ensures `rustup` selects the correct stable toolchain for contributors cloning the repo. The `rust-version = "1.85"` in `Cargo.toml` provides MSRV enforcement, while this file ensures `rustup` auto-selects the stable channel.
- **Verification:** `rustup show` shows the stable toolchain is selected when run from the project root
- **Done when:** `rust-toolchain.toml` exists and `rustup show` confirms stable toolchain selection

### Task 1.6: Verify rmcp v0.16 API surface

- **File(s):** (no files created; verification-only task)
- **Action:** After `cargo check` succeeds from Tasks 1.1-1.2, verify the exact rmcp 0.16 macro API by running `cargo doc -p rmcp --no-deps` and inspecting the generated documentation. Specifically determine:
  1. **Macro names:** Whether the tool routing macro is `#[tool_router]` or `#[tool_box]` (approach research Open Question 1 notes both have appeared in different guides)
  2. **Parameter passing pattern:** Whether tool parameters use `Parameters<T>` wrapper, `#[tool(aggr)]` attribute on a struct parameter, or `#[tool(param)]` on individual parameters
  3. **`CallToolResult` constructors:** The exact method names for creating success and error responses (e.g., `CallToolResult::success(...)` vs `CallToolResult::new(...)`)
  4. **`ServerHandler` trait requirements:** The exact trait name and required methods (e.g., `get_info()` return type)

  Document findings in a comment block at the top of `src/server.rs` (created in Wave 2) or as notes passed to the Wave 2 executor. If the API differs from what is assumed in Task 2.1, the executor must adapt accordingly. The key fallback patterns are:
  - If `#[tool_router]` does not exist, use `#[tool_box]`
  - If `Parameters<T>` does not exist, use `#[tool(aggr)]` on the struct parameter
  - If `CallToolResult::success()` does not exist, construct `CallToolResult` directly with field initialization
- **Verification:** `cargo doc -p rmcp --no-deps` completes successfully; the executor can identify the correct macro names in the generated docs under `rmcp::handler::server` or `rmcp::macros`
- **Done when:** The exact macro names, parameter patterns, and result constructors for rmcp 0.16 are confirmed and documented for use in Wave 2

---

## Wave 2: Server Implementation

> status: pending

Implement the MCP server struct with both tools. After this wave, the server binary should start, respond to MCP protocol messages over stdio, and correctly handle `get_current_time` and `convert_time` tool calls. Implementation must use the rmcp API surface confirmed in Task 1.6.

### Task 2.1: Implement server.rs with TimeServer and both tools

- **File(s):** `src/server.rs`
- **Action:** Create `src/server.rs` with the following. Use jiff (not chrono) per approach research recommendation. Adapt macro names and parameter patterns to match findings from Task 1.6.

  **Error handling strategy (addresses critique Objection 3):**
  Two-tier error handling:
  1. **Input validation errors** (bad timezone, malformed time): Return `CallToolResult` with `is_error: true` and a human-readable message. Use a private helper `fn tool_error(msg: impl Into<String>) -> CallToolResult` that constructs `CallToolResult { content: vec![Content::text(msg)], is_error: true }` to avoid inline construction and string duplication.
  2. **Internal/unexpected errors** (jiff panics caught, serialization failures): Propagate as `Err(McpError::internal_error("description", None))` via the `?` operator so rmcp returns a JSON-RPC error.

  Define error message constants or an enum for reuse across tools and tests:
  ```rust
  const ERR_INVALID_TIMEZONE: &str = "Invalid timezone: {}. Please use a valid IANA timezone name (e.g., 'America/New_York').";
  const ERR_INVALID_TIME_FORMAT: &str = "Invalid time format: {}. Expected HH:MM in 24-hour format (e.g., '14:30').";
  ```

  Note on `CONVENTIONS.md` alignment: `CONVENTIONS.md` says "Use `McpError` for tool-level errors returned to MCP clients." However, the MCP specification and risks research (lines 60-61) clarify that tool execution failures (bad input) should return a successful JSON-RPC response with `is_error: true` in `CallToolResult`, not a JSON-RPC error. `McpError` / `Err(...)` is reserved for protocol-level and internal failures. This plan follows the MCP spec. `CONVENTIONS.md` should be updated post-implementation to reflect this distinction.

  **Struct definition:**
  - `pub struct TimeServer` -- if rmcp 0.16 uses `#[tool_router]`: include a `tool_router: ToolRouter` field initialized via `Self::tool_router()`. If rmcp uses `#[tool_box]`: adapt accordingly per Task 1.6 findings.
  - Constructor `TimeServer::new()` that initializes the router field.

  **Tool impl block (use `#[tool_router]` or `#[tool_box]` per Task 1.6 findings):**

  1. `get_current_time` tool:
     - Description: `"Get the current time in a specific timezone. Defaults to UTC if no timezone is provided."`
     - Parameter: `timezone: Option<String>` with schemars description `"IANA timezone name (e.g., 'America/New_York', 'Europe/London', 'Asia/Tokyo'). Defaults to UTC."`
     - Parameter passing: Use `#[tool(param)]` on the parameter if rmcp supports it, OR wrap in a `GetCurrentTimeParams` struct with `#[tool(aggr)]` / `Parameters<T>`. Adapt per Task 1.6.
     - Logic: Parse timezone string to `jiff::tz::TimeZone` via `parse_timezone()` helper (default to `jiff::tz::TimeZone::UTC` if None or empty string). Get current time via `jiff::Zoned::now().with_time_zone(tz)`. Format response as JSON object containing: `timezone` (IANA name), `datetime` (ISO 8601 via `zdt.strftime("%Y-%m-%dT%H:%M:%S%:z")`), `utc_offset` (formatted via `format_utc_offset()` helper), `is_dst` (boolean, see DST detection below).
     - **DST detection algorithm (addresses critique Objection 2):** Determine DST status by comparing the current offset against the zone's standard (non-DST) offset. Compute the standard offset by getting the offset for January 1 of the current year in the same timezone (January is non-DST in the Northern Hemisphere and DST-free in most Southern Hemisphere zones that observe DST from October-March). Specifically:
       ```
       let now = jiff::Zoned::now().with_time_zone(tz);
       let jan1 = jiff::civil::date(now.year(), 1, 1).at(12, 0, 0, 0).to_zoned(tz)?;
       let is_dst = now.offset() != jan1.offset();
       ```
       Add a code comment explaining this heuristic and noting that it may be inaccurate for zones that observe DST during their winter (e.g., some Southern Hemisphere zones where January IS summer/DST). For this server's purposes (informational `is_dst` field matching the Python reference), this heuristic is acceptable.
     - Error handling: Invalid timezone calls `tool_error(format!(ERR_INVALID_TIMEZONE, input))` and returns early.
     - Return `CallToolResult::success(vec![Content::text(json_string)])` (adapt constructor per Task 1.6).

  2. `convert_time` tool:
     - Description: `"Convert a time from one timezone to another."`
     - Parameters: Define a `ConvertTimeParams` struct with `schemars::JsonSchema` + `serde::Deserialize`. Use `#[tool(aggr)]` or `Parameters<ConvertTimeParams>` per Task 1.6 findings. Fields:
       - `source_timezone: String` -- `#[schemars(description = "Source IANA timezone name (e.g., 'America/New_York')")]`
       - `time: String` -- `#[schemars(description = "Time to convert in 24-hour format (HH:MM)")]`
       - `target_timezone: String` -- `#[schemars(description = "Target IANA timezone name (e.g., 'Europe/London')")]`
     - **Design decision on time format (addresses critique Objection 4):** Accept only `HH:MM` format (strict). Reject `HH:MM:SS` (with seconds), `24:00`, and full ISO 8601 datetimes with a clear error message indicating the expected format. This matches the Python reference implementation's behaviour.
     - Logic: Trim whitespace from `time` input. Parse source/target timezone strings via `parse_timezone()`. Parse time with `jiff::civil::Time::strptime("%H:%M", &trimmed_time)` -- if this fails, return `tool_error(format!(ERR_INVALID_TIME_FORMAT, input))`. Additionally validate that the parsed hour is 0-23 and minute is 0-59 (jiff's parser should handle this, but verify). Combine with today's date in source timezone: `jiff::civil::date(today.year(), today.month(), today.day()).at(hour, minute, 0, 0).to_zoned(source_tz)?`. Handle DST gaps: if `to_zoned()` returns an error for a nonexistent time (spring forward gap), return a `tool_error` explaining the time does not exist due to DST. Convert to target timezone via `zdt.with_time_zone(target_tz)`. Calculate time difference as the difference between source and target UTC offsets. Format response as JSON object containing: `source` (object with `timezone`, `datetime`, `utc_offset`), `target` (object with `timezone`, `datetime`, `utc_offset`), `time_difference` (formatted offset string like "+5:30" or "-3:00").
     - Error handling: Invalid timezone or time format calls `tool_error()`. DST gap times return `tool_error` with explanatory message.

  **`ServerHandler` impl (use `#[tool_handler]` or equivalent per Task 1.6):**
  - `fn get_info(&self) -> ServerInfo` returning name `"mcp-time"`, version `env!("CARGO_PKG_VERSION")`

  **Helper functions (private):**
  - `fn tool_error(msg: impl Into<String>) -> CallToolResult` -- constructs a `CallToolResult` with `is_error: true` and the message as text content. Single point of construction for all input validation errors.
  - `fn parse_timezone(input: &str) -> Result<jiff::tz::TimeZone, String>` -- validates and parses IANA timezone string. Returns human-readable error on failure. Rejects timezone abbreviations (`"EST"`, `"PST"`) and raw UTC offsets (`"UTC+5"`, `"+05:30"`) with a helpful message suggesting the IANA equivalent.
  - `fn format_utc_offset(offset: jiff::tz::Offset) -> String` -- formats offset as "+HH:MM" or "-HH:MM", handling fractional hours correctly (e.g., +05:45 for Asia/Kathmandu).

  **Documentation:**
  - `///` doc comments on `TimeServer`, both tool methods, and public helpers
  - All parameter structs have `#[schemars(description = "...")]` on every field

- **Verification:** `cargo build` succeeds; `cargo run` starts the server (blocks on stdin)
- **Done when:** `cargo build` compiles without errors or warnings; the binary starts and waits for MCP input on stdin

### Task 2.2: Wire up server in main.rs

- **File(s):** `src/main.rs`
- **Action:** Update `src/main.rs` to:
  - Uncomment `mod server;` and add `use server::TimeServer;`
  - Import `rmcp::{transport::stdio, ServiceExt}`
  - In `main()`: after tracing init, create `TimeServer::new()`, call `.serve(stdio()).await?`, then `.waiting().await?` to keep server alive until client disconnects
  - Follow the exact pattern: `let service = TimeServer::new().serve(stdio()).await?; service.waiting().await?;`
  - Ensure tracing subscriber writes to stderr (not stdout) since stdout is the MCP transport channel
- **Verification:** `cargo build` succeeds; binary starts and waits for input
- **Done when:** `cargo build` succeeds; running the binary shows no output on stdout (clean MCP server startup)

---

## Wave 3: Unit Tests

> status: pending

Add comprehensive unit tests for timezone parsing, time formatting, and both tool handlers. After this wave, `cargo test` passes with coverage of valid inputs, invalid inputs, edge cases, and DST scenarios.

### Task 3.1: Add unit tests to server.rs

- **File(s):** `src/server.rs`
- **Action:** Add a `#[cfg(test)] mod tests { }` block at the bottom of `src/server.rs` per `TESTING.md` convention (co-located unit tests). Include the following test cases using `#[tokio::test]` for async tool methods:

  **get_current_time tests:**
  - `test_get_current_time_default_utc` -- call with `timezone: None`, verify response contains `"UTC"` and valid ISO 8601 datetime
  - `test_get_current_time_valid_timezone` -- call with `"America/New_York"`, verify response contains timezone name and a valid datetime
  - `test_get_current_time_invalid_timezone` -- call with `"Not/A/Timezone"`, verify response has `is_error: true` and message contains "Invalid timezone"
  - `test_get_current_time_empty_string` -- call with `Some("")`, verify defaults to UTC (matching `None` behaviour)
  - `test_get_current_time_fractional_offset` -- call with `"Asia/Kathmandu"`, verify UTC offset contains `+05:45`
  - `test_get_current_time_deprecated_timezone` -- call with `"US/Eastern"`, verify it works (IANA backward-compat link)
  - `test_get_current_time_dst_true` -- call with `"America/New_York"` using a known DST datetime context (July). Parse response JSON and assert `is_dst` field is `true`. Note: since `get_current_time` uses `Zoned::now()`, this test asserts structural correctness (the `is_dst` key exists and is boolean) rather than a specific value, since the actual DST status depends on when the test runs. Add a code comment noting this limitation.
  - `test_get_current_time_abbreviation_timezone` -- call with `"EST"`, verify `is_error: true` and message suggests using IANA name (e.g., "America/New_York")
  - `test_get_current_time_offset_timezone` -- call with `"UTC+5"`, verify `is_error: true` and message suggests using IANA name

  **convert_time tests:**
  - `test_convert_time_valid` -- convert `"12:00"` from `"UTC"` to `"America/New_York"`, verify target time is offset by -5 or -4 hours (depending on DST)
  - `test_convert_time_fractional_offset` -- convert `"12:00"` from `"UTC"` to `"Asia/Kathmandu"`, verify target time is `"17:45"` equivalent
  - `test_convert_time_invalid_source_timezone` -- call with invalid source tz, verify `is_error: true`
  - `test_convert_time_invalid_target_timezone` -- call with invalid target tz, verify `is_error: true`
  - `test_convert_time_invalid_time_format` -- call with `"25:99"`, verify `is_error: true` with "Invalid time format" message
  - `test_convert_time_non_numeric_time` -- call with `"abc"`, verify `is_error: true`
  - `test_convert_time_midnight` -- convert `"00:00"` between zones, verify correct result
  - `test_convert_time_end_of_day` -- convert `"23:59"` from UTC to a positive-offset zone, verify date rollover handled correctly
  - `test_convert_time_whitespace_trimmed` -- call with `"  14:30  "`, verify it parses successfully
  - `test_convert_time_24_00` -- call with `"24:00"`, verify `is_error: true` with "Invalid time format" message (HH:MM only accepts 00-23 for hours)
  - `test_convert_time_with_seconds` -- call with `"14:30:00"`, verify `is_error: true` with "Invalid time format" message indicating expected HH:MM format (strict: seconds not accepted per design decision in Task 2.1)
  - `test_convert_time_iso_datetime_input` -- call with `"2026-02-24T14:30:00"`, verify `is_error: true` with "Invalid time format" message

  **Helper function tests (non-async `#[test]`):**
  - `test_parse_timezone_valid` -- verify `"America/New_York"` parses successfully
  - `test_parse_timezone_invalid` -- verify `"Fake/Zone"` returns Err
  - `test_parse_timezone_abbreviation` -- verify `"PST"` returns Err with message suggesting IANA name
  - `test_parse_timezone_offset_string` -- verify `"+05:30"` returns Err with message suggesting IANA name
  - `test_format_utc_offset_positive` -- verify positive offset formats as `"+HH:MM"`
  - `test_format_utc_offset_negative` -- verify negative offset formats as `"-HH:MM"`
  - `test_format_utc_offset_fractional` -- verify `+05:45` formats correctly
  - `test_tool_error_helper` -- verify `tool_error("test message")` returns a `CallToolResult` with `is_error: true` and content containing "test message"

  **Assertion patterns:** For `get_current_time`, assert on structure (valid JSON, contains expected keys, timezone name matches) rather than exact time values (since time changes between calls). Parse the response text as `serde_json::Value` to check fields. For error responses, assert on both `is_error: true` and that the error message contains the expected substring.

- **Verification:** `cargo test` passes all tests
- **Done when:** `cargo test` reports zero failures; all listed test cases exist and pass

---

## Wave 4: Integration Tests and CI

> status: pending

Add integration tests that exercise the full MCP protocol flow and set up GitHub Actions CI. After this wave, the project has end-to-end test coverage and automated quality checks.

### Task 4.1: Create integration tests

- **File(s):** `tests/integration.rs`
- **Action:** Create `tests/integration.rs` per `TESTING.md` convention (integration tests in `tests/` directory). Use `#[tokio::test]` for all tests. Test the server at the MCP protocol level:

  - `test_server_lists_tools` -- Create a `TimeServer`, verify it exposes exactly 2 tools via the tool router. Check tool names are `"get_current_time"` and `"convert_time"`. Verify each tool has a description and input schema.
  - `test_get_current_time_via_protocol` -- Create a `TimeServer`, invoke the `get_current_time` tool through the tool router dispatch (simulating what rmcp does on a `tools/call` request). Verify the response is a successful `CallToolResult` with text content.
  - `test_convert_time_via_protocol` -- Same approach for `convert_time` with valid inputs. Verify successful response.
  - `test_tool_error_propagation` -- Invoke `get_current_time` with an invalid timezone through the router. Verify the response has `is_error: true`.

  Note: These tests instantiate `TimeServer` directly and call methods on it. Full stdio transport tests (spawning the binary, writing JSON-RPC to stdin) are valuable but complex -- defer to a follow-up if needed.

- **Verification:** `cargo test --test integration` passes all tests
- **Done when:** `cargo test` runs integration tests alongside unit tests with zero failures

### Task 4.2: Create GitHub Actions CI workflow

- **File(s):** `.github/workflows/ci.yml`
- **Action:** Create `.github/workflows/ci.yml` per quality-standards research. Configuration:
  - `name: CI`
  - Trigger: `on: [push, pull_request]`
  - `env: CARGO_TERM_COLOR: always`
  - Single job `check` on `ubuntu-latest`:
    - `actions/checkout@v4`
    - `dtolnay/rust-toolchain@stable` (uses `rust-toolchain.toml` from repo root automatically, ensuring consistent toolchain)
    - `Swatinem/rust-cache@v2`
    - `cargo fmt --all -- --check`
    - `cargo clippy --all-targets --all-features -- -D warnings`
    - `cargo test --all-features`
  - Omit `cargo audit` for now (requires separate install step; can add later)
- **Verification:** File exists and is valid YAML (validate with `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` or similar)
- **Done when:** `.github/workflows/ci.yml` exists with fmt, clippy, and test steps

### Task 4.3: Create README.md

- **File(s):** `README.md`
- **Action:** Create `README.md` with:
  - Project title and one-line description
  - **Tools** section: list both tools with their parameters and example responses
  - **Installation** section: `cargo install --path .` or `cargo build --release`
  - **Usage** section: MCP client configuration JSON (matching `INTEGRATIONS.md` pattern)
  - **Development** section: `make help` to list commands, `make test`, `make lint`, `make fmt`
  - **Requirements** section: Rust 1.85+ (Edition 2024)
  - Keep it concise -- no more than 80 lines
- **Verification:** File exists and contains installation and usage sections
- **Done when:** `README.md` exists with tools, installation, usage, and development sections
