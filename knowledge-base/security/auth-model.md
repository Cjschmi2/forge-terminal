# Auth Model

## Authentication

### Wire Protocol Level
- `ControlMessage::Hello/Welcome` handshake with version negotiation and feature flags
- `ControlMessage::Auth { token }` / `AuthResult { success, message }` for JWT validation
- `WireClient` (TypeScript) sends token as `?token=` query parameter on WebSocket upgrade
- Feature flag: `AUTH_REQUIRED` (bit 4 in `FeatureFlags`)
- **Not yet wired end-to-end** — the Tauri app uses direct IPC (invoke), not WebSocket

### Tauri IPC
- No authentication between frontend and backend — Tauri IPC is same-process
- All commands are trusted (they run in the same app context)

## Authorization

### Working Directory Allowlist
- `validate_working_directory()` in `session-pty-core`
- Default allowed prefixes: `/home`, `/tmp`, `/var/tmp`
- Override: `ALLOWED_WORKING_DIRS` env var (comma-separated)
- Checked before spawning any OS resources

### Environment Variable Filtering
- `filter_env()` strips vars matching: PASSWORD, TOKEN, SECRET, KEY, CREDENTIAL, AUTH, PRIVATE, CERT, API_KEY, ACCESS_KEY
- Allowed exceptions: `AGENT_CALLSIGN`, `MOTHERDUCK_TOKEN`
- Case-insensitive matching
- Applied to all child process environments before spawn

## Auth Logic Location

| Component | File | Purpose |
|-----------|------|---------|
| Env filter | `backend/crates/session-pty-core/src/lib.rs:229` | Strip secrets from child env |
| Dir allowlist | `backend/crates/session-pty-core/src/lib.rs:264` | Restrict working directories |
| Wire auth | `backend/crates/protocol-wire-core/src/control.rs` | JWT in handshake (not wired) |
| Feature flags | `backend/crates/protocol-wire-core/src/control.rs` | AUTH_REQUIRED flag |

## Session Lifecycle

No session tokens or cookies. Sessions are identified by `SessionId` (UUID v4) and referenced by human-readable name strings. No persistence — all sessions are ephemeral and die with the process.
