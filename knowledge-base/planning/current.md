# Current Planning

## Status

Project is in early development (v0.1.0). Core PTY infrastructure is complete and tested. The Tauri desktop app is functional with basic terminal emulation.

## What's Working

- Full PTY lifecycle: spawn, read/write, resize, kill
- Multi-session router with named sessions and broadcast output
- Tag scanner extracting structured `[{...}]` patterns from PTY output
- Session recording (raw + asciicast)
- Wire protocol frame codec (Rust + TypeScript parity)
- Tauri desktop app with tabbed terminal UI
- File browser sidebar
- Remote SSH sessions via Tailscale (r5 machine)

## Known Gaps

- Wire protocol WebSocket server not yet integrated (WireClient exists but no server endpoint)
- No Svelte component tests or frontend test infrastructure
- No CI/CD pipeline
- Tauri commands not unit tested (only integration via manual UI testing)
- CSP is disabled in tauri.conf.json
- FileExplorer.svelte exists but is not used (FileTree.svelte is the active component)

## Priorities

To be determined by user. The codebase is ready for feature work on top of the PTY infrastructure.
