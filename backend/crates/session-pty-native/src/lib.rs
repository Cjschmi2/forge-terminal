use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use async_trait::async_trait;
use nix::pty::openpty;
use nix::sys::termios::{self, SetArg};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tracing::{debug, warn};

use session_pty_core::{
    PtyError, PtyResult, PtySession, SessionId, PtySessionState, SessionStats, SpawnConfig,
    TerminalSize, filter_env, validate_working_directory,
};

// ---------------------------------------------------------------------------
// Window-size ioctl helper
// ---------------------------------------------------------------------------

#[repr(C)]
struct Winsize {
    ws_row: libc::c_ushort,
    ws_col: libc::c_ushort,
    ws_xpixel: libc::c_ushort,
    ws_ypixel: libc::c_ushort,
}

/// Apply a terminal size to a raw fd via the `TIOCSWINSZ` ioctl.
fn set_window_size_fd(fd: RawFd, cols: u16, rows: u16) -> PtyResult<()> {
    let ws = Winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: ws is a valid Winsize struct and fd is a valid pty fd.
    let ret = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &ws) };
    if ret == -1 {
        return Err(PtyError::Io(format!(
            "TIOCSWINSZ failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// NativePtySession
// ---------------------------------------------------------------------------

/// Native Linux PTY session backed by `openpty`.
pub struct NativePtySession {
    id: SessionId,
    master_fd: Option<OwnedFd>,
    child: Option<Child>,
    size: TerminalSize,
    state: PtySessionState,
    bytes_written: AtomicU64,
    bytes_read: AtomicU64,
    #[allow(dead_code)]
    created_at: Instant,
    async_master: Option<tokio::fs::File>,
}

// The atomic counters and OwnedFd are safe to send/share between threads
// when access is properly synchronised at the caller level (e.g. via Mutex).
unsafe impl Send for NativePtySession {}
unsafe impl Sync for NativePtySession {}

impl NativePtySession {
    /// Create a new session in the `Created` state.
    pub fn new() -> Self {
        Self {
            id: SessionId::new(),
            master_fd: None,
            child: None,
            size: TerminalSize::default(),
            state: PtySessionState::Created,
            bytes_written: AtomicU64::new(0),
            bytes_read: AtomicU64::new(0),
            created_at: Instant::now(),
            async_master: None,
        }
    }

    /// Create a new session with a specific id.
    pub fn with_id(id: SessionId) -> Self {
        Self {
            id,
            master_fd: None,
            child: None,
            size: TerminalSize::default(),
            state: PtySessionState::Created,
            bytes_written: AtomicU64::new(0),
            bytes_read: AtomicU64::new(0),
            created_at: Instant::now(),
            async_master: None,
        }
    }

    // -- internal helpers ---------------------------------------------------

    /// Open a pseudo-terminal pair, configure raw mode and window size.
    fn open_pty(&self) -> PtyResult<(OwnedFd, OwnedFd)> {
        let pty = openpty(None, None).map_err(|e| PtyError::Io(format!("openpty: {e}")))?;
        let master = pty.master;
        let slave = pty.slave;

        // Set window size on the master side.
        set_window_size_fd(master.as_raw_fd(), self.size.cols, self.size.rows)?;

        // NOTE: We intentionally do NOT call cfmakeraw on the slave.
        // The slave starts in cooked mode (default) so the kernel line discipline
        // handles echo, line buffering, and signal generation (Ctrl-C → SIGINT).
        // The child process (bash, claude, etc.) will configure raw mode itself
        // via readline/termios when it needs character-by-character input.
        // This matches how real terminal emulators (kitty, xterm, etc.) work.

        Ok((master, slave))
    }

    /// Wrap the master fd in an async tokio `File` for non-blocking I/O.
    fn create_async_master(fd: RawFd) -> PtyResult<tokio::fs::File> {
        // SAFETY: we duplicate the fd so the OwnedFd keeps ownership of the
        // original while tokio gets its own handle.
        let dup_fd = unsafe { libc::dup(fd) };
        if dup_fd == -1 {
            return Err(PtyError::Io(format!(
                "dup: {}",
                std::io::Error::last_os_error()
            )));
        }
        // Set non-blocking.
        let flags = unsafe { libc::fcntl(dup_fd, libc::F_GETFL) };
        if flags == -1 {
            return Err(PtyError::Io("fcntl F_GETFL failed".into()));
        }
        let ret = unsafe { libc::fcntl(dup_fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        if ret == -1 {
            return Err(PtyError::Io("fcntl F_SETFL O_NONBLOCK failed".into()));
        }
        // SAFETY: dup_fd is a valid, non-blocking file descriptor.
        let std_file = unsafe { std::fs::File::from_raw_fd(dup_fd) };
        Ok(tokio::fs::File::from_std(std_file))
    }
}

impl NativePtySession {
    /// Create an independent async read handle by duplicating the master fd.
    ///
    /// The returned `tokio::fs::File` can be used from a separate task to read
    /// PTY output without holding a lock on the session itself.  The caller
    /// owns the handle and must drop it when done.
    ///
    /// Returns `None` if the session has not been spawned yet (no master fd).
    pub fn dup_read_handle(&self) -> PtyResult<Option<tokio::fs::File>> {
        match &self.master_fd {
            Some(fd) => {
                let handle = Self::create_async_master(fd.as_raw_fd())?;
                Ok(Some(handle))
            }
            None => Ok(None),
        }
    }
}

impl Default for NativePtySession {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// PtySession implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl PtySession for NativePtySession {
    async fn spawn(&mut self, config: &SpawnConfig) -> PtyResult<()> {
        if self.state == PtySessionState::Running {
            return Err(PtyError::AlreadySpawned);
        }

        // Validate working directory against the allowlist before spawning
        // any OS resources (F-S7-006, B-secret-leak-prevention).
        validate_working_directory(&config.working_dir)?;

        let (master, slave) = self.open_pty()?;
        let slave_fd = slave.as_raw_fd();

        // Build the child process.
        let stdin_fd = unsafe { Stdio::from_raw_fd(libc::dup(slave_fd)) };
        let stdout_fd = unsafe { Stdio::from_raw_fd(libc::dup(slave_fd)) };
        let stderr_fd = unsafe { Stdio::from_raw_fd(libc::dup(slave_fd)) };

        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .current_dir(&config.working_dir)
            .stdin(stdin_fd)
            .stdout(stdout_fd)
            .stderr(stderr_fd);

        // Filter environment variables to prevent secret leakage into child
        // processes (F-S7-006, B-secret-leak-prevention). The filter removes
        // variables matching forbidden patterns (PASSWORD, TOKEN, SECRET, etc.)
        // while allowing reviewed exceptions like AGENT_CALLSIGN.
        let env_pairs: Vec<(String, String)> = config
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let filtered = filter_env(&env_pairs);

        for (k, v) in &filtered {
            cmd.env(k, v);
        }

        // Ensure the child knows it is running inside a capable terminal
        // emulator. GUI-launched Tauri apps may not have TERM set at all,
        // causing tools like Claude Code to fall back to dumb/no-color output.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        // Spawn as session leader so it gets its own controlling terminal.
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        let child = cmd
            .spawn()
            .map_err(|e| PtyError::Io(format!("spawn: {e}")))?;

        debug!(
            session_id = %self.id,
            pid = child.id().unwrap_or(0),
            command = %config.command,
            "PTY child spawned"
        );

        // Close slave side in parent — the child has its own copies.
        drop(slave);

        let async_master = Self::create_async_master(master.as_raw_fd())?;

        self.master_fd = Some(master);
        self.child = Some(child);
        self.async_master = Some(async_master);
        self.state = PtySessionState::Running;

        Ok(())
    }

    async fn write(&mut self, data: &[u8]) -> PtyResult<usize> {
        let master = self
            .async_master
            .as_mut()
            .ok_or(PtyError::SessionNotRunning)?;
        let n = master
            .write(data)
            .await
            .map_err(|e| PtyError::Io(format!("write: {e}")))?;
        self.bytes_written.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }

    async fn read(&mut self, buf: &mut [u8]) -> PtyResult<usize> {
        let master = self
            .async_master
            .as_mut()
            .ok_or(PtyError::SessionNotRunning)?;
        let n = master
            .read(buf)
            .await
            .map_err(|e| PtyError::Io(format!("read: {e}")))?;
        self.bytes_read.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }

    async fn resize(&mut self, cols: u16, rows: u16) -> PtyResult<()> {
        let fd = self
            .master_fd
            .as_ref()
            .ok_or(PtyError::MissingFd)?
            .as_raw_fd();
        set_window_size_fd(fd, cols, rows)?;
        self.size = TerminalSize { cols, rows };
        Ok(())
    }

    async fn kill(&mut self) -> PtyResult<()> {
        if let Some(ref mut child) = self.child {
            let _ = child.kill().await;
        }
        self.child.take();
        self.master_fd.take();
        self.async_master.take();
        self.state = PtySessionState::Killed;
        debug!(session_id = %self.id, "session killed");
        Ok(())
    }

    fn pid(&self) -> Option<u32> {
        self.child.as_ref().and_then(|c| c.id())
    }

    fn id(&self) -> &SessionId {
        &self.id
    }

    fn state(&self) -> PtySessionState {
        self.state
    }

    fn stats(&self) -> SessionStats {
        SessionStats {
            id: self.id,
            state: self.state,
            pid: self.pid(),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
            bytes_read: self.bytes_read.load(Ordering::Relaxed),
            size: self.size,
        }
    }

    fn size(&self) -> TerminalSize {
        self.size
    }

    async fn try_read(&mut self, buf: &mut [u8]) -> PtyResult<Option<usize>> {
        let master = self
            .async_master
            .as_mut()
            .ok_or(PtyError::SessionNotRunning)?;
        match tokio::time::timeout(std::time::Duration::from_micros(100), master.read(buf)).await {
            Ok(Ok(n)) => {
                self.bytes_read.fetch_add(n as u64, Ordering::Relaxed);
                Ok(Some(n))
            }
            Ok(Err(e)) => Err(PtyError::Io(format!("try_read: {e}"))),
            Err(_) => Ok(None), // timeout — no data available
        }
    }

    async fn wait(&mut self) -> PtyResult<i32> {
        let child = self.child.as_mut().ok_or(PtyError::SessionNotRunning)?;
        let status = child
            .wait()
            .await
            .map_err(|e| PtyError::Io(format!("wait: {e}")))?;
        let code = status.code().unwrap_or(-1);
        self.state = PtySessionState::Exited(code);
        debug!(session_id = %self.id, exit_code = code, "session exited");
        Ok(code)
    }
}

// ---------------------------------------------------------------------------
// Drop — SIGKILL child if still alive
// ---------------------------------------------------------------------------

impl Drop for NativePtySession {
    fn drop(&mut self) {
        if let Some(ref child) = self.child
            && let Some(pid) = child.id()
        {
            warn!(session_id = %self.id, pid, "sending SIGKILL on drop");
            // SAFETY: pid is a valid process id from the child we spawned.
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGKILL);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_is_created() {
        let s = NativePtySession::new();
        assert_eq!(s.state(), PtySessionState::Created);
        assert!(!s.is_running());
        assert!(s.pid().is_none());
    }

    #[test]
    fn default_terminal_size() {
        let s = NativePtySession::new();
        let sz = s.size();
        assert_eq!(sz.cols, 80);
        assert_eq!(sz.rows, 24);
    }

    #[tokio::test]
    async fn spawn_and_wait_echo() {
        let mut s = NativePtySession::new();
        let cfg = SpawnConfig::new("echo", "/tmp").arg("hello");
        s.spawn(&cfg).await.expect("spawn failed");
        assert!(s.is_running());
        assert!(s.pid().is_some());

        let code = s.wait().await.expect("wait failed");
        assert_eq!(code, 0);
        assert_eq!(s.state(), PtySessionState::Exited(0));
    }

    #[tokio::test]
    async fn spawn_and_kill() {
        let mut s = NativePtySession::new();
        let cfg = SpawnConfig::new("sleep", "/tmp").arg("60");
        s.spawn(&cfg).await.expect("spawn failed");
        assert!(s.is_running());

        s.kill().await.expect("kill failed");
        assert_eq!(s.state(), PtySessionState::Killed);
        assert!(s.pid().is_none());
    }

    #[tokio::test]
    async fn write_read_roundtrip() {
        let mut s = NativePtySession::new();
        let cfg = SpawnConfig::new("cat", "/tmp");
        s.spawn(&cfg).await.expect("spawn failed");

        let written = s.write(b"hello\n").await.expect("write failed");
        assert!(written > 0);

        // Give cat a moment to echo back.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let mut buf = [0u8; 256];
        let n = s.try_read(&mut buf).await.expect("try_read failed");
        assert!(n.is_some());
        let data = std::str::from_utf8(&buf[..n.unwrap()]).unwrap_or("");
        assert!(data.contains("hello"), "expected 'hello' in output, got: {data}");

        s.kill().await.ok();
    }

    #[tokio::test]
    async fn double_spawn_fails() {
        let mut s = NativePtySession::new();
        let cfg = SpawnConfig::new("sleep", "/tmp").arg("60");
        s.spawn(&cfg).await.expect("spawn failed");
        let result = s.spawn(&cfg).await;
        assert!(matches!(result, Err(PtyError::AlreadySpawned)));
        s.kill().await.ok();
    }

    #[tokio::test]
    async fn stats_after_write() {
        let mut s = NativePtySession::new();
        let cfg = SpawnConfig::new("cat", "/tmp");
        s.spawn(&cfg).await.expect("spawn failed");

        s.write(b"abc").await.expect("write failed");
        let stats = s.stats();
        assert!(stats.bytes_written >= 3);

        s.kill().await.ok();
    }

    #[tokio::test]
    async fn resize_works() {
        let mut s = NativePtySession::new();
        let cfg = SpawnConfig::new("sleep", "/tmp").arg("60");
        s.spawn(&cfg).await.expect("spawn failed");

        s.resize(120, 40).await.expect("resize failed");
        let sz = s.size();
        assert_eq!(sz.cols, 120);
        assert_eq!(sz.rows, 40);

        s.kill().await.ok();
    }
}
