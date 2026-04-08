use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// SessionId
// ---------------------------------------------------------------------------

/// Unique identifier for a PTY session, wrapping a UUID v4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct SessionId(pub Uuid);

impl SessionId {
    /// Create a new random session identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// TerminalSize
// ---------------------------------------------------------------------------

/// Terminal dimensions in columns and rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
        }
    }
}

// ---------------------------------------------------------------------------
// PtySessionState
// ---------------------------------------------------------------------------

/// Lifecycle state of a PTY session.
///
/// Renamed from `SessionState` to `PtySessionState` to resolve the naming
/// collision with `session_contracts::SessionState` (provider-level metadata).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PtySessionState {
    /// Session has been allocated but not yet spawned.
    Created,
    /// Process is running inside the PTY.
    Running,
    /// Process exited normally with the given exit code.
    Exited(i32),
    /// Process was killed by the session manager.
    Killed,
    /// An unrecoverable error occurred.
    Error,
}

impl PtySessionState {
    /// Returns `true` when the session is in the `Running` state.
    pub fn is_running(&self) -> bool {
        matches!(self, PtySessionState::Running)
    }
}

// ---------------------------------------------------------------------------
// SpawnConfig
// ---------------------------------------------------------------------------

/// Configuration used to spawn a new process inside a PTY.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnConfig {
    /// The command (executable) to run.
    pub command: String,
    /// Arguments passed to the command.
    pub args: Vec<String>,
    /// Working directory for the child process.
    pub working_dir: PathBuf,
    /// Extra environment variables injected into the child.
    pub env: HashMap<String, String>,
}

impl SpawnConfig {
    /// Convenience constructor.
    pub fn new(command: impl Into<String>, working_dir: impl Into<PathBuf>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            working_dir: working_dir.into(),
            env: HashMap::new(),
        }
    }

    /// Builder: add a single argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Builder: add multiple arguments.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Builder: set an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

// ---------------------------------------------------------------------------
// SessionStats
// ---------------------------------------------------------------------------

/// Runtime statistics for a PTY session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub id: SessionId,
    pub state: PtySessionState,
    pub pid: Option<u32>,
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub size: TerminalSize,
}

// ---------------------------------------------------------------------------
// PtyError
// ---------------------------------------------------------------------------

/// Errors that can occur during PTY operations.
#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("I/O error: {0}")]
    Io(String),

    #[error("session is not running")]
    SessionNotRunning,

    #[error("session has already been spawned")]
    AlreadySpawned,

    #[error("missing file descriptor")]
    MissingFd,

    #[error("working directory rejected: {0}")]
    WorkingDirectoryRejected(String),
}

impl From<std::io::Error> for PtyError {
    fn from(err: std::io::Error) -> Self {
        PtyError::Io(err.to_string())
    }
}

/// Convenience alias for PTY operation results.
pub type PtyResult<T> = Result<T, PtyError>;

// ---------------------------------------------------------------------------
// Environment variable filtering (F-S7-006, B-secret-leak-prevention)
// ---------------------------------------------------------------------------

/// Patterns that indicate a secret or credential. Any environment variable whose
/// upper-cased name contains one of these substrings is stripped before the child
/// process is spawned — unless it appears in [`ALLOWED_OVERRIDE`].
///
/// @session-dev @env-var-filtering
const FORBIDDEN_PATTERNS: &[&str] = &[
    "PASSWORD",
    "TOKEN",
    "SECRET",
    "KEY",
    "CREDENTIAL",
    "AUTH",
    "PRIVATE",
    "CERT",
    "API_KEY",
    "ACCESS_KEY",
];

/// Environment variables that match a forbidden pattern but are explicitly allowed
/// because they carry non-secret operational data reviewed by the security team.
///
/// @session-dev @env-var-filtering
const ALLOWED_OVERRIDE: &[&str] = &[
    "AGENT_CALLSIGN",   // Agent identity — required for mesh routing
    "MOTHERDUCK_TOKEN",  // DuckDB access — reviewed exception
];

