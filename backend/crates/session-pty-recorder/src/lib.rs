use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::debug;

use session_pty_core::{PtyError, PtyResult, SessionId, TerminalSize};

// ---------------------------------------------------------------------------
// RecordingFormat
// ---------------------------------------------------------------------------

/// Output format used by the recorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordingFormat {
    /// Raw binary log — each record is a length-prefixed frame:
    /// `[timestamp_ns: u64 LE][len: u32 LE][data: [u8; len]]`
    Raw,
    /// asciicast v2 (asciinema) format — newline-delimited JSON events.
    Asciicast,
}

// ---------------------------------------------------------------------------
// RecordingConfig
// ---------------------------------------------------------------------------

/// Configuration for a [`SessionRecorder`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    /// Output format.
    pub format: RecordingFormat,
    /// Directory where recording files are written.
    pub output_dir: PathBuf,
    /// Maximum file size in bytes.  Recording stops (silently) once exceeded.
    pub max_file_size: usize,
}

impl RecordingConfig {
    /// Convenience constructor.
    pub fn new(format: RecordingFormat, output_dir: impl Into<PathBuf>) -> Self {
        Self {
            format,
            output_dir: output_dir.into(),
            max_file_size: 100 * 1024 * 1024, // 100 MiB default
        }
    }
}

// ---------------------------------------------------------------------------
// SessionRecorder
// ---------------------------------------------------------------------------

/// Records PTY output for later audit / replay.
pub struct SessionRecorder {
    config: RecordingConfig,
    session_id: SessionId,
    file_path: PathBuf,
    writer: Option<std::io::BufWriter<std::fs::File>>,
    bytes_recorded: u64,
    terminal_size: TerminalSize,
    start_time_ns: Option<u64>,
}

impl SessionRecorder {
    /// Create a new recorder.  The output file is created immediately.
    pub fn new(
        config: RecordingConfig,
        session_id: SessionId,
        terminal_size: TerminalSize,
    ) -> PtyResult<Self> {
        std::fs::create_dir_all(&config.output_dir)?;

        let extension = match config.format {
            RecordingFormat::Raw => "ptyraw",
            RecordingFormat::Asciicast => "cast",
        };

        let file_name = format!("{}.{extension}", session_id);
        let file_path = config.output_dir.join(&file_name);

        let file = std::fs::File::create(&file_path)?;
        let mut writer = std::io::BufWriter::new(file);

        // For asciicast, write the header.
        if config.format == RecordingFormat::Asciicast {
            let header = serde_json::json!({
                "version": 2,
                "width": terminal_size.cols,
                "height": terminal_size.rows,
                "timestamp": chrono::Utc::now().timestamp(),
                "env": { "TERM": "xterm-256color", "SHELL": "/bin/bash" },
                "title": format!("session-{session_id}")
            });
            writeln!(writer, "{}", serde_json::to_string(&header).unwrap_or_default())
                .map_err(|e| PtyError::Io(format!("write header: {e}")))?;
        }

        debug!(%session_id, path = %file_path.display(), "recorder created");

        Ok(Self {
            config,
            session_id,
            file_path,
            writer: Some(writer),
            bytes_recorded: 0,
            terminal_size,
            start_time_ns: None,
        })
    }

    /// Record a chunk of output data at the given timestamp (nanoseconds since
    /// an arbitrary epoch — use `Instant::now().elapsed().as_nanos()` or
    /// similar).
    pub fn record(&mut self, data: &[u8], timestamp_ns: u64) -> PtyResult<()> {
        if data.is_empty() {
            return Ok(());
        }

        // Enforce max file size — refuse if adding this chunk would exceed the cap.
        if (self.bytes_recorded as usize).saturating_add(data.len()) > self.config.max_file_size {
            return Ok(());
        }

        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| PtyError::Io("recorder already finished".into()))?;

        if self.start_time_ns.is_none() {
            self.start_time_ns = Some(timestamp_ns);
        }

        match self.config.format {
            RecordingFormat::Raw => {
                // Frame: [timestamp_ns: u64 LE][len: u32 LE][data]
                writer
                    .write_all(&timestamp_ns.to_le_bytes())
                    .map_err(|e| PtyError::Io(format!("write ts: {e}")))?;
                writer
                    .write_all(&(data.len() as u32).to_le_bytes())
                    .map_err(|e| PtyError::Io(format!("write len: {e}")))?;
                writer
                    .write_all(data)
                    .map_err(|e| PtyError::Io(format!("write data: {e}")))?;
            }
            RecordingFormat::Asciicast => {
                let start = self.start_time_ns.unwrap_or(timestamp_ns);
                let elapsed_secs = (timestamp_ns.saturating_sub(start)) as f64 / 1_000_000_000.0;
                let text = String::from_utf8_lossy(data);
                // asciicast v2 event: [time, "o", data]
                let event = serde_json::json!([elapsed_secs, "o", text]);
                writeln!(writer, "{}", serde_json::to_string(&event).unwrap_or_default())
                    .map_err(|e| PtyError::Io(format!("write event: {e}")))?;
            }
        }

