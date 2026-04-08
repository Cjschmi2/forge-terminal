#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# forge-terminal — install all dependencies
# Run once after clone or when deps change
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "=== System dependencies ==="
# mold linker (fast linking)
if ! command -v mold &>/dev/null; then
  echo "Installing mold linker..."
  sudo apt-get install -y mold
else
  echo "mold: $(mold --version)"
fi

# Tauri system deps
if ! dpkg -l libwebkit2gtk-4.1-dev &>/dev/null 2>&1; then
  echo "Installing Tauri system deps..."
  sudo apt-get install -y \
    libwebkit2gtk-4.1-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf
else
  echo "Tauri system deps: installed"
fi

echo ""
echo "=== Rust toolchain ==="
rustc --version
cargo --version
cargo tauri --version 2>/dev/null || cargo install tauri-cli --version "^2"

echo ""
echo "=== Node dependencies ==="
cd "$ROOT/frontend"
npm install

echo ""
echo "=== Ready ==="
echo "Run: ./tooling/dev.sh      (hot-reload dev mode)"
echo "Run: ./tooling/fast-build.sh  (quick release binary)"
echo "Run: ./tooling/build.sh       (full build with bundles)"
