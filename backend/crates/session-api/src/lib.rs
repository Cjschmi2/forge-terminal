// @session-dev @session-api
//! Session API -- PTY-only.
//!
//! The external provider abstraction (ProviderRouter, LaunchService,
//! session-provider-core, session-provider-kitty) has been removed.
//! forge-terminal is now a native desktop terminal emulator. All
//! sessions are PTY sessions managed through [`PtyBridge`].

pub mod pty_bridge;
pub mod registry;

pub use pty_bridge::{PtyBridge, PtyLaunchRequest};
pub use registry::SessionRegistry;

use std::collections::{HashMap, VecDeque};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use session_contracts::{AuthorityPosture, SessionMirrorChunk, SessionState};
use session_pty_core::SessionId;
use session_pty_router::OutputChunk;
use tokio::sync::broadcast;
use tracing::debug;

// ---------------------------------------------------------------------------
// Inline mirror buffer (replaces session-mirror crate + Redis pub/sub)
// ---------------------------------------------------------------------------

/// In-memory ring buffer that keeps the most recent mirror chunks per session.
///
/// Previously backed by `session-mirror` which coupled to Redis pub/sub.  The
/// migration rules forbid new Redis PUBLISH/SUBSCRIBE patterns, so this
/// simplified version drops the Redis path entirely while preserving the same
/// public interface used by [`SessionApi`].
#[derive(Debug)]
struct MirrorBuffer {
    by_session: HashMap<String, VecDeque<SessionMirrorChunk>>,
    limit: usize,
}

impl MirrorBuffer {
    fn new(limit: usize) -> Self {
        Self { by_session: HashMap::new(), limit }
    }

    fn push(&mut self, session_id: &str, text: impl Into<String>) -> SessionMirrorChunk {
        let queue = self.by_session.entry(session_id.to_string()).or_default();
        let chunk = SessionMirrorChunk {
            session_id: session_id.to_string(),
            sequence: queue.back().map(|entry| entry.sequence + 1).unwrap_or(1),
            emitted_at: chrono::Utc::now(),
            text: text.into(),
            authority: AuthorityPosture::Ephemeral,
        };
        queue.push_back(chunk.clone());
        while queue.len() > self.limit {
            queue.pop_front();
        }
        chunk
    }

