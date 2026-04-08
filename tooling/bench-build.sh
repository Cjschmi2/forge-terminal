#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# forge-terminal — benchmark build times
# Shows incremental vs clean build performance
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "=== Incremental check (no changes) ==="
time cargo check --manifest-path "$ROOT/Cargo.toml" -p forge-terminal-app 2>/dev/null

echo ""
echo "=== Incremental build (no changes) ==="
time cargo build --release --manifest-path "$ROOT/Cargo.toml" -p forge-terminal-app 2>/dev/null

echo ""
echo "=== Touch lib.rs + incremental rebuild ==="
touch "$ROOT/frontend/src-tauri/src/lib.rs"
time cargo build --release --manifest-path "$ROOT/Cargo.toml" -p forge-terminal-app 2>/dev/null

echo ""
echo "=== Frontend build ==="
cd "$ROOT/frontend"
time npm run build 2>/dev/null
