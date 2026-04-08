use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, anyhow};
use session_contracts::{AuthorityPosture, SessionMirrorChunk};
use session_pty_core::SessionId;
use session_pty_router::{CommandType, OutputChunk, PtyRouter, RouterConfig, TagEvent};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// PtyLaunchRequest
// ---------------------------------------------------------------------------

/// Parameters for launching a new PTY session through the bridge.
#[derive(Debug, Clone)]
pub struct PtyLaunchRequest {
    /// Human-readable session name (must be unique within the router).
    pub name: String,
    /// The command type to spawn.
    pub command_type: CommandType,
    /// Working directory for the child process.
    pub working_dir: PathBuf,
    /// Project that owns the session (for mirror chunk tagging).
    pub project_id: String,
    /// Identity of the user who requested the launch.
    pub requested_by: String,
}

// ---------------------------------------------------------------------------
// PtyBridge
// ---------------------------------------------------------------------------

/// Bridges the PTY router into the session-api surface, providing launch,
/// send, subscribe, and mirror-chunk conversion.
pub struct PtyBridge {
    router: PtyRouter,
    /// Monotonically increasing sequence counter for mirror chunks.
    sequence: AtomicU64,
}

impl PtyBridge {
    /// Create a new bridge wrapping the given [`PtyRouter`].
    pub fn new(router: PtyRouter) -> Self {
        Self {
            router,
            sequence: AtomicU64::new(1),
        }
    }

    /// Create a bridge with a default-configured router.
    pub fn with_defaults() -> Self {
        Self::new(PtyRouter::with_defaults())
    }

    /// Create a bridge from a [`RouterConfig`].
    pub fn from_config(config: RouterConfig) -> Self {
        Self::new(PtyRouter::new(config))
    }

    // -- public API ---------------------------------------------------------

    /// Launch a new PTY session.  Returns the router-assigned [`SessionId`].
    pub async fn launch(&self, request: PtyLaunchRequest) -> Result<SessionId> {
        let id = self
            .router
            .create_session(&request.name, request.command_type, &request.working_dir)
            .await
            .map_err(|e| anyhow!("pty launch failed: {e}"))?;

        info!(
            session_id = %id,
            name = %request.name,
            project_id = %request.project_id,
            requested_by = %request.requested_by,
            "launched pty session via bridge"
        );

        Ok(id)
    }

    /// Send raw input bytes to a named PTY session.  Returns the number of
    /// bytes written.
    pub async fn send(&self, name: &str, input: &[u8]) -> Result<usize> {
        let n = self
            .router
            .send(name, input)
            .await
            .map_err(|e| anyhow!("pty send failed: {e}"))?;

        debug!(name, bytes = n, "sent input to pty session");
        Ok(n)
    }

    /// Subscribe to the output broadcast for a named PTY session.
    pub fn subscribe(&self, name: &str) -> Result<broadcast::Receiver<OutputChunk>> {
        self.router
            .subscribe(name)
            .map_err(|e| anyhow!("pty subscribe failed: {e}"))
    }

    /// Subscribe to tag events from a named PTY session.
    ///
    /// The output pump runs the tag scanner on every chunk of bytes read from
    /// the PTY. When `[{...}]` tags are detected, they are emitted as
    /// [`TagEvent`]s containing both the parsed tag and its wire
    /// [`TokenFrame`]. Downstream consumers (e.g. the NATS bridge) use this
    /// to publish only tag-originated frames.
    pub fn subscribe_tags(&self, name: &str) -> Result<broadcast::Receiver<TagEvent>> {
        self.router
            .subscribe_tags(name)
            .map_err(|e| anyhow!("pty tag subscribe failed: {e}"))
    }

    /// Kill a named PTY session.
    pub async fn kill(&self, name: &str) -> Result<()> {
        self.router
            .kill(name)
            .await
            .map_err(|e| anyhow!("pty kill failed: {e}"))?;
        info!(name, "killed pty session via bridge");
        Ok(())
    }

    /// Kill all PTY sessions managed by the bridge.
    pub async fn kill_all(&self) -> Result<()> {
        self.router
            .kill_all()
            .await
            .map_err(|e| anyhow!("pty kill_all failed: {e}"))?;
        info!("killed all pty sessions via bridge");
        Ok(())
    }

