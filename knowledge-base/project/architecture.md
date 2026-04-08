# Architecture

## What It Does

Forge Terminal is a native desktop terminal emulator built with Tauri (Rust backend + SvelteKit frontend). It manages multiple PTY sessions — each running bash, Claude Code, Codex, Cursor, or custom commands — through a tabbed UI with an integrated file browser. Sessions can target the local machine or remote hosts via SSH over Tailscale.

## Major Components

### Backend (Rust workspace, 8 crates + Tauri app)

1. **session-pty-core** — Core trait (`PtySession`) and types: `SessionId`, `TerminalSize`, `SpawnConfig`, `PtySessionState`, `PtyError`. Also provides env-var filtering (`filter_env`) and working-directory allowlist validation.

2. **session-pty-native** — Linux PTY implementation (`NativePtySession`) using `openpty`, `nix`, and `tokio`. Handles spawn, read/write, resize, kill, and `dup_read_handle` for output pumps.

3. **session-pty-router** — Named-session router (`PtyRouter`) with broadcast output subscriptions. Manages session lifecycle, multiplexes output via `tokio::sync::broadcast`, and runs a tag scanner on PTY output to extract structured `[{...}]` tags into `TagEvent`s.

4. **session-pty-recorder** — Session recording in raw binary or asciicast v2 format. Used for audit/replay.

5. **protocol-wire-core** — Binary wire frame format (21-byte header + variable data). Defines `TokenFrame`, `TokenType` (u8 vocabulary of ~100 types), `StreamManager`, `ControlMessage` (handshake/auth), and `TraceContext`.

6. **protocol-tag-scanner** — Regex-based scanner that extracts `[{tag_type:payload}]` patterns from PTY stdout and converts them to `TokenFrame`s. Supports: task, status, @agent-message, broadcast, gate, metric.

7. **session-contracts** — Shared contract types: `AgentCliTool`, `SessionState`, `SessionRegistration`, `SessionMirrorChunk`, `AuthorityPosture`, `SessionLifecycle`.

8. **session-api** — Top-level API wrapping `PtyBridge` + `SessionRegistry` + in-memory `MirrorBuffer`. Provides launch, send, subscribe, kill, mirror operations.

### Frontend (SvelteKit + Tauri)

- **Tauri app** (`frontend/src-tauri/src/lib.rs`) — Rust Tauri commands: `sessions_launch`, `pty_send`, `pty_resize`, `pty_kill`, `sessions_list`, `filesystem_list/read/cwd`. Output pump emits `pty-output` events to the webview.
- **SvelteKit UI** — Tab bar with session management, xterm.js terminal emulator (`Terminal.svelte`), file tree browser (`FileTree.svelte`).
- **Wire client** (`frontend/src/lib/terminal/`) — TypeScript wire frame codec matching the Rust binary format. `WireClient` class for WebSocket connections with reconnect and heartbeat.

## Data Flow

```
User Input (keyboard) -> xterm.js onData -> Tauri invoke(pty_send) -> PtyBridge.send -> PtyRouter.send -> NativePtySession.write -> PTY master fd
PTY master fd -> output pump task (async read loop) -> broadcast::Sender<OutputChunk> -> Tauri event(pty-output) -> xterm.js write
                                                    -> TagScanner -> broadcast::Sender<TagEvent> (for structured tags)
```

## Key Entry Points

- `frontend/src-tauri/src/main.rs` -> `forge_terminal_app::run()` (Tauri app entry)
- `frontend/src-tauri/src/lib.rs` — All Tauri commands and state
- `frontend/src/routes/+page.svelte` — Main UI page
- `backend/crates/session-pty-router/src/lib.rs` — Core session management
