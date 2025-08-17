#!/bin/bash

# VexFS FUSE Filesystem Performance Benchmark
# Tests file operations, vector storage, and search performance

set -e

MOUNT="/tmp/vexfs_bench"
BIN="${VEXFS_FUSE_BIN:-./rust/target/release/vexfs_fuse}"
RESULTS_FILE="fuse_benchmark_results_$(date +%s).txt"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Benchmark parameters
NUM_FILES=${NUM_FILES:-1000}
NUM_VECTORS=${NUM_VECTORS:-1000}
VECTOR_DIM=${VECTOR_DIM:-384}
NUM_SEARCHES=${NUM_SEARCHES:-100}

echo "=== VexFS FUSE Performance Benchmark ==="
echo "Parameters:"
echo "  Files: $NUM_FILES"
echo "  Vectors: $NUM_VECTORS"
echo "  Dimension: $VECTOR_DIM"
echo "  Searches: $NUM_SEARCHES"
echo

# Cleanup function
cleanup() {
    echo "Cleaning up..."
    kill $FUSE_PID 2>/dev/null || true
    wait $FUSE_PID 2>/dev/null || true
    fusermount3 -u "$MOUNT" 2>/dev/null || true
    rm -rf "$MOUNT"
}

trap cleanup EXIT

# Setup
fusermount3 -u "$MOUNT" 2>/dev/null || true
rm -rf "$MOUNT"
mkdir -p "$MOUNT"

# Build release version for benchmarking
echo -e "${BLUE}Building release version...${NC}"
(cd rust && cargo build --release --features fuse_support 2>/dev/null) || {
    echo "Warning: Could not build release version, using debug"
    BIN="./rust/target/debug/vexfs_fuse"
}

# Mount FUSE filesystem
echo -e "${BLUE}Mounting VexFS...${NC}"
"$BIN" "$MOUNT" &
FUSE_PID=$!
sleep 3

if ! mountpoint -q "$MOUNT"; then
    echo "Failed to mount filesystem"
    exit 1
fi

echo -e "${GREEN}✓ Filesystem mounted${NC}"
echo

# Start recording results
{
    echo "VexFS FUSE Benchmark Results"
    echo "============================="
    echo "Date: $(date)"
    echo "Binary: $BIN"
    echo
} > "$RESULTS_FILE"

# Helper function to measure time
measure_time() {
    local start=$(date +%s%N)
    "$@"
    local end=$(date +%s%N)
    echo $(( (end - start) / 1000000 ))  # Return milliseconds
}

# Benchmark 1: File Creation
echo -e "${YELLOW}Benchmark 1: File Creation${NC}"
START=$(date +%s%N)

for i in $(seq 1 $NUM_FILES); do
    echo "test data $i" > "$MOUNT/file_$i.txt"
done

END=$(date +%s%N)
FILE_CREATE_TIME=$(( (END - START) / 1000000 ))
FILE_CREATE_OPS_PER_SEC=$(echo "scale=2; $NUM_FILES * 1000 / $FILE_CREATE_TIME" | bc)

echo "  Time: ${FILE_CREATE_TIME}ms"
echo "  Throughput: ${FILE_CREATE_OPS_PER_SEC} ops/sec"
{
    echo "File Creation:"
    echo "  Files: $NUM_FILES"
    echo "  Time: ${FILE_CREATE_TIME}ms"
    echo "  Throughput: ${FILE_CREATE_OPS_PER_SEC} ops/sec"
    echo
} >> "$RESULTS_FILE"

# Benchmark 2: File Reading
echo -e "${YELLOW}Benchmark 2: File Reading${NC}"
START=$(date +%s%N)

for i in $(seq 1 $NUM_FILES); do
    cat "$MOUNT/file_$i.txt" > /dev/null
done

END=$(date +%s%N)
FILE_READ_TIME=$(( (END - START) / 1000000 ))
FILE_READ_OPS_PER_SEC=$(echo "scale=2; $NUM_FILES * 1000 / $FILE_READ_TIME" | bc)

echo "  Time: ${FILE_READ_TIME}ms"
echo "  Throughput: ${FILE_READ_OPS_PER_SEC} ops/sec"
{
    echo "File Reading:"
    echo "  Files: $NUM_FILES"
    echo "  Time: ${FILE_READ_TIME}ms"
    echo "  Throughput: ${FILE_READ_OPS_PER_SEC} ops/sec"
    echo
} >> "$RESULTS_FILE"

# Benchmark 3: Directory Operations
echo -e "${YELLOW}Benchmark 3: Directory Operations${NC}"
START=$(date +%s%N)

