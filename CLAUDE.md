# Project

Forge Terminal — a native desktop terminal emulator (Tauri + Rust + SvelteKit) that manages multiple PTY sessions running bash, Claude Code, Codex, or custom commands with tabbed UI, file browser, and structured tag extraction from agent output.

Depends on **forge-communication** (sibling repo at `../forge-communication`) for wire protocol, NATS bridge, and network discovery.

## Commands

```bash
# ── Build ──────────────────────────────────────────────────────
cargo test                                # run all Rust tests
cd frontend && npm run tauri:build        # production build (local-only)
./tooling/build-network.sh                # production build (with NATS support)

# ── Launch ─────────────────────────────────────────────────────
./tooling/launch-local.sh                 # local-only mode (no network)
./tooling/start-nats.sh                   # start NATS (required for network mode)
./tooling/launch-network.sh               # network mode (NATS cross-terminal messaging)

# ── Dev ────────────────────────────────────────────────────────
cd frontend && npm run dev                # SvelteKit dev server (port 5180)
cd frontend && npm run check              # TypeScript/Svelte type check
cd frontend && npm run tauri:dev          # full Tauri dev mode (frontend + backend)
cargo build -p forge-terminal-app --features network  # dev build with network
```

## Modes

### Local-Only (default)
- Standard terminal emulator: tabs, PTY sessions, file tree, settings
- No external dependencies beyond the OS
- Build: `cd frontend && npm run tauri:build`
- Launch: `./tooling/launch-local.sh` or just run the binary

### Network-Enabled (opt-in)
- Everything in local mode, PLUS:
- Tag events from PTY sessions published to NATS (agent-to-agent messaging)
- Multiple forge-terminal instances share agent output via project broadcasts
- Network discovery: Tailscale peers, mosh availability, machine identity
- Build: `./tooling/build-network.sh` (adds `--features network`)
- Launch: `./tooling/launch-network.sh` (sets `NATS_URL` + `FORGE_PROJECT_ID`)
- Requires: NATS server running (`./tooling/start-nats.sh`)

### Environment Variables (network mode)

| Variable | Default | Purpose |
|----------|---------|---------|
| `NATS_URL` | *(none — network disabled if unset)* | NATS server URL, e.g. `nats://127.0.0.1:4222` |
| `FORGE_PROJECT_ID` | `default` | Project ID for NATS message routing |

### Repo Dependencies

```
~/Desktop/
├── forge-terminal/        ← this repo (Tauri app)
└── forge-communication/   ← sibling repo (wire protocol, NATS, gRPC, mesh)
    └── deploy/docker-compose.yml  ← NATS server
```

## Hard Rules

1. Always run tests before saying you're done.
2. Read the file before editing it.
3. Check imports compile after adding dependencies.
4. Never commit secrets, .env files, or credentials.
5. When you hit an unexpected failure, write it to `knowledge-base/gotchas/{your-domain}.md`.

## Team

This project uses domain agents. Each agent owns their slice end to end — code, tests, KB maintenance.

- **Team Lead** — decomposes work, dispatches agents, handles cross-cutting concerns
- **Backend** — server, APIs, data model, business logic
- **Frontend** — UI, components, client state, user flows
- **Data/Infra** — database, schemas, pipelines, deployment, CI/CD

All agents share the same skills: `dev`, `ops`, `kb`. Domain knowledge lives in `knowledge-base/`.
