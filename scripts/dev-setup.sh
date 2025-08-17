#!/bin/bash

# VexFS Quick Development Setup Script
# One-command setup for development environment

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "╔══════════════════════════════════════════════════════╗"
echo "║          VexFS Development Environment Setup          ║"
echo "╚══════════════════════════════════════════════════════╝"
echo

# Configuration
VEXFS_ROOT=$(pwd)
VEXFS_DATA="$HOME/.vexfs"
VEXFS_MOUNT="$HOME/vexfs-mount"
VEXFS_CONFIG="$HOME/.vexfs/config"
VEXFS_LOGS="$HOME/.vexfs/logs"

# Function to check dependencies
check_dependency() {
    local cmd=$1
    local install_msg=$2
    if ! command -v "$cmd" &> /dev/null; then
        echo -e "${YELLOW}⚠ $cmd not found${NC}"
        echo "  Install with: $install_msg"
        return 1
    fi
    return 0
}

# Function to setup directory structure
setup_directories() {
    echo -e "${BLUE}Setting up VexFS directories...${NC}"
    mkdir -p "$VEXFS_DATA"
    mkdir -p "$VEXFS_MOUNT"
    mkdir -p "$VEXFS_CONFIG"
    mkdir -p "$VEXFS_LOGS"
    echo -e "${GREEN}✓ Directories created${NC}"
}

# Function to build components
build_components() {
    echo -e "${BLUE}Building VexFS components...${NC}"
    
    # Build FUSE filesystem
    echo "  Building FUSE filesystem..."
    (cd rust && cargo build --release --features fuse_support --bin vexfs_fuse) || {
        echo -e "${RED}Failed to build FUSE${NC}"
        exit 1
    }
    
    # Build API server
    echo "  Building API server..."
    (cd rust && cargo build --release --features server --bin vexfs_unified_server) || {
        echo -e "${RED}Failed to build API server${NC}"
        exit 1
    }
    
    # Build tools
    echo "  Building tools..."
    if [ -d "tools" ]; then
        (cd tools && make) || echo -e "${YELLOW}Tools build skipped${NC}"
    fi
    
    echo -e "${GREEN}✓ Build complete${NC}"
}

# Function to create config files
create_configs() {
    echo -e "${BLUE}Creating configuration files...${NC}"
    
    # Create FUSE config
    cat > "$VEXFS_CONFIG/fuse.conf" << EOF
# VexFS FUSE Configuration
mount_point=$VEXFS_MOUNT
allow_other=false
debug=false
max_threads=4
cache_size=1048576
vector_dimensions=384
EOF
    
    # Create API config
    cat > "$VEXFS_CONFIG/api.conf" << EOF
# VexFS API Server Configuration
host=0.0.0.0
port=7680
allow_anonymous=true
jwt_secret=dev-secret-change-in-production
api_key_1=dev-key:admin::1000
dashboard_path=$VEXFS_ROOT/vexfs-dashboard/build
EOF
    
    # Create development environment file
    cat > "$VEXFS_CONFIG/dev.env" << EOF
# VexFS Development Environment
export VEXFS_ROOT=$VEXFS_ROOT
export VEXFS_DATA=$VEXFS_DATA
export VEXFS_MOUNT=$VEXFS_MOUNT
export VEXFS_CONFIG=$VEXFS_CONFIG
export VEXFS_LOGS=$VEXFS_LOGS
export VEXFS_API_URL=http://localhost:7680
export VEXFS_API_KEY=dev-key
export RUST_LOG=info
export ALLOW_ANONYMOUS=true
EOF
    
    echo -e "${GREEN}✓ Configuration files created${NC}"
}

