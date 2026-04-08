# Data Model

## Core Entities

### SessionId (`session-pty-core`)
- Newtype wrapping `Uuid` (v4). Unique per PTY session.
- Used as primary key in `PtyRouter.sessions` map.

### NativePtySession (`session-pty-native`)
- Implements `PtySession` trait
- Holds: master fd, child process, terminal size, lifecycle state, byte counters
- States: Created -> Running -> Exited(i32) / Killed / Error

### RuntimeSession (`session-pty-router`, internal)
- Combines: `NativePtySession` + `SessionMetadata` + `broadcast::Sender<OutputChunk>` + `broadcast::Sender<TagEvent>` + timing

### SessionMetadata (`session-pty-router`)
- name, command_type, working_dir, created_at, last_activity, tags

### OutputChunk (`session-pty-router`)
- session_id, session_name, data (Bytes), timestamp, stream (Stdout/Stderr)

### TagEvent (`session-pty-router`)
- session_id, session_name, parsed tag, wire TokenFrame, timestamp

### TokenFrame (`protocol-wire-core`)
- Binary: token_type (u8), stream_id (u32), sequence (u32), timestamp_ns (u64), data (Vec<u8>)
- 21-byte header + variable payload, max 16 MiB

### SessionState (`session-contracts`)
- session_id, machine_id, project_id, identity, role, session_name, lifecycle, authority
- Lifecycle: Launched -> Registered -> Detached
- Authority: Source / View / Cache / Ephemeral

### SessionMirrorChunk (`session-contracts`)
- session_id, sequence (u64), emitted_at, text, authority

## Where Types Are Defined

| Type | Crate | File |
|------|-------|------|
| SessionId, PtySession trait, SpawnConfig | session-pty-core | `backend/crates/session-pty-core/src/lib.rs` |
| NativePtySession | session-pty-native | `backend/crates/session-pty-native/src/lib.rs` |
| PtyRouter, CommandType, OutputChunk, TagEvent | session-pty-router | `backend/crates/session-pty-router/src/lib.rs` |
| TokenFrame, TokenType, WireVersion | protocol-wire-core | `backend/crates/protocol-wire-core/src/` |
| TagScanner, ParsedTag, TagType | protocol-tag-scanner | `backend/crates/protocol-tag-scanner/src/lib.rs` |
| SessionState, AgentCliTool | session-contracts | `backend/crates/session-contracts/src/lib.rs` |
| PtyBridge, SessionApi | session-api | `backend/crates/session-api/src/` |
