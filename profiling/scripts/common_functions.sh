#!/bin/bash

# Common Functions Library for VexFS FUSE Stack Overflow Testing
# Shared utilities and functions used across all test scenarios

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*" >&2
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*" >&2
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*" >&2
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*" >&2
}

log_debug() {
    if [[ "${VERBOSE:-false}" == "true" ]]; then
        echo -e "${PURPLE}[DEBUG]${NC} $(date '+%Y-%m-%d %H:%M:%S') $*" >&2
    fi
}

# Progress indicator
show_progress() {
    local current=$1
    local total=$2
    local description=$3
    local percentage=$((current * 100 / total))
    local bar_length=50
    local filled_length=$((percentage * bar_length / 100))
    
    printf "\r${CYAN}[PROGRESS]${NC} ["
    printf "%*s" $filled_length | tr ' ' '='
    printf "%*s" $((bar_length - filled_length)) | tr ' ' '-'
    printf "] %d%% (%d/%d) %s" $percentage $current $total "$description"
    
    if [[ $current -eq $total ]]; then
        echo ""
    fi
}

# System information gathering
get_system_info() {
    cat << EOF
System Information:
- OS: $(uname -s) $(uname -r)
- Architecture: $(uname -m)
- CPU: $(nproc) cores
- Memory: $(free -h | awk '/^Mem:/ {print $2}')
- Disk Space: $(df -h / | awk 'NR==2 {print $4}') available
- Kernel: $(uname -v)
- Date: $(date)
EOF
}

# Check if running as root
check_root() {
    if [[ $EUID -eq 0 ]]; then
        return 0
    else
        return 1
    fi
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Wait for condition with timeout
wait_for_condition() {
    local condition_cmd="$1"
    local timeout_seconds="${2:-30}"
    local check_interval="${3:-1}"
    local description="${4:-condition}"
    
    local elapsed=0
    
    log_debug "Waiting for $description (timeout: ${timeout_seconds}s)"
    
    while [[ $elapsed -lt $timeout_seconds ]]; do
        if eval "$condition_cmd" >/dev/null 2>&1; then
            log_debug "$description met after ${elapsed}s"
            return 0
        fi
        
        sleep "$check_interval"
        elapsed=$((elapsed + check_interval))
    done
    
    log_error "$description not met within ${timeout_seconds}s"
    return 1
}

# Check if filesystem is mounted
is_mounted() {
    local mount_point="$1"
    mountpoint -q "$mount_point" 2>/dev/null
}

# Safe unmount with retry
safe_unmount() {
    local mount_point="$1"
    local max_attempts="${2:-3}"
    local attempt=1
    
    if ! is_mounted "$mount_point"; then
        log_debug "$mount_point is not mounted"
        return 0
    fi
    
    while [[ $attempt -le $max_attempts ]]; do
        log_debug "Unmounting $mount_point (attempt $attempt/$max_attempts)"
        
        if fusermount -u "$mount_point" 2>/dev/null; then
            log_debug "Successfully unmounted $mount_point"
            return 0
        fi
        
        # Try force unmount
        if fusermount -uz "$mount_point" 2>/dev/null; then
            log_debug "Force unmounted $mount_point"
            return 0
        fi
        
        # Wait before retry
        sleep 2
        ((attempt++))
    done
    
    log_error "Failed to unmount $mount_point after $max_attempts attempts"
    return 1
}

# Create directory with proper permissions
create_directory() {
    local dir_path="$1"
    local permissions="${2:-755}"
    
    if [[ ! -d "$dir_path" ]]; then
        log_debug "Creating directory: $dir_path"
        mkdir -p "$dir_path"
        chmod "$permissions" "$dir_path"
    fi
}

# Generate random test data
generate_random_data() {
    local size_bytes="$1"
    local output_file="$2"
    
    log_debug "Generating ${size_bytes} bytes of random data to $output_file"
    dd if=/dev/urandom of="$output_file" bs=1 count="$size_bytes" 2>/dev/null
}

# Calculate file checksum
calculate_checksum() {
    local file_path="$1"
    local algorithm="${2:-sha256}"
    
    case "$algorithm" in
        "md5")
            md5sum "$file_path" | cut -d' ' -f1
            ;;
        "sha1")
            sha1sum "$file_path" | cut -d' ' -f1
            ;;
        "sha256")
            sha256sum "$file_path" | cut -d' ' -f1
            ;;
        *)
            log_error "Unsupported checksum algorithm: $algorithm"
            return 1
            ;;
    esac
}

