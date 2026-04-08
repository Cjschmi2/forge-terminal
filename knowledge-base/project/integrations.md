# Integrations

## External Services

### SSH over Tailscale (Remote Sessions)
- Remote sessions connect via `ssh -t user@host 'cd dir && tool'`
- Machine mapping in `frontend/src-tauri/src/lib.rs:104`: `r5` -> `ec2-user@data-analysis.tailf29e20.ts.net`
- Working directory on remote is passed as the SSH command argument; local working_dir is `/tmp`

### Agent CLI Tools
- Claude Code (`claude`), OpenCode (`opencode`), Codex CLI (`codex`), Cursor (`agent`)
- Defined in `session-contracts` `AgentCliTool` enum with binary names and install commands
- Launched as PTY child processes via `CommandType::spawn_config`

## Databases / Caches

None. All state is in-memory:
- `PtyRouter` — `BTreeMap<SessionId, RuntimeSession>` behind `Arc<RwLock<>>`
- `MirrorBuffer` — `HashMap<String, VecDeque<SessionMirrorChunk>>` (capped ring buffer, 200 entries)
- `SessionRegistry` — `HashMap<String, RegistryLaunchRecord/SessionRegistration>`

## Configuration

### Environment Variables
- `ALLOWED_WORKING_DIRS` — Comma-separated list of allowed directory prefixes for PTY sessions (default: `/home`, `/tmp`, `/var/tmp`)
- `AGENT_CALLSIGN` — Allowed through env filter (agent identity for mesh routing)
- `MOTHERDUCK_TOKEN` — Allowed through env filter (reviewed exception)

### Tauri Config
- `frontend/src-tauri/tauri.conf.json` — App metadata, window size (1400x900), CSP disabled, dev server port 5180

### Router Defaults (`RouterConfig`)
- `max_sessions: 16`
- `default_timeout: 30s`
- `output_buffer_size: 256` (broadcast channel capacity)
- `tag_buffer_size: 64` (tag event broadcast capacity)
