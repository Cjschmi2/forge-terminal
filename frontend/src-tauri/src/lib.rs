use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

use session_api::{PtyBridge, PtyLaunchRequest};
use session_pty_router::{CommandType, OutputChunk};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct AppState {
    pub pty: PtyBridge,
    pub output_pumps: RwLock<HashMap<String, tauri::async_runtime::JoinHandle<()>>>,
    pub pty_rt: tokio::runtime::Runtime,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub name: String,
    pub tool: String,
    pub working_directory: String,
    #[serde(default)]
    pub machine: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchResponse {
    pub session_id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub name: String,
    pub command_type: String,
    pub working_dir: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FsListResponse {
    pub cwd: String,
    pub entries: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtyOutputEvent {
    pub session_name: String,
    pub data: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn to_command_type(tool: &str) -> CommandType {
    match tool.to_lowercase().as_str() {
        "claude" => CommandType::Claude,
        "codex" => CommandType::Codex,
        "cursor" => CommandType::Cursor,
        "bash" | "shell" => CommandType::Bash,
        other => CommandType::Custom { command: other.to_string(), args: vec![] },
    }
}

#[tauri::command]
async fn sessions_launch(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    request: LaunchRequest,
) -> Result<LaunchResponse, String> {
    let s = state.inner().clone();
    let name = request.name.clone();
    let tool = request.tool.clone();
    let workdir = request.working_directory.clone();

    let machine = request.machine.clone();

    let session_id = tokio::task::spawn_blocking(move || {
        let is_remote = !machine.is_empty() && machine != "local";

        let (cmd, work_dir) = if is_remote {
            // Look up machine from registry.
            let machines = load_machines();
            let mach = machines.iter().find(|m| m.id == machine);

            let ssh_base = if let Some(m) = mach {
                build_ssh_command(m)
            } else {
                // Fallback: treat machine id as host
                format!("ssh -t {machine}")
            };

            let remote_tool = match tool.as_str() {
                "bash" => "bash -l".to_string(),
                other => other.to_string(),
            };
            let ssh_cmd = format!("{} 'cd {} && {}'", ssh_base, workdir, remote_tool);
            (
                CommandType::Custom {
                    command: "bash".to_string(),
                    args: vec!["-c".to_string(), ssh_cmd],
                },
                PathBuf::from("/tmp"),
            )
        } else {
            (to_command_type(&tool), PathBuf::from(&workdir))
        };

        s.pty_rt
            .block_on(s.pty.launch(PtyLaunchRequest {
                name: name.clone(),
                command_type: cmd,
                working_dir: work_dir,
                project_id: if is_remote { "r5".into() } else { "local".into() },
                requested_by: "operator".into(),
            }))
            .map_err(|e| format!("{e}"))
    })
    .await
    .map_err(|e| format!("{e}"))??;

    let rx = state.pty.subscribe(&request.name).map_err(|e| format!("{e}"))?;

    let pump = spawn_output_pump(app, request.name.clone(), rx);
    state.output_pumps.write().await.insert(request.name.clone(), pump);

    Ok(LaunchResponse { session_id: session_id.to_string(), name: request.name })
}

#[tauri::command]
async fn pty_send(
    state: State<'_, Arc<AppState>>,
    session_name: String,
    input: String,
) -> Result<usize, String> {
    let s = state.inner().clone();
    tokio::task::spawn_blocking(move || {
        s.pty_rt.block_on(s.pty.send(&session_name, input.as_bytes())).map_err(|e| format!("{e}"))
    })
    .await
    .map_err(|e| format!("{e}"))?
}

#[tauri::command]
async fn pty_resize(
    state: State<'_, Arc<AppState>>,
    session_name: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let s = state.inner().clone();
    tokio::task::spawn_blocking(move || {
        s.pty_rt.block_on(s.pty.resize(&session_name, cols, rows)).map_err(|e| format!("{e}"))
    })
    .await
    .map_err(|e| format!("{e}"))?
}

#[tauri::command]
async fn pty_kill(state: State<'_, Arc<AppState>>, session_name: String) -> Result<(), String> {
    if let Some(h) = state.output_pumps.write().await.remove(&session_name) {
        h.abort();
    }
    let s = state.inner().clone();
    tokio::task::spawn_blocking(move || {
        s.pty_rt.block_on(s.pty.kill(&session_name)).map_err(|e| format!("{e}"))
    })
    .await
    .map_err(|e| format!("{e}"))?
}

#[tauri::command]
async fn sessions_list(state: State<'_, Arc<AppState>>) -> Result<Vec<SessionInfo>, String> {
    Ok(state
        .pty
        .list_sessions()
        .into_iter()
        .map(|s| SessionInfo {
            name: s.name,
            command_type: format!("{:?}", s.command_type),
            working_dir: s.working_dir.display().to_string(),
        })
        .collect())
}

#[tauri::command]
async fn filesystem_list(path: String) -> Result<FsListResponse, String> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {path}"));
    }
    let rd = std::fs::read_dir(&dir).map_err(|e| format!("{e}"))?;
    let mut entries = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_str().unwrap_or("?").to_owned();
        let meta = e.metadata().ok();
        entries.push(FileEntry {
            name,
            path: e.path().display().to_string(),
            is_dir: meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
        });
    }
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(FsListResponse { cwd: path, entries })
}

#[tauri::command]
async fn filesystem_read(path: String, max_bytes: Option<usize>) -> Result<String, String> {
    let max = max_bytes.unwrap_or(65536);
    let bytes = std::fs::read(&path).map_err(|e| format!("{e}"))?;
    Ok(if bytes.len() > max {
        format!("{}\n--- truncated at {} bytes ---", String::from_utf8_lossy(&bytes[..max]), max)
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

#[tauri::command]
async fn filesystem_cwd(
    state: State<'_, Arc<AppState>>,
    session_name: String,
) -> Result<String, String> {
    let sessions = state.pty.list_sessions();
    let session = sessions
        .iter()
        .find(|s| s.name == session_name)
        .ok_or_else(|| format!("Session not found: {session_name}"))?;
    Ok(session.working_dir.display().to_string())
}

// ---------------------------------------------------------------------------
// Theme / Settings
// ---------------------------------------------------------------------------

fn settings_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    config_dir.join("forge-terminal").join("settings.json")
}

#[tauri::command]
async fn settings_load() -> Result<String, String> {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(_) => Ok("{}".to_string()),
    }
}

#[tauri::command]
async fn settings_save(settings: String) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{e}"))?;
    }
    std::fs::write(&path, &settings).map_err(|e| format!("{e}"))
}

