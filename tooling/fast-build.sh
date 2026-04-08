#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# forge-terminal — fast rebuild
# Only rebuilds what changed. Uses cargo tauri build to embed frontend.
# Skips bundle generation (.deb/.AppImage) for speed.
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BINARY="$ROOT/target/release/forge-terminal-app"
DESKTOP="$HOME/Desktop/forge-terminal.desktop"

cd "$ROOT/frontend"

echo "=== Building (frontend + Rust, no bundles) ==="
cargo tauri build --no-bundle 2>&1 | grep -E "Compiling|Finished|error"

# Update desktop shortcut
if [ -f "$DESKTOP" ]; then
  sed -i "s|^Exec=.*|Exec=$BINARY|" "$DESKTOP"
fi

# Clean stale bundles
rm -rf "$ROOT/target/release/bundle" 2>/dev/null || true

echo ""
ls -lh "$BINARY"
