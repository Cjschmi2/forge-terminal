# Project

Forge Terminal — a native desktop terminal emulator (Tauri + Rust + SvelteKit) that manages multiple PTY sessions running bash, Claude Code, Codex, or custom commands with tabbed UI, file browser, and structured tag extraction from agent output.

## Commands

```bash
cargo test                    # run all Rust tests (workspace)
cargo build                   # build all backend crates
cargo build -p forge-terminal-app  # build Tauri app only
cd frontend && npm run dev    # SvelteKit dev server (port 5180)
cd frontend && npm run check  # TypeScript/Svelte type check
cd frontend && npm run tauri:dev   # full Tauri dev mode (frontend + backend)
cd frontend && npm run tauri:build # production build
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
