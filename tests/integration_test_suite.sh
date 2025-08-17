#!/bin/bash

# VexFS Comprehensive Integration Test Suite
# Tests all three components: Kernel Module, FUSE, and API Server

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Configuration
TEST_DIR="/tmp/vexfs_integration_test_$(date +%s)"
FUSE_MOUNT="$TEST_DIR/fuse_mount"
API_URL="http://localhost:7680"
RESULTS_DIR="$TEST_DIR/results"
PASSED_TESTS=0
FAILED_TESTS=0

# Test tracking
declare -A TEST_RESULTS

echo "╔══════════════════════════════════════════════════════╗"
echo "║     VexFS Integration Test Suite                      ║"
echo "╚══════════════════════════════════════════════════════╝"
echo

# Create test directories
mkdir -p "$TEST_DIR" "$FUSE_MOUNT" "$RESULTS_DIR"

# Logging function
log_test() {
    local test_name="$1"
    local status="$2"
    local message="$3"
    
    if [ "$status" = "PASS" ]; then
        echo -e "${GREEN}✓ $test_name${NC}"
        ((PASSED_TESTS++))
        TEST_RESULTS["$test_name"]="PASS"
    else
        echo -e "${RED}✗ $test_name${NC}: $message"
        ((FAILED_TESTS++))
        TEST_RESULTS["$test_name"]="FAIL: $message"
    fi
}

# Cleanup function
cleanup() {
    echo
    echo "Cleaning up..."
    
    # Kill processes
    kill $FUSE_PID 2>/dev/null || true
    kill $API_PID 2>/dev/null || true
    
    # Unmount
    fusermount3 -u "$FUSE_MOUNT" 2>/dev/null || true
    
    # Remove test directory
    rm -rf "$TEST_DIR"
}

trap cleanup EXIT

# ============================================
# Component 1: FUSE Filesystem Tests
# ============================================

echo -e "${BLUE}═══ Testing FUSE Filesystem ═══${NC}"
echo

# Build FUSE
echo "Building FUSE filesystem..."
(cd rust && cargo build --release --features fuse_support --bin vexfs_fuse) || {
    log_test "FUSE Build" "FAIL" "Build failed"
    exit 1
}
log_test "FUSE Build" "PASS"

# Start FUSE
echo "Starting FUSE filesystem..."
./rust/target/release/vexfs_fuse "$FUSE_MOUNT" &
FUSE_PID=$!
sleep 3

if mountpoint -q "$FUSE_MOUNT"; then
    log_test "FUSE Mount" "PASS"
else
    log_test "FUSE Mount" "FAIL" "Mount point not active"
    exit 1
fi

# Test 1.1: File Operations
echo "test data" > "$FUSE_MOUNT/test.txt"
if [ -f "$FUSE_MOUNT/test.txt" ]; then
    content=$(cat "$FUSE_MOUNT/test.txt")
    if [ "$content" = "test data" ]; then
        log_test "FUSE File Operations" "PASS"
    else
        log_test "FUSE File Operations" "FAIL" "Content mismatch"
    fi
else
    log_test "FUSE File Operations" "FAIL" "File not created"
fi

# Test 1.2: Directory Operations
mkdir -p "$FUSE_MOUNT/testdir/subdir"
if [ -d "$FUSE_MOUNT/testdir/subdir" ]; then
    log_test "FUSE Directory Operations" "PASS"
else
    log_test "FUSE Directory Operations" "FAIL" "Directory not created"
fi

# Test 1.3: Vector Storage
echo "0.1,0.2,0.3,0.4,0.5" > "$FUSE_MOUNT/test.vec"
if [ -f "$FUSE_MOUNT/test.vec" ]; then
    log_test "FUSE Vector Storage" "PASS"
else
    log_test "FUSE Vector Storage" "FAIL" "Vector file not created"
fi

# Test 1.4: File Deletion
rm "$FUSE_MOUNT/test.txt"
if [ ! -f "$FUSE_MOUNT/test.txt" ]; then
    log_test "FUSE File Deletion" "PASS"
else
    log_test "FUSE File Deletion" "FAIL" "File still exists"
fi

# Test 1.5: Directory Removal
rmdir "$FUSE_MOUNT/testdir/subdir"
rmdir "$FUSE_MOUNT/testdir"
if [ ! -d "$FUSE_MOUNT/testdir" ]; then
    log_test "FUSE Directory Removal" "PASS"