# Function to create helper scripts
create_helpers() {
    echo -e "${BLUE}Creating helper scripts...${NC}"
    
    # Start script
    cat > "$VEXFS_CONFIG/start-vexfs.sh" << 'EOF'
#!/bin/bash
source ~/.vexfs/config/dev.env

echo "Starting VexFS services..."

# Start FUSE filesystem
echo "Starting FUSE filesystem..."
fusermount3 -u "$VEXFS_MOUNT" 2>/dev/null || true
"$VEXFS_ROOT/rust/target/release/vexfs_fuse" "$VEXFS_MOUNT" > "$VEXFS_LOGS/fuse.log" 2>&1 &
echo $! > "$VEXFS_LOGS/fuse.pid"

# Start API server
echo "Starting API server..."
"$VEXFS_ROOT/rust/target/release/vexfs_unified_server" > "$VEXFS_LOGS/api.log" 2>&1 &
echo $! > "$VEXFS_LOGS/api.pid"

sleep 3

# Check status
if mountpoint -q "$VEXFS_MOUNT"; then
    echo "✓ FUSE filesystem mounted at $VEXFS_MOUNT"
else
    echo "✗ FUSE mount failed"
fi

if curl -s http://localhost:7680/health > /dev/null 2>&1; then
    echo "✓ API server running at http://localhost:7680"
else
    echo "✗ API server failed to start"
fi

echo
echo "VexFS is ready!"
echo "  Mount: $VEXFS_MOUNT"
echo "  API: http://localhost:7680"
echo "  Logs: $VEXFS_LOGS"
EOF
    
    # Stop script
    cat > "$VEXFS_CONFIG/stop-vexfs.sh" << 'EOF'
#!/bin/bash
source ~/.vexfs/config/dev.env

echo "Stopping VexFS services..."

# Stop API server
if [ -f "$VEXFS_LOGS/api.pid" ]; then
    kill $(cat "$VEXFS_LOGS/api.pid") 2>/dev/null || true
    rm "$VEXFS_LOGS/api.pid"
    echo "✓ API server stopped"
fi

# Unmount FUSE
fusermount3 -u "$VEXFS_MOUNT" 2>/dev/null || true

# Stop FUSE process
if [ -f "$VEXFS_LOGS/fuse.pid" ]; then
    kill $(cat "$VEXFS_LOGS/fuse.pid") 2>/dev/null || true
    rm "$VEXFS_LOGS/fuse.pid"
    echo "✓ FUSE filesystem stopped"
fi

echo "VexFS stopped"
EOF
    
    # Status script
    cat > "$VEXFS_CONFIG/status-vexfs.sh" << 'EOF'
#!/bin/bash
source ~/.vexfs/config/dev.env

echo "VexFS Status:"
echo "============="

# Check FUSE
if mountpoint -q "$VEXFS_MOUNT" 2>/dev/null; then
    echo "✓ FUSE: Mounted at $VEXFS_MOUNT"
    df -h "$VEXFS_MOUNT" | tail -1
else
    echo "✗ FUSE: Not mounted"
fi

# Check API
if curl -s http://localhost:7680/health > /dev/null 2>&1; then
    echo "✓ API: Running at http://localhost:7680"
    curl -s http://localhost:7680/metrics | jq '.collections_count' 2>/dev/null || echo "  Collections: N/A"
else
    echo "✗ API: Not running"
fi

# Check processes
echo
echo "Processes:"
ps aux | grep -E "vexfs_(fuse|unified)" | grep -v grep || echo "  No VexFS processes running"

# Check logs
echo
echo "Recent logs:"
if [ -f "$VEXFS_LOGS/api.log" ]; then
    echo "  API: $(tail -1 $VEXFS_LOGS/api.log)"
fi
if [ -f "$VEXFS_LOGS/fuse.log" ]; then
    echo "  FUSE: $(tail -1 $VEXFS_LOGS/fuse.log)"
fi
EOF
    
    # Test script
    cat > "$VEXFS_CONFIG/test-vexfs.sh" << 'EOF'
#!/bin/bash
source ~/.vexfs/config/dev.env

echo "Testing VexFS..."

# Test FUSE
echo -n "1. FUSE write test... "
echo "test data" > "$VEXFS_MOUNT/test.txt" 2>/dev/null && echo "✓" || echo "✗"

echo -n "2. FUSE read test... "
cat "$VEXFS_MOUNT/test.txt" > /dev/null 2>&1 && echo "✓" || echo "✗"

echo -n "3. Vector storage test... "
echo "0.1,0.2,0.3" > "$VEXFS_MOUNT/test.vec" 2>/dev/null && echo "✓" || echo "✗"

# Test API
echo -n "4. API health check... "
curl -s http://localhost:7680/health > /dev/null 2>&1 && echo "✓" || echo "✗"

echo -n "5. API create collection... "
curl -s -X POST http://localhost:7680/api/v1/collections \
    -H "Content-Type: application/json" \
    -d '{"name": "test", "metadata": {"dimension": 384}}' > /dev/null 2>&1 && echo "✓" || echo "✗"

echo
echo "Test complete!"
EOF
    
    chmod +x "$VEXFS_CONFIG"/*.sh
    
    echo -e "${GREEN}✓ Helper scripts created${NC}"
}

# Function to install systemd service (optional)
create_systemd_service() {
    echo -e "${BLUE}Creating systemd service (optional)...${NC}"
    
    cat > "$VEXFS_CONFIG/vexfs.service" << EOF
[Unit]
Description=VexFS Vector Filesystem
After=network.target

[Service]
Type=forking
User=$USER
WorkingDirectory=$VEXFS_ROOT
Environment="VEXFS_ROOT=$VEXFS_ROOT"
Environment="VEXFS_DATA=$VEXFS_DATA"
Environment="VEXFS_MOUNT=$VEXFS_MOUNT"
Environment="RUST_LOG=info"
ExecStart=$VEXFS_CONFIG/start-vexfs.sh
ExecStop=$VEXFS_CONFIG/stop-vexfs.sh
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
    
    echo -e "${YELLOW}To install as systemd service:${NC}"
    echo "  sudo cp $VEXFS_CONFIG/vexfs.service /etc/systemd/system/"
    echo "  sudo systemctl daemon-reload"
    echo "  sudo systemctl enable vexfs"
    echo "  sudo systemctl start vexfs"
}

# Function to create Docker setup
create_docker_setup() {
    echo -e "${BLUE}Creating Docker development setup...${NC}"
    
    cat > "$VEXFS_ROOT/docker-compose.dev.yml" << 'EOF'
version: '3.8'

services:
  vexfs-dev:
    build:
      context: .
      dockerfile: Dockerfile.dev
    volumes:
      - .:/workspace
      - vexfs-data:/data
      - vexfs-mount:/mnt/vexfs
    ports:
      - "7680:7680"  # API server
      - "3000:3000"  # Dashboard
    environment:
      - RUST_LOG=info
      - ALLOW_ANONYMOUS=true
    privileged: true
    devices:
      - /dev/fuse
    cap_add:
      - SYS_ADMIN
    command: /workspace/dev-setup.sh --docker

volumes:
  vexfs-data:
  vexfs-mount:
EOF
    
    cat > "$VEXFS_ROOT/Dockerfile.dev" << 'EOF'
FROM rust:1.75

# Install dependencies
RUN apt-get update && apt-get install -y \
    fuse3 \
    libfuse3-dev \
    build-essential \
    pkg-config \
    curl \
    jq \
    nodejs \
    npm \
    && rm -rf /var/lib/apt/lists/*

# Install Rust tools
RUN rustup component add rustfmt clippy

WORKDIR /workspace

# Entry point
CMD ["bash"]
EOF
    
    echo -e "${GREEN}✓ Docker setup created${NC}"
}

# Main setup flow
main() {
    echo -e "${BLUE}Checking dependencies...${NC}"
    
    # Check required dependencies
    local missing_deps=0
    check_dependency "cargo" "curl https://sh.rustup.rs -sSf | sh" || ((missing_deps++))
    check_dependency "fusermount3" "apt install fuse3" || ((missing_deps++))
    check_dependency "curl" "apt install curl" || ((missing_deps++))
    
    if [ $missing_deps -gt 0 ]; then
        echo -e "${YELLOW}Please install missing dependencies first${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Dependencies OK${NC}"
    echo
    
    # Run setup steps
    setup_directories
    build_components
    create_configs
    create_helpers
    create_systemd_service
    create_docker_setup
    
    # Create aliases
    echo -e "${BLUE}Adding shell aliases...${NC}"
    
    cat >> ~/.bashrc << 'EOF'

# VexFS aliases
alias vexfs-start='~/.vexfs/config/start-vexfs.sh'
alias vexfs-stop='~/.vexfs/config/stop-vexfs.sh'
alias vexfs-status='~/.vexfs/config/status-vexfs.sh'
alias vexfs-test='~/.vexfs/config/test-vexfs.sh'
alias vexfs-logs='tail -f ~/.vexfs/logs/*.log'
alias vexfs-mount='cd ~/vexfs-mount'
EOF
    
    echo -e "${GREEN}✓ Aliases added to ~/.bashrc${NC}"
    echo
    
    # Final instructions
    echo "╔══════════════════════════════════════════════════════╗"
    echo "║              Setup Complete!                          ║"
    echo "╚══════════════════════════════════════════════════════╝"
    echo
    echo -e "${GREEN}VexFS development environment is ready!${NC}"
    echo
    echo "Quick Start:"
    echo "  1. Source the environment: source ~/.vexfs/config/dev.env"
    echo "  2. Start VexFS: ~/.vexfs/config/start-vexfs.sh"
    echo "  3. Test it: ~/.vexfs/config/test-vexfs.sh"
    echo
    echo "Or use aliases (after reloading shell):"
    echo "  vexfs-start   - Start all services"
    echo "  vexfs-stop    - Stop all services"
    echo "  vexfs-status  - Check status"
    echo "  vexfs-test    - Run tests"
    echo "  vexfs-logs    - View logs"
    echo "  vexfs-mount   - Go to mount directory"
    echo
    echo "Docker development:"
    echo "  docker-compose -f docker-compose.dev.yml up"
    echo
    echo "Files created:"
    echo "  Config: ~/.vexfs/config/"
    echo "  Logs: ~/.vexfs/logs/"
    echo "  Mount: ~/vexfs-mount/"
    echo
    
    # Optionally start services
    read -p "Start VexFS services now? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        source "$VEXFS_CONFIG/dev.env"
        "$VEXFS_CONFIG/start-vexfs.sh"
    fi
}

# Handle Docker mode
if [ "$1" = "--docker" ]; then
    echo "Running in Docker mode..."
    setup_directories
    build_components
    exec "$VEXFS_CONFIG/start-vexfs.sh"
else
    main
fi