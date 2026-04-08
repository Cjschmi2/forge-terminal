//! Stream management for multiplexed wire connections.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A stream identifier within a wire connection.
pub type StreamId = u32;

/// Information about an active stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub id: StreamId,
    pub last_sequence: u32,
    pub frame_count: u64,
    pub byte_count: u64,
    pub created_at_ns: u64,
    pub last_activity_ns: u64,
    pub is_closed: bool,
}

impl StreamInfo {
    pub fn new(id: StreamId, now_ns: u64) -> Self {
        Self {
            id,
            last_sequence: 0,
            frame_count: 0,
            byte_count: 0,
            created_at_ns: now_ns,
            last_activity_ns: now_ns,
            is_closed: false,
        }
    }

    pub fn record_frame(&mut self, sequence: u32, data_len: usize, now_ns: u64) {
        self.last_sequence = sequence;
        self.frame_count += 1;
        self.byte_count += data_len as u64;
        self.last_activity_ns = now_ns;
    }

    pub fn close(&mut self) {
        self.is_closed = true;
    }
}

/// Manages multiple concurrent streams within a single wire connection.
pub struct StreamManager {
    streams: HashMap<StreamId, StreamInfo>,
    next_id: StreamId,
}

impl StreamManager {
    pub fn new() -> Self {
        Self { streams: HashMap::new(), next_id: 1 }
    }

    /// Allocate a new stream ID and register it.
    pub fn open(&mut self) -> StreamId {
        let id = self.next_id;
        self.next_id += 1;
        let now = crate::current_timestamp_ns();
        self.streams.insert(id, StreamInfo::new(id, now));
        id
    }

    /// Record a frame on a stream. Creates the stream if it doesn't exist.
    pub fn record(&mut self, stream_id: StreamId, sequence: u32, data_len: usize) {
        let now = crate::current_timestamp_ns();
        self.streams
            .entry(stream_id)
            .or_insert_with(|| StreamInfo::new(stream_id, now))
            .record_frame(sequence, data_len, now);
    }

    /// Close a stream.
    pub fn close(&mut self, stream_id: StreamId) {
        if let Some(info) = self.streams.get_mut(&stream_id) {
            info.close();
        }
    }

    /// Get info about a stream.
    pub fn get(&self, stream_id: StreamId) -> Option<&StreamInfo> {
        self.streams.get(&stream_id)
    }

    /// List all active (non-closed) streams.
    pub fn active_streams(&self) -> Vec<&StreamInfo> {
        self.streams.values().filter(|s| !s.is_closed).collect()
    }

    /// Total number of tracked streams (including closed).
    pub fn total_streams(&self) -> usize {
        self.streams.len()
    }

    /// Remove closed streams older than the given threshold.
    pub fn gc(&mut self, max_age_ns: u64) {
        let now = crate::current_timestamp_ns();
        self.streams
            .retain(|_, info| !info.is_closed || (now - info.last_activity_ns) < max_age_ns);
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}
