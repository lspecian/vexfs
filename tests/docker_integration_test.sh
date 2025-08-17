#!/bin/bash

# VexFS Docker Integration Test
# Tests all components running in Docker containers

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "╔══════════════════════════════════════════════════════╗"
echo "║     VexFS Docker Integration Test                     ║"
echo "╚══════════════════════════════════════════════════════╝"
echo

# Check Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Docker is not installed${NC}"
    exit 1
fi

# Check docker-compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo -e "${RED}docker-compose is not installed${NC}"
    exit 1
fi

# Test results
PASSED=0
FAILED=0

# Log test result
log_test() {
    local name="$1"
    local status="$2"
    
    if [ "$status" = "PASS" ]; then
        echo -e "${GREEN}✓ $name${NC}"
        ((PASSED++))
    else
        echo -e "${RED}✗ $name${NC}"
        ((FAILED++))
    fi
}

# ============================================
# Build Docker Images
# ============================================

echo -e "${BLUE}Building Docker images...${NC}"

# Build API server image
docker build -t vexfs-api:test -f docker/Dockerfile.api . 2>/dev/null || {
    log_test "Docker API Build" "FAIL"
    exit 1
}
log_test "Docker API Build" "PASS"

# Build FUSE image
docker build -t vexfs-fuse:test -f docker/Dockerfile.fuse . 2>/dev/null || {
    log_test "Docker FUSE Build" "FAIL"
    exit 1
}
log_test "Docker FUSE Build" "PASS"

# Build dashboard image
docker build -t vexfs-dashboard:test -f docker/Dockerfile.dashboard . 2>/dev/null || {
    log_test "Docker Dashboard Build" "FAIL"
    exit 1
}
log_test "Docker Dashboard Build" "PASS"

echo

# ============================================
# Start Docker Compose Stack
# ============================================

echo -e "${BLUE}Starting Docker Compose stack...${NC}"

# Create docker-compose.test.yml
cat > docker-compose.test.yml << 'EOF'
version: '3.8'

services:
  vexfs-api:
    image: vexfs-api:test
    ports:
      - "7680:7680"
    environment:
      - VEXFS_HOST=0.0.0.0
      - VEXFS_PORT=7680
      - ALLOW_ANONYMOUS=true
      - JWT_SECRET=test-secret
    volumes:
      - vexfs-data:/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:7680/health"]
      interval: 5s
      timeout: 3s
      retries: 5

  vexfs-fuse:
    image: vexfs-fuse:test
    privileged: true
    devices:
      - /dev/fuse
    cap_add:
      - SYS_ADMIN
    volumes:
      - vexfs-mount:/mnt/vexfs
      - vexfs-data:/data
    depends_on:
      - vexfs-api

  vexfs-dashboard:
    image: vexfs-dashboard:test
    ports:
      - "3000:80"
    environment:
      - REACT_APP_API_URL=http://localhost:7680
    depends_on:
      - vexfs-api

volumes:
  vexfs-data:
  vexfs-mount:
EOF

# Start stack
docker-compose -f docker-compose.test.yml up -d 2>/dev/null || {
    log_test "Docker Compose Start" "FAIL"
    exit 1
}
log_test "Docker Compose Start" "PASS"

# Wait for services to be ready
echo "Waiting for services to be ready..."
sleep 10

echo

# ============================================
# Test API Server
# ============================================

echo -e "${BLUE}Testing API Server...${NC}"

# Health check
if curl -s http://localhost:7680/health > /dev/null 2>&1; then
    log_test "API Health Check" "PASS"
else
    log_test "API Health Check" "FAIL"
fi

# Create collection
response=$(curl -s -X POST http://localhost:7680/api/v1/collections \
    -H "Content-Type: application/json" \
    -d '{"name": "docker_test", "metadata": {"dimension": 384}}' \
    -w "\n%{http_code}")

http_code=$(echo "$response" | tail -n 1)
if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
    log_test "API Create Collection" "PASS"
else
    log_test "API Create Collection" "FAIL"
fi

