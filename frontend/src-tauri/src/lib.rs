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
    #[cfg(feature = "network")]
    pub forge_node: Option<forge_node::ForgeNode>,
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

    let pump = spawn_output_pump(app.clone(), request.name.clone(), rx);
    state.output_pumps.write().await.insert(request.name.clone(), pump);

    // Bridge tag events to NATS + handle room tags when network is enabled
    #[cfg(feature = "network")]
    if state.forge_node.is_some() {
        if let Ok(mut tag_rx) = state.pty.subscribe_tags(&request.name) {
            let s = state.inner().clone();
            let session_name = request.name.clone();
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(ref node) = s.forge_node {
                    while let Ok(event) = tag_rx.recv().await {
                        // Check if this is a layer 2 room tag
                        let inner = &event.tag.payload;
                        if let Some(room_tag) = forge_room_tags::parse_room_tag(inner) {
                            // Handle the room tag
                            let response = handle_room_tag(
                                node,
                                &session_name,
                                &room_tag,
                                &s,
                                &app_handle,
                            ).await;
                            // Inject response into the terminal
                            if let Some(response_text) = response {
                                let _ = app_handle.emit(
                                    "pty-output",
                                    &PtyOutputEvent {
                                        session_name: session_name.clone(),
                                        data: response_text.into_bytes(),
                                    },
                                );
                            }
                        } else {
                            // Layer 1 tag — forward to NATS
                            if let Some(nats) = node.try_nats() {
                                let project_id = &node.config().project_id;
                                if let Err(e) = nats.broadcast(project_id, &event.frame).await {
                                    log::warn!("NATS tag publish failed: {e}");
                                }
                            }
                        }
                    }
                }
            });
        }
    }

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
// Room tag handler
// ---------------------------------------------------------------------------