/// Filter environment variables, removing any that match one of the
/// [`FORBIDDEN_PATTERNS`] unless they are explicitly listed in
/// [`ALLOWED_OVERRIDE`]. The comparison is case-insensitive on the
/// variable name.
///
/// # Examples
///
/// ```
/// use session_pty_core::filter_env;
///
/// let env = vec![
///     ("PATH".into(), "/usr/bin".into()),
///     ("SECRET_KEY".into(), "abc".into()),
///     ("AGENT_CALLSIGN".into(), "ARCHITECT".into()),
/// ];
///
/// let filtered = filter_env(&env);
/// assert_eq!(filtered.len(), 2); // PATH + AGENT_CALLSIGN
/// ```
pub fn filter_env(env: &[(String, String)]) -> Vec<(String, String)> {
    env.iter()
        .filter(|(k, _)| {
            let upper = k.to_uppercase();
            // Always allow explicitly overridden names.
            if ALLOWED_OVERRIDE.iter().any(|a| upper == *a) {
                return true;
            }
            // Reject any name that contains a forbidden pattern.
            !FORBIDDEN_PATTERNS.iter().any(|p| upper.contains(p))
        })
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// Working directory allowlist (F-S7-006, B-secret-leak-prevention)
// ---------------------------------------------------------------------------

/// Default directory prefixes that are considered safe for session working
/// directories. Can be overridden at runtime via the `ALLOWED_WORKING_DIRS`
/// environment variable (comma-separated list of absolute paths).
///
/// @session-dev @working-dir-allowlist
const DEFAULT_ALLOWED_DIR_PREFIXES: &[&str] = &["/home", "/tmp", "/var/tmp"];

/// Validate that the given working directory path falls under one of the
/// allowed prefixes.  Returns `Ok(())` when the path is permitted, or a
/// `PtyError::WorkingDirectoryRejected` when it is not.
///
/// The allowlist is loaded from the `ALLOWED_WORKING_DIRS` environment
/// variable (comma-separated absolute paths).  When the variable is unset
/// or empty, [`DEFAULT_ALLOWED_DIR_PREFIXES`] is used.
///
/// @session-dev @working-dir-allowlist
pub fn validate_working_directory(dir: &std::path::Path) -> PtyResult<()> {
    let allowed = match std::env::var("ALLOWED_WORKING_DIRS") {
        Ok(val) if !val.is_empty() => val
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>(),
        _ => DEFAULT_ALLOWED_DIR_PREFIXES
            .iter()
            .map(|s| s.to_string())
            .collect(),
    };

    for prefix in &allowed {
        if dir.starts_with(prefix) {
            return Ok(());
        }
    }

    Err(PtyError::WorkingDirectoryRejected(format!(
        "'{}' is not under any allowed prefix: [{}]",
        dir.display(),
        allowed.join(", ")
    )))
}

// ---------------------------------------------------------------------------
// PtySession trait
// ---------------------------------------------------------------------------

/// Core trait implemented by every PTY backend.
#[async_trait]
pub trait PtySession: Send + Sync {
    /// Spawn a child process inside the PTY.
    async fn spawn(&mut self, config: &SpawnConfig) -> PtyResult<()>;

    /// Write `data` to the PTY master side. Returns the number of bytes written.
    async fn write(&mut self, data: &[u8]) -> PtyResult<usize>;

    /// Read from the PTY master side into `buf`. Returns the number of bytes read.
    async fn read(&mut self, buf: &mut [u8]) -> PtyResult<usize>;

    /// Resize the terminal to the given dimensions.
    async fn resize(&mut self, cols: u16, rows: u16) -> PtyResult<()>;

    /// Kill the child process.
    async fn kill(&mut self) -> PtyResult<()>;

    /// Return the PID of the child process, if running.
    fn pid(&self) -> Option<u32>;

    /// Return the session identifier.
    fn id(&self) -> &SessionId;

    /// Return the current lifecycle state.
    fn state(&self) -> PtySessionState;

    /// Return runtime statistics.
    fn stats(&self) -> SessionStats;

    /// Return the current terminal size.
    fn size(&self) -> TerminalSize;

    /// Returns `true` when the session is running. Default implementation
    /// delegates to [`PtySessionState::is_running`].
    fn is_running(&self) -> bool {
        self.state().is_running()
    }

    /// Non-blocking read attempt. Returns `Ok(None)` when no data is available.
    async fn try_read(&mut self, buf: &mut [u8]) -> PtyResult<Option<usize>>;

    /// Block until the child process exits, returning its exit code.
    async fn wait(&mut self) -> PtyResult<i32>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_is_unique() {
        let a = SessionId::new();
        let b = SessionId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn terminal_size_default() {
        let sz = TerminalSize::default();
        assert_eq!(sz.cols, 80);
        assert_eq!(sz.rows, 24);
    }

    #[test]
    fn pty_session_state_is_running() {
        assert!(PtySessionState::Running.is_running());
        assert!(!PtySessionState::Created.is_running());
        assert!(!PtySessionState::Exited(0).is_running());
        assert!(!PtySessionState::Killed.is_running());
        assert!(!PtySessionState::Error.is_running());
    }

    #[test]
    fn spawn_config_builder() {
        let cfg = SpawnConfig::new("bash", "/tmp")
            .arg("-c")
            .arg("echo hello")
            .env("FOO", "bar");

        assert_eq!(cfg.command, "bash");
        assert_eq!(cfg.args, vec!["-c", "echo hello"]);
        assert_eq!(cfg.env.get("FOO").unwrap(), "bar");
    }

    #[test]
    fn pty_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken");
        let pty_err: PtyError = io_err.into();
        assert!(matches!(pty_err, PtyError::Io(_)));
        assert!(pty_err.to_string().contains("broken"));
    }

    #[test]
    fn session_id_display() {
        let id = SessionId::new();
        let s = id.to_string();
        // UUID v4 format: 8-4-4-4-12
        assert_eq!(s.len(), 36);
    }

    #[test]
    fn session_id_serde_roundtrip() {
        let id = SessionId::new();
        let json = serde_json::to_string(&id).unwrap();
        let back: SessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    // -- filter_env tests (@session-dev @env-var-filtering) -------------------

    #[test]
    fn filter_env_removes_secret_key() {
        let env = vec![("SECRET_KEY".into(), "x".into())];
        let filtered = filter_env(&env);
        assert!(filtered.is_empty(), "SECRET_KEY must be filtered out");
    }

    #[test]
    fn filter_env_removes_my_password() {
        let env = vec![("MY_PASSWORD".into(), "x".into())];
        let filtered = filter_env(&env);
        assert!(filtered.is_empty(), "MY_PASSWORD must be filtered out");
    }

    #[test]
    fn filter_env_passes_path() {
        let env = vec![("PATH".into(), "/usr/bin".into())];
        let filtered = filter_env(&env);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "PATH");
    }

    #[test]
    fn filter_env_passes_home() {
        let env = vec![("HOME".into(), "/home/user".into())];
        let filtered = filter_env(&env);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "HOME");
    }

    #[test]
    fn filter_env_allows_agent_callsign() {
        let env = vec![("AGENT_CALLSIGN".into(), "ARCHITECT".into())];
        let filtered = filter_env(&env);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "AGENT_CALLSIGN");
    }

    #[test]
    fn filter_env_allows_motherduck_token() {
        let env = vec![("MOTHERDUCK_TOKEN".into(), "tok_abc".into())];
        let filtered = filter_env(&env);
        assert_eq!(filtered.len(), 1, "MOTHERDUCK_TOKEN is an allowed override");
    }

    #[test]
    fn filter_env_is_case_insensitive() {
        let env = vec![
            ("my_secret_value".into(), "x".into()),
            ("Aws_Access_Key_Id".into(), "y".into()),
        ];
        let filtered = filter_env(&env);
        assert!(filtered.is_empty(), "case-insensitive match must filter both");
    }

    #[test]
    fn filter_env_mixed_bag() {
        let env = vec![
            ("PATH".into(), "/usr/bin".into()),
            ("HOME".into(), "/home/user".into()),
            ("USER".into(), "test".into()),
            ("DB_PASSWORD".into(), "secret".into()),
            ("GITHUB_TOKEN".into(), "ghp_xxx".into()),
            ("AGENT_CALLSIGN".into(), "WORKER".into()),
            ("AWS_SECRET_ACCESS_KEY".into(), "yyy".into()),
            ("TERM".into(), "xterm-256color".into()),
            ("MOTHERDUCK_TOKEN".into(), "md_tok".into()),
        ];
        let filtered = filter_env(&env);
        let names: Vec<&str> = filtered.iter().map(|(k, _)| k.as_str()).collect();
        assert!(names.contains(&"PATH"));
        assert!(names.contains(&"HOME"));
        assert!(names.contains(&"USER"));
        assert!(names.contains(&"AGENT_CALLSIGN"));
        assert!(names.contains(&"TERM"));
        assert!(names.contains(&"MOTHERDUCK_TOKEN"));
        assert!(!names.contains(&"DB_PASSWORD"));
        assert!(!names.contains(&"GITHUB_TOKEN"));
        assert!(!names.contains(&"AWS_SECRET_ACCESS_KEY"));
        assert_eq!(filtered.len(), 6);
    }

    #[test]
    fn filter_env_removes_all_10_forbidden_patterns() {
        let patterns = [
            "MY_PASSWORD", "MY_TOKEN", "MY_SECRET", "MY_KEY",
            "MY_CREDENTIAL", "MY_AUTH", "MY_PRIVATE", "MY_CERT",
            "MY_API_KEY", "MY_ACCESS_KEY",
        ];
        let env: Vec<(String, String)> = patterns
            .iter()
            .map(|p| (p.to_string(), "val".into()))
            .collect();
        let filtered = filter_env(&env);
        assert!(filtered.is_empty(), "all 10 forbidden patterns must be filtered");
    }

    // -- working directory validation tests (@session-dev @working-dir-allowlist)
    //
    // Environment variable mutations are not thread-safe, so all env-dependent
    // working directory tests are combined into a single test function to avoid
    // races with the parallel test runner.

    #[test]
    fn validate_working_directory_allowlist() {
        // SAFETY: test-only env mutation. This is the only test that touches
        // ALLOWED_WORKING_DIRS, so there is no data race.

        // -- Part 1: default allowlist --
        unsafe { std::env::remove_var("ALLOWED_WORKING_DIRS") };

        assert!(
            validate_working_directory(std::path::Path::new("/home/user/project")).is_ok(),
            "/home should be allowed by default"
        );
        assert!(
            validate_working_directory(std::path::Path::new("/tmp/build")).is_ok(),
            "/tmp should be allowed by default"
        );
        assert!(
            validate_working_directory(std::path::Path::new("/var/tmp/scratch")).is_ok(),
            "/var/tmp should be allowed by default"
        );
        assert!(
            matches!(
                validate_working_directory(std::path::Path::new("/etc/shadow")),
                Err(PtyError::WorkingDirectoryRejected(_))
            ),
            "/etc should be rejected by default"
        );
        assert!(
            matches!(
                validate_working_directory(std::path::Path::new("/")),
                Err(PtyError::WorkingDirectoryRejected(_))
            ),
            "/ should be rejected by default"
        );

        // -- Part 2: custom env override --
        unsafe { std::env::set_var("ALLOWED_WORKING_DIRS", "/opt/app, /srv/data") };

        assert!(
            validate_working_directory(std::path::Path::new("/opt/app/sessions")).is_ok(),
            "/opt/app should be allowed via env override"
        );
        assert!(
            validate_working_directory(std::path::Path::new("/srv/data/run")).is_ok(),
            "/srv/data should be allowed via env override"
        );
        assert!(
            matches!(
                validate_working_directory(std::path::Path::new("/home/user")),
                Err(PtyError::WorkingDirectoryRejected(_))
            ),
            "/home should be rejected when env override replaces defaults"
        );

        // Restore.
        unsafe { std::env::remove_var("ALLOWED_WORKING_DIRS") };
    }
}
