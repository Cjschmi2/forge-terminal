#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# Start the NATS message bus (required for network mode)
#
# Uses the docker-compose in forge-communication/deploy/.
# NATS runs on:
#   - port 4222 (client connections)
#   - port 8222 (monitoring/health: http://localhost:8222/healthz)
#
# To stop: docker compose -f ../forge-communication/deploy/docker-compose.yml down
# ═══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

COMM_ROOT="$(cd "$(dirname "$0")/../../forge-communication" && pwd)"

if [ ! -f "$COMM_ROOT/deploy/docker-compose.yml" ]; then
    echo "ERROR: forge-communication repo not found at $COMM_ROOT"
    echo "Expected sibling directory: ../forge-communication/"
    exit 1
fi

echo "Starting NATS + JetStream..."
docker compose -f "$COMM_ROOT/deploy/docker-compose.yml" up -d

echo ""
echo "Waiting for NATS to accept connections..."
for i in $(seq 1 15); do
    # NATS client port responds with its protocol info on raw TCP connect
    if timeout 1 bash -c 'echo "" > /dev/tcp/127.0.0.1/4222' 2>/dev/null; then
        echo "NATS is ready: nats://127.0.0.1:4222"
        echo "Monitoring:    http://localhost:8222"
        exit 0
    fi
    sleep 1
done

echo "WARNING: NATS started but port 4222 not responding after 15s."
echo "Check: docker logs deploy-nats-1"
