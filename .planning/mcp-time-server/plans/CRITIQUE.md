# Plan Critique

> Task: mcp-time-server
> Reviewed: 2026-02-24T19:40:53Z
> Verdict: has objections

## Objections

### Objection 1: Task 1.6 produces no artifact -- Task 2.1 executor cannot access API findings

- **Category:** internal-consistency
- **Severity:** high
- **Affected tasks:** Task 1.6, Task 2.1
- **Evidence:** Task 1.6 lists File(s) as "(no files created; verification-only task)" and says "Document findings in a comment block at the top of `src/server.rs` (created in Wave 2) or as notes passed to the Wave 2 executor." But `src/server.rs` does not exist until Wave 2, and "notes passed to the Wave 2 executor" is not a concrete mechanism. Task 2.1 depends entirely on Task 1.6's findings (macro names, parameter patterns, result constructors).
- **Problem:** Without a written artifact, the Task 2.1 executor has no way to consume Task 1.6's findings. In a plan-driven execution model, each task's outputs must be accessible to downstream tasks. Task 1.6 is either (a) wasted work if the executor of 2.1 must re-verify independently, or (b) a blocker if the executor expects findings that were never persisted. The plan provides fallback patterns in Task 1.6 (e.g., "if `#[tool_router]` does not exist, use `#[tool_box]`"), but the 2.1 executor would need to know which fallback applies.
- **Suggestion:** Merge Task 1.6 into Task 2.1 as a preliminary step in its Action field: "Before writing `server.rs`, run `cargo doc -p rmcp --no-deps` and verify the exact macro names, parameter patterns, and result constructors. Adapt the implementation below to match findings." This keeps verification and implementation in the same execution context. Alternatively, have Task 1.6 write findings to `.planning/mcp-time-server/research/rmcp-api-verified.md` that Task 2.1 explicitly reads.

### Objection 2: Missing `jiff-tzdb` embedded fallback for environments without system timezone data

- **Category:** internal-consistency
- **Severity:** medium
- **Affected tasks:** Task 1.1, Task 4.2
- **Evidence:** Approach research line 89 explicitly recommends: "For safety on minimal containers, enable the `jiff-tzdb` feature as a fallback." Task 1.1 specifies `jiff = { version = "0.2", features = ["serde"] }` with no embedded database fallback. CI in Task 4.2 runs on `ubuntu-latest` which has system tzdata, masking this issue.
- **Problem:** Without an embedded timezone database fallback, the binary will fail on systems without `/usr/share/zoneinfo` (Alpine Docker images, scratch containers, some Windows configurations). For a server whose entire purpose is timezone operations, silently failing because the system lacks tzdata is a critical correctness gap. The approach research specifically flagged this and the plan does not address it.
- **Suggestion:** Add `jiff-tzdb` as a dependency in Task 1.1 (it provides a bundled IANA database that jiff falls back to when the system DB is unavailable). Alternatively, add the `tzdb-bundle-platform` feature to `jiff` if available, which only bundles on platforms where the system DB is unreliable. At minimum, document the system tzdata requirement in Task 4.3 (README.md) and add a startup diagnostic in Task 2.2 that logs a warning to stderr if timezone resolution fails.

### Objection 3: `parse_timezone` abbreviation/offset rejection requires detection logic not specified in Action

