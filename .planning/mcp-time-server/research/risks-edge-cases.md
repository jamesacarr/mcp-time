# Risks & Edge Cases Research

> Task: Create a new Rust-based MCP server with two tools: (1) fetch current time in a specific timezone (defaulting to UTC), and (2) convert time between timezones. Reference Python implementation: https://github.com/modelcontextprotocol/servers/tree/main/src/time
> Last researched: 2026-02-24T19:25:21Z

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| rmcp API breaking changes between versions | high | high | Pin to a specific version (latest stable is 0.16.0). The crate has gone through significant API churn (56 releases, breaking changes at 0.8.0). Isolate rmcp usage behind clean interfaces so upgrades are localised. |
| rmcp requires Rust Edition 2024 | medium | high | Rust Edition 2024 was stabilised in Rust 1.85.0 (Feb 20, 2025). Ensure `rust-toolchain.toml` specifies a minimum of 1.85 or the project Cargo.toml sets `edition = "2024"`. Users on older Rust will get build failures. Document the MSRV clearly. |
| STACK.md pins rmcp at `0.1` but current is `0.16.0` | high | medium | The version in `.planning/codebase/STACK.md` (`rmcp = { version = "0.1", ... }`) is severely outdated. The API at 0.1 differs significantly from current. Must use a recent version. Feature flags and macro syntax may have changed. |
| Stale timezone data in compiled binary | medium | low | `chrono-tz` embeds IANA tzdata at compile time. Countries occasionally change timezone rules (e.g., Morocco, Turkey, Russia). Binary must be recompiled with updated `chrono-tz` to pick up changes. Acceptable for a local tool; document that rebuilds incorporate latest tzdata. |
| Case-sensitive timezone parsing rejects valid input | medium | medium | `chrono-tz` is case-sensitive by default. `"america/new_york"` would fail. Enable the `case-insensitive` feature flag, or document that inputs must match IANA casing exactly. The reference Python impl uses `zoneinfo` which is also case-sensitive, so matching that behaviour is acceptable. |
| DST ambiguous/nonexistent times during conversion | medium | medium | When converting a time that falls in a DST gap (spring forward) or fold (fall back), `chrono` returns `MappedLocalTime::None` or `MappedLocalTime::Ambiguous`. The convert tool must handle both variants with clear error messages rather than panicking. |
| Malformed time string input causes panic | low | high | If `NaiveTime::parse_from_str` receives garbage input and the error is unwrapped instead of handled, the server crashes. All parse operations must use `?` or explicit matching, never `.unwrap()`. |
| Concurrent MCP requests on stdio transport | low | low | stdio transport is inherently serial (one request/response at a time). The rmcp SDK handles message framing. No concurrency risk for stdio, but if HTTP transport is added later, tool handlers should be stateless (they already are for time queries). |
| System clock inaccuracy | low | low | `get_current_time` relies on the host system clock. If the host clock is wrong, results are wrong. Not something the server can fix; document the dependency. |

## Edge Cases

### Timezone Input Validation

- **Empty string timezone** -- should return `INVALID_PARAMS` error, not panic. `"".parse::<chrono_tz::Tz>()` returns `Err`.
- **`None`/missing timezone parameter** -- `get_current_time` should default to UTC (per reference impl). `convert_time` requires both source and target; missing either is an error.
- **Offset-based timezone (e.g., `UTC+5`, `+05:30`)** -- `chrono-tz` only supports IANA names, not raw offsets. Should return a clear error suggesting the IANA equivalent.
- **Deprecated timezone names (e.g., `US/Eastern`)** -- `chrono-tz` includes IANA backward-compat links, so these should work. Worth testing.
- **Unicode/non-ASCII timezone strings** -- parse will fail; ensure the error message is useful.

### Time Format Parsing (convert_time)

- **`"24:00"` as input** -- `chrono` NaiveTime does not accept `24:00`; it is out of range. Should return a clear error.
- **`"25:00"` or `"12:60"`** -- invalid hours/minutes must be rejected by parse.
- **Seconds included (e.g., `"14:30:00"`)** -- if the format is `%H:%M`, extra `:00` causes a parse error. Decide whether to accept `%H:%M:%S` as well, or document the expected format strictly.
- **Leading/trailing whitespace** -- `"  14:30 "` will fail `parse_from_str`. Consider trimming input.
- **ISO 8601 full datetime as input** -- users might pass `"2026-02-24T14:30:00"`. Should return a helpful error indicating expected format.

### DST Transitions

