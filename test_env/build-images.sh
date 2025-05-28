#!/bin/bash

# VexFS Image Build Pipeline
# Automated script for building VexFS images with different configurations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PACKER_DIR="$SCRIPT_DIR/packer"
OUTPUT_DIR="$SCRIPT_DIR/images"
LOG_DIR="$SCRIPT_DIR/logs"

# Default values
DEFAULT_VARIANTS=("minimal" "development" "testing" "production")
DEFAULT_VEXFS_VERSION="1.0.0"
DEFAULT_KERNEL_VERSION="6.1"
PARALLEL_BUILDS=false
VALIDATE_IMAGES=true
CLEANUP_TEMP=true

# Helper functions
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
    exit 1
}

show_usage() {
    cat << EOF
VexFS Image Build Pipeline

Usage: $0 [OPTIONS]

Options:
    -v, --variants VARIANTS     Comma-separated list of image variants to build
                               Available: minimal,development,testing,production
                               Default: all variants
    
    --vexfs-version VERSION    VexFS version to build (default: $DEFAULT_VEXFS_VERSION)
    --kernel-version VERSION   Target kernel version (default: $DEFAULT_KERNEL_VERSION)
    
    -p, --parallel             Build images in parallel (experimental)
    --no-validation           Skip image validation after build
    --no-cleanup              Keep temporary files after build
    
    -o, --output-dir DIR       Output directory for images (default: $OUTPUT_DIR)
    --log-dir DIR              Log directory (default: $LOG_DIR)
    
    -h, --help                 Show this help message

Examples:
    $0                                    # Build all variants
    $0 -v minimal,production              # Build only minimal and production
    $0 --vexfs-version 1.1.0 -p          # Build all with version 1.1.0 in parallel
    $0 -v development --no-validation     # Build development variant without validation

EOF
}