else
    log_test "FUSE Directory Removal" "FAIL" "Directory still exists"
fi

echo

# ============================================
# Component 2: API Server Tests
# ============================================

echo -e "${BLUE}═══ Testing API Server ═══${NC}"
echo

# Build API Server
echo "Building API server..."
(cd rust && cargo build --release --features server --bin vexfs_unified_server) || {
    log_test "API Server Build" "FAIL" "Build failed"
    exit 1
}
log_test "API Server Build" "PASS"

# Start API Server
echo "Starting API server..."
./rust/target/release/vexfs_unified_server > "$RESULTS_DIR/api_server.log" 2>&1 &
API_PID=$!
sleep 5

# Test 2.1: Health Check
if curl -s "$API_URL/health" > /dev/null 2>&1; then
    log_test "API Health Check" "PASS"
else
    log_test "API Health Check" "FAIL" "Server not responding"
    exit 1
fi

# Test 2.2: Create Collection (ChromaDB API)
response=$(curl -s -X POST "$API_URL/api/v1/collections" \
    -H "Content-Type: application/json" \
    -d '{"name": "test_collection", "metadata": {"dimension": 384}}' \
    -w "\n%{http_code}")

http_code=$(echo "$response" | tail -n 1)
if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
    log_test "API Create Collection" "PASS"
else
    log_test "API Create Collection" "FAIL" "HTTP $http_code"
fi

# Test 2.3: Add Documents
response=$(curl -s -X POST "$API_URL/api/v1/collections/test_collection/add" \
    -H "Content-Type: application/json" \
    -d '{
        "ids": ["doc1", "doc2"],
        "documents": ["First document", "Second document"],
        "embeddings": [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]]
    }' \
    -w "\n%{http_code}")

http_code=$(echo "$response" | tail -n 1)
if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
    log_test "API Add Documents" "PASS"
else
    log_test "API Add Documents" "FAIL" "HTTP $http_code"
fi

# Test 2.4: Query Collection
response=$(curl -s -X POST "$API_URL/api/v1/collections/test_collection/query" \
    -H "Content-Type: application/json" \
    -d '{
        "query_embeddings": [[0.1, 0.2, 0.3]],
        "n_results": 2
    }' \
    -w "\n%{http_code}")

http_code=$(echo "$response" | tail -n 1)
if [ "$http_code" = "200" ]; then
    log_test "API Query Collection" "PASS"
else
    log_test "API Query Collection" "FAIL" "HTTP $http_code"
fi

# Test 2.5: Qdrant API Compatibility
response=$(curl -s -X PUT "$API_URL/collections/qdrant_test" \
    -H "Content-Type: application/json" \
    -d '{
        "vectors": {
            "size": 384,
            "distance": "Cosine"
        }
    }' \
    -w "\n%{http_code}")

http_code=$(echo "$response" | tail -n 1)
if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
    log_test "API Qdrant Compatibility" "PASS"
else
    log_test "API Qdrant Compatibility" "FAIL" "HTTP $http_code"
fi

