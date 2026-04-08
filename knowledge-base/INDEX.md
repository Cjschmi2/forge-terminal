# Knowledge Base Index

Last full setup: 2026-04-07

## project/
- architecture.md — System overview: Tauri + Rust PTY backend, SvelteKit frontend, 8 crates, data flow (~2.5K tokens) [2026-04-07]
- integrations.md — SSH/Tailscale, agent CLI tools, env vars, router defaults (~1K tokens) [2026-04-07]
- data-model.md — Core entities: SessionId, NativePtySession, TokenFrame, SessionState, type locations (~1.5K tokens) [2026-04-07]

## testing/
- strategy.md — Rust std tests, per-crate coverage table, 67+ tests total (~1.5K tokens) [2026-04-07]
- dependencies.md — Dev deps, inline fixtures, no mocks (real PTY processes) (~500 tokens) [2026-04-07]
- critical-paths.md — Env filter, dir allowlist, PTY lifecycle, wire codec, tag scanner (~1.5K tokens) [2026-04-07]

## security/
- auth-model.md — Wire JWT (not wired), Tauri IPC (trusted), dir allowlist, env filter (~1.5K tokens) [2026-04-07]
- secrets.md — No stored secrets; env-var leak prevention via filter_env (~500 tokens) [2026-04-07]

## planning/
- current.md — v0.1.0 early dev, core PTY working, known gaps (no CI, no frontend tests) (~500 tokens) [2026-04-07]

## gotchas/
- backend.md — Backend pitfalls [2026-04-07]
- frontend.md — Frontend pitfalls [2026-04-07]
- data-infra.md — Data & infra pitfalls [2026-04-07]
