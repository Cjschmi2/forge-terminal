//! PTY stdout tag scanner.
//!
//! Scans byte streams from agent PTY output for structured `[{tag_type:payload}]`
//! patterns and converts them into wire protocol [`TokenFrame`]s.
//!
//! Tag format: `[{tag_type:payload}]`
//!
//! Supported tag types:
//! - `task:ID:action` — task lifecycle events
//! - `status:message` — status updates
//! - `@agent-name:message` — directed agent messages
//! - `broadcast:message` — broadcast to all agents
//! - `gate:pass` / `gate:fail:reason` — phase gate results
//! - `metric:name:value` — metric reports

use protocol_wire_core::frame::TokenFrame;
use protocol_wire_core::token;
use regex::Regex;
use std::sync::LazyLock;

// ═══════════════════════════════════════════════════════════════════════════════
// TAG TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// A parsed tag extracted from PTY output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTag {
    /// The classification of this tag.
    pub tag_type: TagType,
    /// The raw text of the tag including delimiters, e.g. `[{status:done}]`.
    pub raw: String,
    /// The inner payload text (everything between `[{` and `}]`).
    pub payload: String,
}

/// Classification of a parsed tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagType {
    /// Task lifecycle event: `task:T-123:complete`.
    Task { id: String, action: String },
    /// Status update: `status:building`.
    Status { message: String },
    /// Directed agent message: `@agent-name:message text`.
    AgentMessage { target: String, message: String },
    /// Broadcast to all agents: `broadcast:announcement`.
    Broadcast { message: String },
    /// Phase gate result: `gate:pass` or `gate:fail:reason`.
    Gate { result: GateResult },
    /// Metric report: `metric:build_time:42.5`.
    Metric { name: String, value: String },
    /// Unrecognized tag type — preserved for forward compatibility.
    Unknown { tag: String, payload: String },
}

/// Result of a phase gate evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateResult {
    Pass,
    Fail { reason: String },
}

// ═══════════════════════════════════════════════════════════════════════════════
// SCAN RESULT
// ═══════════════════════════════════════════════════════════════════════════════

/// Output of a single `feed` call.
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    /// Tags found in the input.
    pub tags: Vec<ParsedTag>,
    /// Non-tag text chunks (interleaved between tags).
    pub text: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// TAG SCANNER
// ═══════════════════════════════════════════════════════════════════════════════

/// Regex that matches a complete `[{...}]` tag.
static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\{([^}]+)\}\]").expect("tag regex must compile"));

/// Scanner that processes a byte stream and extracts structured tags.
///
/// The scanner maintains an internal buffer so that tags split across
/// multiple `feed` calls are reassembled correctly.
pub struct TagScanner {
    buffer: String,
}

impl TagScanner {
    /// Create a new scanner with an empty buffer.
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feed bytes into the scanner. Returns extracted tags and non-tag text chunks.
    ///
    /// Bytes that cannot be decoded as UTF-8 are replaced with U+FFFD. Partial
    /// tags at the end of the input are buffered for the next `feed` call.
    pub fn feed(&mut self, data: &[u8]) -> ScanResult {
        self.buffer.push_str(&String::from_utf8_lossy(data));

        let mut result = ScanResult::default();
        let mut search_start = 0;
        let buf = self.buffer.clone();

        for m in TAG_RE.find_iter(&buf) {
            // Text chunk before this tag.
            if m.start() > search_start {
                let text = &buf[search_start..m.start()];
                if !text.is_empty() {
                    result.text.push(text.to_owned());
                }
            }

            let raw = m.as_str().to_owned();
            // Inner payload: strip `[{` and `}]`.
            let inner = &raw[2..raw.len() - 2];
            let tag = parse_inner(inner);

            result.tags.push(ParsedTag {
                tag_type: tag,
                raw: raw.clone(),
                payload: inner.to_owned(),
            });

            search_start = m.end();
        }

        // Check if there is a partial tag at the end (starts with `[{` but no `}]`).
        let remaining = &buf[search_start..];
        if let Some(open_pos) = remaining.rfind("[{") {
            // Everything before the potential partial tag is plain text.
            let before = &remaining[..open_pos];
            if !before.is_empty() {
                result.text.push(before.to_owned());
            }
            // Keep the partial tag in the buffer for next feed.
            self.buffer = remaining[open_pos..].to_owned();
        } else {
            // No partial tag — emit remaining text and clear buffer.
            if !remaining.is_empty() {
                result.text.push(remaining.to_owned());
            }
            self.buffer.clear();
        }

        result
    }

