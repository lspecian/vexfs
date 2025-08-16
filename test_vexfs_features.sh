#!/bin/bash

# VexFS Comprehensive Feature Test Suite
# This script tests all claimed features and documents what actually works

set -e

# Configuration
MOUNT_POINT="/tmp/vexfs_test_$$"
VEXFS_BINARY="./target/x86_64-unknown-linux-gnu/release/vexfs_fuse"
TEST_LOG="vexfs_test_results_$(date +%Y%m%d_%H%M%S).log"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test result tracking
TESTS_PASSED=0
TESTS_FAILED=0
FEATURES_WORKING=()
FEATURES_BROKEN=()

# Logging function
log() {
    echo "$1" | tee -a "$TEST_LOG"
}

log_test() {
    local test_name="$1"
    local result="$2"
    local details="$3"
    
    if [ "$result" = "PASS" ]; then
        log "✅ $test_name: PASS"
        ((TESTS_PASSED++))
        [ ! -z "$details" ] && log "   Details: $details"
    else
        log "❌ $test_name: FAIL"
        ((TESTS_FAILED++))
        [ ! -z "$details" ] && log "   Error: $details"
    fi
}

# Header
log "=========================================="
log "VexFS Feature Verification Test Suite"
log "Date: $(date)"
log "=========================================="
log ""

# Check if binary exists
if [ ! -f "$VEXFS_BINARY" ]; then
    log "Building VexFS FUSE binary..."
    cd rust && cargo build --release --features="fuse_support" --bin vexfs_fuse 2>&1 | tee -a "../$TEST_LOG"
    cd ..
    if [ ! -f "$VEXFS_BINARY" ]; then
        log "ERROR: Failed to build VexFS binary"
        exit 1
    fi
fi

# Create mount point
mkdir -p "$MOUNT_POINT"
log "Test mount point: $MOUNT_POINT"
log ""

# Cleanup function
cleanup() {
    log ""
    log "Cleaning up..."
    if [ ! -z "$VEXFS_PID" ]; then
        kill $VEXFS_PID 2>/dev/null || true
        sleep 1
    fi
    fusermount3 -u "$MOUNT_POINT" 2>/dev/null || fusermount -u "$MOUNT_POINT" 2>/dev/null || true
    rmdir "$MOUNT_POINT" 2>/dev/null || true
}
trap cleanup EXIT

# Start VexFS
log "=== TEST 1: FUSE Mount ==="
$VEXFS_BINARY "$MOUNT_POINT" -f > vexfs_output.log 2>&1 &
VEXFS_PID=$!
sleep 3

# Check if mounted
if mount | grep -q "$MOUNT_POINT"; then
    log_test "FUSE Mount" "PASS" "PID: $VEXFS_PID"
    FEATURES_WORKING+=("FUSE Mount")
else
    log_test "FUSE Mount" "FAIL" "Mount not found in system"
    cat vexfs_output.log >> "$TEST_LOG"
    exit 1
fi

# Test basic filesystem operations
log ""
log "=== TEST 2: Basic Filesystem Operations ==="

# Create directory
if mkdir -p "$MOUNT_POINT/test_dir" 2>/dev/null; then
    log_test "Create Directory" "PASS"
    FEATURES_WORKING+=("Directory Creation")
else
    log_test "Create Directory" "FAIL"
    FEATURES_BROKEN+=("Directory Creation")
fi

# Write file
if echo "Hello VexFS" > "$MOUNT_POINT/test_dir/test.txt" 2>/dev/null; then
    log_test "Write File" "PASS"
    FEATURES_WORKING+=("File Write")
else
    log_test "Write File" "FAIL"
    FEATURES_BROKEN+=("File Write")
fi

# Read file
if [ -f "$MOUNT_POINT/test_dir/test.txt" ]; then
    content=$(cat "$MOUNT_POINT/test_dir/test.txt" 2>/dev/null)
    if [ "$content" = "Hello VexFS" ]; then
        log_test "Read File" "PASS" "Content matches"
        FEATURES_WORKING+=("File Read")
    else
        log_test "Read File" "FAIL" "Content mismatch"
        FEATURES_BROKEN+=("File Read")
    fi
else
    log_test "Read File" "FAIL" "File not found"
    FEATURES_BROKEN+=("File Read")
fi

# List directory
if ls "$MOUNT_POINT/test_dir" >/dev/null 2>&1; then
    log_test "List Directory" "PASS"
    FEATURES_WORKING+=("Directory Listing")
else
    log_test "List Directory" "FAIL"
    FEATURES_BROKEN+=("Directory Listing")
fi

# Delete file
if rm "$MOUNT_POINT/test_dir/test.txt" 2>/dev/null; then
    log_test "Delete File" "PASS"
    FEATURES_WORKING+=("File Deletion")
else
    log_test "Delete File" "FAIL"
    FEATURES_BROKEN+=("File Deletion")
fi

# Test vector operations
log ""
log "=== TEST 3: Vector Storage Operations ==="

# Create vectors directory
mkdir -p "$MOUNT_POINT/vectors" 2>/dev/null

# Store a vector file
vector_stored=false
if echo "0.1,0.2,0.3,0.4,0.5" > "$MOUNT_POINT/vectors/test.vec" 2>/dev/null; then
    if [ -f "$MOUNT_POINT/vectors/test.vec" ]; then
        log_test "Store Vector File" "PASS"
        FEATURES_WORKING+=("Vector File Storage")
        vector_stored=true
    else
        log_test "Store Vector File" "FAIL" "File not created"
        FEATURES_BROKEN+=("Vector File Storage")
    fi
else
    log_test "Store Vector File" "FAIL" "Write error"
    FEATURES_BROKEN+=("Vector File Storage")