# Add documents
response=$(curl -s -X POST http://localhost:7680/api/v1/collections/docker_test/add \
    -H "Content-Type: application/json" \
    -d '{
        "ids": ["doc1"],
        "documents": ["Test document"],
        "embeddings": [[0.1, 0.2, 0.3]]
    }' \
    -w "\n%{http_code}")

http_code=$(echo "$response" | tail -n 1)
if [ "$http_code" = "200" ]; then
    log_test "API Add Documents" "PASS"
else
    log_test "API Add Documents" "FAIL"
fi

echo

# ============================================
# Test FUSE Mount
# ============================================

echo -e "${BLUE}Testing FUSE Mount...${NC}"

# Check if FUSE container is running
if docker ps | grep -q vexfs-fuse; then
    log_test "FUSE Container Running" "PASS"
    
    # Create file in FUSE mount
    docker exec vexfs-fuse sh -c 'echo "test" > /mnt/vexfs/test.txt' 2>/dev/null
    if [ $? -eq 0 ]; then
        log_test "FUSE File Creation" "PASS"
    else
        log_test "FUSE File Creation" "FAIL"
    fi
    
    # Read file from FUSE mount
    content=$(docker exec vexfs-fuse cat /mnt/vexfs/test.txt 2>/dev/null)
    if [ "$content" = "test" ]; then
        log_test "FUSE File Read" "PASS"
    else
        log_test "FUSE File Read" "FAIL"
    fi
else
    log_test "FUSE Container Running" "FAIL"
fi

echo

# ============================================
# Test Dashboard
# ============================================

echo -e "${BLUE}Testing Dashboard...${NC}"

# Check if dashboard is accessible
if curl -s http://localhost:3000 | grep -q "VexFS" 2>/dev/null; then
    log_test "Dashboard Accessible" "PASS"
else
    log_test "Dashboard Accessible" "FAIL"
fi

echo

# ============================================
# Test Container Communication
# ============================================

echo -e "${BLUE}Testing Container Communication...${NC}"

# Test API from FUSE container
docker exec vexfs-fuse curl -s http://vexfs-api:7680/health > /dev/null 2>&1
if [ $? -eq 0 ]; then
    log_test "FUSE->API Communication" "PASS"
else
    log_test "FUSE->API Communication" "FAIL"
fi

# Test shared volume
docker exec vexfs-api sh -c 'echo "shared" > /data/shared.txt' 2>/dev/null
content=$(docker exec vexfs-fuse cat /data/shared.txt 2>/dev/null)
if [ "$content" = "shared" ]; then
    log_test "Shared Volume" "PASS"
else
    log_test "Shared Volume" "FAIL"
fi

echo

# ============================================
# Performance Test
# ============================================

echo -e "${BLUE}Running Performance Test...${NC}"

# Measure API response time
start_time=$(date +%s%N)
for i in {1..100}; do
    curl -s http://localhost:7680/health > /dev/null 2>&1
done
end_time=$(date +%s%N)
elapsed_ms=$(( (end_time - start_time) / 1000000 ))
avg_ms=$(( elapsed_ms / 100 ))

if [ "$avg_ms" -lt 50 ]; then
    log_test "API Response Time" "PASS"
else
    log_test "API Response Time" "FAIL"
fi

echo "Average response time: ${avg_ms}ms"
echo

# ============================================
# Resource Usage
# ============================================

echo -e "${BLUE}Checking Resource Usage...${NC}"

# Get container stats
docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}" | grep vexfs

echo

# ============================================
# Cleanup
# ============================================

echo -e "${BLUE}Cleaning up...${NC}"

# Stop and remove containers
docker-compose -f docker-compose.test.yml down -v 2>/dev/null

# Remove test images
docker rmi vexfs-api:test vexfs-fuse:test vexfs-dashboard:test 2>/dev/null || true

# Remove compose file
rm -f docker-compose.test.yml

echo

# ============================================
# Summary
# ============================================

echo "╔══════════════════════════════════════════════════════╗"
echo "║                  Test Summary                         ║"
echo "╚══════════════════════════════════════════════════════╝"
echo
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Total: $((PASSED + FAILED))"
echo

if [ "$FAILED" -eq 0 ]; then
    echo -e "${GREEN}✓ All Docker integration tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi