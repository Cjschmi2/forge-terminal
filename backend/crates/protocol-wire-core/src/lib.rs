//! Wire protocol core types for the project-management platform.
//!
//! Defines the binary frame format, token type vocabulary, stream management,
//! and control messages used for agent-to-agent and agent-to-UI communication.
//!
//! Forked from forge-wire-core, stripped of forge SDK annotations.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub mod control;
pub mod frame;
pub mod stream;
pub mod token;
pub mod trace;

pub use frame::{FrameIterator, HEADER_SIZE, MAX_DATA_SIZE, TokenFrame, TokenFrameRef};
pub use token::{TokenCategory, TokenMetadata, TokenType};

/// Canonical result type for wire operations.
pub type WireResult<T> = Result<T, WireError>;

// ============================================================
// WIRE VERSION
// ============================================================

/// Version negotiation type for the wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WireVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl WireVersion {
    pub const fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self { major, minor, patch }
    }

    /// The current wire protocol version.
    pub const CURRENT: Self = Self::new(1, 0, 0);

    /// Returns true if this version is compatible with `other` (same major).
    pub fn is_compatible_with(&self, other: &WireVersion) -> bool {
        self.major == other.major
    }
}

impl std::fmt::Display for WireVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ============================================================
// WIRE ID
// ============================================================

/// Newtype wrapping a UUID for wire message and connection identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WireId(Uuid);

impl WireId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for WireId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WireId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================
// FRAME KIND
// ============================================================

/// Fundamental classification of wire frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameKind {
    /// A token chunk in a streaming response.
    Token,
    /// A complete non-streaming response.
    Complete,
    /// A tool call request from the model.
    ToolCall,
    /// The result of a tool invocation.
    ToolResult,
    /// An error frame.
    Error,
    /// A heartbeat / keep-alive frame.
    Ping,
    /// Acknowledges a ping.
    Pong,
    /// Signals the end of a stream.
    End,
}

impl FrameKind {
    pub fn is_data(self) -> bool {
        matches!(self, Self::Token | Self::Complete | Self::ToolCall | Self::ToolResult)
    }

    pub fn is_control(self) -> bool {
        matches!(self, Self::Ping | Self::Pong | Self::End)
    }
}

impl std::fmt::Display for FrameKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Token => "token",
            Self::Complete => "complete",
            Self::ToolCall => "tool_call",
            Self::ToolResult => "tool_result",
            Self::Error => "error",
            Self::Ping => "ping",
            Self::Pong => "pong",
            Self::End => "end",
        };
        f.write_str(s)
    }
}

// ============================================================
// WIRE ERROR
// ============================================================

/// Base error type for all wire protocol operations.
#[derive(Debug, Error)]
pub enum WireError {
    #[error("version negotiation failed: local={local}, remote={remote}")]
    VersionMismatch { local: WireVersion, remote: WireVersion },

    #[error("malformed frame: {reason}")]
    MalformedFrame { reason: String },

    #[error("frame too large: size={size} exceeds limit={limit}")]
    FrameTooLarge { size: usize, limit: usize },

    #[error("connection closed unexpectedly")]
    ConnectionClosed,

    #[error("serialization error: {detail}")]
    Serialization { detail: String },

    #[error("unknown frame kind: {kind}")]
    UnknownFrameKind { kind: u8 },

    #[error("internal wire error: {detail}")]
    Internal { detail: String },

    #[error("frame too short: got {got} bytes, need {need}")]
    FrameTooShort { got: usize, need: usize },

    #[error("incomplete frame: expected {expected} data bytes, got {got}")]
    IncompleteFrame { expected: usize, got: usize },

    #[error("UTF-8 decode error: {detail}")]
    Utf8Error { detail: String },
}

impl WireError {
    pub fn malformed(reason: impl Into<String>) -> Self {
        Self::MalformedFrame { reason: reason.into() }
    }

    pub fn too_large(size: usize, limit: usize) -> Self {
        Self::FrameTooLarge { size, limit }
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal { detail: detail.into() }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::ConnectionClosed)
    }
}

/// Get current timestamp in nanoseconds.
pub fn current_timestamp_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}