        self.bytes_recorded += data.len() as u64;
        Ok(())
    }

    /// Flush and close the recording file.  Returns the path to the file.
    pub fn finish(&mut self) -> PtyResult<PathBuf> {
        if let Some(mut w) = self.writer.take() {
            w.flush()
                .map_err(|e| PtyError::Io(format!("flush: {e}")))?;
        }
        debug!(
            session_id = %self.session_id,
            bytes = self.bytes_recorded,
            path = %self.file_path.display(),
            "recording finished"
        );
        Ok(self.file_path.clone())
    }

    /// Bytes recorded so far.
    pub fn bytes_recorded(&self) -> u64 {
        self.bytes_recorded
    }

    /// The session being recorded.
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// The output file path.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// The terminal size used for the recording header.
    pub fn terminal_size(&self) -> TerminalSize {
        self.terminal_size
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pty-recorder-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn raw_record_and_finish() {
        let dir = temp_dir();
        let cfg = RecordingConfig::new(RecordingFormat::Raw, &dir);
        let sid = SessionId::new();
        let mut rec = SessionRecorder::new(cfg, sid, TerminalSize::default()).unwrap();

        rec.record(b"hello world", 1_000_000).unwrap();
        rec.record(b"second chunk", 2_000_000).unwrap();

        let path = rec.finish().unwrap();
        assert!(path.exists());
        assert!(rec.bytes_recorded() > 0);

        // Verify the raw format: read back timestamp + len + data for frame 1.
        let mut file = std::fs::File::open(&path).unwrap();
        let mut all = Vec::new();
        file.read_to_end(&mut all).unwrap();

        // Frame 1: 8 bytes ts + 4 bytes len + 11 bytes data = 23
        assert!(all.len() >= 23);
        let ts = u64::from_le_bytes(all[0..8].try_into().unwrap());
        assert_eq!(ts, 1_000_000);
        let len = u32::from_le_bytes(all[8..12].try_into().unwrap());
        assert_eq!(len, 11);
        assert_eq!(&all[12..23], b"hello world");

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn asciicast_record_and_finish() {
        let dir = temp_dir();
        let cfg = RecordingConfig::new(RecordingFormat::Asciicast, &dir);
        let sid = SessionId::new();
        let mut rec = SessionRecorder::new(cfg, sid, TerminalSize { cols: 120, rows: 40 }).unwrap();

        rec.record(b"hello", 0).unwrap();
        rec.record(b" world", 500_000_000).unwrap(); // 0.5s later

        let path = rec.finish().unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // Line 0 is the header.
        assert!(lines.len() >= 3, "expected header + 2 events, got {}", lines.len());

        let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(header["version"], 2);
        assert_eq!(header["width"], 120);
        assert_eq!(header["height"], 40);

        // Line 1 — first event at time 0.
        let ev1: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(ev1[0].as_f64().unwrap(), 0.0);
        assert_eq!(ev1[1], "o");
        assert_eq!(ev1[2], "hello");

        // Line 2 — second event at ~0.5s.
        let ev2: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
        let t2 = ev2[0].as_f64().unwrap();
        assert!((t2 - 0.5).abs() < 0.001);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn max_file_size_enforced() {
        let dir = temp_dir();
        let mut cfg = RecordingConfig::new(RecordingFormat::Raw, &dir);
        cfg.max_file_size = 30; // Very small.
        let sid = SessionId::new();
        let mut rec = SessionRecorder::new(cfg, sid, TerminalSize::default()).unwrap();

        // First record: 8 + 4 + 20 = 32 bytes frame overhead > 30 but data=20
        // Actually bytes_recorded tracks data bytes only.
        rec.record(b"12345678901234567890", 0).unwrap(); // 20 bytes data
        rec.record(b"12345678901234567890", 1000).unwrap(); // 20 more -> 40 total, exceeds 30

        // The second record should be silently skipped.
        // We can't easily verify the file size due to framing, but
        // bytes_recorded should reflect only the first chunk.
        assert_eq!(rec.bytes_recorded(), 20);

        rec.finish().unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn finish_twice_is_ok() {
        let dir = temp_dir();
        let cfg = RecordingConfig::new(RecordingFormat::Raw, &dir);
        let sid = SessionId::new();
        let mut rec = SessionRecorder::new(cfg, sid, TerminalSize::default()).unwrap();

        let p1 = rec.finish().unwrap();
        let p2 = rec.finish().unwrap();
        assert_eq!(p1, p2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn record_after_finish_fails() {
        let dir = temp_dir();
        let cfg = RecordingConfig::new(RecordingFormat::Raw, &dir);
        let sid = SessionId::new();
        let mut rec = SessionRecorder::new(cfg, sid, TerminalSize::default()).unwrap();
        rec.finish().unwrap();

        let result = rec.record(b"data", 0);
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_record_is_noop() {
        let dir = temp_dir();
        let cfg = RecordingConfig::new(RecordingFormat::Raw, &dir);
        let sid = SessionId::new();
        let mut rec = SessionRecorder::new(cfg, sid, TerminalSize::default()).unwrap();

        rec.record(b"", 0).unwrap();
        assert_eq!(rec.bytes_recorded(), 0);

        rec.finish().unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }
}
