//! Wire frame encoding/decoding.
//!
//! Binary frame format: 21-byte header + variable-length data.
//! ```text
//! byte  0:      token_type (u8)
//! bytes 1-4:    stream_id  (u32 LE)
//! bytes 5-8:    sequence   (u32 LE)
//! bytes 9-16:   timestamp  (u64 LE, nanoseconds since epoch)
//! bytes 17-20:  data_len   (u32 LE)
//! bytes 21+:    data       (variable)
//! ```

use crate::token::{TokenCategory, TokenMetadata, TokenType};
use crate::{WireError, current_timestamp_ns};
use serde::{Deserialize, Serialize};

/// Size of the wire frame header in bytes.
pub const HEADER_SIZE: usize = 21;

/// Maximum data payload size (16 MiB).
pub const MAX_DATA_SIZE: usize = 16 * 1024 * 1024;

// ═══════════════════════════════════════════════════════════════════════════════
// OWNED TOKEN FRAME
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenFrame {
    pub token_type: u8,
    pub stream_id: u32,
    pub sequence: u32,
    pub timestamp_ns: u64,
    pub data: Vec<u8>,
}

impl TokenFrame {
    #[must_use]
    pub fn new(token_type: u8, sequence: u32, data: Vec<u8>) -> Self {
        Self { token_type, stream_id: 0, sequence, timestamp_ns: current_timestamp_ns(), data }
    }

    #[must_use]
    pub fn with_stream(token_type: u8, stream_id: u32, sequence: u32, data: Vec<u8>) -> Self {
        Self { token_type, stream_id, sequence, timestamp_ns: current_timestamp_ns(), data }
    }

    #[must_use]
    pub fn with_timestamp(
        token_type: u8,
        stream_id: u32,
        sequence: u32,
        timestamp_ns: u64,
        data: Vec<u8>,
    ) -> Self {
        Self { token_type, stream_id, sequence, timestamp_ns, data }
    }

    #[must_use]
    pub fn from_type(token_type: TokenType, sequence: u32, data: Vec<u8>) -> Self {
        Self::new(token_type.as_u8(), sequence, data)
    }

    #[must_use]
    pub fn content(sequence: u32, text: &str) -> Self {
        Self::new(token::id::CONTENT, sequence, text.as_bytes().to_vec())
    }

    #[must_use]
    pub fn done(sequence: u32) -> Self {
        Self::new(token::id::DONE, sequence, Vec::new())
    }

    #[must_use]
    pub fn error(sequence: u32, message: &str) -> Self {
        Self::new(token::id::ERROR, sequence, message.as_bytes().to_vec())
    }

    /// Try to decode a frame from a byte slice.
    pub fn try_decode(bytes: &[u8]) -> Result<Self, WireError> {
        if bytes.len() < HEADER_SIZE {
            return Err(WireError::FrameTooShort { got: bytes.len(), need: HEADER_SIZE });
        }

        let token_type = bytes[0];
        let stream_id = u32::from_le_bytes(
            bytes[1..5]
                .try_into()
                .map_err(|_| WireError::FrameTooShort { got: bytes.len(), need: HEADER_SIZE })?,
        );
        let sequence = u32::from_le_bytes(
            bytes[5..9]
                .try_into()
                .map_err(|_| WireError::FrameTooShort { got: bytes.len(), need: HEADER_SIZE })?,
        );
        let timestamp_ns = u64::from_le_bytes(
            bytes[9..17]
                .try_into()
                .map_err(|_| WireError::FrameTooShort { got: bytes.len(), need: HEADER_SIZE })?,
        );
        let data_len = u32::from_le_bytes(
            bytes[17..21]
                .try_into()
                .map_err(|_| WireError::FrameTooShort { got: bytes.len(), need: HEADER_SIZE })?,
        ) as usize;

        if data_len > MAX_DATA_SIZE {
            return Err(WireError::FrameTooLarge { size: data_len, limit: MAX_DATA_SIZE });
        }

        if bytes.len() < HEADER_SIZE + data_len {
            return Err(WireError::IncompleteFrame {
                expected: data_len,
                got: bytes.len() - HEADER_SIZE,
            });
        }

        let data = bytes[HEADER_SIZE..HEADER_SIZE + data_len].to_vec();

        Ok(Self { token_type, stream_id, sequence, timestamp_ns, data })
    }

    /// Decode a frame, returning None on failure.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        Self::try_decode(bytes).ok()
    }

    /// Encode this frame into a byte vector.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + self.data.len());
        buf.push(self.token_type);
        buf.extend_from_slice(&self.stream_id.to_le_bytes());
        buf.extend_from_slice(&self.sequence.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buf.extend_from_slice(&(self.data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.data);
        buf
    }

    /// Encode this frame into an existing buffer, returning bytes written.
    pub fn encode_into(&self, buf: &mut Vec<u8>) -> usize {
        let start = buf.len();
        buf.push(self.token_type);
        buf.extend_from_slice(&self.stream_id.to_le_bytes());
        buf.extend_from_slice(&self.sequence.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buf.extend_from_slice(&(self.data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.data);
        buf.len() - start
    }

    #[must_use]
    pub fn token_type_enum(&self) -> TokenType {
        TokenType::from_u8(self.token_type)
    }

    #[must_use]
    pub fn category(&self) -> TokenCategory {
        self.token_type_enum().category()
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }

    #[must_use]
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }

    #[must_use]
    pub fn wire_size(&self) -> usize {
        HEADER_SIZE + self.data.len()
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        TokenMetadata::for_type(self.token_type).is_terminal
    }

    #[must_use]
    pub fn has_text(&self) -> bool {
        TokenMetadata::for_type(self.token_type).has_text
    }

    #[inline]
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.data
    }
}

