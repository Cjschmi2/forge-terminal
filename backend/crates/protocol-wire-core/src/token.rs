//! Token type vocabulary for the wire protocol.
//!
//! Each token type is a u8 identifier organized into ranges:
//! - 0x00-0x0F: Standard LLM Streaming
//! - 0x10-0x1F: Orchestration/Multi-Agent
//! - 0x20-0x2F: Skills + Terminal
//! - 0x30-0x3F: UI Control
//! - 0x40-0x4F: Query + Session Lifecycle
//! - 0x50-0x5F: Analysis + Workers
//! - 0x60-0x6F: Artifacts + Collaboration
//! - 0x70-0x7F: Code + PTY + GoT
//! - 0x80-0x8F: Compose + Documentation
//! - 0x90-0xAF: Learning/Training/Session Protocol
//! - 0xB0-0xBF: Operations + Mode System
//! - 0xC0-0xCF: Cache + UI Subscriptions
//! - 0xD0-0xDF: Geospatial + Hierarchy
//! - 0xE0-0xEF: Validation + Intelligence + Inference
//! - 0xF0-0xFF: Debug/Telemetry/Trace

use serde::{Deserialize, Serialize};

/// Well-known token type ID constants.
pub mod id {
    // Standard LLM Streaming (0x00-0x0F)
    pub const CONTENT: u8 = 0x00;
    pub const ROLE: u8 = 0x01;
    pub const TOOL_CALL: u8 = 0x02;
    pub const TOOL_RESULT: u8 = 0x03;
    pub const DONE: u8 = 0x04;
    pub const ERROR: u8 = 0x05;
    pub const METADATA: u8 = 0x06;
    pub const PLAN_UPDATE: u8 = 0x07;
    pub const THINKING: u8 = 0x09;
    pub const THINKING_DONE: u8 = 0x0A;
    pub const SYSTEM_MESSAGE: u8 = 0x0C;

    // Orchestration/Multi-Agent (0x10-0x1F)
    pub const AGENT_SPAWN: u8 = 0x11;
    pub const AGENT_COMPLETE: u8 = 0x12;
    pub const AGENT_MESSAGE: u8 = 0x16;
    pub const COORDINATION_CHECKPOINT: u8 = 0x17;
    pub const AGENT_HANDOFF: u8 = 0x1F;

    // Skills + Terminal (0x20-0x2F)
    pub const SKILL_START: u8 = 0x20;
    pub const SKILL_PROGRESS: u8 = 0x21;
    pub const SKILL_DATA: u8 = 0x22;
    pub const SKILL_COMPLETE: u8 = 0x23;
    pub const SKILL_ERROR: u8 = 0x24;
    pub const TERMINAL_OUTPUT: u8 = 0x26;
    pub const TERMINAL_PROMPT: u8 = 0x2A;
    pub const TERMINAL_INPUT: u8 = 0x2B;
    pub const TERMINAL_SESSION: u8 = 0x2C;

    // UI Control (0x30-0x3F)
    pub const UI_NOTIFICATION: u8 = 0x32;
    pub const UI_PROGRESS: u8 = 0x33;
    pub const UI_STATE: u8 = 0x34;

    // PTY System (0x78-0x7A)
    pub const PTY_DATA: u8 = 0x78;
    pub const PTY_CONTROL: u8 = 0x79;
    pub const PTY_STATUS: u8 = 0x7A;

    // Session Protocol (0xAB-0xAE)
    pub const SESSION_COMMAND: u8 = 0xAB;
    pub const SESSION_EVENT: u8 = 0xAC;
    pub const SESSION_WELCOME: u8 = 0xAD;
    pub const SESSION_HEARTBEAT: u8 = 0xAE;

    // Presence (0x68-0x6B)
    pub const PRESENCE_JOIN: u8 = 0x68;
    pub const PRESENCE_LEAVE: u8 = 0x69;
    pub const PRESENCE_UPDATE: u8 = 0x6A;
    pub const PRESENCE_HEARTBEAT: u8 = 0x6B;

    // Debug/Telemetry (0xF0-0xFF)
    pub const DEBUG_TRACE: u8 = 0xF0;
    pub const TELEMETRY_HEARTBEAT: u8 = 0xF3;
    pub const TELEMETRY_METRICS: u8 = 0xF4;
    pub const SPAN_START: u8 = 0xF7;
    pub const SPAN_END: u8 = 0xF8;

    pub const UNKNOWN: u8 = 0xFE;
}

