#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# forge-terminal — clean all build artifacts
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Cleaning Rust target..."
cargo clean --manifest-path "$ROOT/Cargo.toml"

echo "Cleaning Svelte build..."
rm -rf "$ROOT/frontend/build" "$ROOT/frontend/.svelte-kit"

echo "Done. Next build will be from scratch."