impl Default for TokenFrame {
    fn default() -> Self {
        Self {
            token_type: token::id::CONTENT,
            stream_id: 0,
            sequence: 0,
            timestamp_ns: 0,
            data: Vec::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ZERO-COPY TOKEN FRAME REFERENCE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy)]
pub struct TokenFrameRef<'a> {
    pub token_type: u8,
    pub stream_id: u32,
    pub sequence: u32,
    pub timestamp_ns: u64,
    pub data: &'a [u8],
}

impl<'a> TokenFrameRef<'a> {
    pub fn try_parse(bytes: &'a [u8]) -> Result<Self, WireError> {
        if bytes.len() < HEADER_SIZE {
            return Err(WireError::FrameTooShort { got: bytes.len(), need: HEADER_SIZE });
        }

        let token_type = bytes[0];
        let stream_id = u32::from_le_bytes(bytes[1..5].try_into().unwrap());
        let sequence = u32::from_le_bytes(bytes[5..9].try_into().unwrap());
        let timestamp_ns = u64::from_le_bytes(bytes[9..17].try_into().unwrap());
        let data_len = u32::from_le_bytes(bytes[17..21].try_into().unwrap()) as usize;

        if data_len > MAX_DATA_SIZE {
            return Err(WireError::FrameTooLarge { size: data_len, limit: MAX_DATA_SIZE });
        }

        if bytes.len() < HEADER_SIZE + data_len {
            return Err(WireError::IncompleteFrame {
                expected: data_len,
                got: bytes.len() - HEADER_SIZE,
            });
        }

        let data = &bytes[HEADER_SIZE..HEADER_SIZE + data_len];

        Ok(Self { token_type, stream_id, sequence, timestamp_ns, data })
    }

    #[must_use]
    pub fn parse(bytes: &'a [u8]) -> Option<Self> {
        Self::try_parse(bytes).ok()
    }

    #[must_use]
    pub fn token_type_enum(&self) -> TokenType {
        TokenType::from_u8(self.token_type)
    }

    #[must_use]
    pub fn category(&self) -> TokenCategory {
        self.token_type_enum().category()
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&'a str> {
        std::str::from_utf8(self.data).ok()
    }

    #[must_use]
    pub fn wire_size(&self) -> usize {
        HEADER_SIZE + self.data.len()
    }

    #[must_use]
    pub fn to_owned(&self) -> TokenFrame {
        TokenFrame {
            token_type: self.token_type,
            stream_id: self.stream_id,
            sequence: self.sequence,
            timestamp_ns: self.timestamp_ns,
            data: self.data.to_vec(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FRAME ITERATOR
// ═══════════════════════════════════════════════════════════════════════════════

pub struct FrameIterator<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> FrameIterator<'a> {
    #[must_use]
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, offset: 0 }
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.offset)
    }
}

impl<'a> Iterator for FrameIterator<'a> {
    type Item = TokenFrameRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.buffer.len() {
            return None;
        }

        let remaining = &self.buffer[self.offset..];
        let frame = TokenFrameRef::parse(remaining)?;
        self.offset += frame.wire_size();
        Some(frame)
    }
}

// Use the token module for id constants
use crate::token;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_round_trip() {
        let frame = TokenFrame::new(0x00, 42, b"hello world".to_vec());
        let encoded = frame.encode();
        let decoded = TokenFrame::try_decode(&encoded).unwrap();
        assert_eq!(frame.token_type, decoded.token_type);
        assert_eq!(frame.stream_id, decoded.stream_id);
        assert_eq!(frame.sequence, decoded.sequence);
        assert_eq!(frame.data, decoded.data);
    }

    #[test]
    fn test_frame_ref_round_trip() {
        let frame = TokenFrame::with_stream(0x16, 7, 99, b"agent msg".to_vec());
        let encoded = frame.encode();
        let frame_ref = TokenFrameRef::try_parse(&encoded).unwrap();
        assert_eq!(frame_ref.token_type, 0x16);
        assert_eq!(frame_ref.stream_id, 7);
        assert_eq!(frame_ref.sequence, 99);
        assert_eq!(frame_ref.as_str(), Some("agent msg"));
    }

    #[test]
    fn test_frame_too_short() {
        let result = TokenFrame::try_decode(&[0u8; 10]);
        assert!(matches!(result, Err(WireError::FrameTooShort { .. })));
    }

    #[test]
    fn test_frame_iterator() {
        let f1 = TokenFrame::new(0x00, 1, b"a".to_vec());
        let f2 = TokenFrame::new(0x04, 2, Vec::new());
        let mut buf = Vec::new();
        f1.encode_into(&mut buf);
        f2.encode_into(&mut buf);

        let frames: Vec<_> = FrameIterator::new(&buf).collect();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].sequence, 1);
        assert_eq!(frames[1].sequence, 2);
    }

    #[test]
    fn test_content_and_done_helpers() {
        let content = TokenFrame::content(1, "hello");
        assert_eq!(content.token_type, 0x00);
        assert_eq!(content.as_str(), Some("hello"));

        let done = TokenFrame::done(2);
        assert_eq!(done.token_type, 0x04);
        assert!(done.data.is_empty());
    }
}