    /// Flush any remaining buffered text. Call when the stream ends.
    pub fn flush(&mut self) -> ScanResult {
        let mut result = ScanResult::default();
        if !self.buffer.is_empty() {
            result.text.push(std::mem::take(&mut self.buffer));
        }
        result
    }

    /// Convert a parsed tag into a wire [`TokenFrame`].
    ///
    /// Mapping:
    /// - `Task` → `AgentMessage` (0x16) with JSON payload
    /// - `Status` → `PtyStatus` (0x7A)
    /// - `AgentMessage` → `AgentMessage` (0x16)
    /// - `Broadcast` → `AgentMessage` (0x16)
    /// - `Gate` → `CoordinationCheckpoint` (0x17)
    /// - `Metric` → `TelemetryMetrics` (0xF4)
    /// - `Unknown` → `PtyData` (0x78)
    pub fn tag_to_frame(tag: &ParsedTag, stream_id: u32, sequence: u32) -> TokenFrame {
        let (token_type, data) = match &tag.tag_type {
            TagType::Task { id, action } => {
                let payload = format!("{{\"task_id\":\"{id}\",\"action\":\"{action}\"}}");
                (token::id::AGENT_MESSAGE, payload.into_bytes())
            }
            TagType::Status { message } => (token::id::PTY_STATUS, message.as_bytes().to_vec()),
            TagType::AgentMessage { target, message } => {
                let payload = format!("{{\"target\":\"{target}\",\"message\":\"{message}\"}}");
                (token::id::AGENT_MESSAGE, payload.into_bytes())
            }
            TagType::Broadcast { message } => {
                let payload = format!("{{\"broadcast\":\"{message}\"}}");
                (token::id::AGENT_MESSAGE, payload.into_bytes())
            }
            TagType::Gate { result } => {
                let payload = match result {
                    GateResult::Pass => r#"{"gate":"pass"}"#.to_owned(),
                    GateResult::Fail { reason } => {
                        format!("{{\"gate\":\"fail\",\"reason\":\"{reason}\"}}")
                    }
                };
                (token::id::COORDINATION_CHECKPOINT, payload.into_bytes())
            }
            TagType::Metric { name, value } => {
                let payload = format!("{{\"metric\":\"{name}\",\"value\":\"{value}\"}}");
                (token::id::TELEMETRY_METRICS, payload.into_bytes())
            }
            TagType::Unknown { .. } => (token::id::PTY_DATA, tag.raw.as_bytes().to_vec()),
        };

        TokenFrame::with_stream(token_type, stream_id, sequence, data)
    }
}