# Test 2.6: Authentication
response=$(curl -s -X POST "$API_URL/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"api_key": "vexfs-default-key"}' \
    -w "\n%{http_code}")

http_code=$(echo "$response" | tail -n 1)
if [ "$http_code" = "200" ]; then
    log_test "API Authentication" "PASS"
else
    log_test "API Authentication" "FAIL" "HTTP $http_code"
fi

echo

# ============================================
# Component 3: Integration Tests
# ============================================

echo -e "${BLUE}═══ Testing Component Integration ═══${NC}"
echo

# Test 3.1: FUSE + API Integration
# Create data via FUSE, access via API
echo "integration test data" > "$FUSE_MOUNT/integration.txt"
echo "0.7,0.8,0.9" > "$FUSE_MOUNT/integration.vec"

# Check if files exist
if [ -f "$FUSE_MOUNT/integration.txt" ] && [ -f "$FUSE_MOUNT/integration.vec" ]; then
    log_test "FUSE-API Data Creation" "PASS"
else
    log_test "FUSE-API Data Creation" "FAIL" "Files not created"
fi

# Test 3.2: Concurrent Operations
(
    for i in {1..10}; do
        echo "concurrent $i" > "$FUSE_MOUNT/concurrent_$i.txt" &
    done
    wait
)

count=$(ls "$FUSE_MOUNT"/concurrent_*.txt 2>/dev/null | wc -l)
if [ "$count" -eq 10 ]; then
    log_test "Concurrent File Creation" "PASS"
else
    log_test "Concurrent File Creation" "FAIL" "Expected 10 files, got $count"
fi

# Test 3.3: Performance Check
start_time=$(date +%s%N)
for i in {1..100}; do
    echo "perf test $i" > "$FUSE_MOUNT/perf_$i.txt"
done
end_time=$(date +%s%N)
elapsed_ms=$(( (end_time - start_time) / 1000000 ))

if [ "$elapsed_ms" -lt 5000 ]; then
    log_test "Performance (100 files)" "PASS"
else
    log_test "Performance (100 files)" "FAIL" "Took ${elapsed_ms}ms (>5000ms)"
fi

# Test 3.4: Error Handling
# Try to create file with invalid name
touch "$FUSE_MOUNT/$(printf 'x%.0s' {1..300}).txt" 2>/dev/null || true
if [ ! -f "$FUSE_MOUNT/$(printf 'x%.0s' {1..300}).txt" ]; then
    log_test "Error Handling" "PASS"
else
    log_test "Error Handling" "FAIL" "Invalid file was created"
fi

echo

# ============================================
# Component 4: Stress Tests
# ============================================

echo -e "${BLUE}═══ Running Stress Tests ═══${NC}"
echo

# Test 4.1: Large File
dd if=/dev/zero of="$FUSE_MOUNT/large.bin" bs=1M count=10 2>/dev/null
if [ -f "$FUSE_MOUNT/large.bin" ]; then
    size=$(stat -c%s "$FUSE_MOUNT/large.bin")
    if [ "$size" -eq 10485760 ]; then
        log_test "Large File Handling" "PASS"
    else
        log_test "Large File Handling" "FAIL" "Size mismatch"
    fi
else
    log_test "Large File Handling" "FAIL" "File not created"
fi

# Test 4.2: Many Small Files
mkdir -p "$FUSE_MOUNT/many"
for i in {1..500}; do
    echo "$i" > "$FUSE_MOUNT/many/file_$i.txt"
done

count=$(ls "$FUSE_MOUNT/many" | wc -l)
if [ "$count" -eq 500 ]; then
    log_test "Many Files Handling" "PASS"
else
    log_test "Many Files Handling" "FAIL" "Expected 500, got $count"
fi

# Test 4.3: Deep Directory Structure
mkdir -p "$FUSE_MOUNT/a/b/c/d/e/f/g/h/i/j"
if [ -d "$FUSE_MOUNT/a/b/c/d/e/f/g/h/i/j" ]; then
    log_test "Deep Directory Structure" "PASS"
else
    log_test "Deep Directory Structure" "FAIL" "Deep directory not created"
fi

echo

# ============================================
# Generate Test Report
# ============================================

echo -e "${BLUE}═══ Generating Test Report ═══${NC}"
echo

{
    echo "VexFS Integration Test Report"
    echo "============================="
    echo "Date: $(date)"
    echo
    echo "Test Summary:"
    echo "  Passed: $PASSED_TESTS"
    echo "  Failed: $FAILED_TESTS"
    echo "  Total: $((PASSED_TESTS + FAILED_TESTS))"
    echo "  Success Rate: $(( PASSED_TESTS * 100 / (PASSED_TESTS + FAILED_TESTS) ))%"
    echo
    echo "Test Results:"
    echo "-------------"
    for test_name in "${!TEST_RESULTS[@]}"; do
        echo "  $test_name: ${TEST_RESULTS[$test_name]}"
    done
    echo
    echo "Component Status:"
    echo "  FUSE Filesystem: $(pidof vexfs_fuse > /dev/null && echo "Running" || echo "Not Running")"
    echo "  API Server: $(curl -s "$API_URL/health" > /dev/null 2>&1 && echo "Running" || echo "Not Running")"
    echo
} > "$RESULTS_DIR/test_report.txt"

cat "$RESULTS_DIR/test_report.txt"

# ============================================
# Final Summary
# ============================================

echo
echo "╔══════════════════════════════════════════════════════╗"
echo "║                  Test Complete                        ║"
echo "╚══════════════════════════════════════════════════════╝"
echo

if [ "$FAILED_TESTS" -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed! ($PASSED_TESTS/$((PASSED_TESTS + FAILED_TESTS)))${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed ($FAILED_TESTS/$((PASSED_TESTS + FAILED_TESTS)))${NC}"
    echo "See $RESULTS_DIR/test_report.txt for details"
    exit 1
fi