#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# forge-terminal — full release build
# Builds frontend + Tauri binary + bundles (.deb, .rpm, .AppImage)
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/frontend"

echo "=== Frontend build ==="
npm run build

echo "=== Tauri release build ==="
cargo tauri build

echo ""
echo "=== Done ==="
echo "Binary: $ROOT/target/release/forge-terminal-app"
ls -lh "$ROOT/target/release/forge-terminal-app"
echo ""
echo "Bundles:"
ls -lh "$ROOT/target/release/bundle/deb/"*.deb 2>/dev/null || true
ls -lh "$ROOT/target/release/bundle/appimage/"*.AppImage 2>/dev/null || true