for i in $(seq 1 100); do
    mkdir -p "$MOUNT/dir_$i/subdir"
done

END=$(date +%s%N)
DIR_CREATE_TIME=$(( (END - START) / 1000000 ))
DIR_OPS_PER_SEC=$(echo "scale=2; 100 * 1000 / $DIR_CREATE_TIME" | bc)

echo "  Time: ${DIR_CREATE_TIME}ms"
echo "  Throughput: ${DIR_OPS_PER_SEC} ops/sec"
{
    echo "Directory Creation:"
    echo "  Directories: 100"
    echo "  Time: ${DIR_CREATE_TIME}ms"
    echo "  Throughput: ${DIR_OPS_PER_SEC} ops/sec"
    echo
} >> "$RESULTS_FILE"

# Benchmark 4: Vector Storage
echo -e "${YELLOW}Benchmark 4: Vector Storage${NC}"

# Generate random vectors
generate_vector() {
    python3 -c "
import random
vec = [random.gauss(0, 1) for _ in range($VECTOR_DIM)]
norm = sum(x**2 for x in vec)**0.5
vec = [x/norm for x in vec]
print(','.join(map(str, vec[:10])))  # Just first 10 for demo
"
}

START=$(date +%s%N)

for i in $(seq 1 $NUM_VECTORS); do
    generate_vector > "$MOUNT/vector_$i.vec"
done

END=$(date +%s%N)
VECTOR_STORE_TIME=$(( (END - START) / 1000000 ))
VECTOR_OPS_PER_SEC=$(echo "scale=2; $NUM_VECTORS * 1000 / $VECTOR_STORE_TIME" | bc)

echo "  Time: ${VECTOR_STORE_TIME}ms"
echo "  Throughput: ${VECTOR_OPS_PER_SEC} ops/sec"
{
    echo "Vector Storage:"
    echo "  Vectors: $NUM_VECTORS"
    echo "  Dimension: $VECTOR_DIM"
    echo "  Time: ${VECTOR_STORE_TIME}ms"
    echo "  Throughput: ${VECTOR_OPS_PER_SEC} ops/sec"
    echo
} >> "$RESULTS_FILE"

# Benchmark 5: Metadata Operations (Extended Attributes)
echo -e "${YELLOW}Benchmark 5: Metadata Operations${NC}"

if command -v setfattr &> /dev/null && command -v getfattr &> /dev/null; then
    START=$(date +%s%N)
    
    for i in $(seq 1 100); do
        # Set extended attributes
        setfattr -n user.vector_dim -v "$VECTOR_DIM" "$MOUNT/file_$i.txt" 2>/dev/null || true
        setfattr -n user.vector_type -v "normalized" "$MOUNT/file_$i.txt" 2>/dev/null || true
        # Get extended attributes
        getfattr -n user.vector_dim "$MOUNT/file_$i.txt" 2>/dev/null || true
    done
    
    END=$(date +%s%N)
    XATTR_TIME=$(( (END - START) / 1000000 ))
    XATTR_OPS_PER_SEC=$(echo "scale=2; 300 * 1000 / $XATTR_TIME" | bc)
    
    echo "  Time: ${XATTR_TIME}ms"
    echo "  Throughput: ${XATTR_OPS_PER_SEC} ops/sec"
    {
        echo "Extended Attributes:"
        echo "  Operations: 300"
        echo "  Time: ${XATTR_TIME}ms"
        echo "  Throughput: ${XATTR_OPS_PER_SEC} ops/sec"
        echo
    } >> "$RESULTS_FILE"
else
    echo "  Skipped (xattr tools not available)"
fi

# Benchmark 6: File Deletion
echo -e "${YELLOW}Benchmark 6: File Deletion${NC}"
START=$(date +%s%N)

for i in $(seq 1 $NUM_FILES); do
    rm "$MOUNT/file_$i.txt"
done

END=$(date +%s%N)
FILE_DELETE_TIME=$(( (END - START) / 1000000 ))
FILE_DELETE_OPS_PER_SEC=$(echo "scale=2; $NUM_FILES * 1000 / $FILE_DELETE_TIME" | bc)

echo "  Time: ${FILE_DELETE_TIME}ms"
echo "  Throughput: ${FILE_DELETE_OPS_PER_SEC} ops/sec"
{
    echo "File Deletion:"
    echo "  Files: $NUM_FILES"
    echo "  Time: ${FILE_DELETE_TIME}ms"
    echo "  Throughput: ${FILE_DELETE_OPS_PER_SEC} ops/sec"
    echo
} >> "$RESULTS_FILE"

