#!/bin/bash

# VexFS Comprehensive Benchmark Suite Runner
# Runs all performance benchmarks and generates a report

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Configuration
REPORT_DIR="benchmark_reports_$(date +%Y%m%d_%H%M%S)"
API_SERVER_URL="${VEXFS_API_URL:-http://localhost:7680}"
VEXFS_FUSE_BIN="${VEXFS_FUSE_BIN:-./rust/target/release/vexfs_fuse}"

echo "╔══════════════════════════════════════════════════════╗"
echo "║        VexFS Comprehensive Performance Suite          ║"
echo "╚══════════════════════════════════════════════════════╝"
echo

# Create report directory
mkdir -p "$REPORT_DIR"

# Function to check if server is running
check_server() {
    curl -s "$API_SERVER_URL/health" > /dev/null 2>&1
}

# Function to start API server if needed
start_api_server() {
    if ! check_server; then
        echo -e "${YELLOW}Starting VexFS API server...${NC}"
        ./rust/target/release/vexfs_unified_server > "$REPORT_DIR/server.log" 2>&1 &
        SERVER_PID=$!
        sleep 5
        
        if ! check_server; then
            echo -e "${RED}Failed to start API server${NC}"
            exit 1
        fi
        echo -e "${GREEN}✓ API server started (PID: $SERVER_PID)${NC}"
    else
        echo -e "${GREEN}✓ API server already running${NC}"
    fi
}

# Build release binaries
echo -e "${BLUE}═══ Building Release Binaries ═══${NC}"
echo

echo "Building API server..."
(cd rust && cargo build --release --features server --bin vexfs_unified_server) || {
    echo -e "${YELLOW}Warning: Could not build API server${NC}"
}

echo "Building FUSE filesystem..."
(cd rust && cargo build --release --features fuse_support --bin vexfs_fuse) || {
    echo -e "${YELLOW}Warning: Could not build FUSE filesystem${NC}"
}

echo "Building benchmarks..."
(cd rust && cargo build --release --bin performance_benchmark) || {
    echo -e "${YELLOW}Warning: Could not build performance benchmark${NC}"
}

echo
echo -e "${GREEN}✓ Build complete${NC}"
echo

# 1. HNSW Index Benchmark
echo -e "${BLUE}═══ Benchmark 1: HNSW Index Performance ═══${NC}"
echo

if [ -f "./rust/target/release/performance_benchmark" ]; then
    ./rust/target/release/performance_benchmark > "$REPORT_DIR/hnsw_benchmark.txt" 2>&1 || {
        echo -e "${YELLOW}HNSW benchmark failed${NC}"
    }
    
    # Extract key metrics
    if [ -f "$REPORT_DIR/hnsw_benchmark.txt" ]; then
        echo "HNSW Results:"
        grep -E "Insert:|Search:|Memory:" "$REPORT_DIR/hnsw_benchmark.txt" | head -5 || true
    fi
else
    echo -e "${YELLOW}Skipping HNSW benchmark (binary not found)${NC}"
fi
echo

# 2. API Server Benchmarks
echo -e "${BLUE}═══ Benchmark 2: API Server Performance ═══${NC}"
echo

# Start server if needed
start_api_server

# Check if Python is available
if command -v python3 &> /dev/null; then
    # Install required packages if needed
    pip3 install -q numpy requests 2>/dev/null || true
    
    # Run vector benchmarks
    python3 benchmarks/vector_benchmark.py \
        --url "$API_SERVER_URL" \
        --sizes 100 500 1000 \
        --output "$REPORT_DIR/api_benchmark.json" || {
        echo -e "${YELLOW}API benchmark failed${NC}"
    }
else
    echo -e "${YELLOW}Skipping API benchmarks (Python not available)${NC}"
fi
echo

# 3. FUSE Filesystem Benchmarks
echo -e "${BLUE}═══ Benchmark 3: FUSE Filesystem Performance ═══${NC}"
echo

if [ -f "$VEXFS_FUSE_BIN" ]; then
    NUM_FILES=500 NUM_VECTORS=100 bash benchmarks/fuse_benchmark.sh || {
        echo -e "${YELLOW}FUSE benchmark failed${NC}"
    }
    
    # Move results to report directory
    mv fuse_benchmark_results_*.txt "$REPORT_DIR/" 2>/dev/null || true
