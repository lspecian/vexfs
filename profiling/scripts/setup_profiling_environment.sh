#!/bin/bash

# VexFS FUSE Profiling Environment Setup
# Task 23.1: Comprehensive profiling infrastructure setup

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILING_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$PROFILING_DIR")"
RESULTS_DIR="$PROFILING_DIR/results"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Check if running as root for certain operations
check_root() {
    if [[ $EUID -eq 0 ]]; then
        warning "Running as root. Some operations may require elevated privileges."
    fi
}

# Check system requirements
check_requirements() {
    log "Checking system requirements..."
    
    local missing_tools=()
    
    # Check for required tools
    if ! command -v valgrind &> /dev/null; then
        missing_tools+=("valgrind")
    fi
    
    if ! command -v perf &> /dev/null; then
        missing_tools+=("perf")
    fi
    
    if ! command -v bpftrace &> /dev/null; then
        missing_tools+=("bpftrace")
    fi
    
    if ! command -v cargo &> /dev/null; then
        missing_tools+=("cargo")
    fi
    
    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        error "Missing required tools: ${missing_tools[*]}"
        log "Please install missing tools:"
        log "  Ubuntu/Debian: sudo apt-get install valgrind linux-tools-\$(uname -r) bpftrace"
        log "  Fedora/RHEL: sudo dnf install valgrind perf bpftrace"
        return 1
    fi
    
    success "All required tools are available"
}

# Check kernel version and eBPF support
check_kernel_support() {
    log "Checking kernel support..."
    
    local kernel_version
    kernel_version=$(uname -r)
    log "Kernel version: $kernel_version"
    
    # Check for eBPF support
    if [[ ! -d /sys/kernel/debug/tracing ]]; then
        error "eBPF tracing not available. Please enable CONFIG_BPF_SYSCALL and mount debugfs."
        return 1
    fi
    
    # Check for perf events support
    if [[ ! -d /sys/kernel/debug/tracing/events ]]; then
        warning "Perf events may not be fully available"
    fi
    
    success "Kernel support verified"
}

# Create directory structure
setup_directories() {
    log "Setting up directory structure..."
    
    local dirs=(
        "$RESULTS_DIR"
        "$RESULTS_DIR/valgrind"
        "$RESULTS_DIR/perf"
        "$RESULTS_DIR/ebpf"
        "$RESULTS_DIR/analysis"
        "$PROFILING_DIR/scripts/bpftrace"
        "$PROFILING_DIR/test_environments"
        "$PROFILING_DIR/docs"
    )
    
    for dir in "${dirs[@]}"; do
        if [[ ! -d "$dir" ]]; then
            mkdir -p "$dir"
            log "Created directory: $dir"
        fi
    done
    
    success "Directory structure created"
}

# Set up system limits for profiling
setup_system_limits() {
    log "Configuring system limits for profiling..."
    
    # Check current stack size limit
    local current_stack_limit
    current_stack_limit=$(ulimit -s)
    log "Current stack size limit: ${current_stack_limit}KB"
    
    # Set appropriate limits for profiling
    # Note: These are temporary for the current session
    ulimit -s 16384  # 16MB stack size
    ulimit -c unlimited  # Enable core dumps
    
    # Set memory limits for profiling tools
    # Valgrind can use significant memory
    local available_memory
    available_memory=$(free -m | awk 'NR==2{printf "%.0f", $7}')
    log "Available memory: ${available_memory}MB"
    
    if [[ $available_memory -lt 4096 ]]; then
        warning "Low available memory ($available_memory MB). Profiling may be limited."
    fi
    
    success "System limits configured"
}

# Build VexFS with profiling configuration
build_vexfs_profiling() {
    log "Building VexFS with profiling configuration..."
    
    cd "$PROJECT_ROOT"
    
    # Clean previous builds
    cargo clean
    
    # Build with profiling profile
    if cargo build --profile profiling --features fuse_support --bin vexfs_fuse; then
        success "VexFS FUSE binary built with profiling configuration"
    else
        error "Failed to build VexFS FUSE binary"
        return 1
    fi
    
    # Verify the binary exists and has debug symbols
    local binary_path="$PROJECT_ROOT/target/profiling/vexfs_fuse"
    if [[ -f "$binary_path" ]]; then
        log "Binary location: $binary_path"
        
        # Check for debug symbols
        if file "$binary_path" | grep -q "not stripped"; then
            success "Debug symbols present in binary"
        else
            warning "Debug symbols may not be present"
        fi
        
        # Check binary size
        local binary_size
        binary_size=$(du -h "$binary_path" | cut -f1)
        log "Binary size: $binary_size"
    else
        error "Binary not found at expected location: $binary_path"
        return 1
    fi
}

# Set up eBPF integration with existing infrastructure
setup_ebpf_integration() {
    log "Setting up eBPF integration..."
    
    local ebpf_dir="$PROJECT_ROOT/tests/ebpf_tracing"
    
    if [[ ! -d "$ebpf_dir" ]]; then
        error "Existing eBPF infrastructure not found at: $ebpf_dir"
        return 1
    fi
    
    # Create symbolic links to existing eBPF tools
    local profiling_ebpf_dir="$PROFILING_DIR/scripts/ebpf_integration"
    mkdir -p "$profiling_ebpf_dir"
    
    if [[ ! -L "$profiling_ebpf_dir/vexfs_trace_manager.sh" ]]; then
        ln -s "$ebpf_dir/tools/vexfs_trace_manager.sh" "$profiling_ebpf_dir/vexfs_trace_manager.sh"
        log "Linked existing trace manager"
    fi
    
    # Copy and modify configuration for FUSE-specific tracing
    if [[ -f "$ebpf_dir/configs/default_trace_config.yaml" ]]; then
        cp "$ebpf_dir/configs/default_trace_config.yaml" "$PROFILING_DIR/configs/base_trace_config.yaml"
        log "Copied base eBPF configuration"
    fi
    
    success "eBPF integration configured"
}