    fn session_chunks(&self, session_id: &str) -> Vec<SessionMirrorChunk> {
        self.by_session.get(session_id).map(|q| q.iter().cloned().collect()).unwrap_or_default()
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDetail {
    pub state: SessionState,
    pub mirror: Vec<SessionMirrorChunk>,
    /// Session identifier for attach (replaces the old `AttachTarget` struct
    /// from the deleted provider abstraction).
    pub attach: Option<String>,
}

/// PTY-only session API.
///
/// All provider routing has been removed. The API wraps a [`PtyBridge`] for
/// session lifecycle and an inline mirror buffer for output history.
pub struct SessionApi {
    registry: SessionRegistry,
    mirror: MirrorBuffer,
    pty_bridge: PtyBridge,
}

impl SessionApi {
    /// Create a `SessionApi` with a PTY bridge.
    pub fn new(pty_bridge: PtyBridge) -> Self {
        let api = Self {
            registry: SessionRegistry::default(),
            mirror: MirrorBuffer::new(200),
            pty_bridge,
        };
        debug!("initialized session api (pty-only)");
        api
    }

    pub fn mirror_line(&mut self, session_id: &str, text: impl Into<String>) {
        self.mirror.push(session_id, text);
    }

    pub fn mirror(&self, session_id: &str) -> Vec<SessionMirrorChunk> {
        self.mirror.session_chunks(session_id)
    }

    pub fn registry(&self) -> &SessionRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut SessionRegistry {
        &mut self.registry
    }

    // -- PTY methods -----------------------------------------------------------

    /// Launch a new PTY session through the bridge.
    ///
    /// Returns the PTY [`SessionId`] on success.
    pub async fn launch_pty_session(&self, request: PtyLaunchRequest) -> Result<SessionId> {
        self.pty_bridge.launch(request).await
    }

    /// Send raw input to a named PTY session.
    pub async fn send_to_pty(&self, name: &str, input: &[u8]) -> Result<usize> {
        self.pty_bridge.send(name, input).await
    }

    /// Subscribe to the output broadcast for a named PTY session.
    pub fn subscribe_pty(&self, name: &str) -> Result<broadcast::Receiver<OutputChunk>> {
        self.pty_bridge.subscribe(name)
    }

    /// Kill a named PTY session.
    pub async fn kill_pty(&self, name: &str) -> Result<()> {
        self.pty_bridge.kill(name).await
    }

    /// List PTY sessions.
    pub fn list_pty_sessions(&self) -> Vec<session_pty_router::SessionInfo> {
        self.pty_bridge.list_sessions()
    }

    /// Drain buffered PTY output as [`SessionMirrorChunk`] values for
    /// backward compatibility with the existing mirror pipeline.
    pub fn drain_pty_mirror(&self, name: &str) -> Result<Vec<SessionMirrorChunk>> {
        self.pty_bridge.drain_mirror_chunks(name)
    }

    /// Access the underlying [`PtyBridge`].
    pub fn pty_bridge(&self) -> &PtyBridge {
        &self.pty_bridge
    }

    /// Build a detail view for a PTY session.
    pub fn detail(&self, session_id: &str) -> Result<SessionDetail> {
        let state = self.registry.state(session_id)
            .ok_or_else(|| anyhow!("session not found: {session_id}"))?;
        let mirror = self.mirror.session_chunks(session_id);
        let attach = if state.interactive_capable {
            Some(session_id.to_string())
        } else {
            None
        };
        let detail = SessionDetail { state, mirror, attach };
        debug!(
            session_id,
            mirror_chunk_count = detail.mirror.len(),
            attach_available = detail.attach.is_some(),
            "loaded session detail"
        );
        Ok(detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_reports_pty_bridge() {
        let bridge = PtyBridge::with_defaults();
        let api = SessionApi::new(bridge);
        assert!(api.pty_bridge().list_sessions().is_empty());
    }

    #[tokio::test]
    async fn pty_launch_and_kill_through_api() {
        let bridge = PtyBridge::with_defaults();
        let api = SessionApi::new(bridge);

        let id = api
            .launch_pty_session(PtyLaunchRequest {
                name: "api-bash".into(),
                command_type: session_pty_router::CommandType::Bash,
                working_dir: "/tmp".into(),
                project_id: "sample".into(),
                requested_by: "lead".into(),
            })
            .await
            .expect("launch pty session");

        let sessions = api.list_pty_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, id);

        api.kill_pty("api-bash").await.expect("kill pty session");
        let sessions = api.list_pty_sessions();
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn pty_send_through_api() {
        let bridge = PtyBridge::with_defaults();
        let api = SessionApi::new(bridge);

        api.launch_pty_session(PtyLaunchRequest {
            name: "cat-api".into(),
            command_type: session_pty_router::CommandType::Custom {
                command: "cat".into(),
                args: vec![],
            },
            working_dir: "/tmp".into(),
            project_id: "sample".into(),
            requested_by: "lead".into(),
        })
        .await
        .expect("launch");

        let n = api.send_to_pty("cat-api", b"hello\n").await.expect("send");
        assert!(n > 0);

        api.kill_pty("cat-api").await.ok();
    }

    #[tokio::test]
    async fn pty_subscribe_through_api() {
        let bridge = PtyBridge::with_defaults();
        let api = SessionApi::new(bridge);

        api.launch_pty_session(PtyLaunchRequest {
            name: "sub-api".into(),
            command_type: session_pty_router::CommandType::Bash,
            working_dir: "/tmp".into(),
            project_id: "sample".into(),
            requested_by: "lead".into(),
        })
        .await
        .expect("launch");

        let _rx = api.subscribe_pty("sub-api").expect("subscribe");
        api.kill_pty("sub-api").await.ok();
    }

    #[test]
    fn mirror_buffer_records_lines() {
        let bridge = PtyBridge::with_defaults();
        let mut api = SessionApi::new(bridge);
        api.mirror_line("s1", "line 1");
        api.mirror_line("s1", "line 2");
        let chunks = api.mirror("s1");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "line 1");
        assert_eq!(chunks[1].text, "line 2");
    }
}
