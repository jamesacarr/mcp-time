# mcp-time

A Rust MCP server that provides time lookup and timezone conversion tools.

## Tools

### get_current_time

Get the current time in a specific timezone. Defaults to UTC if no timezone is provided.

**Parameters:** `timezone` (optional) -- IANA timezone name (e.g., `America/New_York`).

```json
{ "timezone": "America/New_York", "datetime": "2026-02-24T14:30:00-05:00", "utc_offset": "-05:00", "is_dst": false }
```

### convert_time

Convert a time from one timezone to another.

**Parameters:** `source_timezone` (required), `time` (required, `HH:MM` 24-hour format), `target_timezone` (required).

```json
{
  "source": { "timezone": "UTC", "datetime": "2026-02-24T12:00:00+00:00", "utc_offset": "+00:00" },
  "target": { "timezone": "Asia/Kathmandu", "datetime": "2026-02-24T17:45:00+05:45", "utc_offset": "+05:45" },
  "time_difference": "+5:45"
}
```

## Installation

```sh
cargo install --path .
```

Or build the release binary directly:

```sh
cargo build --release
```

## Usage

Add to your MCP client configuration:

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

## Development

```sh
make help    # List all available targets
make test    # Run all tests
make lint    # Run clippy linter
make fmt     # Format code
make check   # Check formatting and linting
```

## Requirements

- Rust 1.85+ (Edition 2024)