# Create Valgrind suppressions file
create_valgrind_suppressions() {
    log "Creating Valgrind suppressions file..."
    
    local suppressions_file="$PROFILING_DIR/configs/valgrind_suppressions.supp"
    
    cat > "$suppressions_file" << 'EOF'
# Valgrind suppressions for VexFS FUSE profiling
# These suppress known false positives and irrelevant warnings

# Rust standard library suppressions
{
   rust_std_thread_local
   Memcheck:Leak
   match-leak-kinds: reachable
   fun:malloc
   ...
   fun:std::thread::local::*
}

{
   rust_std_once
   Memcheck:Leak
   match-leak-kinds: reachable
   fun:malloc
   ...
   fun:std::sync::once::*
}

# FUSE library suppressions
{
   fuse_lib_init
   Memcheck:Leak
   match-leak-kinds: reachable
   fun:malloc
   ...
   fun:fuse_*
}

{
   fuse_session_loop
   Memcheck:Leak
   match-leak-kinds: reachable
   fun:malloc
   ...
   fun:fuse_session_loop*
}

# System library suppressions
{
   glibc_dl_init
   Memcheck:Leak
   match-leak-kinds: reachable
   fun:malloc
   ...
   fun:_dl_*
}

{
   glibc_pthread_create
   Memcheck:Leak
   match-leak-kinds: possible
   fun:calloc
   ...
   fun:pthread_create*
}

# Kernel interaction suppressions
{
   kernel_syscall_uninitialized
   Memcheck:Param
   write(buf)
   ...
   fun:syscall
}

{
   kernel_ioctl_uninitialized
   Memcheck:Param
   ioctl(generic)
   ...
   fun:ioctl
}
EOF

    success "Valgrind suppressions file created"
}

# Verify profiling setup
verify_setup() {
    log "Verifying profiling setup..."
    
    local verification_failed=false
    
    # Check binary
    local binary_path="$PROJECT_ROOT/target/profiling/vexfs_fuse"
    if [[ ! -f "$binary_path" ]]; then
        error "VexFS FUSE binary not found"
        verification_failed=true
    fi
    
    # Check configuration files
    local config_files=(
        "$PROFILING_DIR/configs/valgrind_stack_analysis.conf"
        "$PROFILING_DIR/configs/perf_memory_profile.conf"
        "$PROFILING_DIR/configs/ebpf_fuse_tracing.yaml"
        "$PROFILING_DIR/configs/valgrind_suppressions.supp"
    )
    
    for config_file in "${config_files[@]}"; do
        if [[ ! -f "$config_file" ]]; then
            error "Configuration file not found: $config_file"
            verification_failed=true
        fi
    done
    
    # Check result directories
    local result_dirs=(
        "$RESULTS_DIR/valgrind"
        "$RESULTS_DIR/perf"
        "$RESULTS_DIR/ebpf"
        "$RESULTS_DIR/analysis"
    )
    
    for result_dir in "${result_dirs[@]}"; do
        if [[ ! -d "$result_dir" ]]; then
            error "Result directory not found: $result_dir"
            verification_failed=true
        fi
    done
    
    if [[ "$verification_failed" == "true" ]]; then
        error "Profiling setup verification failed"
        return 1
    fi
    
    success "Profiling setup verification completed successfully"
}

# Print setup summary
print_summary() {
    log "Profiling Environment Setup Summary"
    echo "=================================="
    echo
    echo "Project Root: $PROJECT_ROOT"
    echo "Profiling Directory: $PROFILING_DIR"
    echo "Results Directory: $RESULTS_DIR"
    echo
    echo "VexFS FUSE Binary: $PROJECT_ROOT/target/profiling/vexfs_fuse"
    echo
    echo "Available Profiling Tools:"
    echo "  - Valgrind (stack analysis)"
    echo "  - Perf (memory profiling)"
    echo "  - eBPF/bpftrace (kernel tracing)"
    echo
    echo "Configuration Files:"
    echo "  - Valgrind: $PROFILING_DIR/configs/valgrind_stack_analysis.conf"
    echo "  - Perf: $PROFILING_DIR/configs/perf_memory_profile.conf"
    echo "  - eBPF: $PROFILING_DIR/configs/ebpf_fuse_tracing.yaml"
    echo
    echo "Next Steps:"
    echo "  1. Run baseline profiling: ./scripts/run_comprehensive_profiling.sh --baseline"
    echo "  2. Test individual components: ./test_environments/incremental_component_test.sh"
    echo "  3. Analyze results: ./scripts/analyze_profiling_results.sh"
    echo
    success "Profiling environment setup complete!"
}

# Main execution
main() {
    log "Starting VexFS FUSE profiling environment setup..."
    
    check_root
    check_requirements
    check_kernel_support
    setup_directories
    setup_system_limits
    build_vexfs_profiling
    setup_ebpf_integration
    create_valgrind_suppressions
    verify_setup
    print_summary
}

# Execute main function
main "$@"