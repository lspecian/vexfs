#!/bin/bash
set -e

# VexFS Restore Script
# This script restores VexFS from a backup

# Configuration
VEXFS_DATA_DIR="/var/lib/vexfs"
VEXFS_CONFIG_DIR="/etc/vexfs"
BACKUP_BASE_DIR="/opt/vexfs/backups"

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

# Show help
show_help() {
    echo "VexFS Restore Script"
    echo
    echo "Usage: $0 [OPTIONS] <backup_file>"
    echo
    echo "Arguments:"
    echo "  backup_file           Path to the backup file to restore"
    echo
    echo "Options:"
    echo "  --data-dir DIR        VexFS data directory (default: /var/lib/vexfs)"
    echo "  --config-dir DIR      VexFS config directory (default: /etc/vexfs)"
    echo "  --backup-dir DIR      Backup base directory (default: /opt/vexfs/backups)"
    echo "  --dry-run             Show what would be restored without making changes"
    echo "  --force               Skip confirmation prompts"
    echo "  --help                Show this help message"
    echo
    echo "Examples:"
    echo "  $0 /opt/vexfs/backups/daily/vexfs_daily_20240529_120000.tar.gz"
    echo "  $0 --dry-run vexfs_daily_20240529_120000.tar.gz"
    echo "  $0 --force --data-dir /custom/path backup.tar.gz"
}

# Parse command line arguments
parse_args() {
    DRY_RUN=false
    FORCE=false
    BACKUP_FILE=""
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --data-dir)
                VEXFS_DATA_DIR="$2"
                shift 2
                ;;
            --config-dir)
                VEXFS_CONFIG_DIR="$2"
                shift 2
                ;;
            --backup-dir)
                BACKUP_BASE_DIR="$2"
                shift 2
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --force)
                FORCE=true
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            -*)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
            *)
                if [[ -z "$BACKUP_FILE" ]]; then
                    BACKUP_FILE="$1"
                else
                    log_error "Multiple backup files specified"
                    show_help
                    exit 1
                fi
                shift
                ;;
        esac
    done
    
    if [[ -z "$BACKUP_FILE" ]]; then
        log_error "No backup file specified"
        show_help
        exit 1
    fi
}