/// High-level token type enum for type-safe matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TokenType {
    // Standard LLM Streaming (0x00-0x0F)
    Content = 0x00,
    Role = 0x01,
    ToolCall = 0x02,
    ToolResult = 0x03,
    Done = 0x04,
    Error = 0x05,
    Metadata = 0x06,
    PlanUpdate = 0x07,
    ContentCompressed = 0x08,
    Thinking = 0x09,
    ThinkingDone = 0x0A,
    PromptInjection = 0x0B,
    SystemMessage = 0x0C,
    McpServerCall = 0x0D,
    McpServerResult = 0x0E,
    McpResourceAccess = 0x0F,

    // Orchestration/Multi-Agent (0x10-0x1F)
    OrchestrationPhase = 0x10,
    AgentSpawn = 0x11,
    AgentComplete = 0x12,
    DebateStatement = 0x13,
    DebateJudgment = 0x14,
    WorkspaceArtifact = 0x15,
    AgentMessage = 0x16,
    CoordinationCheckpoint = 0x17,
    HiveWorkerStatus = 0x18,
    DebateRound = 0x19,
    HiveFanOut = 0x1A,
    HiveFanIn = 0x1B,
    SpecialistResult = 0x1C,
    CheckpointSave = 0x1D,
    CheckpointResume = 0x1E,
    AgentHandoff = 0x1F,

    // Skills + Terminal (0x20-0x2F)
    SkillStart = 0x20,
    SkillProgress = 0x21,
    SkillData = 0x22,
    SkillComplete = 0x23,
    SkillError = 0x24,
    SkillCancelled = 0x25,
    TerminalOutput = 0x26,
    TerminalBgStart = 0x27,
    TerminalBgOutput = 0x28,
    TerminalBgComplete = 0x29,
    TerminalPrompt = 0x2A,
    TerminalInput = 0x2B,
    TerminalSession = 0x2C,
    GotPathOpen = 0x2D,
    GotPathClosed = 0x2E,
    GotBacktrack = 0x2F,

    // UI Control (0x30-0x3F)
    UiScrollHint = 0x30,
    UiFocus = 0x31,
    UiNotification = 0x32,
    UiProgress = 0x33,
    UiState = 0x34,
    UiModal = 0x35,
    UiTheme = 0x36,
    UiPanel = 0x37,
    UiArtifactPreview = 0x38,
    UiControlRequest = 0x39,
    UiControlResponse = 0x3A,
    UiControlDiscovery = 0x3B,
    UiControlBatchRequest = 0x3C,
    UiControlBatchResponse = 0x3D,
    UiControlSessionInit = 0x3E,
    UiControlSubscribe = 0x3F,

    // Query (0x40-0x4F)
    QueryStart = 0x40,
    QueryPlan = 0x41,
    QueryRows = 0x42,
    QuerySchema = 0x43,
    QueryStats = 0x44,
    QueryComplete = 0x45,
    QueryCacheHit = 0x46,
    HydrationStart = 0x47,
    HydrationTableReady = 0x48,
    HydrationProgress = 0x49,
    HydrationComplete = 0x4A,
    HierarchyCreated = 0x4B,
    HierarchyStatus = 0x4C,
    HierarchyNavigate = 0x4D,
    ChatMessage = 0x4E,
    ChatTyping = 0x4F,

    // Analyze (0x50-0x5F)
    AnalyzeStart = 0x50,
    AnalyzeStats = 0x51,
    AnalyzeCorrelation = 0x52,
    AnalyzeOutliers = 0x53,
    AnalyzeTrend = 0x54,
    AnalyzeGeo = 0x55,
    AnalyzePrediction = 0x56,
    AnalyzeComplete = 0x57,
    WorkerSpawn = 0x58,
    WorkerComplete = 0x59,
    WorkerProgress = 0x5A,
    Escalation = 0x5F,

    // Artifacts + Collaboration (0x60-0x6F)
    ArtifactStart = 0x60,
    ArtifactChart = 0x61,
    ArtifactTable = 0x62,
    ArtifactExport = 0x63,
    ArtifactHtml = 0x64,
    ArtifactSvg = 0x65,
    ArtifactMetadata = 0x66,
    ArtifactComplete = 0x67,
    PresenceJoin = 0x68,
    PresenceLeave = 0x69,
    PresenceUpdate = 0x6A,
    PresenceHeartbeat = 0x6B,
    LockAcquired = 0x6C,
    LockReleased = 0x6D,
    LockDenied = 0x6E,
    ApprovalRequested = 0x6F,

    // Code + PTY (0x70-0x7F)
    CodeStart = 0x70,
    CodeStdout = 0x71,
    CodeStderr = 0x72,
    CodeComplete = 0x73,
    CodeFileOp = 0x74,
    CodeGitOp = 0x75,
    CodeDiff = 0x76,
    CodeLint = 0x77,
    PtyData = 0x78,
    PtyControl = 0x79,
    PtyStatus = 0x7A,
    GotStart = 0x7B,
    GotDecompose = 0x7C,
    GotConclusion = 0x7D,
    GotEnd = 0x7E,

    // Session Protocol (0xAB-0xAE)
    SessionCommand = 0xAB,
    SessionEvent = 0xAC,
    SessionWelcome = 0xAD,
    SessionHeartbeat = 0xAE,

    // Operations (0xB0-0xBF)
    OperationStart = 0xB0,
    OperationProgress = 0xB1,
    OperationComplete = 0xB4,
    OperationError = 0xB5,

    // Debug/Telemetry (0xF0-0xFF)
    DebugTrace = 0xF0,
    DebugTiming = 0xF1,
    TelemetryHeartbeat = 0xF3,
    TelemetryMetrics = 0xF4,
    SpanStart = 0xF7,
    SpanEnd = 0xF8,
    TraceContext = 0xF9,

    Unknown = 0xFE,
}