#[cfg(feature = "network")]
async fn handle_room_tag(
    node: &forge_node::ForgeNode,
    session_name: &str,
    tag: &forge_room_tags::RoomTag,
    state: &Arc<AppState>,
    app: &AppHandle,
) -> Option<String> {
    use forge_room_tags::*;

    let rooms = node.try_rooms()?;

    match tag {
        RoomTag::Register { name } => {
            let workdir = state.pty.list_sessions().iter()
                .find(|s| s.name == session_name)
                .map(|s| s.working_dir.display().to_string())
                .unwrap_or_else(|| "/tmp".to_string());

            let agent = forge_node::rooms::AgentIdentity::new(
                name,
                "claude",
                &workdir,
                node.machine_id(),
                session_name,
            );
            match rooms.register(agent) {
                Ok(()) => {
                    let resp = TagResponse::Registered { name: name.clone(), room: workdir };
                    Some(format_response(tag, &resp))
                }
                Err(e) => {
                    Some(format_response(tag, &TagResponse::Error { message: e.to_string() }))
                }
            }
        }

        RoomTag::Discover { target } => {
            // Find the agent name for this session
            let agent_name = find_agent_name(rooms, session_name)?;
            match target.as_deref() {
                None => {
                    match rooms.discover(&agent_name) {
                        Ok(room_list) => {
                            let data: Vec<(String, Vec<AgentInfo>)> = room_list.iter().map(|(path, agents)| {
                                let infos = agents.iter().map(|a| AgentInfo {
                                    name: a.name.clone(),
                                    tool: a.tool.clone(),
                                    room: a.root_room.clone(),
                                    status: a.status.to_string(),
                                    last_active: a.last_heartbeat.format("%H:%M:%S").to_string(),
                                }).collect();
                                (path.clone(), infos)
                            }).collect();
                            Some(format_response(tag, &TagResponse::DiscoverAll { rooms: data }))
                        }
                        Err(e) => Some(format_response(tag, &TagResponse::Error { message: e.to_string() })),
                    }
                }
                Some(t) if t.starts_with('@') => {
                    let name = t.trim_start_matches('@');
                    match rooms.get_agent(name) {
                        Some(a) => {
                            let info = AgentInfo {
                                name: a.name.clone(), tool: a.tool.clone(),
                                room: a.root_room.clone(), status: a.status.to_string(),
                                last_active: a.last_heartbeat.format("%H:%M:%S").to_string(),
                            };
                            Some(format_response(tag, &TagResponse::DiscoverAgent { agent: info }))
                        }
                        None => Some(format_response(tag, &TagResponse::Error { message: format!("agent not found: @{name}") })),
                    }
                }
                _ => {
                    // discover:room — list agents in root room only
                    let agent_name = find_agent_name(rooms, session_name)?;
                    match rooms.discover(&agent_name) {
                        Ok(room_list) => {
                            let data: Vec<(String, Vec<AgentInfo>)> = room_list.into_iter().take(1).map(|(path, agents)| {
                                let infos = agents.iter().map(|a| AgentInfo {
                                    name: a.name.clone(), tool: a.tool.clone(),
                                    room: a.root_room.clone(), status: a.status.to_string(),
                                    last_active: a.last_heartbeat.format("%H:%M:%S").to_string(),
                                }).collect();
                                (path, infos)
                            }).collect();
                            Some(format_response(tag, &TagResponse::DiscoverAll { rooms: data }))
                        }
                        Err(e) => Some(format_response(tag, &TagResponse::Error { message: e.to_string() })),
                    }
                }
            }
        }

        RoomTag::Rooms { tree } => {
            let agent_name = find_agent_name(rooms, session_name)?;
            if *tree {
                match rooms.visible_rooms(&agent_name) {
                    Ok(visible) => {
                        let data = visible.iter().map(|(p, c)| RoomInfo { path: p.clone(), agent_count: *c }).collect();
                        Some(format_response(tag, &TagResponse::RoomsList { rooms: data }))
                    }
                    Err(e) => Some(format_response(tag, &TagResponse::Error { message: e.to_string() })),
                }
            } else {
                match rooms.agent_rooms(&agent_name) {
                    Ok(paths) => {
                        let data = paths.iter().map(|p| RoomInfo { path: p.clone(), agent_count: 0 }).collect();
                        Some(format_response(tag, &TagResponse::RoomsList { rooms: data }))
                    }
                    Err(e) => Some(format_response(tag, &TagResponse::Error { message: e.to_string() })),
                }
            }
        }

        RoomTag::Msg { target, text } => {
            let sender = find_agent_name(rooms, session_name)?;
            // Find target's session and inject message
            if let Some(target_agent) = rooms.get_agent(target) {
                let msg_text = forge_room_tags::format_inbound_message(&sender, text);
                let _ = app.emit("pty-output", &PtyOutputEvent {
                    session_name: target_agent.session_name.clone(),
                    data: msg_text.into_bytes(),
                });
                // Record in room buffer
                let _ = rooms.record_message(
                    &sender, &target_agent.root_room,
                    forge_node::rooms::room::RoomMessageType::Message,
                    forge_node::rooms::room::MessageTarget::Agent(target.clone()),
                    text,
                );
                Some(format_response(tag, &TagResponse::MessageSent { target: target.clone() }))
            } else {
                Some(format_response(tag, &TagResponse::Error { message: format!("agent not found: @{target}") }))
            }
        }

        RoomTag::Broadcast { room: target_room, text } => {
            let sender = find_agent_name(rooms, session_name)?;
            let room_path = target_room.clone().unwrap_or_else(|| {
                rooms.agent_rooms(&sender).ok().and_then(|r| r.first().cloned()).unwrap_or_default()
            });
            // Send to all agents in the room
            let msg_text = forge_room_tags::format_inbound_broadcast(&sender, &room_path, text);
            if let Ok(discovered) = rooms.discover(&sender) {
                let mut count = 0;
                for (path, agents) in &discovered {
                    if *path == room_path {
                        for agent in agents {
                            if agent.name != sender {
                                let _ = app.emit("pty-output", &PtyOutputEvent {
                                    session_name: agent.session_name.clone(),
                                    data: msg_text.clone().into_bytes(),
                                });
                                count += 1;
                            }
                        }
                    }
                }
                let _ = rooms.record_message(
                    &sender, &room_path,
                    forge_node::rooms::room::RoomMessageType::Broadcast,
                    forge_node::rooms::room::MessageTarget::Broadcast,
                    text,
                );
                Some(format_response(tag, &TagResponse::BroadcastSent { room: room_path, count }))
            } else {
                Some(format_response(tag, &TagResponse::Error { message: "not registered".into() }))
            }
        }

        RoomTag::Nudge { target, text } => {
            let sender = find_agent_name(rooms, session_name)?;
            rooms.push_nudge(target, forge_node::rooms::manager::Nudge {
                sender: sender.clone(),
                text: text.clone(),
                timestamp: chrono::Utc::now(),
            });
            // Send notification to target
            if let Some(agent) = rooms.get_agent(target) {
                let notif = forge_room_tags::format_nudge_notification(&sender);
                let _ = app.emit("pty-output", &PtyOutputEvent {
                    session_name: agent.session_name.clone(),
                    data: notif.into_bytes(),
                });
            }
            Some(format_response(tag, &TagResponse::NudgeSent { target: target.clone() }))
        }

        RoomTag::Inbox { clear } => {
            let agent_name = find_agent_name(rooms, session_name)?;
            if *clear {
                let nudges = rooms.drain_nudges(&agent_name);
                Some(format_response(tag, &TagResponse::InboxCleared { count: nudges.len() }))
            } else {
                let nudges = rooms.drain_nudges(&agent_name);
                let displays: Vec<NudgeDisplay> = nudges.iter().map(|n| NudgeDisplay {
                    sender: n.sender.clone(),
                    timestamp: n.timestamp.format("%H:%M:%S").to_string(),
                    text: n.text.clone(),
                }).collect();
                Some(format_response(tag, &TagResponse::InboxContents { nudges: displays }))
            }
        }

        RoomTag::Poll { target: forge_room_tags::PollTarget::Room { path, limit } } => {
            let agent_name = find_agent_name(rooms, session_name)?;
            let room_path = path.clone().unwrap_or_else(|| {
                rooms.agent_rooms(&agent_name).ok().and_then(|r| r.first().cloned()).unwrap_or_default()
            });
            match rooms.poll_room(&room_path, *limit) {
                Ok(msgs) => {
                    let displays: Vec<RoomMessageDisplay> = msgs.iter().map(|m| RoomMessageDisplay {
                        sender: m.sender.clone(),
                        timestamp: m.timestamp.format("%H:%M:%S").to_string(),
                        message_type: m.message_type.to_string(),
                        target: match &m.target {
                            forge_node::rooms::room::MessageTarget::Agent(n) => Some(format!("@{n}")),
                            forge_node::rooms::room::MessageTarget::Room(r) => Some(r.clone()),
                            forge_node::rooms::room::MessageTarget::Broadcast => None,
                        },
                        text: m.payload.clone(),
                    }).collect();
                    Some(format_response(tag, &TagResponse::PollResult {
                        header: format!("Room: {} (last {})", room_path, limit),
                        messages: displays,
                    }))
                }
                Err(e) => Some(format_response(tag, &TagResponse::Error { message: e.to_string() })),
            }
        }

        RoomTag::Poll { target: forge_room_tags::PollTarget::Names { path } } => {
            let agent_name = find_agent_name(rooms, session_name)?;
            let names = rooms.all_names(&agent_name).unwrap_or_default();
            Some(format_response(tag, &TagResponse::NamesList { names, room: path.clone() }))
        }

        RoomTag::Status { text } => {
            let sender = find_agent_name(rooms, session_name)?;
            let room_path = rooms.agent_rooms(&sender).ok().and_then(|r| r.first().cloned()).unwrap_or_default();
            let _ = rooms.record_message(
                &sender, &room_path,
                forge_node::rooms::room::RoomMessageType::Status,
                forge_node::rooms::room::MessageTarget::Broadcast,
                text,
            );
            Some(format_response(tag, &TagResponse::StatusEmitted { text: text.clone() }))
        }

        RoomTag::Help => {
            Some(format_response(tag, &TagResponse::Help { text: forge_room_tags::help_text() }))
        }

        // cmd, spawn, poll:agent — these need more wiring (task #22)
        _ => {
            Some(format_response(tag, &TagResponse::Error { message: "tag not yet implemented".into() }))
        }
    }
}

