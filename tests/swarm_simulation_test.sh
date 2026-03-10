#!/bin/bash
# Swarm Network Simulation Test (Phase 15 Hardening Verification)
set -e

echo "🧪 [Simulation] Starting Swarm Network Simulation..."

# 0. Setup
export FEDERATION_SECRET="swarm_test_secret_2026"
export SAMSARA_HUB_REST="http://127.0.0.1:3016"
export HUB_DATABASE_URL="sqlite:hub_test.db?mode=rwc"
export RUST_LOG=info

# Cleanup previous runs
pkill samsara-hub || true
pkill api-server || true
rm -f hub_test.db node_a.db node_b.db hub.log node_a.log node_b.log
rm -rf workspace_a workspace_b
mkdir -p workspace_a workspace_b

echo "🔨 [Simulation] Pre-building binaries..."
cargo build -p samsara-hub -p api-server > /dev/null 2>&1

# 1. Start Samsara Hub
echo "🏔️ [Simulation] Launching Samsara Hub..."
DATABASE_URL=$HUB_DATABASE_URL cargo run -p samsara-hub > hub.log 2>&1 &
HUB_PID=$!

# 2. Start Node A
echo "🧬 [Simulation] Launching Node A (Port 3017)..."
AIOME_DB_PATH="sqlite://workspace_a/node_a.db" PORT=3017 cargo run -p api-server > node_a.log 2>&1 &
NODE_A_PID=$!

# 3. Start Node B
echo "🌐 [Simulation] Launching Node B (Port 3018)..."
AIOME_DB_PATH="sqlite://workspace_b/node_b.db" PORT=3018 cargo run -p api-server > node_b.log 2>&1 &
NODE_B_PID=$!

echo "⏳ Waiting for nodes to be ready..."
for i in {1..30}; do
    if grep -q "listening on" node_a.log && grep -q "listening on" node_b.log && grep -q "listening on" hub.log; then
        echo " Ready!"
        break
    fi
    echo -n "."
    sleep 2
    if [ $i -eq 30 ]; then
        echo "❌ [ERROR] Nodes failed to start in time."
        exit 1
    fi
done

# 4. Trigger Karma generation on Node A
echo "⚡ [Simulation] Triggering failure demo on Node A..."
curl -s -X POST http://127.0.0.1:3017/api/synergy/test/failure > /dev/null

echo "🔍 [Simulation] Verifying Node A Local Karma..."
sleep 2
A_LOCAL_COUNT=$(sqlite3 workspace_a/node_a.db "SELECT COUNT(*) FROM karma_logs;")
echo "Node A Local Karma Count: $A_LOCAL_COUNT"

echo "⏳ Waiting for Node A to push and Hub to approve (~75s)..."
sleep 80

# 5. Verify Hub has either Quarantined or Approved data
echo "🔍 [Simulation] Checking Hub Data..."
Q_COUNT=$(sqlite3 hub_test.db "SELECT COUNT(*) FROM quarantined_karma;")
A_COUNT=$(sqlite3 hub_test.db "SELECT COUNT(*) FROM approved_karma;")
echo "Quarantined: $Q_COUNT, Approved: $A_COUNT"

if [ "$((Q_COUNT + A_COUNT))" -eq 0 ]; then
    echo "❌ [ERROR] Hub received no data from Node A."
    tail -n 20 hub.log
    kill $HUB_PID $NODE_A_PID $NODE_B_PID
    exit 1
fi

echo "✅ [SUCCESS] Data reached Hub and passed verification pipeline."

# 6. Wait for Node B to Sync
echo "⏳ Waiting for Node B to Sync from Hub (~70s)..."
sleep 80

# 7. Verify Node B has Federated Karma
echo "🔍 [Simulation] Checking Node B Local DB..."
B_FED_COUNT=$(sqlite3 workspace_b/node_b.db "SELECT COUNT(*) FROM karma_logs WHERE is_federated = 1;")
echo "Node B Federated Karma Count: $B_FED_COUNT"

if [ "$B_FED_COUNT" -gt 0 ]; then
    echo "🎉 [FINAL SUCCESS] End-to-End Swarm Sync Verified!"
else
    echo "❌ [ERROR] Node B failed to sync."
    kill $HUB_PID $NODE_A_PID $NODE_B_PID
    exit 1
fi

# Cleanup
echo "🛑 [Simulation] Shutting down nodes..."
kill $HUB_PID $NODE_A_PID $NODE_B_PID || true
echo "🏁 [Simulation] Test Finished."
