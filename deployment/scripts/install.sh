#!/bin/bash
set -e

# VexFS Bare Metal Installation Script
# This script installs VexFS directly on a Linux server

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
VEXFS_USER="vexfs"
VEXFS_GROUP="vexfs"
VEXFS_HOME="/var/lib/vexfs"
VEXFS_LOG_DIR="/var/log/vexfs"
VEXFS_CONFIG_DIR="/etc/vexfs"
VEXFS_RUN_DIR="/run/vexfs"
VEXFS_BINARY="/usr/local/bin/vexfs_server"
SYSTEMD_SERVICE="/etc/systemd/system/vexfs.service"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

# Detect OS and package manager
detect_os() {
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        OS=$ID
        OS_VERSION=$VERSION_ID
    else
        log_error "Cannot detect operating system"
        exit 1
    fi
    
    log_info "Detected OS: $OS $OS_VERSION"
}

# Install system dependencies
install_dependencies() {
    log_info "Installing system dependencies..."
    
    case $OS in
        ubuntu|debian)
            apt-get update
            apt-get install -y curl wget ca-certificates openssl
            ;;
        centos|rhel|fedora)
            if command -v dnf >/dev/null 2>&1; then
                dnf install -y curl wget ca-certificates openssl
            else
                yum install -y curl wget ca-certificates openssl
            fi
            ;;
        *)
            log_warning "Unsupported OS: $OS. Please install dependencies manually."
            ;;
    esac
}

# Check system requirements
check_requirements() {
    log_info "Checking system requirements..."
    
    # Check minimum RAM (2GB)
    TOTAL_RAM=$(free -m | awk 'NR==2{printf "%.0f", $2}')
    if [[ $TOTAL_RAM -lt 2048 ]]; then
        log_warning "System has ${TOTAL_RAM}MB RAM. Minimum recommended: 2048MB"
    fi
    
    # Check available disk space (10GB)
    AVAILABLE_SPACE=$(df / | awk 'NR==2{printf "%.0f", $4/1024/1024}')
    if [[ $AVAILABLE_SPACE -lt 10 ]]; then
        log_warning "Available disk space: ${AVAILABLE_SPACE}GB. Minimum recommended: 10GB"
    fi
    
    # Check if systemd is available
    if ! command -v systemctl >/dev/null 2>&1; then
        log_error "systemd is required but not found"
        exit 1
    fi
    
    log_success "System requirements check completed"
}

# Create user and group
create_user() {
    log_info "Creating VexFS user and group..."
    
    if ! getent group $VEXFS_GROUP >/dev/null; then
        groupadd --system $VEXFS_GROUP
        log_success "Created group: $VEXFS_GROUP"
    else
        log_info "Group $VEXFS_GROUP already exists"
    fi
    
    if ! getent passwd $VEXFS_USER >/dev/null; then
        useradd --system --gid $VEXFS_GROUP --home-dir $VEXFS_HOME \
                --shell /bin/false --comment "VexFS service account" $VEXFS_USER
        log_success "Created user: $VEXFS_USER"
    else
        log_info "User $VEXFS_USER already exists"
    fi
}

# Create directories
create_directories() {
    log_info "Creating VexFS directories..."
    
    mkdir -p $VEXFS_HOME
    mkdir -p $VEXFS_LOG_DIR
    mkdir -p $VEXFS_CONFIG_DIR
    mkdir -p $VEXFS_RUN_DIR
    
    # Set ownership and permissions
    chown $VEXFS_USER:$VEXFS_GROUP $VEXFS_HOME
    chown $VEXFS_USER:$VEXFS_GROUP $VEXFS_LOG_DIR
    chown root:$VEXFS_GROUP $VEXFS_CONFIG_DIR
    chown $VEXFS_USER:$VEXFS_GROUP $VEXFS_RUN_DIR
    
    chmod 750 $VEXFS_HOME
    chmod 750 $VEXFS_LOG_DIR
    chmod 750 $VEXFS_CONFIG_DIR
    chmod 755 $VEXFS_RUN_DIR
    
    log_success "Created and configured directories"
}

# Build and install VexFS binary
install_binary() {
    log_info "Building and installing VexFS binary..."
    
    # Check if Rust is installed
    if ! command -v cargo >/dev/null 2>&1; then
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
    fi
    
    # Build VexFS
    cd "$PROJECT_ROOT"
    log_info "Building VexFS server (this may take several minutes)..."
    cargo build --release --features server --bin vexfs_server
    
    # Install binary
    cp target/release/vexfs_server $VEXFS_BINARY
    chown root:root $VEXFS_BINARY
    chmod 755 $VEXFS_BINARY
    
    log_success "VexFS binary installed to $VEXFS_BINARY"
}