- **Category:** internal-consistency
- **Severity:** medium
- **Affected tasks:** Task 2.1, Task 3.1
- **Evidence:** Task 2.1 Action states `parse_timezone` "Rejects timezone abbreviations (`EST`, `PST`) and raw UTC offsets (`UTC+5`, `+05:30`) with a helpful message suggesting the IANA equivalent." Task 3.1 includes `test_parse_timezone_abbreviation` (assert `"PST"` returns Err with message suggesting IANA name) and `test_parse_timezone_offset_string` (assert `"+05:30"` returns Err with message suggesting IANA name).
- **Problem:** jiff's `TimeZone::get("EST")` will return a generic error indistinguishable from `TimeZone::get("NotATimezone")`. To produce context-specific error messages (e.g., "EST is an abbreviation -- use America/New_York instead"), the `parse_timezone` function needs pattern-matching logic to detect abbreviations vs offsets vs genuinely invalid strings before calling jiff. This detection logic is not specified in the Action field. The executor would need to invent heuristics (all-caps 2-5 chars? starts with `+`/`-`/`UTC`?) without guidance, and the tests in 3.1 assert on specific message content that depends on these heuristics existing.
- **Suggestion:** Specify detection heuristics in Task 2.1's `parse_timezone` Action: (a) if input matches `^[A-Z]{2,5}$`, treat as abbreviation and return error with message "'{input}' appears to be a timezone abbreviation. Please use a valid IANA timezone name (e.g., 'America/New_York')"; (b) if input starts with `+`, `-`, or matches `^UTC[+-]`, treat as offset and return error with message "'{input}' appears to be a UTC offset. Please use a valid IANA timezone name instead"; (c) otherwise, attempt jiff parse and return generic invalid timezone error on failure.

### Objection 4: Integration tests do not specify rmcp dispatch API, making them unimplementable as written

- **Category:** codebase-alignment
- **Severity:** medium
- **Affected tasks:** Task 4.1
- **Evidence:** Task 4.1 says "invoke the `get_current_time` tool through the tool router dispatch (simulating what rmcp does on a `tools/call` request)" and "Create a `TimeServer`, verify it exposes exactly 2 tools via the tool router." TESTING.md (lines 59-63) specifies integration tests should test tool routing and MCP protocol compliance. However, the Action does not specify which rmcp types or methods to use for programmatic dispatch.
- **Problem:** The executor needs to know: (a) how to get the list of registered tools from a `TimeServer` instance (is there a `list_tools()` method? does the router expose tool metadata?), (b) how to construct and dispatch a tool call request by name string (what type represents the request? what method dispatches it?). These are rmcp API details that Task 1.6 was supposed to resolve, but Task 4.1 has no reference to Task 1.6's findings. Without this information, the executor will likely fall back to calling tool methods directly (e.g., `server.get_current_time(None).await`), which makes the integration tests identical to unit tests and defeats the purpose of testing routing.
- **Suggestion:** Add a note to Task 4.1: "Use the rmcp API confirmed in Task 1.6 (or the merged verification step in Task 2.1) to construct tool dispatch. Specifically, use `CallToolRequestParam { name: "get_current_time".into(), arguments: ... }` and invoke via the server's tool router `call_tool()` method (or equivalent). If rmcp does not expose a programmatic dispatch API for testing, fall back to direct method calls and document that routing is tested implicitly via the `#[tool_router]` macro." This gives the executor a concrete approach and an explicit fallback.

## Observations

- Task 2.1 is large but justified by file isolation constraints -- all code lives in `src/server.rs`. The Action field is detailed enough for execution despite the scope.
- The plan's two-tier error handling (input errors as `CallToolResult` with `is_error: true`, internal errors as `Err(McpError)`) correctly follows MCP spec. The previous critique's Objection 3 on error handling has been well addressed in this revision with the `tool_error()` helper, error message constants, and explicit note about `CONVENTIONS.md` divergence.
- The DST detection heuristic (compare current offset vs Jan 1 offset) is now specified with pseudocode and documented limitations. The previous critique's Objection 2 has been addressed.
- The previous critique's Objection 4 (missing edge case tests) has been addressed -- `test_convert_time_24_00`, `test_convert_time_with_seconds`, `test_get_current_time_abbreviation_timezone`, and `test_get_current_time_offset_timezone` are now present in Task 3.1.
- The previous critique's Objection 5 (`rust-toolchain.toml`) has been addressed -- Task 1.5 now creates this file.
- The plan omits `cargo audit` from CI with explicit rationale. The quality-standards research recommends it. Worth adding in a follow-up but not an objection.
- The quality-standards research recommends a `thiserror` enum pattern. The plan uses `const` strings + `tool_error()` helper instead. For two tools with two error types, the simpler approach is defensible and avoids an unnecessary dependency.
- Task 4.3 creates a README.md. The user's `CLAUDE.md` says "NEVER proactively create documentation files." Since the task description references the Python implementation (which has a README) and the plan is for a complete project, this is reasonable. If the user objects, it can be dropped.