else
    echo -e "${YELLOW}Skipping FUSE benchmarks (binary not found)${NC}"
fi
echo

# 4. Memory and Resource Usage
echo -e "${BLUE}═══ Benchmark 4: Resource Usage Analysis ═══${NC}"
echo

{
    echo "Resource Usage Report"
    echo "===================="
    echo "Date: $(date)"
    echo
    
    # Check API server memory
    if [ ! -z "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "API Server (PID: $SERVER_PID):"
        ps aux | grep "$SERVER_PID" | grep -v grep | awk '{print "  Memory: " $4 "%, CPU: " $3 "%"}'
    fi
    
    # System resources
    echo
    echo "System Resources:"
    free -h | grep "Mem:" | awk '{print "  Memory: " $3 "/" $2 " used"}'
    uptime | awk -F'load average:' '{print "  Load Average:" $2}'
    
    # Disk usage
    echo
    echo "Disk Usage:"
    df -h . | tail -1 | awk '{print "  " $3 "/" $2 " used (" $5 ")"}'
    
} > "$REPORT_DIR/resource_usage.txt"

cat "$REPORT_DIR/resource_usage.txt"
echo

# 5. Generate Summary Report
echo -e "${BLUE}═══ Generating Summary Report ═══${NC}"
echo

{
    echo "VexFS Performance Benchmark Summary"
    echo "===================================="
    echo "Date: $(date)"
    echo "Report Directory: $REPORT_DIR"
    echo
    
    echo "Configuration:"
    echo "  API Server: $API_SERVER_URL"
    echo "  FUSE Binary: $VEXFS_FUSE_BIN"
    echo
    
    echo "Benchmark Results:"
    echo "------------------"
    
    # HNSW results
    if [ -f "$REPORT_DIR/hnsw_benchmark.txt" ]; then
        echo
        echo "HNSW Index Performance:"
        grep -E "Overall improvement|ops/second" "$REPORT_DIR/hnsw_benchmark.txt" | head -3 || echo "  No results"
    fi
    
    # API results
    if [ -f "$REPORT_DIR/api_benchmark.json" ]; then
        echo
        echo "API Server Performance:"
        python3 -c "
import json
with open('$REPORT_DIR/api_benchmark.json') as f:
    data = json.load(f)
    results = data.get('results', [])
    if results:
        avg_throughput = sum(r['throughput'] for r in results) / len(results)
        print(f'  Average Throughput: {avg_throughput:.2f} ops/sec')
        print(f'  Total Operations: {sum(r[\"num_operations\"] for r in results)}')
" 2>/dev/null || echo "  No results"
    fi
    
    # FUSE results
    FUSE_RESULT=$(ls "$REPORT_DIR"/fuse_benchmark_results_*.txt 2>/dev/null | head -1)
    if [ ! -z "$FUSE_RESULT" ]; then
        echo
        echo "FUSE Filesystem Performance:"
        grep "Overall Performance Score:" "$FUSE_RESULT" || echo "  No results"
    fi
    
    echo
    echo "Performance vs Target:"
    echo "----------------------"
    echo "Target: 361,000 ops/sec"
    echo "Status: Measurement in progress"
    
    echo
    echo "Files Generated:"
    echo "----------------"
    ls -la "$REPORT_DIR"
    
} > "$REPORT_DIR/summary.txt"

cat "$REPORT_DIR/summary.txt"

# Cleanup
if [ ! -z "$SERVER_PID" ]; then
    echo
    echo -e "${YELLOW}Stopping API server...${NC}"
    kill "$SERVER_PID" 2>/dev/null || true
fi

echo
echo "╔══════════════════════════════════════════════════════╗"
echo "║                  Benchmark Complete!                  ║"
echo "╚══════════════════════════════════════════════════════╝"
echo
echo -e "${GREEN}✓ All results saved to: $REPORT_DIR${NC}"
echo
echo "View reports:"
echo "  Summary: cat $REPORT_DIR/summary.txt"
echo "  HNSW: cat $REPORT_DIR/hnsw_benchmark.txt"
echo "  API: cat $REPORT_DIR/api_benchmark.json"
echo "  FUSE: cat $REPORT_DIR/fuse_benchmark_results_*.txt"
echo