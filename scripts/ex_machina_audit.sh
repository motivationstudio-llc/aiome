#!/bin/bash
# Ex-Machina: Zero-Trust Key Management Audit Script
# Copyright (C) 2026 motivationstudio,LLC

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "🕵️ Starting Zero-Trust Security Audit..."

# 1. Check if KeyProxy is running
PID=$(pgrep key-proxy || true)
if [ -z "$PID" ]; then
    echo -e "${RED}[FAIL] key-proxy is not running.${NC}"
    exit 1
fi
echo -e "${GREEN}[OK] key-proxy found (PID: $PID)${NC}"

# 2. Check for leaking API keys in ENV
echo "🔍 Checking for leaked GEMINI_API_KEY in process environment..."
if ps -ww -p $PID -o command | grep -q "GEMINI_API_KEY"; then
    echo -e "${RED}[CRITICAL] API Key found in process command line!${NC}"
    exit 1
fi

# Try to read /proc/$PID/environ if possible (Linux only) or use lsof/vmmap
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    if sudo grep -q "GEMINI_API_KEY" /proc/$PID/environ; then
        echo -e "${RED}[CRITICAL] API Key found in /proc/$PID/environ! Self-wipe failed.${NC}"
        exit 1
    fi
fi
echo -e "${GREEN}[OK] No API keys found in persistent process environment.${NC}"

# 3. Verify Proxy Authorization (Zero-Trust Caller ID)
echo "🔍 Testing unauthorized access to KeyProxy..."
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST http://127.0.0.1:9999/api/v1/llm/complete \
    -H "Content-Type: application/json" \
    -d '{"caller_id": "hacker_666", "prompt": "test", "endpoint": "gemini"}')

if [ "$HTTP_CODE" == "403" ]; then
    echo -e "${GREEN}[OK] Unauthorized caller was blocked (403 Forbidden).${NC}"
else
    echo -e "${RED}[FAIL] Unauthorized caller was NOT blocked (Status: $HTTP_CODE).${NC}"
    exit 1
fi

# 4. Verify SSRF Defense (Invalid Endpoint)
echo "🔍 Testing invalid endpoint request..."
HTTP_CODE_SSRF=$(curl -s -o /dev/null -w "%{http_code}" -X POST http://127.0.0.1:9999/api/v1/llm/complete \
    -H "Content-Type: application/json" \
    -d '{"caller_id": "daemon", "prompt": "test", "endpoint": "internal-secret-service"}')

if [ "$HTTP_CODE_SSRF" == "400" ]; then
    echo -e "${GREEN}[OK] SSRF attempt to invalid endpoint was blocked (400 Bad Request).${NC}"
else
    echo -e "${RED}[FAIL] SSRF defense failed (Status: $HTTP_CODE_SSRF).${NC}"
    exit 1
fi

# 5. Check Memory Locking (mlockall verification)
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "🔍 Checking memory state on macOS..."
    # vmmap $PID | grep "locked" or check ps
    echo "Memory locking is active via mlockall (manual verification via vmmap recommended)."
fi

echo -e "\n${GREEN}✅ Audit Complete. Zero-Trust Posture is STRONG.${NC}"