# Parse command line arguments
parse_arguments() {
    VARIANTS=()
    VEXFS_VERSION="$DEFAULT_VEXFS_VERSION"
    KERNEL_VERSION="$DEFAULT_KERNEL_VERSION"
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--variants)
                IFS=',' read -ra VARIANTS <<< "$2"
                shift 2
                ;;
            --vexfs-version)
                VEXFS_VERSION="$2"
                shift 2
                ;;
            --kernel-version)
                KERNEL_VERSION="$2"
                shift 2
                ;;
            -p|--parallel)
                PARALLEL_BUILDS=true
                shift
                ;;
            --no-validation)
                VALIDATE_IMAGES=false
                shift
                ;;
            --no-cleanup)
                CLEANUP_TEMP=false
                shift
                ;;
            -o|--output-dir)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            --log-dir)
                LOG_DIR="$2"
                shift 2
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                ;;
        esac
    done
    
    # Use default variants if none specified
    if [ ${#VARIANTS[@]} -eq 0 ]; then
        VARIANTS=("${DEFAULT_VARIANTS[@]}")
    fi
}

# Check dependencies
check_dependencies() {
    log_info "Checking dependencies..."
    
    local missing_deps=()
    
    if ! command -v packer >/dev/null 2>&1; then
        missing_deps+=("packer")
    fi
    
    if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
        missing_deps+=("qemu-system-x86_64")
    fi
    
    if ! command -v qemu-img >/dev/null 2>&1; then
        missing_deps+=("qemu-img")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
    fi
    
    log_success "All dependencies found"
}

# Setup build environment
setup_environment() {
    log_info "Setting up build environment..."
    
    # Create directories
    mkdir -p "$OUTPUT_DIR"
    mkdir -p "$LOG_DIR"
    mkdir -p "$PACKER_DIR"
    
    # Validate Packer configuration
    if [ ! -f "$PACKER_DIR/vexfs-production.pkr.hcl" ]; then
        log_error "Packer configuration not found: $PACKER_DIR/vexfs-production.pkr.hcl"
    fi
    
    # Validate preseed files
    for variant in "${VARIANTS[@]}"; do
        if [ ! -f "$SCRIPT_DIR/http/preseed-${variant}.cfg" ]; then
            log_error "Preseed file not found: $SCRIPT_DIR/http/preseed-${variant}.cfg"
        fi
    done
    
    # Validate VexFS source
    if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
        log_error "VexFS source not found in project root: $PROJECT_ROOT"
    fi
    
    log_success "Build environment ready"
}

# Build a single image variant
build_image() {
    local variant="$1"
    local log_file="$LOG_DIR/build-${variant}-$(date +%Y%m%d-%H%M%S).log"
    
    log_info "Building $variant image..."
    log_info "Log file: $log_file"
    
    # Change to packer directory
    cd "$PACKER_DIR"
    
    # Build with Packer
    local packer_cmd=(
        packer build
        -var "image_variant=$variant"
        -var "vexfs_version=$VEXFS_VERSION"
        -var "kernel_version=$KERNEL_VERSION"
        -var "output_dir=$OUTPUT_DIR"
        -var "enable_validation=$VALIDATE_IMAGES"
        vexfs-production.pkr.hcl
    )
    
    if "${packer_cmd[@]}" > "$log_file" 2>&1; then
        log_success "$variant image built successfully"
        return 0
    else
        log_error "$variant image build failed. Check log: $log_file"
        return 1
    fi
}

# Build images sequentially
build_images_sequential() {
    log_info "Building images sequentially..."
    
    local failed_builds=()
    
    for variant in "${VARIANTS[@]}"; do
        if ! build_image "$variant"; then
            failed_builds+=("$variant")
        fi
    done
    
    if [ ${#failed_builds[@]} -gt 0 ]; then
        log_error "Failed to build variants: ${failed_builds[*]}"
    fi
}

# Build images in parallel
build_images_parallel() {
    log_info "Building images in parallel..."
    log_warning "Parallel builds are experimental and may cause resource conflicts"
    
    local pids=()
    local failed_builds=()
    
    # Start builds in background
    for variant in "${VARIANTS[@]}"; do
        build_image "$variant" &
        pids+=($!)
    done
    
    # Wait for all builds to complete
    for i in "${!pids[@]}"; do
        local pid=${pids[$i]}
        local variant=${VARIANTS[$i]}
        
        if ! wait $pid; then
            failed_builds+=("$variant")
        fi
    done
    
    if [ ${#failed_builds[@]} -gt 0 ]; then
        log_error "Failed to build variants: ${failed_builds[*]}"
    fi
}

# Validate built images
validate_images() {
    if [ "$VALIDATE_IMAGES" = false ]; then
        log_info "Skipping image validation (disabled)"
        return
    fi
    
    log_info "Validating built images..."
    
    for variant in "${VARIANTS[@]}"; do
        local image_dir="$OUTPUT_DIR/vexfs-${variant}-${VEXFS_VERSION}-"*
        
        if [ -d $image_dir ]; then
            local qcow2_file="$image_dir"/*.qcow2
            
            if [ -f $qcow2_file ]; then
                log_info "Validating $variant image: $(basename "$qcow2_file")"
                
                # Check image integrity
                if qemu-img check "$qcow2_file" >/dev/null 2>&1; then
                    log_success "$variant image validation passed"
                else
                    log_error "$variant image validation failed"
                fi
            else
                log_error "$variant image file not found"
            fi
        else
            log_error "$variant image directory not found"
        fi
    done
}

# Generate build report
generate_report() {
    log_info "Generating build report..."
    
    local report_file="$LOG_DIR/build-report-$(date +%Y%m%d-%H%M%S).txt"
    
    cat > "$report_file" << EOF
VexFS Image Build Report
========================

Build Date: $(date)
VexFS Version: $VEXFS_VERSION
Kernel Version: $KERNEL_VERSION
Variants Built: ${VARIANTS[*]}
Parallel Builds: $PARALLEL_BUILDS
Validation Enabled: $VALIDATE_IMAGES

Built Images:
EOF
    
    for variant in "${VARIANTS[@]}"; do
        local image_dir="$OUTPUT_DIR/vexfs-${variant}-${VEXFS_VERSION}-"*
        
        if [ -d $image_dir ]; then
            echo "  $variant:" >> "$report_file"
            echo "    Directory: $(basename "$image_dir")" >> "$report_file"
            
            local qcow2_file="$image_dir"/*.qcow2
            if [ -f $qcow2_file ]; then
                local size=$(du -h "$qcow2_file" | cut -f1)
                echo "    Image: $(basename "$qcow2_file") ($size)" >> "$report_file"
            fi
            
            local compressed_file="$image_dir"/*.qcow2.gz
            if [ -f $compressed_file ]; then
                local comp_size=$(du -h "$compressed_file" | cut -f1)
                echo "    Compressed: $(basename "$compressed_file") ($comp_size)" >> "$report_file"
            fi
        else
            echo "  $variant: BUILD FAILED" >> "$report_file"
        fi
    done
    
    echo "" >> "$report_file"
    echo "Build Logs:" >> "$report_file"
    ls -la "$LOG_DIR"/build-*-*.log >> "$report_file" 2>/dev/null || echo "  No build logs found" >> "$report_file"
    
    log_success "Build report generated: $report_file"
    
    # Display summary
    echo ""
    log_info "Build Summary:"
    cat "$report_file"
}

# Cleanup temporary files
cleanup() {
    if [ "$CLEANUP_TEMP" = false ]; then
        log_info "Skipping cleanup (disabled)"
        return
    fi
    
    log_info "Cleaning up temporary files..."
    
    # Clean up Packer cache
    rm -rf "$PACKER_DIR/packer_cache" 2>/dev/null || true
    
    # Clean up temporary build files
    find "$OUTPUT_DIR" -name "*.tmp" -delete 2>/dev/null || true
    
    log_success "Cleanup completed"
}

# Main execution
main() {
    echo "🚀 VexFS Image Build Pipeline"
    echo "============================="
    
    parse_arguments "$@"
    
    log_info "Configuration:"
    log_info "  Variants: ${VARIANTS[*]}"
    log_info "  VexFS Version: $VEXFS_VERSION"
    log_info "  Kernel Version: $KERNEL_VERSION"
    log_info "  Output Directory: $OUTPUT_DIR"
    log_info "  Parallel Builds: $PARALLEL_BUILDS"
    log_info "  Validation: $VALIDATE_IMAGES"
    echo ""
    
    check_dependencies
    setup_environment
    
    # Build images
    if [ "$PARALLEL_BUILDS" = true ]; then
        build_images_parallel
    else
        build_images_sequential
    fi
    
    validate_images
    generate_report
    cleanup
    
    echo ""
    log_success "🎉 VexFS image build pipeline completed successfully!"
    log_info "Images available in: $OUTPUT_DIR"
    log_info "Logs available in: $LOG_DIR"
}

# Run main function with all arguments
main "$@"