impl TokenType {
    #[inline]
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0x00 => Self::Content,
            0x01 => Self::Role,
            0x02 => Self::ToolCall,
            0x03 => Self::ToolResult,
            0x04 => Self::Done,
            0x05 => Self::Error,
            0x06 => Self::Metadata,
            0x07 => Self::PlanUpdate,
            0x08 => Self::ContentCompressed,
            0x09 => Self::Thinking,
            0x0A => Self::ThinkingDone,
            0x0B => Self::PromptInjection,
            0x0C => Self::SystemMessage,
            0x0D => Self::McpServerCall,
            0x0E => Self::McpServerResult,
            0x0F => Self::McpResourceAccess,
            0x10 => Self::OrchestrationPhase,
            0x11 => Self::AgentSpawn,
            0x12 => Self::AgentComplete,
            0x13 => Self::DebateStatement,
            0x14 => Self::DebateJudgment,
            0x15 => Self::WorkspaceArtifact,
            0x16 => Self::AgentMessage,
            0x17 => Self::CoordinationCheckpoint,
            0x18 => Self::HiveWorkerStatus,
            0x19 => Self::DebateRound,
            0x1A => Self::HiveFanOut,
            0x1B => Self::HiveFanIn,
            0x1C => Self::SpecialistResult,
            0x1D => Self::CheckpointSave,
            0x1E => Self::CheckpointResume,
            0x1F => Self::AgentHandoff,
            0x20 => Self::SkillStart,
            0x21 => Self::SkillProgress,
            0x22 => Self::SkillData,
            0x23 => Self::SkillComplete,
            0x24 => Self::SkillError,
            0x25 => Self::SkillCancelled,
            0x26 => Self::TerminalOutput,
            0x27 => Self::TerminalBgStart,
            0x28 => Self::TerminalBgOutput,
            0x29 => Self::TerminalBgComplete,
            0x2A => Self::TerminalPrompt,
            0x2B => Self::TerminalInput,
            0x2C => Self::TerminalSession,
            0x2D => Self::GotPathOpen,
            0x2E => Self::GotPathClosed,
            0x2F => Self::GotBacktrack,
            0x30 => Self::UiScrollHint,
            0x31 => Self::UiFocus,
            0x32 => Self::UiNotification,
            0x33 => Self::UiProgress,
            0x34 => Self::UiState,
            0x35 => Self::UiModal,
            0x36 => Self::UiTheme,
            0x37 => Self::UiPanel,
            0x38 => Self::UiArtifactPreview,
            0x39 => Self::UiControlRequest,
            0x3A => Self::UiControlResponse,
            0x3B => Self::UiControlDiscovery,
            0x3C => Self::UiControlBatchRequest,
            0x3D => Self::UiControlBatchResponse,
            0x3E => Self::UiControlSessionInit,
            0x3F => Self::UiControlSubscribe,
            0x40 => Self::QueryStart,
            0x41 => Self::QueryPlan,
            0x42 => Self::QueryRows,
            0x43 => Self::QuerySchema,
            0x44 => Self::QueryStats,
            0x45 => Self::QueryComplete,
            0x46 => Self::QueryCacheHit,
            0x47 => Self::HydrationStart,
            0x48 => Self::HydrationTableReady,
            0x49 => Self::HydrationProgress,
            0x4A => Self::HydrationComplete,
            0x4B => Self::HierarchyCreated,
            0x4C => Self::HierarchyStatus,
            0x4D => Self::HierarchyNavigate,
            0x4E => Self::ChatMessage,
            0x4F => Self::ChatTyping,
            0x50 => Self::AnalyzeStart,
            0x51 => Self::AnalyzeStats,
            0x52 => Self::AnalyzeCorrelation,
            0x53 => Self::AnalyzeOutliers,
            0x54 => Self::AnalyzeTrend,
            0x55 => Self::AnalyzeGeo,
            0x56 => Self::AnalyzePrediction,
            0x57 => Self::AnalyzeComplete,
            0x58 => Self::WorkerSpawn,
            0x59 => Self::WorkerComplete,
            0x5A => Self::WorkerProgress,
            0x5F => Self::Escalation,
            0x60 => Self::ArtifactStart,
            0x61 => Self::ArtifactChart,
            0x62 => Self::ArtifactTable,
            0x63 => Self::ArtifactExport,
            0x64 => Self::ArtifactHtml,
            0x65 => Self::ArtifactSvg,
            0x66 => Self::ArtifactMetadata,
            0x67 => Self::ArtifactComplete,
            0x68 => Self::PresenceJoin,
            0x69 => Self::PresenceLeave,
            0x6A => Self::PresenceUpdate,
            0x6B => Self::PresenceHeartbeat,
            0x6C => Self::LockAcquired,
            0x6D => Self::LockReleased,
            0x6E => Self::LockDenied,
            0x6F => Self::ApprovalRequested,
            0x70 => Self::CodeStart,
            0x71 => Self::CodeStdout,
            0x72 => Self::CodeStderr,
            0x73 => Self::CodeComplete,
            0x74 => Self::CodeFileOp,
            0x75 => Self::CodeGitOp,
            0x76 => Self::CodeDiff,
            0x77 => Self::CodeLint,
            0x78 => Self::PtyData,
            0x79 => Self::PtyControl,
            0x7A => Self::PtyStatus,
            0x7B => Self::GotStart,
            0x7C => Self::GotDecompose,
            0x7D => Self::GotConclusion,
            0x7E => Self::GotEnd,
            0xAB => Self::SessionCommand,
            0xAC => Self::SessionEvent,
            0xAD => Self::SessionWelcome,
            0xAE => Self::SessionHeartbeat,
            0xB0 => Self::OperationStart,
            0xB1 => Self::OperationProgress,
            0xB4 => Self::OperationComplete,
            0xB5 => Self::OperationError,
            0xF0 => Self::DebugTrace,
            0xF1 => Self::DebugTiming,
            0xF3 => Self::TelemetryHeartbeat,
            0xF4 => Self::TelemetryMetrics,
            0xF7 => Self::SpanStart,
            0xF8 => Self::SpanEnd,
            0xF9 => Self::TraceContext,
            _ => Self::Unknown,
        }
    }

    #[inline]
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub fn category(self) -> TokenCategory {
        match self.as_u8() {
            0x00..=0x0F => TokenCategory::LlmStreaming,
            0x10..=0x1F => TokenCategory::Orchestration,
            0x20..=0x2F => TokenCategory::Skill,
            0x30..=0x3F => TokenCategory::UiControl,
            0x40..=0x4F => TokenCategory::Query,
            0x50..=0x5F => TokenCategory::Analyze,
            0x60..=0x6F => TokenCategory::Artifact,
            0x70..=0x7F => TokenCategory::Code,
            0xAB..=0xAE => TokenCategory::Session,
            0xB0..=0xBF => TokenCategory::Operation,
            0xF0..=0xFF => TokenCategory::Debug,
            _ => TokenCategory::Unknown,
        }
    }

    #[must_use]
    pub fn is_terminal_token(self) -> bool {
        matches!(
            self,
            Self::Done
                | Self::Error
                | Self::SkillComplete
                | Self::SkillError
                | Self::AgentComplete
                | Self::OperationComplete
                | Self::OperationError
        )
    }
}

/// High-level category grouping for token types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenCategory {
    LlmStreaming,
    Orchestration,
    Skill,
    UiControl,
    Query,
    Analyze,
    Artifact,
    Code,
    Session,
    Operation,
    Debug,
    Unknown,
}

/// Metadata about a token type: whether it's terminal or carries text.
pub struct TokenMetadata {
    pub is_terminal: bool,
    pub has_text: bool,
}

impl TokenMetadata {
    pub fn for_type(token_type: u8) -> Self {
        let tt = TokenType::from_u8(token_type);
        Self {
            is_terminal: tt.is_terminal_token(),
            has_text: matches!(
                tt,
                TokenType::Content
                    | TokenType::Role
                    | TokenType::Error
                    | TokenType::Metadata
                    | TokenType::SystemMessage
                    | TokenType::AgentMessage
                    | TokenType::TerminalOutput
                    | TokenType::TerminalInput
                    | TokenType::ChatMessage
                    | TokenType::CodeStdout
                    | TokenType::CodeStderr
                    | TokenType::PtyData
                    | TokenType::DebugTrace
            ),
        }
    }
}
