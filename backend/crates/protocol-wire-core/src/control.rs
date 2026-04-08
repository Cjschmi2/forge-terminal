//! Control messages for wire protocol handshake and feature negotiation.

use serde::{Deserialize, Serialize};

use crate::WireVersion;

/// Feature flags negotiated during handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeatureFlags {
    bits: u32,
}

impl FeatureFlags {
    pub const NONE: Self = Self { bits: 0 };

    pub const COMPRESSION: Self = Self { bits: 1 << 0 };
    pub const MULTI_STREAM: Self = Self { bits: 1 << 1 };
    pub const CHECKPOINT_RESUME: Self = Self { bits: 1 << 2 };
    pub const BINARY_CODEC: Self = Self { bits: 1 << 3 };
    pub const AUTH_REQUIRED: Self = Self { bits: 1 << 4 };
    pub const TRACING: Self = Self { bits: 1 << 5 };

    pub const fn all() -> Self {
        Self { bits: 0x3F }
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    pub const fn union(self, other: Self) -> Self {
        Self { bits: self.bits | other.bits }
    }

    pub const fn intersect(self, other: Self) -> Self {
        Self { bits: self.bits & other.bits }
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self { bits }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::NONE
    }
}

/// Control messages exchanged during connection lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessage {
    /// Initial handshake request from client.
    Hello { version: WireVersion, requested_features: FeatureFlags, client_id: String },

    /// Handshake response from server.
    Welcome { version: WireVersion, negotiated_features: FeatureFlags, server_id: String },

    /// Ping for keep-alive.
    Ping { sequence: u32 },

    /// Pong response.
    Pong { sequence: u32 },

    /// Graceful shutdown request.
    Goodbye { reason: String },

    /// Auth token for JWT validation during handshake.
    Auth { token: String },

    /// Auth result.
    AuthResult { success: bool, message: String },
}

impl ControlMessage {
    pub fn hello(client_id: impl Into<String>, features: FeatureFlags) -> Self {
        Self::Hello {
            version: WireVersion::CURRENT,
            requested_features: features,
            client_id: client_id.into(),
        }
    }

    pub fn welcome(server_id: impl Into<String>, features: FeatureFlags) -> Self {
        Self::Welcome {
            version: WireVersion::CURRENT,
            negotiated_features: features,
            server_id: server_id.into(),
        }
    }

    pub fn auth(token: impl Into<String>) -> Self {
        Self::Auth { token: token.into() }
    }
}