// ---------------------------------------------------------------------------
// Machine configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfig {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub conn_type: String, // "tailscale" or "ssh"
    pub host: String,
    #[serde(default = "default_ssh_user")]
    pub user: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    #[serde(default)]
    pub identity_file: String,
}

fn default_ssh_user() -> String { "ubuntu".into() }
fn default_ssh_port() -> u16 { 22 }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MachinesFile {
    machines: Vec<MachineConfig>,
}

fn machines_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    config_dir.join("forge-terminal").join("machines.json")
}

fn load_machines() -> Vec<MachineConfig> {
    let path = machines_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            serde_json::from_str::<MachinesFile>(&content)
                .map(|f| f.machines)
                .unwrap_or_default()
        }
        Err(_) => vec![],
    }
}

fn save_machines(machines: &[MachineConfig]) -> Result<(), String> {
    let path = machines_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{e}"))?;
    }
    let file = MachinesFile { machines: machines.to_vec() };
    let json = serde_json::to_string_pretty(&file).map_err(|e| format!("{e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("{e}"))
}

fn build_ssh_command(machine: &MachineConfig) -> String {
    let mut parts = vec!["ssh".to_string(), "-t".to_string()];
    parts.push("-o".to_string());
    parts.push("ConnectTimeout=10".to_string());
    if machine.port != 22 {
        parts.push("-p".to_string());
        parts.push(machine.port.to_string());
    }
    if !machine.identity_file.is_empty() {
        parts.push("-i".to_string());
        let expanded = machine.identity_file.replace("~", &dirs::home_dir().unwrap_or_default().display().to_string());
        parts.push(expanded);
    }
    parts.push(format!("{}@{}", machine.user, machine.host));
    parts.join(" ")
}

#[tauri::command]
async fn get_home_dir() -> Result<String, String> {
    Ok(dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .display()
        .to_string())
}

#[tauri::command]
async fn machines_list() -> Result<Vec<MachineConfig>, String> {
    Ok(load_machines())
}

#[tauri::command]
async fn machines_save(machines: Vec<MachineConfig>) -> Result<(), String> {
    save_machines(&machines)
}

#[tauri::command]
async fn remote_ls(machine_id: String, path: String) -> Result<FsListResponse, String> {
    let machines = load_machines();
    let machine = machines
        .iter()
        .find(|m| m.id == machine_id)
        .ok_or_else(|| format!("Machine not found: {machine_id}"))?;

    let ssh_base = build_ssh_command(machine);
    // Run: ssh user@host "ls -1pa <path>" to get directory listing
    let cmd = format!("{} 'ls -1pa {} 2>/dev/null'", ssh_base, path);

    let output = std::process::Command::new("bash")
        .args(["-c", &cmd])
        .output()
        .map_err(|e| format!("SSH failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Remote ls failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line == "./" || line == "../" {
            continue;
        }
        let is_dir = line.ends_with('/');
        let name = if is_dir {
            line.trim_end_matches('/').to_string()
        } else {
            line.to_string()
        };
        if name.is_empty() {
            continue;
        }
        let entry_path = if path.ends_with('/') {
            format!("{}{}", path, &name)
        } else {
            format!("{}/{}", path, &name)
        };
        entries.push(FileEntry {
            name,
            path: entry_path,
            is_dir,
            size: 0,
        });
    }

    // Sort: dirs first, then alphabetical
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(FsListResponse { cwd: path, entries })
}

#[tauri::command]
async fn remote_home(machine_id: String) -> Result<String, String> {
    let machines = load_machines();
    let machine = machines
        .iter()
        .find(|m| m.id == machine_id)
        .ok_or_else(|| format!("Machine not found: {machine_id}"))?;

    let ssh_base = build_ssh_command(machine);
    let cmd = format!("{} 'echo $HOME'", ssh_base);

    let output = std::process::Command::new("bash")
        .args(["-c", &cmd])
        .output()
        .map_err(|e| format!("SSH failed: {e}"))?;

    let home = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if home.is_empty() {
        Ok("/home".to_string())
    } else {
        Ok(home)
    }
}

// ---------------------------------------------------------------------------
// File opening
// ---------------------------------------------------------------------------

#[tauri::command]
async fn open_file(path: String) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open file: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Output pump
// ---------------------------------------------------------------------------

fn spawn_output_pump(
    app: AppHandle,
    session_name: String,
    mut rx: tokio::sync::broadcast::Receiver<OutputChunk>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(chunk) => {
                    let _ = app.emit(
                        "pty-output",
                        &PtyOutputEvent {
                            session_name: session_name.clone(),
                            data: chunk.data.to_vec(),
                        },
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    log::warn!("output lagged by {n} for {session_name}");
                }
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Entry
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pty_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("pty runtime");

    let state = Arc::new(AppState {
        pty: PtyBridge::with_defaults(),
        output_pumps: RwLock::new(HashMap::new()),
        pty_rt,
    });

    tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            sessions_launch,
            sessions_list,
            pty_send,
            pty_resize,
            pty_kill,
            filesystem_list,
            filesystem_read,
            filesystem_cwd,
            get_home_dir,
            settings_load,
            settings_save,
            machines_list,
            machines_save,
            remote_ls,
            remote_home,
            open_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
