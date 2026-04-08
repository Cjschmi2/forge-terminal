# Critical Test Paths

## Must-Test Areas

### Environment Variable Filtering
- File: `backend/crates/session-pty-core/src/lib.rs`
- Security-critical: prevents secret leakage into child processes
- 8 dedicated tests covering all 10 forbidden patterns, allowed overrides, case insensitivity
- Any change to `FORBIDDEN_PATTERNS` or `ALLOWED_OVERRIDE` must update tests

### Working Directory Allowlist
- File: `backend/crates/session-pty-core/src/lib.rs`
- Security-critical: prevents PTY sessions from operating in sensitive directories
- Tests cover default allowlist, custom env override, rejection of `/etc`, `/`
- Env-dependent tests consolidated to avoid parallel race conditions

### PTY Lifecycle (spawn -> read/write -> kill)
- File: `backend/crates/session-pty-native/src/lib.rs`
- Tests: `spawn_and_wait_echo`, `spawn_and_kill`, `write_read_roundtrip`, `double_spawn_fails`
- Critical for correctness: fd handling, process lifecycle, SIGKILL on drop

### Wire Frame Codec
- File: `backend/crates/protocol-wire-core/src/frame.rs`
- Binary format must match between Rust and TypeScript
- Tests: encode/decode round trip, zero-copy ref, too-short rejection
- TypeScript parity: `frontend/src/lib/terminal/frame.ts` must match

### Tag Scanner
- File: `backend/crates/protocol-tag-scanner/src/lib.rs`
- Tests: all tag types, partial tag buffering across feeds, flush
- Critical for agent communication — incorrect parsing breaks orchestration

## Known Flaky Tests

None identified. PTY tests use generous timeouts (200ms for echo).

## Low Coverage Areas

- **Tauri commands** (`frontend/src-tauri/src/lib.rs`) — No Rust-level tests; tested manually via the UI
- **Frontend components** — No Svelte component tests
- **Wire client** (`frontend/src/lib/terminal/client.ts`) — No JS tests for WebSocket reconnect/heartbeat logic
- **Remote SSH sessions** — Only tested manually; the SSH command construction in the Tauri lib is not unit tested
