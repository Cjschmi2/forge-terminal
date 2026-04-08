// @session-dev @session-contracts
//! Session contract types.
//!
//! All external provider types (ProviderKind, ProviderHealth,
//! ProviderCapabilitySummary, SessionLaunchRequest, SessionLaunchRecord)
//! have been removed. forge-terminal is now a native desktop terminal
//! emulator; sessions are PTY-only.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Agent CLI tools that can be launched via the command center
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCliTool {
    Claude,
    OpenCode,
    Codex,
    Cursor,
}

impl AgentCliTool {
    /// The binary name to invoke
    pub fn binary_name(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::OpenCode => "opencode",
            Self::Codex => "codex",
            Self::Cursor => "agent",
        }
    }

    /// Install command if the binary is missing
    pub fn install_command(&self) -> &'static str {
        match self {
            Self::Claude => "npm install -g @anthropic-ai/claude-code",
            Self::OpenCode => "npm install -g open-code",
            Self::Codex => "npm install -g @openai/codex",
            Self::Cursor => "curl -fsSL https://www.cursor.com/install.sh | sh",
        }
    }

    /// Human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::OpenCode => "OpenCode",
            Self::Codex => "Codex CLI",
            Self::Cursor => "Cursor",
        }
    }

    pub fn all() -> &'static [AgentCliTool] {
        &[Self::Claude, Self::OpenCode, Self::Codex, Self::Cursor]
    }
}

/// Authority level for session data. Determines how the data should be treated
/// by consumers (source of truth, read-only view, cached copy, or ephemeral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthorityPosture {
    Source,
    View,
    Cache,
    Ephemeral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionLifecycle {
    Launched,
    Registered,
    Detached,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRegistration {
    pub session_id: String,
    pub project_id: String,
    pub identity: String,
    pub role: String,
    pub machine_id: String,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMirrorChunk {
    pub session_id: String,
    pub sequence: u64,
    pub emitted_at: DateTime<Utc>,
    pub text: String,
    pub authority: AuthorityPosture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachRequest {
    pub session_id: String,
    pub requested_by: String,
    pub interactive: bool,
}

/// State of a PTY session.
///
/// Previously carried `provider_kind`, `provider_id`, and
/// `provider_session_id` for the external provider abstraction.  Those
/// fields are gone; all sessions are native PTY sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub machine_id: String,
    pub project_id: String,
    pub identity: Option<String>,
    pub role: Option<String>,
    pub session_name: Option<String>,
    pub session_ref: Option<String>,
    pub source: Option<String>,
    pub status: Option<String>,
    pub notify_target: Option<String>,
    pub last_heartbeat_at: DateTime<Utc>,
    pub interactive_capable: bool,
    pub lifecycle: SessionLifecycle,
    pub authority: AuthorityPosture,
}

impl SessionState {
    pub fn is_registered(&self) -> bool {
        matches!(self.lifecycle, SessionLifecycle::Registered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_is_distinct_from_launch() {
        let state = SessionState {
            session_id: "s-1".into(),
            machine_id: "machine-a".into(),
            project_id: "sample".into(),
            identity: None,
            role: None,
            session_name: Some("alpha".into()),
            session_ref: None,
            source: Some("launch".into()),
            status: Some("active".into()),
            notify_target: None,
            last_heartbeat_at: Utc::now(),
            interactive_capable: true,
            lifecycle: SessionLifecycle::Launched,
            authority: AuthorityPosture::View,
        };

        assert!(!state.is_registered());
    }
}