# Find backup file
find_backup_file() {
    # If backup file is a relative path, search in backup directories
    if [[ ! -f "$BACKUP_FILE" ]] && [[ "$BACKUP_FILE" != /* ]]; then
        log_info "Searching for backup file: $BACKUP_FILE"
        
        for backup_type in daily weekly monthly; do
            local search_path="$BACKUP_BASE_DIR/$backup_type/$BACKUP_FILE"
            if [[ -f "$search_path" ]]; then
                BACKUP_FILE="$search_path"
                log_info "Found backup file: $BACKUP_FILE"
                break
            fi
        done
    fi
    
    # Check if backup file exists
    if [[ ! -f "$BACKUP_FILE" ]]; then
        log_error "Backup file not found: $BACKUP_FILE"
        exit 1
    fi
    
    # Get absolute path
    BACKUP_FILE=$(realpath "$BACKUP_FILE")
    log_info "Using backup file: $BACKUP_FILE"
}

# Verify backup file
verify_backup() {
    log_info "Verifying backup file..."
    
    local info_file="${BACKUP_FILE%.*}.info"
    
    # Check if info file exists and verify checksum
    if [[ -f "$info_file" ]]; then
        local stored_checksum=$(grep "Checksum" "$info_file" | cut -d' ' -f3)
        local current_checksum=$(sha256sum "$BACKUP_FILE" | cut -d' ' -f1)
        
        if [[ "$stored_checksum" == "$current_checksum" ]]; then
            log_success "Backup checksum verified"
        else
            log_error "Backup checksum mismatch!"
            log_error "Stored: $stored_checksum"
            log_error "Current: $current_checksum"
            exit 1
        fi
    else
        log_warning "Backup info file not found, skipping checksum verification"
    fi
    
    # Test archive integrity
    if [[ "$BACKUP_FILE" == *.tar.gz ]]; then
        if tar -tzf "$BACKUP_FILE" >/dev/null 2>&1; then
            log_success "Backup archive integrity verified"
        else
            log_error "Backup archive is corrupted!"
            exit 1
        fi
    elif [[ "$BACKUP_FILE" == *.tar ]]; then
        if tar -tf "$BACKUP_FILE" >/dev/null 2>&1; then
            log_success "Backup archive integrity verified"
        else
            log_error "Backup archive is corrupted!"
            exit 1
        fi
    else
        log_error "Unsupported backup file format"
        exit 1
    fi
}

# Show backup information
show_backup_info() {
    log_info "Backup Information:"
    echo
    
    local info_file="${BACKUP_FILE%.*}.info"
    if [[ -f "$info_file" ]]; then
        cat "$info_file"
    else
        echo "Backup File: $BACKUP_FILE"
        echo "Size: $(du -h "$BACKUP_FILE" | cut -f1)"
        echo "Modified: $(stat -c %y "$BACKUP_FILE")"
    fi
    echo
}

# List backup contents
list_backup_contents() {
    log_info "Backup Contents:"
    echo
    
    if [[ "$BACKUP_FILE" == *.tar.gz ]]; then
        tar -tzf "$BACKUP_FILE" | head -20
    else
        tar -tf "$BACKUP_FILE" | head -20
    fi
    
    local total_files
    if [[ "$BACKUP_FILE" == *.tar.gz ]]; then
        total_files=$(tar -tzf "$BACKUP_FILE" | wc -l)
    else
        total_files=$(tar -tf "$BACKUP_FILE" | wc -l)
    fi
    
    if [[ $total_files -gt 20 ]]; then
        echo "... and $((total_files - 20)) more files"
    fi
    echo
    echo "Total files: $total_files"
    echo
}

# Check VexFS service status
check_service_status() {
    if systemctl is-active --quiet vexfs.service; then
        VEXFS_RUNNING=true
        log_warning "VexFS service is currently running"
    else
        VEXFS_RUNNING=false
        log_info "VexFS service is not running"
    fi
}

# Confirm restore operation
confirm_restore() {
    if [[ $FORCE == true ]]; then
        return 0
    fi
    
    echo "========================================"
    log_warning "RESTORE CONFIRMATION"
    echo "========================================"
    echo
    log_warning "This will restore VexFS from the backup and may overwrite existing data!"
    echo
    echo "Backup file: $BACKUP_FILE"
    echo "Data directory: $VEXFS_DATA_DIR"
    echo "Config directory: $VEXFS_CONFIG_DIR"
    echo
    
    if [[ $VEXFS_RUNNING == true ]]; then
        log_warning "VexFS service will be stopped during restore"
    fi
    
    echo
    read -p "Are you sure you want to proceed with the restore? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Restore cancelled."
        exit 0
    fi
    echo
}

# Create backup of current state
backup_current_state() {
    if [[ $DRY_RUN == true ]]; then
        log_info "[DRY RUN] Would create backup of current state"
        return 0
    fi
    
    log_info "Creating backup of current state..."
    
    local timestamp=$(date +"%Y%m%d_%H%M%S")
    local current_backup_dir="/tmp/vexfs_pre_restore_$timestamp"
    
    mkdir -p "$current_backup_dir"
    
    # Backup current data if it exists
    if [[ -d "$VEXFS_DATA_DIR" ]]; then
        cp -r "$VEXFS_DATA_DIR" "$current_backup_dir/data_backup"
        log_success "Current data backed up to: $current_backup_dir/data_backup"
    fi
    
    # Backup current config if it exists
    if [[ -d "$VEXFS_CONFIG_DIR" ]]; then
        cp -r "$VEXFS_CONFIG_DIR" "$current_backup_dir/config_backup"
        log_success "Current config backed up to: $current_backup_dir/config_backup"
    fi
    
    echo "Pre-restore backup created at: $current_backup_dir" > "$current_backup_dir/README.txt"
    echo "Created: $(date)" >> "$current_backup_dir/README.txt"
    echo "Original backup: $BACKUP_FILE" >> "$current_backup_dir/README.txt"
    
    log_success "Pre-restore backup created: $current_backup_dir"
}

# Perform the restore
perform_restore() {
    if [[ $DRY_RUN == true ]]; then
        log_info "[DRY RUN] Would perform restore operations"
        return 0
    fi
    
    local temp_dir="/tmp/vexfs_restore_$$"
    
    log_info "Extracting backup to temporary directory..."
    mkdir -p "$temp_dir"
    
    # Extract backup
    if [[ "$BACKUP_FILE" == *.tar.gz ]]; then
        tar -xzf "$BACKUP_FILE" -C "$temp_dir"
    else
        tar -xf "$BACKUP_FILE" -C "$temp_dir"
    fi
    
    # Stop VexFS service if running
    local service_was_stopped=false
    if [[ $VEXFS_RUNNING == true ]]; then
        log_info "Stopping VexFS service..."
        systemctl stop vexfs.service
        service_was_stopped=true
        sleep 2
    fi
    
    # Restore data directory
    if [[ -d "$temp_dir/data" ]]; then
        log_info "Restoring data directory..."
        rm -rf "$VEXFS_DATA_DIR"
        mkdir -p "$(dirname "$VEXFS_DATA_DIR")"
        cp -r "$temp_dir/data" "$VEXFS_DATA_DIR"
        chown -R vexfs:vexfs "$VEXFS_DATA_DIR"
        chmod 750 "$VEXFS_DATA_DIR"
        log_success "Data directory restored"
    else
        log_warning "No data directory found in backup"
    fi
    
    # Restore configuration directory
    if [[ -d "$temp_dir/config" ]]; then
        log_info "Restoring configuration directory..."
        rm -rf "$VEXFS_CONFIG_DIR"
        mkdir -p "$(dirname "$VEXFS_CONFIG_DIR")"
        cp -r "$temp_dir/config" "$VEXFS_CONFIG_DIR"
        chown -R root:vexfs "$VEXFS_CONFIG_DIR"
        chmod 750 "$VEXFS_CONFIG_DIR"
        chmod 640 "$VEXFS_CONFIG_DIR"/*.conf 2>/dev/null || true
        log_success "Configuration directory restored"
    else
        log_warning "No configuration directory found in backup"
    fi
    
    # Restore systemd service file
    if [[ -f "$temp_dir/systemd/vexfs.service" ]]; then
        log_info "Restoring systemd service file..."
        cp "$temp_dir/systemd/vexfs.service" "/etc/systemd/system/"
        systemctl daemon-reload
        log_success "Systemd service file restored"
    else
        log_warning "No systemd service file found in backup"
    fi
    
    # Restore logrotate configuration
    if [[ -f "$temp_dir/logrotate/vexfs" ]]; then
        log_info "Restoring logrotate configuration..."
        cp "$temp_dir/logrotate/vexfs" "/etc/logrotate.d/"
        log_success "Logrotate configuration restored"
    else
        log_warning "No logrotate configuration found in backup"
    fi
    
    # Start VexFS service if it was running
    if [[ $service_was_stopped == true ]]; then
        log_info "Starting VexFS service..."
        systemctl start vexfs.service
        sleep 3
        
        if systemctl is-active --quiet vexfs.service; then
            log_success "VexFS service started successfully"
        else
            log_error "Failed to start VexFS service!"
            log_error "Check service status: systemctl status vexfs"
        fi
    fi
    
    # Clean up temporary directory
    rm -rf "$temp_dir"
    
    log_success "Restore completed successfully"
}

# Verify restore
verify_restore() {
    if [[ $DRY_RUN == true ]]; then
        log_info "[DRY RUN] Would verify restore"
        return 0
    fi
    
    log_info "Verifying restore..."
    
    # Check if directories exist
    if [[ -d "$VEXFS_DATA_DIR" ]]; then
        log_success "Data directory exists: $VEXFS_DATA_DIR"
    else
        log_error "Data directory missing: $VEXFS_DATA_DIR"
    fi
    
    if [[ -d "$VEXFS_CONFIG_DIR" ]]; then
        log_success "Configuration directory exists: $VEXFS_CONFIG_DIR"
    else
        log_error "Configuration directory missing: $VEXFS_CONFIG_DIR"
    fi
    
    # Check service status
    if systemctl is-active --quiet vexfs.service; then
        log_success "VexFS service is running"
        
        # Test API endpoint if service is running
        local port=$(grep "^PORT=" "$VEXFS_CONFIG_DIR/vexfs.conf" 2>/dev/null | cut -d'=' -f2 || echo "8000")
        if curl -s "http://localhost:$port/api/v1/version" >/dev/null 2>&1; then
            log_success "VexFS API is responding"
        else
            log_warning "VexFS API is not responding (this may be normal if binding to different address)"
        fi
    else
        log_info "VexFS service is not running"
    fi
}

# Main function
main() {
    echo "========================================"
    echo "VexFS Restore Script"
    echo "========================================"
    echo
    
    parse_args "$@"
    
    if [[ $DRY_RUN == false ]]; then
        check_root
    fi
    
    find_backup_file
    verify_backup
    show_backup_info
    list_backup_contents
    check_service_status
    confirm_restore
    backup_current_state
    perform_restore
    verify_restore
    
    echo
    echo "========================================"
    if [[ $DRY_RUN == true ]]; then
        log_success "Dry run completed successfully!"
    else
        log_success "Restore completed successfully!"
    fi
    echo "========================================"
    echo
    
    if [[ $DRY_RUN == false ]]; then
        echo "VexFS has been restored from: $BACKUP_FILE"
        echo "Data directory: $VEXFS_DATA_DIR"
        echo "Config directory: $VEXFS_CONFIG_DIR"
        echo
        echo "Service management:"
        echo "  Status:  sudo systemctl status vexfs"
        echo "  Logs:    sudo journalctl -u vexfs -f"
        echo
        if systemctl is-active --quiet vexfs.service; then
            local port=$(grep "^PORT=" "$VEXFS_CONFIG_DIR/vexfs.conf" 2>/dev/null | cut -d'=' -f2 || echo "8000")
            echo "API endpoint: http://localhost:$port/api/v1"
            echo "Health check: curl http://localhost:$port/api/v1/version"
        fi
    fi
}

# Run main function
main "$@"