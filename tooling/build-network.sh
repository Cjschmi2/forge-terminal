#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# forge-terminal — release build WITH network feature (NATS integration)
#
# This builds the same app as build.sh but with the "network" Cargo feature
# enabled. The resulting binary can connect to NATS for cross-terminal
# agent messaging when NATS_URL is set at launch time.
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/frontend"

echo "=== Frontend build ==="
npm run build

echo "=== Tauri release build (with network feature) ==="
cd "$ROOT"
cargo build --release -p forge-terminal-app --features network

echo ""
echo "=== Done ==="
echo "Binary: $ROOT/target/release/forge-terminal-app"
ls -lh "$ROOT/target/release/forge-terminal-app"
echo ""
echo "Network feature: ENABLED"
echo "To launch with NATS: NATS_URL=nats://127.0.0.1:4222 $ROOT/target/release/forge-terminal-app"
echo "To launch local-only: $ROOT/target/release/forge-terminal-app  (no NATS_URL = no network)"
