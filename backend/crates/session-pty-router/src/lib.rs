use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use protocol_tag_scanner::{ParsedTag, TagScanner};
use protocol_wire_core::frame::TokenFrame;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, warn};

use session_pty_core::{
    PtyError, PtySession, SessionId, PtySessionState, SpawnConfig, TerminalSize,
};
use session_pty_native::NativePtySession;

// ---------------------------------------------------------------------------
// CommandType
// ---------------------------------------------------------------------------

/// Well-known command types and a custom escape hatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandType {
    Claude,
    Codex,
    Cursor,
    Bash,
    Custom { command: String, args: Vec<String> },
}

impl CommandType {
    /// Produce a [`SpawnConfig`] for this command type rooted at `working_dir`.
    pub fn spawn_config(&self, working_dir: impl Into<PathBuf>) -> SpawnConfig {
        let wd = working_dir.into();
        match self {
            CommandType::Claude => SpawnConfig::new("claude", &wd),
            CommandType::Codex => SpawnConfig::new("codex", &wd),
            CommandType::Cursor => SpawnConfig::new("cursor", &wd),
            CommandType::Bash => SpawnConfig::new("bash", &wd),
            CommandType::Custom { command, args } => {
                let mut cfg = SpawnConfig::new(command.clone(), &wd);
                cfg.args = args.clone();
                cfg
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Metadata & info types
// ---------------------------------------------------------------------------

/// Metadata attached to a named session at creation time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub name: String,
    pub command_type: CommandType,
    pub working_dir: PathBuf,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub tags: Vec<String>,
}

/// A chunk of output captured from a session.
#[derive(Debug, Clone)]
pub struct OutputChunk {
    pub session_id: SessionId,
    pub session_name: String,
    pub data: Bytes,
    pub timestamp: DateTime<Utc>,
    pub stream: OutputStream,
}

/// Which output stream a chunk came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// A tag extracted from PTY output by the tag scanner.
///
/// The output pump runs the tag scanner on every chunk of bytes read from the
/// PTY. When tags are detected, they are converted to [`TokenFrame`]s and
/// emitted on a separate broadcast channel. Downstream consumers (e.g. the
/// NATS bridge) subscribe to this channel to publish tag-originated frames
/// without ever seeing raw PTY I/O.
#[derive(Debug, Clone)]
pub struct TagEvent {
    /// The session that produced this tag.
    pub session_id: SessionId,
    /// The session's human-readable name.
    pub session_name: String,
    /// The parsed tag from the scanner.
    pub tag: ParsedTag,
    /// The wire frame built from this tag (via `TagScanner::tag_to_frame`).
    pub frame: TokenFrame,
    /// When the tag was detected.
    pub timestamp: DateTime<Utc>,
}

/// Summary information about a session exposed by the router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub name: String,
    pub command_type: CommandType,
    pub working_dir: PathBuf,
    pub state: PtySessionState,
    pub age: Duration,
    pub idle_time: Duration,
    pub tags: Vec<String>,
}

// ---------------------------------------------------------------------------
// RouterConfig
// ---------------------------------------------------------------------------

/// Configuration for the [`PtyRouter`].
#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub max_sessions: usize,
    pub default_timeout: Duration,
    pub output_buffer_size: usize,
    /// Broadcast channel capacity for tag events extracted by the tag scanner.
    pub tag_buffer_size: usize,
    pub default_terminal_size: TerminalSize,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            max_sessions: 16,
            default_timeout: Duration::from_secs(30),
            output_buffer_size: 256,
            tag_buffer_size: 64,
            default_terminal_size: TerminalSize::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// RouterError
// ---------------------------------------------------------------------------

/// Errors specific to the routing layer.
#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("session name already exists: {0}")]
    SessionNameExists(String),

    #[error("session name not found: {0}")]
    SessionNameNotFound(String),

    #[error("no route matched")]
    RouteNoMatch,

    #[error("broadcast partially failed: {0}")]
    BroadcastPartialFailure(String),

    #[error("subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("wait_for pattern timed out")]
    PatternTimeout,

    #[error("pty error: {0}")]
    Pty(#[from] PtyError),
}

/// Convenience alias.
pub type RouterResult<T> = Result<T, RouterError>;

// ---------------------------------------------------------------------------
// RuntimeSession (internal)
// ---------------------------------------------------------------------------

/// Internal bookkeeping for a running session.
struct RuntimeSession {
    session: NativePtySession,
    metadata: SessionMetadata,
    tx: broadcast::Sender<OutputChunk>,
    /// Broadcast channel for tag events extracted by the tag scanner.
    tag_tx: broadcast::Sender<TagEvent>,
    created_instant: Instant,
    last_activity_instant: Instant,
}

// ---------------------------------------------------------------------------
// PtyRouter
// ---------------------------------------------------------------------------

/// Named-session router with broadcast output subscriptions.
pub struct PtyRouter {
    config: RouterConfig,
    /// Maps human-readable name -> SessionId.
    named_sessions: Arc<RwLock<BTreeMap<String, SessionId>>>,
    /// Maps SessionId -> runtime state.
    sessions: Arc<RwLock<BTreeMap<SessionId, RuntimeSession>>>,
}

impl PtyRouter {
    /// Create a new router with the given configuration.
    pub fn new(config: RouterConfig) -> Self {
        Self {
            config,
            named_sessions: Arc::new(RwLock::new(BTreeMap::new())),
            sessions: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Create a new router with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(RouterConfig::default())
    }

    // -- public API ---------------------------------------------------------

    /// Create and spawn a named session.
    pub async fn create_session(
        &self,
        name: impl Into<String>,
        cmd_type: CommandType,
        working_dir: impl Into<PathBuf>,
    ) -> RouterResult<SessionId> {
        let name = name.into();
        let working_dir = working_dir.into();

        // Check name uniqueness.
        {
            let names = self.named_sessions.read();
            if names.contains_key(&name) {
                return Err(RouterError::SessionNameExists(name));
            }
        }

        // Check capacity.
        {
            let sessions = self.sessions.read();
            if sessions.len() >= self.config.max_sessions {
                return Err(RouterError::Pty(PtyError::Io(
                    "max sessions reached".into(),
                )));
            }
        }

        let mut pty = NativePtySession::new();
        let id = *pty.id();

        // Spawn.
        let spawn_cfg = cmd_type.spawn_config(&working_dir);
        pty.spawn(&spawn_cfg).await?;

        let now = Utc::now();
        let (tx, _) = broadcast::channel(self.config.output_buffer_size);
        let (tag_tx, _) = broadcast::channel(self.config.tag_buffer_size);

        let metadata = SessionMetadata {
            name: name.clone(),
            command_type: cmd_type,
            working_dir,
            created_at: now,
            last_activity: now,
            tags: Vec::new(),
        };

        let runtime = RuntimeSession {
            session: pty,
            metadata,
            tx,
            tag_tx,
            created_instant: Instant::now(),
            last_activity_instant: Instant::now(),
        };

        {
            let mut sessions = self.sessions.write();
            sessions.insert(id, runtime);
        }
        {
            let mut names = self.named_sessions.write();
            names.insert(name.clone(), id);
        }

        debug!(%id, %name, "session created in router");

        // Start the output pump: continuously read from a dup'd master fd
        // and push OutputChunks to the broadcast channel.  The dup'd handle
        // is owned by the pump task, so no lock is held across the async read.
        //
        // The pump also runs a TagScanner over every chunk. When tags are
        // found, they are converted to TokenFrames and emitted on the
        // separate `tag_tx` channel. Downstream consumers (e.g. NATS bridge)
        // subscribe to `tag_tx` to publish only tag-originated frames.
        {
            let channels = {
                let sessions_guard = self.sessions.read();
                let rt = sessions_guard.get(&id).expect("just inserted");
                let read_handle = rt.session.dup_read_handle()
                    .expect("dup master fd for output pump")
                    .expect("session was just spawned, master fd must exist");
                (rt.tx.clone(), rt.tag_tx.clone(), read_handle)
            };
            let (tx, tag_tx, mut read_handle) = channels;
            let pump_name = name.clone();

            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut buf = [0u8; 4096];
                let mut scanner = TagScanner::new();
                let mut tag_sequence: u32 = 0;

                loop {
                    match read_handle.read(&mut buf).await {
                        Ok(0) => {
                            debug!(%id, name = %pump_name, "output pump: EOF");
                            break;
                        }
                        Ok(n) => {
                            let data = &buf[..n];
                            let now = Utc::now();

                            // Broadcast the raw output chunk (for gRPC mirroring,
                            // terminal rendering, etc.).
                            let chunk = OutputChunk {
                                session_id: id,
                                session_name: pump_name.clone(),
                                data: Bytes::copy_from_slice(data),
                                timestamp: now,
                                stream: OutputStream::Stdout,
                            };
                            // Best-effort: if no subscribers, data is dropped.
                            let _ = tx.send(chunk);

                            // Run tag scanner on the same bytes. Tags found are
                            // emitted as TagEvents on the separate channel.
                            let scan_result = scanner.feed(data);
                            for parsed_tag in scan_result.tags {
                                let frame = TagScanner::tag_to_frame(
                                    &parsed_tag,
                                    id.0.as_u128() as u32, // stream_id from session
                                    tag_sequence,
                                );
                                tag_sequence = tag_sequence.wrapping_add(1);

                                let event = TagEvent {
                                    session_id: id,
                                    session_name: pump_name.clone(),
                                    tag: parsed_tag,
                                    frame,
                                    timestamp: now,
                                };
                                // Best-effort: if no tag subscribers, event is dropped.
                                let _ = tag_tx.send(event);
                            }
                        }
                        Err(e) => {
                            // EAGAIN/WouldBlock is normal on non-blocking fds — retry after yield.
                            if e.to_string().contains("os error 11")
                                || e.to_string().contains("WouldBlock")
                            {
                                tokio::time::sleep(Duration::from_millis(10)).await;
                                continue;
                            }
                            debug!(%id, name = %pump_name, error = %e, "output pump: read error, exiting");
                            break;
                        }
                    }
                }
            });
        }

        Ok(id)
    }

    /// Send `instruction` (bytes) to the named session.
    ///
    /// The lock is acquired, the session reference extracted, the write
    /// performed, and metadata updated before the lock is released.
    #[allow(clippy::await_holding_lock)]
    pub async fn send(
        &self,
        name: &str,
        instruction: &[u8],
    ) -> RouterResult<usize> {
        let id = self.resolve_name(name)?;
        let mut sessions = self.sessions.write();
        let rt = sessions
            .get_mut(&id)
            .ok_or_else(|| RouterError::SessionNameNotFound(name.into()))?;
        let n = rt
            .session
            .write(instruction)
            .await
            .map_err(RouterError::Pty)?;
        rt.last_activity_instant = Instant::now();
        rt.metadata.last_activity = Utc::now();
        Ok(n)
    }

    /// Resize the terminal of a named session.
    #[allow(clippy::await_holding_lock)]
    pub async fn resize(
        &self,
        name: &str,
        cols: u16,
        rows: u16,
    ) -> RouterResult<()> {
        let id = self.resolve_name(name)?;
        let mut sessions = self.sessions.write();
        let rt = sessions
            .get_mut(&id)
            .ok_or_else(|| RouterError::SessionNameNotFound(name.into()))?;
        rt.session
            .resize(cols, rows)
            .await
            .map_err(RouterError::Pty)?;
        Ok(())
    }

    /// Broadcast `instruction` to all named sessions.  Returns the list of
    /// session names that failed.
    pub async fn broadcast(
        &self,
        names: &[String],
        instruction: &[u8],
    ) -> RouterResult<()> {
        let mut failures = Vec::new();
        for name in names {
            if let Err(e) = self.send(name, instruction).await {
                failures.push(format!("{name}: {e}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(RouterError::BroadcastPartialFailure(failures.join("; ")))
        }
    }

    /// Subscribe to output from the named session.
    pub fn subscribe(
        &self,
        name: &str,
    ) -> RouterResult<broadcast::Receiver<OutputChunk>> {
        let id = self.resolve_name(name)?;
        let sessions = self.sessions.read();
        let rt = sessions
            .get(&id)
            .ok_or_else(|| RouterError::SessionNameNotFound(name.into()))?;
        Ok(rt.tx.subscribe())
    }

    /// Subscribe to tag events from the named session.
    ///
    /// The tag scanner runs on every chunk of PTY output in the output pump.
    /// When `[{...}]` tags are detected, they are parsed and converted to
    /// [`TagEvent`]s containing both the parsed tag and its wire
    /// [`TokenFrame`]. Downstream consumers (e.g. the NATS bridge) subscribe
    /// to this channel to publish only tag-originated frames.
    pub fn subscribe_tags(
        &self,
        name: &str,
    ) -> RouterResult<broadcast::Receiver<TagEvent>> {
        let id = self.resolve_name(name)?;
        let sessions = self.sessions.read();
        let rt = sessions
            .get(&id)
            .ok_or_else(|| RouterError::SessionNameNotFound(name.into()))?;
        Ok(rt.tag_tx.subscribe())
    }

    /// Read output from the named session and publish it, then wait for
    /// `pattern` to appear in the accumulated output.  Returns the matched
    /// line.
    #[allow(clippy::await_holding_lock)]
    pub async fn wait_for(
        &self,
        name: &str,
        pattern: &str,
        timeout: Duration,
    ) -> RouterResult<String> {
        let id = self.resolve_name(name)?;
        let deadline = Instant::now() + timeout;
        let mut accumulated = String::new();

        loop {
            if Instant::now() >= deadline {
                return Err(RouterError::PatternTimeout);
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            let chunk = {
                let mut sessions = self.sessions.write();
                let rt = sessions
                    .get_mut(&id)
                    .ok_or_else(|| RouterError::SessionNameNotFound(name.into()))?;
                let mut buf = [0u8; 4096];
                match tokio::time::timeout(
                    remaining.min(Duration::from_millis(100)),
                    rt.session.read(&mut buf),
                )
                .await
                {
                    Ok(Ok(n)) if n > 0 => {
                        let data = Bytes::copy_from_slice(&buf[..n]);
                        let chunk = OutputChunk {
                            session_id: id,
                            session_name: name.to_string(),
                            data: data.clone(),
                            timestamp: Utc::now(),
                            stream: OutputStream::Stdout,
                        };
                        // Best-effort broadcast to subscribers.
                        let _ = rt.tx.send(chunk);
                        rt.last_activity_instant = Instant::now();
                        Some(data)
                    }
                    _ => None,
                }
            };

            if let Some(data) = chunk
                && let Ok(text) = std::str::from_utf8(&data)
            {
                accumulated.push_str(text);
            }

            if accumulated.contains(pattern) {
                // Return the line containing the pattern.
                for line in accumulated.lines() {
                    if line.contains(pattern) {
                        return Ok(line.to_string());
                    }
                }
                return Ok(accumulated);
            }

            // Yield to avoid tight-looping.
            tokio::task::yield_now().await;
        }
    }

    /// Kill the named session and remove it from the router.
    pub async fn kill(&self, name: &str) -> RouterResult<()> {
        let id = self.resolve_name(name)?;
        // Remove session from the map while holding the lock, then release
        // the lock before the async kill to avoid await_holding_lock.
        let removed = {
            let mut sessions = self.sessions.write();
            sessions.remove(&id)
        };
        if let Some(mut rt) = removed {
            rt.session.kill().await.map_err(RouterError::Pty)?;
        }
        {
            let mut names = self.named_sessions.write();
            names.remove(name);
        }
        debug!(%id, name, "session killed via router");
        Ok(())
    }

    /// Kill all sessions and clear the router.
    pub async fn kill_all(&self) -> RouterResult<()> {
        let ids: Vec<(String, SessionId)> = {
            let names = self.named_sessions.read();
            names.iter().map(|(n, id)| (n.clone(), *id)).collect()
        };
        for (name, _) in &ids {
            if let Err(e) = self.kill(name).await {
                warn!(name, error = %e, "error killing session during kill_all");
            }
        }
        Ok(())
    }

    /// Return information about all sessions managed by this router.
    pub fn sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read();
        let names = self.named_sessions.read();

        // Build a reverse map id -> name.
        let id_to_name: BTreeMap<SessionId, String> =
            names.iter().map(|(n, id)| (*id, n.clone())).collect();

        sessions
            .iter()
            .map(|(id, rt)| {
                let name = id_to_name
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| id.to_string());
                SessionInfo {
                    id: *id,
                    name,
                    command_type: rt.metadata.command_type.clone(),
                    working_dir: rt.metadata.working_dir.clone(),
                    state: rt.session.state(),
                    age: rt.created_instant.elapsed(),
                    idle_time: rt.last_activity_instant.elapsed(),
                    tags: rt.metadata.tags.clone(),
                }
            })
            .collect()
    }

    /// Return the current router configuration.
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    // -- internal helpers ---------------------------------------------------

    fn resolve_name(&self, name: &str) -> RouterResult<SessionId> {
        let names = self.named_sessions.read();
        names
            .get(name)
            .copied()
            .ok_or_else(|| RouterError::SessionNameNotFound(name.into()))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_type_spawn_config_bash() {
        let cfg = CommandType::Bash.spawn_config("/tmp");
        assert_eq!(cfg.command, "bash");
        assert_eq!(cfg.working_dir, PathBuf::from("/tmp"));
    }

    #[test]
    fn command_type_spawn_config_custom() {
        let ct = CommandType::Custom {
            command: "python3".into(),
            args: vec!["-c".into(), "print('hi')".into()],
        };
        let cfg = ct.spawn_config("/home");
        assert_eq!(cfg.command, "python3");
        assert_eq!(cfg.args, vec!["-c", "print('hi')"]);
    }

    #[test]
    fn router_config_defaults() {
        let cfg = RouterConfig::default();
        assert_eq!(cfg.max_sessions, 16);
        assert_eq!(cfg.output_buffer_size, 256);
        assert_eq!(cfg.tag_buffer_size, 64);
    }

    #[tokio::test]
    async fn create_and_list_sessions() {
        let router = PtyRouter::with_defaults();
        let id = router
            .create_session("test-bash", CommandType::Bash, "/tmp")
            .await
            .expect("create_session");

        let infos = router.sessions();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].id, id);
        assert_eq!(infos[0].name, "test-bash");

        router.kill("test-bash").await.ok();
    }

    #[tokio::test]
    async fn duplicate_name_rejected() {
        let router = PtyRouter::with_defaults();
        router
            .create_session("dup", CommandType::Bash, "/tmp")
            .await
            .expect("first");
        let result = router
            .create_session("dup", CommandType::Bash, "/tmp")
            .await;
        assert!(matches!(result, Err(RouterError::SessionNameExists(_))));
        router.kill_all().await.ok();
    }

    #[tokio::test]
    async fn send_to_session() {
        let router = PtyRouter::with_defaults();
        router
            .create_session("cat-test", CommandType::Custom {
                command: "cat".into(),
                args: vec![],
            }, "/tmp")
            .await
            .expect("create");

        let n = router.send("cat-test", b"hello\n").await.expect("send");
        assert!(n > 0);

        router.kill("cat-test").await.ok();
    }

    #[tokio::test]
    async fn send_to_missing_session() {
        let router = PtyRouter::with_defaults();
        let result = router.send("nope", b"hello").await;
        assert!(matches!(result, Err(RouterError::SessionNameNotFound(_))));
    }

    #[tokio::test]
    async fn kill_all_clears_sessions() {
        let router = PtyRouter::with_defaults();
        router
            .create_session("s1", CommandType::Bash, "/tmp")
            .await
            .ok();
        router
            .create_session("s2", CommandType::Bash, "/tmp")
            .await
            .ok();

        assert_eq!(router.sessions().len(), 2);
        router.kill_all().await.expect("kill_all");
        assert_eq!(router.sessions().len(), 0);
    }

    #[tokio::test]
    async fn subscribe_returns_receiver() {
        let router = PtyRouter::with_defaults();
        router
            .create_session("sub-test", CommandType::Bash, "/tmp")
            .await
            .ok();
        let _rx = router.subscribe("sub-test").expect("subscribe");
        router.kill_all().await.ok();
    }

    #[tokio::test]
    async fn subscribe_tags_returns_receiver() {
        let router = PtyRouter::with_defaults();
        router
            .create_session("tag-sub", CommandType::Bash, "/tmp")
            .await
            .ok();
        let _rx = router.subscribe_tags("tag-sub").expect("subscribe_tags");
        router.kill_all().await.ok();
    }

    #[tokio::test]
    async fn subscribe_tags_missing_session_fails() {
        let router = PtyRouter::with_defaults();
        let result = router.subscribe_tags("nonexistent");
        assert!(matches!(result, Err(RouterError::SessionNameNotFound(_))));
    }

    #[test]
    fn tag_event_has_expected_fields() {
        // Compile-time check: TagEvent has all expected fields.
        fn _assert_fields(event: &TagEvent) {
            let _id: &SessionId = &event.session_id;
            let _name: &String = &event.session_name;
            let _tag: &ParsedTag = &event.tag;
            let _frame: &TokenFrame = &event.frame;
            let _ts: &DateTime<Utc> = &event.timestamp;
        }
    }
}