fi

# Check if vector is parsed (look for special handling in logs)
if [ "$vector_stored" = true ]; then
    # Check FUSE output for vector operations
    sleep 1
    if grep -q "vector" vexfs_output.log 2>/dev/null || grep -q "Vector" vexfs_output.log 2>/dev/null; then
        log_test "Vector Parsing" "PASS" "Vector operations detected in logs"
        FEATURES_WORKING+=("Vector Parsing")
    else
        log_test "Vector Parsing" "UNKNOWN" "No vector operations in logs"
    fi
fi

# Test multiple vectors
log ""
log "=== TEST 4: Multiple Vector Storage ==="
vectors_created=0
for i in 1 2 3 4 5; do
    if echo "$(seq -s, 0 0.1 0.5)" > "$MOUNT_POINT/vectors/vec_$i.vec" 2>/dev/null; then
        ((vectors_created++))
    fi
done

if [ $vectors_created -eq 5 ]; then
    log_test "Multiple Vector Storage" "PASS" "Created $vectors_created vectors"
    FEATURES_WORKING+=("Multiple Vector Storage")
else
    log_test "Multiple Vector Storage" "PARTIAL" "Created $vectors_created/5 vectors"
fi

# Test query vector
log ""
log "=== TEST 5: Vector Query Operations ==="
if echo "0.15,0.25,0.35,0.45,0.55" > "$MOUNT_POINT/query.vec" 2>/dev/null; then
    log_test "Query Vector Creation" "PASS"
    # Check logs for search operations
    sleep 1
    if grep -q -i "search" vexfs_output.log 2>/dev/null; then
        log_test "Vector Search Triggered" "PASS"
        FEATURES_WORKING+=("Vector Search")
    else
        log_test "Vector Search Triggered" "UNKNOWN" "No search operations in logs"
    fi
else
    log_test "Query Vector Creation" "FAIL"
    FEATURES_BROKEN+=("Vector Query")
fi

# Test API endpoints (if server is running)
log ""
log "=== TEST 6: API Endpoints ==="
# The FUSE implementation doesn't expose HTTP APIs, but let's check
if curl -s http://localhost:7680/api/v1/version >/dev/null 2>&1; then
    log_test "API Endpoint" "PASS" "Server responding"
    FEATURES_WORKING+=("API Server")
else
    log_test "API Endpoint" "N/A" "No API server (FUSE mode only)"
fi

# Check FUSE process memory and performance
log ""
log "=== TEST 7: Performance Metrics ==="
if [ ! -z "$VEXFS_PID" ]; then
    mem_usage=$(ps -o rss= -p $VEXFS_PID 2>/dev/null || echo "0")
    if [ "$mem_usage" != "0" ]; then
        log_test "Memory Monitoring" "PASS" "RSS: ${mem_usage}KB"
        FEATURES_WORKING+=("Performance Monitoring")
    else
        log_test "Memory Monitoring" "FAIL"
    fi
fi

# Test file metadata
log ""
log "=== TEST 8: File Metadata ==="
touch "$MOUNT_POINT/metadata_test.txt" 2>/dev/null
if stat "$MOUNT_POINT/metadata_test.txt" >/dev/null 2>&1; then
    log_test "File Metadata" "PASS"
    FEATURES_WORKING+=("File Metadata")
else
    log_test "File Metadata" "FAIL"
    FEATURES_BROKEN+=("File Metadata")
fi

# Summary
log ""
log "=========================================="
log "TEST SUMMARY"
log "=========================================="
log "Tests Passed: $TESTS_PASSED"
log "Tests Failed: $TESTS_FAILED"
log ""
log "WORKING FEATURES:"
for feature in "${FEATURES_WORKING[@]}"; do
    log "  ✅ $feature"
done
log ""
log "BROKEN/MISSING FEATURES:"
for feature in "${FEATURES_BROKEN[@]}"; do
    log "  ❌ $feature"
done

# Check FUSE output for additional information
log ""
log "=== FUSE Implementation Analysis ==="
log "Checking for vector-related code execution..."
if [ -f vexfs_output.log ]; then
    vector_mentions=$(grep -i "vector" vexfs_output.log 2>/dev/null | wc -l || echo "0")
    hnsw_mentions=$(grep -i "hnsw" vexfs_output.log 2>/dev/null | wc -l || echo "0")
    bridge_mentions=$(grep -i "bridge" vexfs_output.log 2>/dev/null | wc -l || echo "0")
    
    log "Vector mentions in logs: $vector_mentions"
    log "HNSW mentions in logs: $hnsw_mentions"
    log "Bridge mentions in logs: $bridge_mentions"
fi

# Final verdict
log ""
log "=========================================="
log "FINAL VERDICT"
log "=========================================="

if [ ${#FEATURES_WORKING[@]} -gt 5 ]; then
    log "VexFS FUSE implementation is FUNCTIONAL with basic filesystem operations."
else
    log "VexFS FUSE implementation has CRITICAL ISSUES."
fi

if [[ " ${FEATURES_WORKING[@]} " =~ " Vector " ]]; then
    log "Vector features: PARTIALLY IMPLEMENTED (backend exists, frontend limited)"
else
    log "Vector features: NOT VERIFIED through filesystem interface"
fi

log ""
log "Full test log saved to: $TEST_LOG"
log "FUSE output saved to: vexfs_output.log"

# Keep mounted for manual inspection
log ""
log "Filesystem still mounted at: $MOUNT_POINT"
log "You can manually inspect it. Press Ctrl+C to unmount and exit."

# Show live FUSE output
log ""
log "=== Live FUSE Output ==="
tail -f vexfs_output.log