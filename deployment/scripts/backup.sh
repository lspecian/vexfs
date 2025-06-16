#!/bin/bash
set -e

# VexFS Backup Script
# This script creates backups of VexFS data and configuration

# Configuration
VEXFS_DATA_DIR="/var/lib/vexfs"
VEXFS_CONFIG_DIR="/etc/vexfs"
BACKUP_BASE_DIR="/opt/vexfs/backups"
RETENTION_DAYS=30
COMPRESS=true

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

# Parse command line arguments
parse_args() {
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
            --retention-days)
                RETENTION_DAYS="$2"
                shift 2
                ;;
            --no-compress)
                COMPRESS=false
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

show_help() {
    echo "VexFS Backup Script"
    echo
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  --data-dir DIR        VexFS data directory (default: /var/lib/vexfs)"
    echo "  --config-dir DIR      VexFS config directory (default: /etc/vexfs)"
    echo "  --backup-dir DIR      Backup base directory (default: /opt/vexfs/backups)"
    echo "  --retention-days N    Keep backups for N days (default: 30)"
    echo "  --no-compress         Don't compress backup archives"
    echo "  --help                Show this help message"
}

# Create backup directory structure
create_backup_dirs() {
    log_info "Creating backup directory structure..."
    
    mkdir -p "$BACKUP_BASE_DIR"
    mkdir -p "$BACKUP_BASE_DIR/daily"
    mkdir -p "$BACKUP_BASE_DIR/weekly"
    mkdir -p "$BACKUP_BASE_DIR/monthly"
    
    log_success "Backup directories created"
}

# Check VexFS service status
check_service_status() {
    if systemctl is-active --quiet vexfs.service; then
        VEXFS_RUNNING=true
        log_info "VexFS service is running"
    else
        VEXFS_RUNNING=false
        log_warning "VexFS service is not running"
    fi
}

# Create consistent backup (with service pause if needed)
create_backup() {
    local backup_type="$1"
    local timestamp=$(date +"%Y%m%d_%H%M%S")
    local backup_name="vexfs_${backup_type}_${timestamp}"
    local backup_dir="$BACKUP_BASE_DIR/$backup_type"
    local temp_dir="/tmp/vexfs_backup_$$"
    
    log_info "Creating $backup_type backup: $backup_name"
    
    # Create temporary directory
    mkdir -p "$temp_dir"
    
    # Pause VexFS service for consistent backup if it's running
    local service_was_paused=false
    if [[ $VEXFS_RUNNING == true ]]; then
        log_info "Pausing VexFS service for consistent backup..."
        systemctl stop vexfs.service
        service_was_paused=true
        sleep 2
    fi
    
    # Create backup manifest
    cat > "$temp_dir/backup_manifest.json" << EOF
{
    "backup_name": "$backup_name",
    "backup_type": "$backup_type",
    "timestamp": "$timestamp",
    "vexfs_version": "1.0.0",
    "data_dir": "$VEXFS_DATA_DIR",
    "config_dir": "$VEXFS_CONFIG_DIR",
    "created_by": "$(whoami)",
    "hostname": "$(hostname)",
    "service_was_running": $VEXFS_RUNNING
}
EOF
    
    # Backup data directory
    if [[ -d "$VEXFS_DATA_DIR" ]]; then
        log_info "Backing up data directory..."
        cp -r "$VEXFS_DATA_DIR" "$temp_dir/data"
        log_success "Data directory backed up"
    else
        log_warning "Data directory not found: $VEXFS_DATA_DIR"
    fi
    
    # Backup configuration directory
    if [[ -d "$VEXFS_CONFIG_DIR" ]]; then
        log_info "Backing up configuration directory..."
        cp -r "$VEXFS_CONFIG_DIR" "$temp_dir/config"
        log_success "Configuration directory backed up"
    else
        log_warning "Configuration directory not found: $VEXFS_CONFIG_DIR"
    fi
    
    # Backup systemd service file
    if [[ -f "/etc/systemd/system/vexfs.service" ]]; then
        log_info "Backing up systemd service file..."
        mkdir -p "$temp_dir/systemd"
        cp "/etc/systemd/system/vexfs.service" "$temp_dir/systemd/"
        log_success "Systemd service file backed up"
    fi
    
    # Backup logrotate configuration
    if [[ -f "/etc/logrotate.d/vexfs" ]]; then
        log_info "Backing up logrotate configuration..."
        mkdir -p "$temp_dir/logrotate"
        cp "/etc/logrotate.d/vexfs" "$temp_dir/logrotate/"
        log_success "Logrotate configuration backed up"
    fi
    
    # Resume VexFS service if it was paused
    if [[ $service_was_paused == true ]]; then
        log_info "Resuming VexFS service..."
        systemctl start vexfs.service
        sleep 2
        if systemctl is-active --quiet vexfs.service; then
            log_success "VexFS service resumed successfully"
        else
            log_error "Failed to resume VexFS service!"
        fi
    fi
    
    # Create archive
    if [[ $COMPRESS == true ]]; then
        log_info "Creating compressed archive..."
        tar -czf "$backup_dir/${backup_name}.tar.gz" -C "$temp_dir" .
        backup_file="$backup_dir/${backup_name}.tar.gz"
    else
        log_info "Creating uncompressed archive..."
        tar -cf "$backup_dir/${backup_name}.tar" -C "$temp_dir" .
        backup_file="$backup_dir/${backup_name}.tar"
    fi
    
    # Calculate backup size and checksum
    backup_size=$(du -h "$backup_file" | cut -f1)
    backup_checksum=$(sha256sum "$backup_file" | cut -d' ' -f1)
    
    # Create backup info file
    cat > "$backup_dir/${backup_name}.info" << EOF
Backup Name: $backup_name
Backup Type: $backup_type
Created: $(date)
Size: $backup_size
Checksum (SHA256): $backup_checksum
Data Directory: $VEXFS_DATA_DIR
Config Directory: $VEXFS_CONFIG_DIR
VexFS Version: 1.0.0
EOF
    
    # Clean up temporary directory
    rm -rf "$temp_dir"
    
    log_success "Backup created: $backup_file ($backup_size)"
    log_info "Backup checksum: $backup_checksum"
}