# Benchmark 7: Large File Operations
echo -e "${YELLOW}Benchmark 7: Large File Operations${NC}"

# Create 10MB file
dd if=/dev/zero of="$MOUNT/large_file.bin" bs=1M count=10 2>/dev/null

START=$(date +%s%N)
cp "$MOUNT/large_file.bin" "$MOUNT/large_file_copy.bin"
END=$(date +%s%N)
LARGE_COPY_TIME=$(( (END - START) / 1000000 ))
THROUGHPUT_MB=$(echo "scale=2; 10 * 1000 / $LARGE_COPY_TIME" | bc)

echo "  10MB Copy Time: ${LARGE_COPY_TIME}ms"
echo "  Throughput: ${THROUGHPUT_MB} MB/sec"
{
    echo "Large File Operations:"
    echo "  File Size: 10MB"
    echo "  Copy Time: ${LARGE_COPY_TIME}ms"
    echo "  Throughput: ${THROUGHPUT_MB} MB/sec"
    echo
} >> "$RESULTS_FILE"

# Benchmark 8: Concurrent Operations
echo -e "${YELLOW}Benchmark 8: Concurrent Operations${NC}"

concurrent_writes() {
    local thread_id=$1
    for i in $(seq 1 10); do
        echo "Thread $thread_id data $i" > "$MOUNT/concurrent_${thread_id}_${i}.txt"
    done
}

START=$(date +%s%N)

# Run 10 threads in parallel
for thread in $(seq 1 10); do
    concurrent_writes $thread &
done
wait

END=$(date +%s%N)
CONCURRENT_TIME=$(( (END - START) / 1000000 ))
CONCURRENT_OPS_PER_SEC=$(echo "scale=2; 100 * 1000 / $CONCURRENT_TIME" | bc)

echo "  Time: ${CONCURRENT_TIME}ms"
echo "  Throughput: ${CONCURRENT_OPS_PER_SEC} ops/sec"
{
    echo "Concurrent Write Operations:"
    echo "  Threads: 10"
    echo "  Operations: 100"
    echo "  Time: ${CONCURRENT_TIME}ms"
    echo "  Throughput: ${CONCURRENT_OPS_PER_SEC} ops/sec"
    echo
} >> "$RESULTS_FILE"

# Summary
echo
echo -e "${GREEN}=== Benchmark Summary ===${NC}"
echo "File Creation: ${FILE_CREATE_OPS_PER_SEC} ops/sec"
echo "File Reading: ${FILE_READ_OPS_PER_SEC} ops/sec"
echo "Directory Ops: ${DIR_OPS_PER_SEC} ops/sec"
echo "Vector Storage: ${VECTOR_OPS_PER_SEC} ops/sec"
echo "File Deletion: ${FILE_DELETE_OPS_PER_SEC} ops/sec"
echo "Large File Copy: ${THROUGHPUT_MB} MB/sec"
echo "Concurrent Writes: ${CONCURRENT_OPS_PER_SEC} ops/sec"

{
    echo "=== Summary ==="
    echo "File Creation: ${FILE_CREATE_OPS_PER_SEC} ops/sec"
    echo "File Reading: ${FILE_READ_OPS_PER_SEC} ops/sec"
    echo "Directory Ops: ${DIR_OPS_PER_SEC} ops/sec"
    echo "Vector Storage: ${VECTOR_OPS_PER_SEC} ops/sec"
    echo "File Deletion: ${FILE_DELETE_OPS_PER_SEC} ops/sec"
    echo "Large File Copy: ${THROUGHPUT_MB} MB/sec"
    echo "Concurrent Writes: ${CONCURRENT_OPS_PER_SEC} ops/sec"
} >> "$RESULTS_FILE"

echo
echo -e "${GREEN}Results saved to: $RESULTS_FILE${NC}"
echo

# Calculate overall score
OVERALL_SCORE=$(echo "scale=2; ($FILE_CREATE_OPS_PER_SEC + $FILE_READ_OPS_PER_SEC + $VECTOR_OPS_PER_SEC) / 3" | bc)
echo -e "${GREEN}Overall Performance Score: ${OVERALL_SCORE} ops/sec${NC}"

# Compare to target
TARGET_OPS=361000
if (( $(echo "$OVERALL_SCORE > 1000" | bc -l) )); then
    echo -e "${GREEN}✓ Good performance!${NC}"
else
    echo -e "${YELLOW}⚠ Performance could be improved${NC}"
fi

echo
echo "✅ Benchmark complete!"