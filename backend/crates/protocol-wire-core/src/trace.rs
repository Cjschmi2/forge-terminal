//! Distributed trace context propagation across wire connections.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trace context for distributed tracing across agent boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub baggage: Vec<(String, String)>,
}

impl TraceContext {
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            span_id: Uuid::new_v4().to_string()[..16].to_string(),
            parent_span_id: None,
            baggage: Vec::new(),
        }
    }

    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: Uuid::new_v4().to_string()[..16].to_string(),
            parent_span_id: Some(self.span_id.clone()),
            baggage: self.baggage.clone(),
        }
    }

    pub fn with_baggage(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.baggage.push((key.into(), value.into()));
        self
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Timing information for a traced span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanTiming {
    pub span_id: String,
    pub name: String,
    pub start_ns: u64,
    pub end_ns: Option<u64>,
}

impl SpanTiming {
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            span_id: Uuid::new_v4().to_string()[..16].to_string(),
            name: name.into(),
            start_ns: crate::current_timestamp_ns(),
            end_ns: None,
        }
    }

    pub fn finish(&mut self) {
        self.end_ns = Some(crate::current_timestamp_ns());
    }

    pub fn duration_ns(&self) -> Option<u64> {
        self.end_ns.map(|end| end.saturating_sub(self.start_ns))
    }
}