#[cfg(feature = "network")]
fn find_agent_name(rooms: &forge_node::rooms::RoomManager, session_name: &str) -> Option<String> {
    // Search all rooms for an agent with this session name
    // This is a linear scan — fine for <100 agents
    for (_, agents) in rooms.discover("__probe__").unwrap_or_default() {
        for agent in agents {
            if agent.session_name == session_name {
                return Some(agent.name.clone());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Room Tauri commands
// ---------------------------------------------------------------------------

#[cfg(feature = "network")]
#[derive(Debug, Serialize)]
struct RoomListEntry {
    path: String,
    agent_count: usize,
}

#[cfg(feature = "network")]
#[derive(Debug, Serialize)]
struct RoomAgentEntry {
    name: String,
    tool: String,
    room: String,
    status: String,
    session_name: String,
}

#[cfg(feature = "network")]
#[tauri::command]
async fn rooms_list(state: State<'_, Arc<AppState>>) -> Result<Vec<RoomListEntry>, String> {
    let node = state.forge_node.as_ref().ok_or("network not enabled")?;
    let rooms = node.try_rooms().ok_or("rooms not enabled")?;
    // List all rooms that have agents — scan by using a dummy discover
    // In practice the UI would need a dedicated method, but this works for now
    Ok(vec![]) // TODO: implement once we have a list_all_rooms method
}

#[cfg(feature = "network")]
#[tauri::command]
async fn room_agents(
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<Vec<RoomAgentEntry>, String> {
    let node = state.forge_node.as_ref().ok_or("network not enabled")?;
    let rooms = node.try_rooms().ok_or("rooms not enabled")?;
    // Get agents in this specific room path
    Ok(vec![]) // TODO: expose room-level query
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

    // Optionally initialize forge-node for NATS integration
    #[cfg(feature = "network")]
    let forge_node = {
        // Check if NATS_URL env var is set — if so, start the network layer
        match std::env::var("NATS_URL") {
            Ok(nats_url) => {
                let config = forge_node::ForgeNodeConfig {
                    node_name: "forge-terminal".into(),
                    project_id: std::env::var("FORGE_PROJECT_ID")
                        .unwrap_or_else(|_| "default".into()),
                    machine_id: None,
                    nats: Some(forge_node::NatsConfig {
                        server_url: nats_url.clone(),
                        auth_token: None,
                        jetstream_max_age_secs: 86400,
                        jetstream_max_messages: 100_000,
                    }),
                };
                match pty_rt.block_on(forge_node::ForgeNode::builder(config).build()) {
                    Ok(node) => {
                        log::info!("forge-node connected to NATS at {nats_url}");
                        Some(node)
                    }
                    Err(e) => {
                        log::warn!("forge-node init failed: {e} — running without network");
                        None
                    }
                }
            }
            Err(_) => None,
        }
    };

    let state = Arc::new(AppState {
        pty: PtyBridge::with_defaults(),
        output_pumps: RwLock::new(HashMap::new()),
        pty_rt,
        #[cfg(feature = "network")]
        forge_node,
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