# Create configuration file
create_config() {
    log_info "Creating VexFS configuration..."
    
    if [[ ! -f "$VEXFS_CONFIG_DIR/vexfs.conf" ]]; then
        cat > "$VEXFS_CONFIG_DIR/vexfs.conf" << 'EOF'
# VexFS Configuration File
# See documentation at: https://github.com/vexfs/vexfs

# Server configuration
PORT=8000
BIND_ADDRESS=127.0.0.1

# Data directory
VEXFS_DATA_DIR=/var/lib/vexfs

# Logging
VEXFS_LOG_LEVEL=info
RUST_LOG=info

# Performance tuning
VEXFS_MAX_CONNECTIONS=1000
VEXFS_REQUEST_TIMEOUT=30s

# Rate limiting
VEXFS_RATE_LIMIT_REQUESTS=100
VEXFS_RATE_LIMIT_WINDOW=60s

# Security
VEXFS_TLS_ENABLED=false
VEXFS_CORS_ENABLED=true

# Monitoring
VEXFS_METRICS_ENABLED=true
VEXFS_HEALTH_CHECK_ENABLED=true
EOF
        chown root:$VEXFS_GROUP "$VEXFS_CONFIG_DIR/vexfs.conf"
        chmod 640 "$VEXFS_CONFIG_DIR/vexfs.conf"
        log_success "Created configuration file"
    else
        log_info "Configuration file already exists"
    fi
}

# Create systemd service
create_service() {
    log_info "Creating systemd service..."
    
    cat > $SYSTEMD_SERVICE << EOF
[Unit]
Description=VexFS - Vector Extended File System Server
Documentation=https://github.com/vexfs/vexfs
After=network.target
Wants=network.target

[Service]
Type=exec
User=$VEXFS_USER
Group=$VEXFS_GROUP
ExecStart=$VEXFS_BINARY
ExecReload=/bin/kill -HUP \$MAINPID
Restart=always
RestartSec=5
TimeoutStartSec=30
TimeoutStopSec=30

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$VEXFS_HOME $VEXFS_LOG_DIR $VEXFS_RUN_DIR
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictRealtime=true
RestrictSUIDSGID=true
LockPersonality=true
MemoryDenyWriteExecute=true
RestrictNamespaces=true
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Environment
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1
EnvironmentFile=-$VEXFS_CONFIG_DIR/vexfs.conf

# Working directory
WorkingDirectory=$VEXFS_HOME

# Standard streams
StandardOutput=journal
StandardError=journal
SyslogIdentifier=vexfs

[Install]
WantedBy=multi-user.target
EOF
    
    systemctl daemon-reload
    log_success "Created systemd service"
}

# Setup log rotation
setup_logrotate() {
    log_info "Setting up log rotation..."
    
    cat > /etc/logrotate.d/vexfs << EOF
$VEXFS_LOG_DIR/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 640 $VEXFS_USER $VEXFS_GROUP
    postrotate
        systemctl reload vexfs || true
    endscript
}
EOF
    
    log_success "Log rotation configured"
}

# Enable and start service
start_service() {
    log_info "Enabling and starting VexFS service..."
    
    systemctl enable vexfs.service
    systemctl start vexfs.service
    
    # Wait a moment and check status
    sleep 2
    if systemctl is-active --quiet vexfs.service; then
        log_success "VexFS service started successfully"
    else
        log_error "Failed to start VexFS service"
        systemctl status vexfs.service
        exit 1
    fi
}

# Main installation function
main() {
    echo "========================================"
    echo "VexFS Installation Script"
    echo "========================================"
    echo
    
    check_root
    detect_os
    check_requirements
    install_dependencies
    create_user
    create_directories
    install_binary
    create_config
    create_service
    setup_logrotate
    start_service
    
    echo
    echo "========================================"
    log_success "VexFS installation completed successfully!"
    echo "========================================"
    echo
    echo "Configuration file: $VEXFS_CONFIG_DIR/vexfs.conf"
    echo "Data directory: $VEXFS_HOME"
    echo "Log directory: $VEXFS_LOG_DIR"
    echo
    echo "Service management:"
    echo "  Start:   sudo systemctl start vexfs"
    echo "  Stop:    sudo systemctl stop vexfs"
    echo "  Status:  sudo systemctl status vexfs"
    echo "  Logs:    sudo journalctl -u vexfs -f"
    echo
    echo "API endpoint: http://localhost:8000/api/v1"
    echo "Health check: curl http://localhost:8000/api/v1/version"
    echo
}

# Run main function
main "$@"