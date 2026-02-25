# Concerns

> Last mapped: 2026-02-25T00:06:45Z

## Tech Debt

| Area | Description | Files | Severity |
|------|------------|-------|----------|
| rmcp version churn | `rmcp` is at 0.16.0 with 56+ releases and significant API churn. The semver-minor range `"0.16"` in `Cargo.toml` means any 0.16.x patch is accepted, but the next minor bump (0.17) will likely break the macro API (`#[tool]`, `#[tool_router]`, `#[tool_handler]`). | `Cargo.toml:16`, `src/server.rs` | medium |
| No MSRV pinning in CI | `rust-toolchain.toml` says `channel = "stable"` and CI uses `dtolnay/rust-toolchain@stable`, but `Cargo.toml` requires `rust-version = "1.85"` (Edition 2024). If stable ever lags behind 1.85 expectations or a contributor uses an older pinned toolchain, builds fail with opaque errors. | `rust-toolchain.toml`, `.github/workflows/ci.yml`, `Cargo.toml:5` | low |
| Duplicate `extract_text` helper | The same `extract_text` function is defined in both `src/server.rs` (line 712) and `tests/integration.rs` (line 5). Should be a shared test utility. | `src/server.rs:712-717`, `tests/integration.rs:5-9` | low |
| Stale planning docs reference wrong crate | `.planning/mcp-time-server/research/risks-edge-cases.md` references `chrono-tz` and `chrono` throughout, but the actual implementation uses `jiff`. These planning docs are misleading if consulted for future work. | `.planning/mcp-time-server/research/risks-edge-cases.md` | low |

## Known Pitfalls

- **`iana_name()` returns `None` for fixed-offset timezones** -- `src/server.rs:114,197,198` call `.unwrap_or("UTC")` on `tz.iana_name()`. If a `jiff::tz::TimeZone` is constructed from a fixed offset (not currently possible via `parse_timezone`, which rejects raw offsets), the displayed timezone name silently falls back to `"UTC"`, which is incorrect. Mitigation: current input validation in `parse_timezone` prevents this path, but be aware if expanding timezone acceptance.

- **`convert_time` uses "today" in the source timezone** -- `src/server.rs:172` calls `jiff::Zoned::now()` to get today's date. This means the conversion result depends on the current date (DST may or may not be active). Callers cannot specify a date, so the tool is inherently imprecise for planning future conversions. This matches the reference Python implementation but is worth noting.

- **DST ambiguity during fall-back is silently resolved** -- `src/server.rs:179` uses `datetime.to_zoned(source_tz)`, which picks one resolution for ambiguous times (fall-back overlap). The tool does not warn the user that the input time is ambiguous. Only the spring-forward gap (nonexistent time) produces an explicit error at `src/server.rs:181-188`.

- **Embedded timezone database becomes stale** -- `jiff` (v0.2.21) embeds IANA tzdata at compile time. Countries occasionally change timezone rules. The binary must be recompiled to pick up updates. Acceptable for a local tool, but worth documenting for users.

## Fragile Areas

- **rmcp macro-generated code** -- The `#[tool]`, `#[tool_router]`, and `#[tool_handler]` proc macros in `src/server.rs` generate the MCP protocol wiring. These macros are rmcp-version-specific and have changed across releases. Any rmcp upgrade requires verifying macro compatibility first. Files: `src/server.rs:84,222`.

- **Time format validation** -- The HH:MM validation at `src/server.rs:152-158` uses a length check plus colon-position check before delegating to `jiff::civil::Time::strptime`. This two-layer validation is somewhat fragile -- if the format string or jiff's parsing behavior changes, the two layers could disagree. Changes to time format acceptance should update both checks.

## Do Not Touch

- **`parse_timezone` rejection logic ordering** -- `src/server.rs:252-287`. The function rejects raw offsets, UTC+N patterns, and abbreviations in a specific order before calling `jiff::tz::TimeZone::get()`. The ordering matters because some abbreviations like `"EST"` exist in the IANA database but are intentionally rejected for ambiguity. Changing the order or removing checks could silently accept ambiguous inputs.

### Prescriptive Guidance

- **Before upgrading rmcp**: check the changelog for macro API changes. Test that `#[tool]`, `#[tool_router]`, and `#[tool_handler]` still compile. Pin to an exact version (e.g., `"=0.16.0"`) if stability is more important than patches.
- **When adding new tool handlers**: follow the existing pattern in `src/server.rs` -- return `Ok(tool_error(...))` for user input errors (sets `is_error: true`), and use `map_err` with `rmcp::ErrorData::internal_error` for internal failures. Never `unwrap()` in production paths.
- **When modifying timezone validation**: add test cases for the specific input class being changed. The abbreviation rejection at `src/server.rs:276-285` is intentionally stricter than what jiff would accept -- do not remove it.
- **When updating jiff**: recompile to pick up the latest IANA timezone database. Run the full test suite including the DST-related tests (`convert_time_fractional_offset`, `convert_time_midnight`, `convert_time_end_of_day`).
