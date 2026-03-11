#!/bin/bash
export SKILL_FORGE_ENABLED=true
export OLLAMA_HOST=http://127.0.0.1:11434
export OLLAMA_MODEL=qwen3.5:9b
export API_SERVER_SECRET=dev_secret

# Start Samsara Hub
cd apps/samsara-hub
cargo run &
HUB_PID=$!
cd ../..

# Wait for Hub
sleep 3

# Start API Server
cd apps/api-server
cargo run &
API_PID=$!
cd ../..

# Wait for Server
sleep 5

echo "--- Starting Agent Forge Test ---"

# Send Chat Request
# We ask the agent to create a skill that returns a greeting.
curl -X POST http://127.0.0.1:3015/api/agent/chat/stream \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer dev_secret" \
     -d '{
       "prompt": "新しいスキル '\''greet_skill'\'' を作成してください。このスキルは引数として名前を受け取り、'\''Hello, [name] from Aiome Forge!'\'' という文字列を返すものです。forge_skill を使ってコードを書き、そのあと forge_test_run で確認し、最後に forge_publish してください。",
       "history": []
     }'

echo "--- Test Request Sent ---"

# Wait for build (it might take a while)
# We expect to see heartbeat events in the curl output.
# But curl -X POST ... stream will output them as they come.

# After first build, we run it again to check speedup.
# In a real test, we would wait for the first one to finish.
# For now, I'll just keep the servers running and wait for user's observation or my own logs.

# Kill servers on exit (manual)
# kill $HUB_PID $API_PID