# Verify backup integrity
verify_backup() {
    local backup_file="$1"
    local info_file="${backup_file%.*}.info"
    
    log_info "Verifying backup: $(basename "$backup_file")"
    
    # Check if backup file exists
    if [[ ! -f "$backup_file" ]]; then
        log_error "Backup file not found: $backup_file"
        return 1
    fi
    
    # Check if info file exists
    if [[ ! -f "$info_file" ]]; then
        log_warning "Backup info file not found: $info_file"
    else
        # Verify checksum
        stored_checksum=$(grep "Checksum" "$info_file" | cut -d' ' -f3)
        current_checksum=$(sha256sum "$backup_file" | cut -d' ' -f1)
        
        if [[ "$stored_checksum" == "$current_checksum" ]]; then
            log_success "Backup checksum verified"
        else
            log_error "Backup checksum mismatch!"
            log_error "Stored: $stored_checksum"
            log_error "Current: $current_checksum"
            return 1
        fi
    fi
    
    # Test archive integrity
    if [[ "$backup_file" == *.tar.gz ]]; then
        if tar -tzf "$backup_file" >/dev/null 2>&1; then
            log_success "Backup archive integrity verified"
        else
            log_error "Backup archive is corrupted!"
            return 1
        fi
    elif [[ "$backup_file" == *.tar ]]; then
        if tar -tf "$backup_file" >/dev/null 2>&1; then
            log_success "Backup archive integrity verified"
        else
            log_error "Backup archive is corrupted!"
            return 1
        fi
    fi
    
    return 0
}

# Clean up old backups
cleanup_old_backups() {
    log_info "Cleaning up backups older than $RETENTION_DAYS days..."
    
    for backup_type in daily weekly monthly; do
        local backup_dir="$BACKUP_BASE_DIR/$backup_type"
        if [[ -d "$backup_dir" ]]; then
            local deleted_count=0
            while IFS= read -r -d '' file; do
                rm -f "$file"
                # Also remove corresponding .info file
                local info_file="${file%.*}.info"
                [[ -f "$info_file" ]] && rm -f "$info_file"
                ((deleted_count++))
            done < <(find "$backup_dir" -name "vexfs_*.tar*" -mtime +$RETENTION_DAYS -print0)
            
            if [[ $deleted_count -gt 0 ]]; then
                log_success "Deleted $deleted_count old $backup_type backups"
            else
                log_info "No old $backup_type backups to delete"
            fi
        fi
    done
}

# List available backups
list_backups() {
    log_info "Available backups:"
    echo
    
    for backup_type in daily weekly monthly; do
        local backup_dir="$BACKUP_BASE_DIR/$backup_type"
        if [[ -d "$backup_dir" ]]; then
            echo "=== $backup_type backups ==="
            local count=0
            for backup_file in "$backup_dir"/vexfs_*.tar*; do
                if [[ -f "$backup_file" ]]; then
                    local info_file="${backup_file%.*}.info"
                    local size=$(du -h "$backup_file" | cut -f1)
                    local date=""
                    if [[ -f "$info_file" ]]; then
                        date=$(grep "Created:" "$info_file" | cut -d' ' -f2-)
                    else
                        date=$(stat -c %y "$backup_file" | cut -d' ' -f1-2)
                    fi
                    echo "  $(basename "$backup_file") - $size - $date"
                    ((count++))
                fi
            done
            if [[ $count -eq 0 ]]; then
                echo "  No backups found"
            fi
            echo
        fi
    done
}

# Determine backup type based on date
determine_backup_type() {
    local day_of_week=$(date +%u)  # 1=Monday, 7=Sunday
    local day_of_month=$(date +%d)
    
    if [[ $day_of_month == "01" ]]; then
        echo "monthly"
    elif [[ $day_of_week == "7" ]]; then  # Sunday
        echo "weekly"
    else
        echo "daily"
    fi
}

# Main function
main() {
    echo "========================================"
    echo "VexFS Backup Script"
    echo "========================================"
    echo
    
    parse_args "$@"
    check_root
    create_backup_dirs
    check_service_status
    
    # Determine backup type
    backup_type=$(determine_backup_type)
    
    # Create backup
    create_backup "$backup_type"
    
    # Verify the backup we just created
    latest_backup=$(find "$BACKUP_BASE_DIR/$backup_type" -name "vexfs_*.tar*" -type f -printf '%T@ %p\n' | sort -n | tail -1 | cut -d' ' -f2-)
    if [[ -n "$latest_backup" ]]; then
        verify_backup "$latest_backup"
    fi
    
    # Clean up old backups
    cleanup_old_backups
    
    echo
    echo "========================================"
    log_success "Backup completed successfully!"
    echo "========================================"
    echo
    echo "Backup location: $BACKUP_BASE_DIR"
    echo "Backup type: $backup_type"
    echo "Retention: $RETENTION_DAYS days"
    echo
    echo "To list all backups: $0 --list"
    echo "To restore a backup: ./restore.sh <backup_file>"
}

# Handle special commands
if [[ "$1" == "--list" ]]; then
    list_backups
    exit 0
fi

# Run main function
main "$@"