- **Spring forward gap** -- e.g., converting `"02:30"` in `America/New_York` on the second Sunday in March. This time does not exist. `MappedLocalTime::None` must be caught and reported as `isError: true` with an explanatory message.
- **Fall back overlap** -- e.g., `"01:30"` in `America/New_York` on the first Sunday in November. This time is ambiguous (occurs twice). `MappedLocalTime::Ambiguous` should pick `earliest()` (or `latest()`) and note the ambiguity in the response, or return an error. The reference Python impl does not explicitly handle this; picking `earliest()` is a reasonable default.
- **Half-hour and 45-minute offsets** -- `Asia/Kolkata` (UTC+5:30), `Asia/Kathmandu` (UTC+5:45), `Australia/Lord_Howe` (UTC+10:30 / +11:00 with DST). Ensure offset formatting handles non-integer hours.

### Extreme Dates

- **`chrono` date range** -- supports approximately year -262144 to +262143. Not a practical concern for `get_current_time` (uses system clock), but `convert_time` only takes HH:MM (no date), so this is irrelevant unless the tool design changes to accept full datetimes.
- **Epoch boundary** -- `1970-01-01T00:00:00Z` and negative timestamps are handled by chrono but not relevant to the current tool design (time-only, no date).
- **Leap seconds** -- `chrono` NaiveTime can represent `:60` seconds for leap seconds. If the format is `%H:%M`, this is not an issue. If seconds are accepted, `23:59:60` is technically valid in chrono.

### MCP Protocol Edge Cases

- **Unknown tool name** -- rmcp SDK should handle this at the framework level, returning `Method not found` (-32601). Verify the SDK does this automatically.
- **Extra/unknown parameters** -- JSON-RPC allows additional fields. `serde` with `#[serde(deny_unknown_fields)]` would reject them; without it, they are silently ignored. The reference impl ignores extras. Match that behaviour.
- **Missing required parameters** -- `serde` deserialization will fail. Ensure the error propagates as `INVALID_PARAMS` (-32602), not an internal server error.
- **Extremely long string inputs** -- a timezone string of 10MB should not cause OOM. `serde_json` has no built-in size limit on strings, but the rmcp stdio transport likely reads line-by-line. Consider whether input size limits are needed (probably not for a local server).
- **Broken pipe / unexpected client disconnect** -- when the MCP client closes stdin, the server should exit cleanly. rmcp's `server.waiting().await` or `server.cancel().await` handles this. Verify the server does not hang.

### Error Response Format

- **Tool execution errors vs protocol errors** -- MCP distinguishes between these. Tool failures (bad timezone, bad time format) should return a successful JSON-RPC response with `isError: true` in the `CallToolResult`, not a JSON-RPC error. Protocol errors (malformed JSON, unknown method) are handled by rmcp.
- **Error message consistency** -- the reference Python impl returns messages like `"Invalid timezone: ..."` and `"Invalid time format. Expected HH:MM [24-hour format]"`. Match this style for compatibility.

## Backward Compatibility

No breaking changes. This is a greenfield project with no existing users, APIs, or data to migrate.

## Fragile Areas

- **`.planning/codebase/STACK.md`** -- specifies `rmcp = { version = "0.1", ... }` which is severely outdated. The actual Cargo.toml (not yet created) must use a current version. The planning docs reference feature flags (`server`, `macros`, `transport-io`, `schemars`) that may have been renamed or reorganised in newer rmcp versions. Verify against current rmcp docs before implementing.
- **rmcp macro API** -- the `#[tool]`, `#[tool_router]`, `#[tool_handler]` macros shown in `.planning/codebase/ARCHITECTURE.md` are based on early rmcp examples. The macro signatures and attribute names may differ in current versions. Must verify against actual rmcp 0.16.x documentation.
- **chrono `LocalResult` vs `MappedLocalTime`** -- chrono renamed `LocalResult` to `MappedLocalTime` in recent versions. Code examples using `LocalResult` will not compile with current chrono. Use `MappedLocalTime`.

## Unknowns

- **rmcp 0.16.0 exact API surface** -- the rmcp crate has evolved rapidly (56 releases). The exact macro syntax, error types, and transport setup for the current version could not be fully verified from documentation alone. The Planner should ensure approach research includes checking current rmcp examples or documentation via Context7/docs.rs before implementation begins.
- **rmcp feature flag names in 0.16.0** -- the feature flags listed in STACK.md (`server`, `macros`, `transport-io`, `schemars`) may have changed. The implementation step should verify available features on the actual crate version used.
- **Whether rmcp handles `isError` flag automatically** -- it is unclear whether `CallToolResult` in rmcp automatically sets `isError` based on the Rust `Result` type, or whether the developer must set it manually. This affects how error responses are constructed in tool handlers.
- **System timezone detection** -- the reference Python impl auto-detects the local timezone via `tzlocal`. Whether to replicate this in Rust (and which crate to use for it) is a design decision. `iana_time_zone` crate exists for this purpose but adds a dependency. Since the default is UTC, this may be unnecessary.
