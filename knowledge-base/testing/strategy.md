# Testing Strategy

## Framework

Rust standard test framework (`#[test]`, `#[tokio::test]`). No external test runner.

## Commands

```bash
cargo test                   # Run all tests (workspace)
cargo test -p session-pty-core   # Run tests for a specific crate
cargo test --lib             # Library tests only (skip doc tests)
```

## Test Organization

Tests are inline (`#[cfg(test)] mod tests`) in each crate's `lib.rs` or source file. No separate `tests/` directories.

### Per-Crate Test Coverage

| Crate | Unit | Integration | Notes |
|-------|------|-------------|-------|
| session-pty-core | 14 tests | - | SessionId, TerminalSize, PtySessionState, SpawnConfig, filter_env (8 tests), validate_working_directory |
| session-pty-native | 6 tests | async (tokio) | spawn+wait, spawn+kill, write/read roundtrip, double spawn, stats, resize |
| session-pty-router | 8 tests | async (tokio) | create/list, duplicate name, send, send missing, kill_all, subscribe, subscribe_tags |
| session-pty-recorder | 7 tests | - | raw record, asciicast, max file size, finish twice, record after finish, empty |
| protocol-wire-core | 5 tests (frame.rs) | - | round trip, ref round trip, too short, iterator, content/done helpers |
| protocol-tag-scanner | 13 tests | - | All tag types, mixed content, partial buffering, tag_to_frame, flush |
| session-contracts | 1 test | - | registration lifecycle |
| session-api | 7 tests | async (tokio) | API bridge, launch/kill, send, subscribe, mirror buffer |
| session-api (pty_bridge) | 11 tests | async (tokio) | output conversion, sequence, lossy UTF-8, launch/list, dup name, send, subscribe, kill_all, drain, tags |

## Test Utilities

- `temp_dir()` helper in session-pty-recorder for unique temp directories
- Tests that modify env vars (`ALLOWED_WORKING_DIRS`) are consolidated into single functions to avoid race conditions with parallel test runner

## Coverage Requirements

None formal. Every crate has tests. The env-var filter and working-dir allowlist have particularly thorough coverage.
