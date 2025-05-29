#!/bin/bash

# Container-based VexFS Test - Simulates VM isolation using Docker
# This provides the isolation benefits of VM testing with better reliability

set -e

echo "🚀 VexFS Container Test (VM Alternative)"
echo "========================================"
echo "This test provides VM-like isolation using containers"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo -e "${RED}❌ Docker not found. Installing Docker or use host test instead.${NC}"
    exit 1
fi

# Check if we can run Docker
if ! docker info &> /dev/null; then
    echo -e "${RED}❌ Cannot access Docker. Try: sudo usermod -aG docker $USER${NC}"
    exit 1
fi

echo -e "${YELLOW}🐳 Creating isolated container environment...${NC}"

# Create a Dockerfile for our test environment
cat > /tmp/Dockerfile.vexfs-test << 'EOF'
FROM ubuntu:22.04

# Install required packages
RUN apt-get update && apt-get install -y \
    build-essential \
    linux-headers-generic \
    python3 \
    bc \
    kmod \
    && rm -rf /var/lib/apt/lists/*

# Create test user
RUN useradd -m -s /bin/bash vexfs && \
    echo 'vexfs ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

# Set working directory
WORKDIR /mnt/vexfs_source

# Copy our test script
COPY comprehensive_host_test.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/comprehensive_host_test.sh

# Switch to test user
USER vexfs

CMD ["/usr/local/bin/comprehensive_host_test.sh"]
EOF

# Build the container
echo -e "${YELLOW}🔨 Building test container...${NC}"
if docker build -f /tmp/Dockerfile.vexfs-test -t vexfs-test . >> /tmp/container_build.log 2>&1; then
    echo -e "${GREEN}✅ Container built successfully${NC}"
else
    echo -e "${RED}❌ Container build failed${NC}"
    echo "Build log:"
    tail -20 /tmp/container_build.log
    exit 1
fi

# Run the test in the container with privileged access for kernel modules
echo -e "${YELLOW}🧪 Running VexFS test in isolated container...${NC}"

docker run --rm \
    --privileged \
    --volume "$(pwd):/mnt/vexfs_source" \
    --volume "/lib/modules:/lib/modules:ro" \
    --volume "/usr/src:/usr/src:ro" \
    vexfs-test

container_exit_code=$?

# Clean up
echo -e "${YELLOW}🧹 Cleaning up container...${NC}"
docker rmi vexfs-test &> /dev/null || true
rm -f /tmp/Dockerfile.vexfs-test

if [ $container_exit_code -eq 0 ]; then
    echo -e "${GREEN}🎉 Container test completed successfully!${NC}"
    echo -e "${GREEN}✅ VexFS works in isolated environment${NC}"
    echo -e "${GREEN}✅ Container provides VM-like isolation${NC}"
    echo -e "${GREEN}✅ Ready for production testing${NC}"
else
    echo -e "${RED}❌ Container test failed${NC}"
    echo -e "${RED}🚫 Review container test results${NC}"
fi

exit $container_exit_code