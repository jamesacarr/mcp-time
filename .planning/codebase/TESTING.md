# Testing

> Generated: 2026-02-24T19:09:23Z

## Framework

- **Rust built-in test framework** (`#[test]`, `#[tokio::test]`)
- Run: `cargo test`

## Test Organization

```
src/
  server.rs          # Unit tests in #[cfg(test)] mod tests { }
  tools/
    current_time.rs  # Unit tests co-located in each module
tests/
  integration.rs     # Integration tests against full server
```

## Patterns

### Unit Tests

Co-located with source in `#[cfg(test)]` blocks:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timezone_valid() {
        // ...
    }

    #[test]
    fn parse_timezone_invalid_returns_error() {
        // ...
    }
}
```

### Async Tests

Use `#[tokio::test]` for async tool handlers:

```rust
#[tokio::test]
async fn get_current_time_returns_iso8601() {
    let server = TimeServer::new();
    let result = server.get_current_time(None).await.unwrap();
    // assert on result content
}
```

### Integration Tests

Test the server's tool routing and MCP protocol compliance in `tests/`:

```rust
#[tokio::test]
async fn server_lists_tools() {
    // Start server, call list_tools, verify tool metadata
}
```

## Coverage

- No specific coverage tool required initially
- Can add `cargo-tarpaulin` or `cargo-llvm-cov` later if needed
