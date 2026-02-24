# Concerns

> Generated: 2026-02-24T19:09:23Z

## Low Risk

### Timezone Handling
- `chrono` has solid timezone support via `chrono-tz`, but edge cases exist (DST transitions, ambiguous times)
- Mitigation: validate timezone inputs, return clear errors for ambiguous/invalid inputs

### rmcp Maturity
- `rmcp` is relatively new (official Rust MCP SDK)
- API may change between versions
- Mitigation: pin version, keep dependency minimal, isolate SDK usage behind clean interfaces

## Informational

### Binary Size
- Rust binaries can be large; not a concern for a local MCP server
- Can optimize with `[profile.release]` settings if needed

### Compilation Time
- Proc macros (`#[tool]`, `#[tool_handler]`) and `serde`/`schemars` derives add compile time
- Acceptable for a small project

### No Authentication
- stdio transport is inherently local — no auth needed
- If HTTP transport is added later, authentication becomes a concern
