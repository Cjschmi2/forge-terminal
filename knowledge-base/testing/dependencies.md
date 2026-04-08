# Test Dependencies

## Dev Dependencies

| Crate | Dev Deps | Purpose |
|-------|----------|---------|
| session-pty-core | serde_json | Serde round-trip tests |
| session-pty-recorder | uuid | Unique temp dir names |
| session-api | chrono | Timestamp construction |

## Fixtures / Test Data

No external fixtures. All tests construct data inline:
- `SpawnConfig::new("echo", "/tmp")` or `SpawnConfig::new("cat", "/tmp")` for PTY tests
- `OutputChunk` with `Bytes::from_static(b"...")` for bridge tests
- Tag strings like `b"[{status:done}]"` for scanner tests
- Inline `RecordingConfig` with temp dirs for recorder tests

## Mock / Stub Patterns

No mocks. All PTY tests spawn real processes (`echo`, `cat`, `sleep`, `bash`) against `/tmp`. This is intentional — the PTY layer must work with real fds and processes.

## External Service Mocks

None needed. No external services are called during tests.

## Database Seeding

N/A — no database. All state is in-memory.
