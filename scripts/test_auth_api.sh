#!/bin/bash

# Test script for VexFS API authentication

set -e

echo "=== VexFS API Authentication Test ==="
echo

# Start the server in background
echo "Starting VexFS unified server..."
./rust/target/debug/vexfs_unified_server &
SERVER_PID=$!
sleep 3

# Check if server is running
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "❌ Server failed to start"
    exit 1
fi

echo "✅ Server started on PID $SERVER_PID"
echo

# Base URL
BASE_URL="http://localhost:7680"

# Test 1: Health check (public endpoint)
echo "Test 1: Health check (public endpoint)"
curl -s "$BASE_URL/health" | jq '.' || echo "Health check failed"
echo

# Test 2: Get collections without auth (should work with anonymous access)
echo "Test 2: List collections (anonymous)"
curl -s "$BASE_URL/api/v1/collections" | jq '.' || echo "List collections failed"
echo

# Test 3: Try to create collection without auth (should fail)
echo "Test 3: Create collection without auth (should fail)"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/v1/collections" \
  -H "Content-Type: application/json" \
  -d '{"name": "test_collection"}')
echo "$RESPONSE"
echo

# Test 4: Login with API key
echo "Test 4: Login with API key"
API_KEY="${VEXFS_API_KEY:-vexfs-default-key}"
TOKEN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"api_key\": \"$API_KEY\"}")
echo "$TOKEN_RESPONSE" | jq '.' || echo "$TOKEN_RESPONSE"

TOKEN=$(echo "$TOKEN_RESPONSE" | jq -r '.token' 2>/dev/null || echo "")
if [ -z "$TOKEN" ] || [ "$TOKEN" = "null" ]; then
    echo "❌ Failed to get token"
else
    echo "✅ Got token: ${TOKEN:0:50}..."
fi
echo

# Test 5: Create collection with auth
if [ ! -z "$TOKEN" ] && [ "$TOKEN" != "null" ]; then
    echo "Test 5: Create collection with auth"
    curl -s -X POST "$BASE_URL/api/v1/collections" \
      -H "Content-Type: application/json" \
      -H "Authorization: $TOKEN" \
      -d '{"name": "test_collection"}' | jq '.' || echo "Create collection failed"
    echo
fi

# Test 6: Verify token
if [ ! -z "$TOKEN" ] && [ "$TOKEN" != "null" ]; then
    echo "Test 6: Verify token"
    curl -s "$BASE_URL/auth/verify" \
      -H "Authorization: $TOKEN" | jq '.' || echo "Verify token failed"
    echo
fi

# Test 7: Use X-API-Key header directly
echo "Test 7: Use X-API-Key header"
curl -s "$BASE_URL/api/v1/collections" \
  -H "X-API-Key: $API_KEY" | jq '.' || echo "API key auth failed"
echo

# Cleanup
echo "Stopping server..."
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

echo "✅ Authentication test complete!"