# Monitor process resource usage
monitor_process() {
    local pid="$1"
    local output_file="$2"
    local interval="${3:-1}"
    
    log_debug "Monitoring process $pid, output to $output_file"
    
    # Write header
    echo "timestamp,pid,cpu_percent,memory_kb,stack_kb,threads" > "$output_file"
    
    while kill -0 "$pid" 2>/dev/null; do
        local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        local cpu_percent=$(ps -p "$pid" -o %cpu --no-headers 2>/dev/null | tr -d ' ')
        local memory_kb=$(ps -p "$pid" -o rss --no-headers 2>/dev/null | tr -d ' ')
        local threads=$(ps -p "$pid" -o nlwp --no-headers 2>/dev/null | tr -d ' ')
        
        # Get stack size from /proc if available
        local stack_kb="0"
        if [[ -f "/proc/$pid/status" ]]; then
            stack_kb=$(grep "VmStk:" "/proc/$pid/status" 2>/dev/null | awk '{print $2}' || echo "0")
        fi
        
        echo "$timestamp,$pid,$cpu_percent,$memory_kb,$stack_kb,$threads" >> "$output_file"
        sleep "$interval"
    done
}

# Parse YAML configuration
parse_yaml_config() {
    local yaml_file="$1"
    local section="$2"
    
    if [[ ! -f "$yaml_file" ]]; then
        log_error "YAML file not found: $yaml_file"
        return 1
    fi
    
    # Simple YAML parser for basic key-value pairs
    # Note: This is a simplified parser, not a full YAML implementation
    python3 -c "
import yaml
import sys

try:
    with open('$yaml_file', 'r') as f:
        data = yaml.safe_load(f)
    
    if '$section':
        section_data = data.get('$section', {})
        for key, value in section_data.items():
            print(f'{key}={value}')
    else:
        for key, value in data.items():
            print(f'{key}={value}')
except Exception as e:
    print(f'Error parsing YAML: {e}', file=sys.stderr)
    sys.exit(1)
"
}

# Execute command with timeout and logging
execute_with_timeout() {
    local timeout_seconds="$1"
    local log_file="$2"
    shift 2
    local command=("$@")
    
    log_debug "Executing with timeout ${timeout_seconds}s: ${command[*]}"
    log_debug "Output will be logged to: $log_file"
    
    # Execute command with timeout
    if timeout "$timeout_seconds" "${command[@]}" > "$log_file" 2>&1; then
        log_debug "Command completed successfully"
        return 0
    else
        local exit_code=$?
        if [[ $exit_code -eq 124 ]]; then
            log_error "Command timed out after ${timeout_seconds}s"
        else
            log_error "Command failed with exit code $exit_code"
        fi
        return $exit_code
    fi
}

# Start background process with monitoring
start_background_process() {
    local command=("$@")
    local log_file="${command[0]##*/}_$(date +%Y%m%d_%H%M%S).log"
    
    log_debug "Starting background process: ${command[*]}"
    log_debug "Process log: $log_file"
    
    # Start process in background
    "${command[@]}" > "$log_file" 2>&1 &
    local pid=$!
    
    # Wait a moment to check if process started successfully
    sleep 1
    if kill -0 "$pid" 2>/dev/null; then
        log_debug "Background process started successfully (PID: $pid)"
        echo "$pid"
        return 0
    else
        log_error "Background process failed to start"
        return 1
    fi
}

# Stop background process gracefully
stop_background_process() {
    local pid="$1"
    local timeout_seconds="${2:-10}"
    
    if ! kill -0 "$pid" 2>/dev/null; then
        log_debug "Process $pid is not running"
        return 0
    fi
    
    log_debug "Stopping process $pid gracefully"
    
    # Send TERM signal
    kill -TERM "$pid" 2>/dev/null
    
    # Wait for graceful shutdown
    local elapsed=0
    while [[ $elapsed -lt $timeout_seconds ]] && kill -0 "$pid" 2>/dev/null; do
        sleep 1
        ((elapsed++))
    done
    
    # Force kill if still running
    if kill -0 "$pid" 2>/dev/null; then
        log_warn "Process $pid did not stop gracefully, force killing"
        kill -KILL "$pid" 2>/dev/null
        sleep 1
    fi
    
    if kill -0 "$pid" 2>/dev/null; then
        log_error "Failed to stop process $pid"
        return 1
    else
        log_debug "Process $pid stopped successfully"
        return 0
    fi
}

