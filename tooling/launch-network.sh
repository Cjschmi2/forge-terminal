#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# forge-terminal — launch with network enabled (NATS + cross-terminal messaging)
#
# Requires:
#   1. Binary built with: ./tooling/build-network.sh
#   2. NATS running: cd ../forge-communication && docker compose -f deploy/docker-compose.yml up -d
#
# Environment variables:
#   NATS_URL          — NATS server (default: nats://127.0.0.1:4222)
#   FORGE_PROJECT_ID  — Project ID for message routing (default: "default")
#
# What network mode adds:
#   - Tag events from PTY sessions are published to NATS
#   - Other forge-terminal instances on the same project receive them
#   - Agents in different terminals can communicate via [{broadcast:msg}] tags
#   - Network discovery reports Tailscale peers and mosh availability
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

export NATS_URL="${NATS_URL:-nats://127.0.0.1:4222}"
export FORGE_PROJECT_ID="${FORGE_PROJECT_ID:-default}"

echo "Forge Terminal — network mode"
echo "  NATS:    $NATS_URL"
echo "  Project: $FORGE_PROJECT_ID"
echo ""

exec "$ROOT/target/release/forge-terminal-app"
