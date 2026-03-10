#!/bin/bash
# Aiome BFT Attack Simulator
# Tests Hub resistance against malicious node behaviors.

HUB_URL=${SAMSARA_HUB_REST:-"http://127.0.0.1:3016"}
SECRET=${FEDERATION_SECRET:-"dev_secret"}
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

echo "🛡️ Starting BFT Attack Simulation..."

# Helper for posting
post_hub() {
    local path=$1
    local payload=$2
    curl -s -w "\nStatus: %{http_code}\n" -X POST "$HUB_URL$path" \
         -H "Content-Type: application/json" \
         -H "Authorization: Bearer $SECRET" \
         -d "$payload"
}

# 1. 🧬 EQUIVOCATION ATTACK (Double-Signing)
# Push two different items with the same Lamport clock from the same node.
echo "1. Attempting EQUIVOCATION ATTACK..."
NODE_A="node_$(openssl rand -base64 32)"
CLOCK=50

echo "  -> Pushing first item..."
post_hub "/api/v1/federation/push" "{
  \"node_id\": \"$NODE_A\",
  \"karmas\": [{\"id\": \"e1\", \"karma_type\": \"T\", \"related_skill\": \"s1\", \"lesson\": \"Legit\", \"weight\": 1, \"lamport_clock\": $CLOCK, \"node_id\": \"$NODE_A\", \"created_at\": \"$TIMESTAMP\"}],
  \"rules\": []
}"

echo "  -> Pushing second item (DIFFERENT lesson, SAME clock)..."
post_hub "/api/v1/federation/push" "{
  \"node_id\": \"$NODE_A\",
  \"karmas\": [{\"id\": \"e2\", \"karma_type\": \"T\", \"related_skill\": \"s1\", \"lesson\": \"FAKED\", \"weight\": 1, \"lamport_clock\": $CLOCK, \"node_id\": \"$NODE_A\", \"created_at\": \"$TIMESTAMP\"}],
  \"rules\": []
}"

echo "  -> Verifying if Node A is now banned..."
post_hub "/api/v1/federation/sync" "{\"node_id\": \"$NODE_A\", \"protocol_version\": \"1.0\"}"

# 2. 🕒 CLOCK POISONING (Gap 3 Mitigation Test)
echo -e "\n2. Attempting CLOCK POISONING (Extreme clock jump)..."
NODE_B="node_$(openssl rand -base64 32)"
post_hub "/api/v1/federation/push" "{
  \"node_id\": \"$NODE_B\",
  \"karmas\": [{\"id\": \"c1\", \"karma_type\": \"T\", \"related_skill\": \"s1\", \"lesson\": \"Future\", \"weight\": 1, \"lamport_clock\": 1000000000, \"node_id\": \"$NODE_B\", \"created_at\": \"$TIMESTAMP\"}],
  \"rules\": []
}"
# This should result in the item being quarantined but not progressing the hub's logical time if we had that check (Hub is stateless for time mostly).

# 3. 🧪 OOM DoS via Sync Pagination
echo -e "\n3. Testing OOM Sync (Requesting huge backlog without pagination)..."
# Hub implements LIMIT 500, so this should return a partial list and status 200.
post_hub "/api/v1/federation/sync" "{\"node_id\": \"attacker\", \"since\": \"1970-01-01T00:00:00\", \"protocol_version\": \"1.0\"}" | head -n 20

# 4. 🔏 INVALID SIGNATURE SPAM
echo -e "\n4. Attempting INVALID SIGNATURE SPAM..."
NODE_C="node_$(openssl rand -base64 32)"
for i in {1..5}; do
  post_hub "/api/v1/federation/push" "{
    \"node_id\": \"$NODE_C\",
    \"karmas\": [{\"id\": \"sig_$(openssl rand -hex 4)\", \"karma_type\": \"T\", \"related_skill\": \"s1\", \"lesson\": \"NoSig\", \"weight\": 1, \"lamport_clock\": $i, \"node_id\": \"$NODE_C\", \"created_at\": \"$TIMESTAMP\", \"signature\": \"invalid_garbage\"}],
    \"rules\": []
  }"
done
# Hub ApprovalWorker should penalize reputation.

echo -e "\n✅ BFT Attack Simulation complete."
