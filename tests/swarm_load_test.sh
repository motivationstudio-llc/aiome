#!/bin/bash
# Aiome Swarm Load Test - Simulating Large Node Federation
# This script simulates multiple nodes pushing data to Samsara Hub.

HUB_URL=${SAMSARA_HUB_REST:-"http://127.0.0.1:3016"}
SECRET=${FEDERATION_SECRET:-"dev_secret"}
NODE_COUNT=20
PUSH_INTERVAL=1

echo "🚀 Starting Swarm Load Test with $NODE_COUNT simulated nodes..."
echo "📍 Targeting Hub: $HUB_URL"

# Create a temporary directory for node IDs
mkdir -p /tmp/aiome_test_nodes

# Pre-generate Node IDs (Base64 simulated Ed25519 Pubkeys)
for i in $(seq 1 $NODE_COUNT); do
    echo "node_$(openssl rand -base64 32)" > /tmp/aiome_test_nodes/node_$i.txt
done

# Simulating data push loop
for i in $(seq 1 $NODE_COUNT); do
    (
        NODE_ID=$(cat /tmp/aiome_test_nodes/node_$i.txt)
        CLOCK=1
        while true; do
            KARMA_ID="karma_$(openssl rand -hex 16)"
            TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
            
            # Simulate FederationPushRequest
            PAYLOAD=$(cat <<EOF
{
  "node_id": "$NODE_ID",
  "karmas": [
    {
      "id": "$KARMA_ID",
      "karma_type": "Technical",
      "related_skill": "skill_$(($i % 5))",
      "lesson": "Simulated lesson from load test node $i at clock $CLOCK",
      "weight": 5,
      "lamport_clock": $CLOCK,
      "node_id": "$NODE_ID",
      "created_at": "$TIMESTAMP",
      "signature": "simulated_sig"
    }
  ],
  "rules": []
}
EOF
)
            
            curl -s -X POST "$HUB_URL/api/v1/federation/push" \
                 -H "Content-Type: application/json" \
                 -H "Authorization: Bearer $SECRET" \
                 -d "$PAYLOAD" > /dev/null
            
            # echo "Node $i: Pushed Karma $KARMA_ID"
            CLOCK=$((CLOCK + 1))
            sleep $PUSH_INTERVAL
        done
    ) &
done

echo "🔥 Load test is RUNNING. Press Ctrl+C to stop all background processes."
wait