impl Default for TagScanner {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// INTERNAL PARSING
// ═══════════════════════════════════════════════════════════════════════════════

/// Parse the inner text of a tag (between `[{` and `}]`) into a `TagType`.
fn parse_inner(inner: &str) -> TagType {
    // Directed agent message: starts with `@`
    if let Some(rest) = inner.strip_prefix('@') {
        if let Some((target, message)) = rest.split_once(':') {
            return TagType::AgentMessage {
                target: target.to_owned(),
                message: message.to_owned(),
            };
        }
    }

    // Split on first colon to get the tag keyword.
    let (keyword, rest) = match inner.split_once(':') {
        Some(pair) => pair,
        None => {
            return TagType::Unknown {
                tag: inner.to_owned(),
                payload: String::new(),
            };
        }
    };

    match keyword {
        "task" => {
            // task:ID:action
            if let Some((id, action)) = rest.split_once(':') {
                TagType::Task {
                    id: id.to_owned(),
                    action: action.to_owned(),
                }
            } else {
                TagType::Task {
                    id: rest.to_owned(),
                    action: String::new(),
                }
            }
        }
        "status" => TagType::Status {
            message: rest.to_owned(),
        },
        "broadcast" => TagType::Broadcast {
            message: rest.to_owned(),
        },
        "gate" => {
            // gate:pass  or  gate:fail:reason
            if rest == "pass" {
                TagType::Gate {
                    result: GateResult::Pass,
                }
            } else if let Some(reason) = rest.strip_prefix("fail:") {
                TagType::Gate {
                    result: GateResult::Fail {
                        reason: reason.to_owned(),
                    },
                }
            } else if rest == "fail" {
                TagType::Gate {
                    result: GateResult::Fail {
                        reason: String::new(),
                    },
                }
            } else {
                TagType::Unknown {
                    tag: keyword.to_owned(),
                    payload: rest.to_owned(),
                }
            }
        }
        "metric" => {
            // metric:name:value
            if let Some((name, value)) = rest.split_once(':') {
                TagType::Metric {
                    name: name.to_owned(),
                    value: value.to_owned(),
                }
            } else {
                TagType::Metric {
                    name: rest.to_owned(),
                    value: String::new(),
                }
            }
        }
        _ => TagType::Unknown {
            tag: keyword.to_owned(),
            payload: rest.to_owned(),
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_task_tag() {
        let mut scanner = TagScanner::new();
        let result = scanner.feed(b"[{task:T-123:complete}]");
        assert_eq!(result.tags.len(), 1);
        assert_eq!(
            result.tags[0].tag_type,
            TagType::Task {
                id: "T-123".into(),
                action: "complete".into(),
            }
        );
        assert_eq!(result.tags[0].raw, "[{task:T-123:complete}]");
        assert_eq!(result.tags[0].payload, "task:T-123:complete");
    }

    #[test]
    fn parse_agent_message_tag() {
        let mut scanner = TagScanner::new();
        let result = scanner.feed(b"[{@agent1:hello}]");
        assert_eq!(result.tags.len(), 1);
        assert_eq!(
            result.tags[0].tag_type,
            TagType::AgentMessage {
                target: "agent1".into(),
                message: "hello".into(),
            }
        );
    }

    #[test]
    fn parse_status_tag() {
        let mut scanner = TagScanner::new();
        let result = scanner.feed(b"[{status:building}]");
        assert_eq!(result.tags.len(), 1);
        assert_eq!(
            result.tags[0].tag_type,
            TagType::Status {
                message: "building".into(),
            }
        );
    }

    #[test]
    fn parse_broadcast_tag() {
        let mut scanner = TagScanner::new();
        let result = scanner.feed(b"[{broadcast:announcement}]");
        assert_eq!(result.tags.len(), 1);
        assert_eq!(
            result.tags[0].tag_type,
            TagType::Broadcast {
                message: "announcement".into(),
            }
        );
    }

    #[test]
    fn parse_gate_pass() {
        let mut scanner = TagScanner::new();
        let result = scanner.feed(b"[{gate:pass}]");
        assert_eq!(result.tags.len(), 1);
        assert_eq!(
            result.tags[0].tag_type,
            TagType::Gate {
                result: GateResult::Pass,
            }
        );
    }

    #[test]
    fn parse_gate_fail_with_reason() {
        let mut scanner = TagScanner::new();
        let result = scanner.feed(b"[{gate:fail:reason text}]");
        assert_eq!(result.tags.len(), 1);
        assert_eq!(
            result.tags[0].tag_type,
            TagType::Gate {
                result: GateResult::Fail {
                    reason: "reason text".into(),
                },
            }
        );
    }

    #[test]
    fn parse_metric_tag() {
        let mut scanner = TagScanner::new();
        let result = scanner.feed(b"[{metric:build_time:42.5}]");
        assert_eq!(result.tags.len(), 1);
        assert_eq!(
            result.tags[0].tag_type,
            TagType::Metric {
                name: "build_time".into(),
                value: "42.5".into(),
            }
        );
    }

    #[test]
    fn mixed_content_produces_tags_and_text() {
        let mut scanner = TagScanner::new();
        let result = scanner.feed(b"building...[{status:done}] finished");
        assert_eq!(result.tags.len(), 1);
        assert_eq!(
            result.tags[0].tag_type,
            TagType::Status {
                message: "done".into(),
            }
        );
        assert_eq!(result.text.len(), 2);
        assert_eq!(result.text[0], "building...");
        assert_eq!(result.text[1], " finished");
    }

    #[test]
    fn multiple_tags_in_single_feed() {
        let mut scanner = TagScanner::new();
        let result =
            scanner.feed(b"[{status:start}] working [{task:T-1:run}] done [{gate:pass}]");
        assert_eq!(result.tags.len(), 3);
        assert_eq!(
            result.tags[0].tag_type,
            TagType::Status {
                message: "start".into(),
            }
        );
        assert_eq!(
            result.tags[1].tag_type,
            TagType::Task {
                id: "T-1".into(),
                action: "run".into(),
            }
        );
        assert_eq!(
            result.tags[2].tag_type,
            TagType::Gate {
                result: GateResult::Pass,
            }
        );
        // Text chunks: " working ", " done "
        assert_eq!(result.text.len(), 2);
        assert_eq!(result.text[0], " working ");
        assert_eq!(result.text[1], " done ");
    }

    #[test]
    fn partial_tag_buffered_across_feeds() {
        let mut scanner = TagScanner::new();

        // First feed contains start of a tag.
        let r1 = scanner.feed(b"prefix [{status:");
        assert!(r1.tags.is_empty());
        assert_eq!(r1.text.len(), 1);
        assert_eq!(r1.text[0], "prefix ");

        // Second feed completes the tag.
        let r2 = scanner.feed(b"done}] suffix");
        assert_eq!(r2.tags.len(), 1);
        assert_eq!(
            r2.tags[0].tag_type,
            TagType::Status {
                message: "done".into(),
            }
        );
        assert_eq!(r2.text.len(), 1);
        assert_eq!(r2.text[0], " suffix");
    }

    #[test]
    fn tag_to_frame_task() {
        let tag = ParsedTag {
            tag_type: TagType::Task {
                id: "T-42".into(),
                action: "complete".into(),
            },
            raw: "[{task:T-42:complete}]".into(),
            payload: "task:T-42:complete".into(),
        };
        let frame = TagScanner::tag_to_frame(&tag, 7, 99);
        assert_eq!(frame.token_type, token::id::AGENT_MESSAGE);
        assert_eq!(frame.stream_id, 7);
        assert_eq!(frame.sequence, 99);
        let text = frame.as_str().unwrap();
        assert!(text.contains("T-42"));
        assert!(text.contains("complete"));
    }

    #[test]
    fn tag_to_frame_status() {
        let tag = ParsedTag {
            tag_type: TagType::Status {
                message: "building".into(),
            },
            raw: "[{status:building}]".into(),
            payload: "status:building".into(),
        };
        let frame = TagScanner::tag_to_frame(&tag, 1, 0);
        assert_eq!(frame.token_type, token::id::PTY_STATUS);
        assert_eq!(frame.as_str(), Some("building"));
    }

    #[test]
    fn tag_to_frame_gate_pass() {
        let tag = ParsedTag {
            tag_type: TagType::Gate {
                result: GateResult::Pass,
            },
            raw: "[{gate:pass}]".into(),
            payload: "gate:pass".into(),
        };
        let frame = TagScanner::tag_to_frame(&tag, 0, 1);
        assert_eq!(frame.token_type, token::id::COORDINATION_CHECKPOINT);
        let text = frame.as_str().unwrap();
        assert!(text.contains("pass"));
    }

    #[test]
    fn tag_to_frame_metric() {
        let tag = ParsedTag {
            tag_type: TagType::Metric {
                name: "build_time".into(),
                value: "42.5".into(),
            },
            raw: "[{metric:build_time:42.5}]".into(),
            payload: "metric:build_time:42.5".into(),
        };
        let frame = TagScanner::tag_to_frame(&tag, 0, 0);
        assert_eq!(frame.token_type, token::id::TELEMETRY_METRICS);
        let text = frame.as_str().unwrap();
        assert!(text.contains("build_time"));
        assert!(text.contains("42.5"));
    }

    #[test]
    fn unknown_tag_preserved() {
        let mut scanner = TagScanner::new();
        let result = scanner.feed(b"[{custom:data here}]");
        assert_eq!(result.tags.len(), 1);
        assert_eq!(
            result.tags[0].tag_type,
            TagType::Unknown {
                tag: "custom".into(),
                payload: "data here".into(),
            }
        );
    }

    #[test]
    fn flush_emits_remaining_buffer() {
        let mut scanner = TagScanner::new();
        let _ = scanner.feed(b"hello [{partial");
        let flushed = scanner.flush();
        assert!(flushed.tags.is_empty());
        assert_eq!(flushed.text.len(), 1);
        assert_eq!(flushed.text[0], "[{partial");
    }

    #[test]
    fn empty_feed_returns_empty() {
        let mut scanner = TagScanner::new();
        let result = scanner.feed(b"");
        assert!(result.tags.is_empty());
        assert!(result.text.is_empty());
    }
}
