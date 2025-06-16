#!/bin/bash
set -e

# VexFS Uninstallation Script
# This script removes VexFS from a Linux server

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

# Confirm uninstallation
confirm_uninstall() {
    echo "========================================"
    echo "VexFS Uninstallation Script"
    echo "========================================"
    echo
    log_warning "This will completely remove VexFS from your system."
    log_warning "All data in $VEXFS_HOME will be preserved unless you choose to remove it."
    echo
    
    read -p "Are you sure you want to uninstall VexFS? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Uninstallation cancelled."
        exit 0
    fi
    
    echo
    read -p "Do you want to remove all VexFS data? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        REMOVE_DATA=true
        log_warning "Data will be removed!"
    else
        REMOVE_DATA=false
        log_info "Data will be preserved."
    fi
    echo
}

# Stop and disable service
stop_service() {
    log_info "Stopping VexFS service..."
    
    if systemctl is-active --quiet vexfs.service; then
        systemctl stop vexfs.service
        log_success "VexFS service stopped"
    else
        log_info "VexFS service is not running"
    fi
    
    if systemctl is-enabled --quiet vexfs.service; then
        systemctl disable vexfs.service
        log_success "VexFS service disabled"
    else
        log_info "VexFS service is not enabled"
    fi
}

# Remove systemd service
remove_service() {
    log_info "Removing systemd service..."
    
    if [[ -f $SYSTEMD_SERVICE ]]; then
        rm -f $SYSTEMD_SERVICE
        systemctl daemon-reload
        log_success "Systemd service removed"
    else
        log_info "Systemd service file not found"
    fi
}

# Remove binary
remove_binary() {
    log_info "Removing VexFS binary..."
    
    if [[ -f $VEXFS_BINARY ]]; then
        rm -f $VEXFS_BINARY
        log_success "VexFS binary removed"
    else
        log_info "VexFS binary not found"
    fi
}

# Remove configuration
remove_config() {
    log_info "Removing configuration files..."
    
    # Remove logrotate configuration
    if [[ -f /etc/logrotate.d/vexfs ]]; then
        rm -f /etc/logrotate.d/vexfs
        log_success "Logrotate configuration removed"
    fi
    
    # Remove configuration directory (preserve if contains user files)
    if [[ -d $VEXFS_CONFIG_DIR ]]; then
        if [[ $(ls -A $VEXFS_CONFIG_DIR 2>/dev/null | wc -l) -eq 1 ]] && [[ -f "$VEXFS_CONFIG_DIR/vexfs.conf" ]]; then
            # Only contains default config file
            rm -rf $VEXFS_CONFIG_DIR
            log_success "Configuration directory removed"
        else
            # Contains other files, just remove our config
            rm -f "$VEXFS_CONFIG_DIR/vexfs.conf"
            log_warning "Configuration directory preserved (contains other files)"
        fi
    fi
}

# Remove directories
remove_directories() {
    log_info "Removing VexFS directories..."
    
    # Remove run directory
    if [[ -d $VEXFS_RUN_DIR ]]; then
        rm -rf $VEXFS_RUN_DIR
        log_success "Run directory removed"
    fi
    
    # Remove log directory
    if [[ -d $VEXFS_LOG_DIR ]]; then
        rm -rf $VEXFS_LOG_DIR
        log_success "Log directory removed"
    fi
    
    # Remove data directory if requested
    if [[ $REMOVE_DATA == true ]] && [[ -d $VEXFS_HOME ]]; then
        rm -rf $VEXFS_HOME
        log_success "Data directory removed"
    elif [[ -d $VEXFS_HOME ]]; then
        log_info "Data directory preserved: $VEXFS_HOME"
    fi
}

# Remove user and group
remove_user() {
    log_info "Removing VexFS user and group..."
    
    if getent passwd $VEXFS_USER >/dev/null; then
        userdel $VEXFS_USER
        log_success "User $VEXFS_USER removed"
    else
        log_info "User $VEXFS_USER not found"
    fi
    
    if getent group $VEXFS_GROUP >/dev/null; then
        groupdel $VEXFS_GROUP
        log_success "Group $VEXFS_GROUP removed"
    else
        log_info "Group $VEXFS_GROUP not found"
    fi
}

# Clean up any remaining files
cleanup() {
    log_info "Performing final cleanup..."
    
    # Remove any remaining VexFS processes
    if pgrep -f vexfs_server >/dev/null; then
        log_warning "Killing remaining VexFS processes..."
        pkill -f vexfs_server || true
    fi
    
    # Remove systemd-tmpfiles configuration if it exists
    if [[ -f /usr/lib/tmpfiles.d/vexfs.conf ]]; then
        rm -f /usr/lib/tmpfiles.d/vexfs.conf
    fi
    
    log_success "Cleanup completed"
}

# Main uninstallation function
main() {
    check_root
    confirm_uninstall
    
    echo "Starting VexFS uninstallation..."
    echo
    
    stop_service
    remove_service
    remove_binary
    remove_config
    remove_directories
    remove_user
    cleanup
    
    echo
    echo "========================================"
    log_success "VexFS uninstallation completed successfully!"
    echo "========================================"
    echo
    
    if [[ $REMOVE_DATA == false ]] && [[ -d $VEXFS_HOME ]]; then
        echo "VexFS data has been preserved in: $VEXFS_HOME"
        echo "You can manually remove it with: sudo rm -rf $VEXFS_HOME"
        echo
    fi
    
    log_info "VexFS has been completely removed from your system."
}

# Run main function
main "$@"