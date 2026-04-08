#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# forge-terminal — launch in local-only mode
#
# No NATS, no network. Just a terminal emulator with tabs, file tree,
# and PTY sessions. This is the default mode.
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/target/release/forge-terminal-app"
