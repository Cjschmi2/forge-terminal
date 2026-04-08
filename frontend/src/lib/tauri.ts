import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface LaunchRequest {
  name: string;
  tool: string;
  working_directory: string;
  machine?: string;
}

export interface LaunchResponse {
  session_id: string;
  name: string;
}

export interface SessionInfo {
  name: string;
  command_type: string;
  working_dir: string;
}

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

export interface FsListResponse {
  cwd: string;
  entries: FileEntry[];
}

export interface PtyOutputEvent {
  session_name: string;
  data: number[];
}

export interface MachineConfig {
  id: string;
  name: string;
  type: string; // "tailscale" | "ssh"
  host: string;
  user: string;
  port: number;
  identity_file: string;
}

// --- PTY commands ---

export function launchSession(request: LaunchRequest): Promise<LaunchResponse> {
  return invoke('sessions_launch', { request });
}

export function ptySend(sessionName: string, input: string): Promise<number> {
  return invoke('pty_send', { sessionName, input });
}

export function ptyResize(sessionName: string, cols: number, rows: number): Promise<void> {
  return invoke('pty_resize', { sessionName, cols, rows });
}

export function ptyKill(sessionName: string): Promise<void> {
  return invoke('pty_kill', { sessionName });
}

export function listSessions(): Promise<SessionInfo[]> {
  return invoke('sessions_list');
}

// --- Filesystem commands ---

export function fsList(path: string): Promise<FsListResponse> {
  return invoke('filesystem_list', { path });
}

export function fsRead(path: string, maxBytes?: number): Promise<string> {
  return invoke('filesystem_read', { path, maxBytes });
}

export function openFile(path: string): Promise<void> {
  return invoke('open_file', { path });
}

export function fsCwd(sessionName: string): Promise<string> {
  return invoke('filesystem_cwd', { sessionName });
}

// --- Machine commands ---

export function getHomeDir(): Promise<string> {
  return invoke('get_home_dir');
}

export function machinesList(): Promise<MachineConfig[]> {
  return invoke('machines_list');
}

export function machinesSave(machines: MachineConfig[]): Promise<void> {
  return invoke('machines_save', { machines });
}

export function remoteLs(machineId: string, path: string): Promise<FsListResponse> {
  return invoke('remote_ls', { machineId, path });
}

export function remoteHome(machineId: string): Promise<string> {
  return invoke('remote_home', { machineId });
}

// --- Settings ---

export function settingsLoad(): Promise<string> {
  return invoke('settings_load');
}

export function settingsSave(settings: string): Promise<void> {
  return invoke('settings_save', { settings });
}

// --- PTY output stream ---

export function onPtyOutput(callback: (event: PtyOutputEvent) => void): Promise<UnlistenFn> {
  return listen<PtyOutputEvent>('pty-output', (e) => callback(e.payload));
}