# Validate profiling tool availability
validate_profiling_tools() {
    local tools=("$@")
    local missing_tools=()
    
    for tool in "${tools[@]}"; do
        case "$tool" in
            "valgrind")
                if ! command_exists valgrind; then
                    missing_tools+=("valgrind")
                fi
                ;;
            "perf")
                if ! command_exists perf; then
                    missing_tools+=("perf")
                fi
                ;;
            "ebpf"|"bpftrace")
                if ! command_exists bpftrace; then
                    missing_tools+=("bpftrace")
                fi
                ;;
            "strace")
                if ! command_exists strace; then
                    missing_tools+=("strace")
                fi
                ;;
            *)
                log_warn "Unknown profiling tool: $tool"
                ;;
        esac
    done
    
    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        log_error "Missing profiling tools: ${missing_tools[*]}"
        return 1
    fi
    
    log_debug "All profiling tools available: ${tools[*]}"
    return 0
}

# Generate test summary
generate_test_summary() {
    local test_name="$1"
    local start_time="$2"
    local end_time="$3"
    local exit_code="$4"
    local output_file="$5"
    
    local duration=$((end_time - start_time))
    local status="FAILED"
    if [[ $exit_code -eq 0 ]]; then
        status="PASSED"
    fi
    
    cat > "$output_file" << EOF
Test Summary: $test_name
Status: $status
Exit Code: $exit_code
Start Time: $(date -d "@$start_time" '+%Y-%m-%d %H:%M:%S')
End Time: $(date -d "@$end_time" '+%Y-%m-%d %H:%M:%S')
Duration: ${duration}s
System: $(uname -a)
Generated: $(date)
EOF
}

# Archive test results
archive_test_results() {
    local source_dir="$1"
    local archive_name="$2"
    local compression="${3:-gzip}"
    
    log_info "Archiving test results from $source_dir"
    
    case "$compression" in
        "gzip")
            tar -czf "${archive_name}.tar.gz" -C "$(dirname "$source_dir")" "$(basename "$source_dir")"
            ;;
        "bzip2")
            tar -cjf "${archive_name}.tar.bz2" -C "$(dirname "$source_dir")" "$(basename "$source_dir")"
            ;;
        "xz")
            tar -cJf "${archive_name}.tar.xz" -C "$(dirname "$source_dir")" "$(basename "$source_dir")"
            ;;
        *)
            log_error "Unsupported compression: $compression"
            return 1
            ;;
    esac
    
    log_success "Test results archived to ${archive_name}.tar.${compression}"
}

# Check system resources
check_system_resources() {
    local min_memory_gb="${1:-4}"
    local min_disk_gb="${2:-10}"
    
    # Check available memory
    local available_memory_kb=$(awk '/MemAvailable/ {print $2}' /proc/meminfo)
    local available_memory_gb=$((available_memory_kb / 1024 / 1024))
    
    if [[ $available_memory_gb -lt $min_memory_gb ]]; then
        log_error "Insufficient memory: ${available_memory_gb}GB available, ${min_memory_gb}GB required"
        return 1
    fi
    
    # Check available disk space
    local available_disk_gb=$(df / | awk 'NR==2 {print int($4/1024/1024)}')
    
    if [[ $available_disk_gb -lt $min_disk_gb ]]; then
        log_error "Insufficient disk space: ${available_disk_gb}GB available, ${min_disk_gb}GB required"
        return 1
    fi
    
    log_debug "System resources check passed: ${available_memory_gb}GB memory, ${available_disk_gb}GB disk"
    return 0
}

# Cleanup function for signal handling
cleanup_on_signal() {
    log_warn "Received signal, cleaning up..."
    
    # Kill background processes
    jobs -p | xargs -r kill 2>/dev/null || true
    
    # Unmount filesystems
    for mount_point in /tmp/vexfs_*; do
        if is_mounted "$mount_point"; then
            safe_unmount "$mount_point"
        fi
    done
    
    log_info "Cleanup completed"
    exit 130
}

# Set up signal handlers
setup_signal_handlers() {
    trap cleanup_on_signal SIGINT SIGTERM
}

# Initialize common environment
init_common_environment() {
    # Set up signal handlers
    setup_signal_handlers
    
    # Set strict error handling
    set -euo pipefail
    
    # Log initialization
    log_debug "Common environment initialized"
    log_debug "PID: $$"
    log_debug "Working directory: $(pwd)"
    log_debug "User: $(whoami)"
}

# Export functions for use in other scripts
export -f log_info log_warn log_error log_success log_debug
export -f show_progress get_system_info check_root command_exists
export -f wait_for_condition is_mounted safe_unmount create_directory
export -f generate_random_data calculate_checksum monitor_process
export -f parse_yaml_config execute_with_timeout
export -f start_background_process stop_background_process
export -f validate_profiling_tools generate_test_summary
export -f archive_test_results check_system_resources
export -f cleanup_on_signal setup_signal_handlers init_common_environment

# Initialize when sourced
init_common_environment