#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# forge-terminal — trim stale build artifacts
# Removes debug build, old bundles, and incremental caches while keeping
# the release binary and incremental release cache intact.
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Before:"
du -sh "$ROOT/target/" 2>/dev/null || echo "No target dir"

# Remove debug build (we use release)
rm -rf "$ROOT/target/debug" 2>/dev/null
# Remove bundle artifacts (large, rebuild with build.sh)
rm -rf "$ROOT/target/release/bundle" 2>/dev/null
# Remove build script outputs for deps (safe to remove, rebuilt on demand)
rm -rf "$ROOT/target/release/build" 2>/dev/null
# Remove Svelte build cache
rm -rf "$ROOT/frontend/.svelte-kit/output" 2>/dev/null

echo "After:"
du -sh "$ROOT/target/" 2>/dev/null || echo "Clean"
