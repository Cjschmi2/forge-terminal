#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# forge-terminal — type check + lint (fast, no build)
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

echo "=== Rust check ==="
if ! cargo check --manifest-path "$ROOT/Cargo.toml" --workspace; then
  FAIL=1
fi

echo ""
echo "=== Rust clippy ==="
if ! cargo clippy --manifest-path "$ROOT/Cargo.toml" --workspace -- -D warnings 2>/dev/null; then
  echo "(clippy warnings — non-blocking)"
fi

echo ""
echo "=== Svelte check ==="
cd "$ROOT/frontend"
if ! npm run check 2>/dev/null; then
  FAIL=1
fi

echo ""
if [ $FAIL -eq 0 ]; then
  echo "=== All checks passed ==="
else
  echo "=== Some checks failed ==="
  exit 1
fi