    /// Resize the terminal of a named PTY session.
    pub async fn resize(&self, name: &str, cols: u16, rows: u16) -> Result<()> {
        self.router
            .resize(name, cols, rows)
            .await
            .map_err(|e| anyhow!("pty resize failed: {e}"))?;
        debug!(name, cols, rows, "resized pty session via bridge");
        Ok(())
    }

    /// List all sessions currently in the router.
    pub fn list_sessions(&self) -> Vec<session_pty_router::SessionInfo> {
        self.router.sessions()
    }

    // -- mirror conversion --------------------------------------------------

    /// Convert a PTY [`OutputChunk`] into a [`SessionMirrorChunk`] suitable
    /// for the existing session-api mirror pipeline.
    pub fn output_to_mirror_chunk(&self, chunk: &OutputChunk) -> SessionMirrorChunk {
        let text = String::from_utf8_lossy(&chunk.data).to_string();
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);

        SessionMirrorChunk {
            session_id: chunk.session_id.to_string(),
            sequence,
            emitted_at: chunk.timestamp,
            text,
            authority: AuthorityPosture::Ephemeral,
        }
    }

    /// Read any available output from the session (via `wait_for` with a very
    /// short timeout or the subscribe channel) and convert it to mirror chunks.
    ///
    /// This is a convenience method that drains the broadcast receiver for
    /// already-buffered chunks without blocking.
    pub fn drain_mirror_chunks(
        &self,
        name: &str,
    ) -> Result<Vec<SessionMirrorChunk>> {
        let mut rx = self.subscribe(name)?;
        let mut chunks = Vec::new();

        loop {
            match rx.try_recv() {
                Ok(output_chunk) => {
                    chunks.push(self.output_to_mirror_chunk(&output_chunk));
                }
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    warn!(name, skipped = n, "lagged on pty output drain");
                    continue;
                }
                Err(broadcast::error::TryRecvError::Closed) => break,
            }
        }

        debug!(name, chunk_count = chunks.len(), "drained pty mirror chunks");
        Ok(chunks)
    }

    /// Access the underlying router (for advanced usage).
    pub fn router(&self) -> &PtyRouter {
        &self.router
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use chrono::Utc;
    use session_pty_core::SessionId;
    use session_pty_router::OutputStream;

    use super::*;

    #[test]
    fn output_chunk_converts_to_mirror_chunk() {
        let bridge = PtyBridge::with_defaults();

        let chunk = OutputChunk {
            session_id: SessionId::new(),
            session_name: "test".into(),
            data: Bytes::from_static(b"hello world"),
            timestamp: Utc::now(),
            stream: OutputStream::Stdout,
        };

        let mirror = bridge.output_to_mirror_chunk(&chunk);
        assert_eq!(mirror.text, "hello world");
        assert_eq!(mirror.session_id, chunk.session_id.to_string());
        assert_eq!(mirror.sequence, 1);
        assert_eq!(mirror.authority, AuthorityPosture::Ephemeral);
    }

    #[test]
    fn mirror_chunk_sequence_increments() {
        let bridge = PtyBridge::with_defaults();

        let chunk = OutputChunk {
            session_id: SessionId::new(),
            session_name: "test".into(),
            data: Bytes::from_static(b"line 1"),
            timestamp: Utc::now(),
            stream: OutputStream::Stdout,
        };

        let m1 = bridge.output_to_mirror_chunk(&chunk);
        let m2 = bridge.output_to_mirror_chunk(&chunk);
        let m3 = bridge.output_to_mirror_chunk(&chunk);

        assert_eq!(m1.sequence, 1);
        assert_eq!(m2.sequence, 2);
        assert_eq!(m3.sequence, 3);
    }

    #[test]
    fn non_utf8_output_uses_lossy_conversion() {
        let bridge = PtyBridge::with_defaults();

        let chunk = OutputChunk {
            session_id: SessionId::new(),
            session_name: "test".into(),
            data: Bytes::from_static(b"hello \xff world"),
            timestamp: Utc::now(),
            stream: OutputStream::Stdout,
        };

        let mirror = bridge.output_to_mirror_chunk(&chunk);
        assert!(mirror.text.contains("hello"));
        assert!(mirror.text.contains("world"));
    }

    #[test]
    fn bridge_with_defaults_creates_router() {
        let bridge = PtyBridge::with_defaults();
        assert!(bridge.list_sessions().is_empty());
    }

    #[test]
    fn bridge_from_config() {
        let config = RouterConfig {
            max_sessions: 4,
            ..RouterConfig::default()
        };
        let bridge = PtyBridge::from_config(config);
        assert_eq!(bridge.router().config().max_sessions, 4);
    }

    #[tokio::test]
    async fn launch_and_list_pty_session() {
        let bridge = PtyBridge::with_defaults();

        let id = bridge
            .launch(PtyLaunchRequest {
                name: "test-bash".into(),
                command_type: CommandType::Bash,
                working_dir: "/tmp".into(),
                project_id: "sample".into(),
                requested_by: "test-user".into(),
            })
            .await
            .expect("launch should succeed");

        let sessions = bridge.list_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, id);
        assert_eq!(sessions[0].name, "test-bash");

        bridge.kill("test-bash").await.ok();
    }

    #[tokio::test]
    async fn launch_duplicate_name_fails() {
        let bridge = PtyBridge::with_defaults();

        bridge
            .launch(PtyLaunchRequest {
                name: "dup".into(),
                command_type: CommandType::Bash,
                working_dir: "/tmp".into(),
                project_id: "sample".into(),
                requested_by: "test-user".into(),
            })
            .await
            .expect("first launch");

        let result = bridge
            .launch(PtyLaunchRequest {
                name: "dup".into(),
                command_type: CommandType::Bash,
                working_dir: "/tmp".into(),
                project_id: "sample".into(),
                requested_by: "test-user".into(),
            })
            .await;

        assert!(result.is_err());
        bridge.kill_all().await.ok();
    }

    #[tokio::test]
    async fn send_to_pty_session() {
        let bridge = PtyBridge::with_defaults();

        bridge
            .launch(PtyLaunchRequest {
                name: "cat-test".into(),
                command_type: CommandType::Custom {
                    command: "cat".into(),
                    args: vec![],
                },
                working_dir: "/tmp".into(),
                project_id: "sample".into(),
                requested_by: "test-user".into(),
            })
            .await
            .expect("launch");

        let n = bridge.send("cat-test", b"hello\n").await.expect("send");
        assert!(n > 0);

        bridge.kill("cat-test").await.ok();
    }

    #[tokio::test]
    async fn send_to_missing_session_fails() {
        let bridge = PtyBridge::with_defaults();
        let result = bridge.send("nonexistent", b"hello").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn subscribe_to_pty_session() {
        let bridge = PtyBridge::with_defaults();

        bridge
            .launch(PtyLaunchRequest {
                name: "sub-test".into(),
                command_type: CommandType::Bash,
                working_dir: "/tmp".into(),
                project_id: "sample".into(),
                requested_by: "test-user".into(),
            })
            .await
            .expect("launch");

        let _rx = bridge.subscribe("sub-test").expect("subscribe");

        bridge.kill("sub-test").await.ok();
    }

    #[tokio::test]
    async fn kill_all_clears_sessions() {
        let bridge = PtyBridge::with_defaults();

        bridge
            .launch(PtyLaunchRequest {
                name: "s1".into(),
                command_type: CommandType::Bash,
                working_dir: "/tmp".into(),
                project_id: "sample".into(),
                requested_by: "test-user".into(),
            })
            .await
            .ok();

        bridge
            .launch(PtyLaunchRequest {
                name: "s2".into(),
                command_type: CommandType::Bash,
                working_dir: "/tmp".into(),
                project_id: "sample".into(),
                requested_by: "test-user".into(),
            })
            .await
            .ok();

        assert_eq!(bridge.list_sessions().len(), 2);
        bridge.kill_all().await.expect("kill_all");
        assert_eq!(bridge.list_sessions().len(), 0);
    }

    #[tokio::test]
    async fn drain_mirror_chunks_on_missing_session_fails() {
        let bridge = PtyBridge::with_defaults();
        let result = bridge.drain_mirror_chunks("nonexistent");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn subscribe_tags_to_pty_session() {
        let bridge = PtyBridge::with_defaults();

        bridge
            .launch(PtyLaunchRequest {
                name: "tag-sub-test".into(),
                command_type: CommandType::Bash,
                working_dir: "/tmp".into(),
                project_id: "sample".into(),
                requested_by: "test-user".into(),
            })
            .await
            .expect("launch");

        let _rx = bridge.subscribe_tags("tag-sub-test").expect("subscribe_tags");

        bridge.kill("tag-sub-test").await.ok();
    }

    #[tokio::test]
    async fn subscribe_tags_missing_session_fails() {
        let bridge = PtyBridge::with_defaults();
        let result = bridge.subscribe_tags("nonexistent");
        assert!(result.is_err());
    }
}
