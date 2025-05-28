#!/bin/bash

# VexFS CI/CD Build Pipeline
# Automated build pipeline for continuous integration

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
CI_OUTPUT_DIR="$SCRIPT_DIR/ci-output"
CI_LOG_DIR="$SCRIPT_DIR/ci-logs"

# CI Environment Variables (with defaults)
CI_BUILD_NUMBER="${CI_BUILD_NUMBER:-$(date +%Y%m%d%H%M%S)}"
CI_COMMIT_SHA="${CI_COMMIT_SHA:-$(git rev-parse HEAD 2>/dev/null || echo 'unknown')}"
CI_BRANCH="${CI_BRANCH:-$(git branch --show-current 2>/dev/null || echo 'unknown')}"
CI_TAG="${CI_TAG:-}"
VEXFS_VERSION="${CI_TAG:-1.0.0-dev}"

# Build configuration
BUILD_VARIANTS=("minimal" "production")  # Reduced for CI
ENABLE_TESTING=true
ENABLE_VALIDATION=true
PARALLEL_BUILDS=false  # Disabled for CI stability
UPLOAD_ARTIFACTS=false
ARTIFACT_REGISTRY=""

# Helper functions
log_info() {
    echo -e "${BLUE}[CI-INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[CI-SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[CI-WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[CI-ERROR]${NC} $1"
    exit 1
}

# Setup CI environment
setup_ci_environment() {
    log_info "Setting up CI environment..."
    
    # Create CI directories
    mkdir -p "$CI_OUTPUT_DIR"
    mkdir -p "$CI_LOG_DIR"
    
    # Log CI environment
    cat > "$CI_LOG_DIR/ci-environment.log" << EOF
VexFS CI/CD Build Pipeline
==========================

Build Information:
  Build Number: $CI_BUILD_NUMBER
  Commit SHA: $CI_COMMIT_SHA
  Branch: $CI_BRANCH
  Tag: $CI_TAG
  VexFS Version: $VEXFS_VERSION

Environment:
  Hostname: $(hostname)
  User: $(whoami)
  Date: $(date)
  Working Directory: $(pwd)
  
System Information:
  OS: $(uname -a)
  CPU Cores: $(nproc)
  Memory: $(free -h | grep '^Mem:' | awk '{print $2}')
  Disk Space: $(df -h . | tail -1 | awk '{print $4}')

Build Configuration:
  Variants: ${BUILD_VARIANTS[*]}
  Testing Enabled: $ENABLE_TESTING
  Validation Enabled: $ENABLE_VALIDATION
  Parallel Builds: $PARALLEL_BUILDS
  Upload Artifacts: $UPLOAD_ARTIFACTS

EOF
    
    log_success "CI environment setup completed"
}

# Check CI dependencies
check_ci_dependencies() {
    log_info "Checking CI dependencies..."
    
    local missing_deps=()
    local required_tools=("packer" "qemu-system-x86_64" "qemu-img" "git" "curl")
    
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing_deps+=("$tool")
        fi
    done
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing required dependencies: ${missing_deps[*]}"
    fi
    
    # Check disk space (require at least 10GB)
    local available_space=$(df . | tail -1 | awk '{print $4}')
    local required_space=$((10 * 1024 * 1024))  # 10GB in KB
    
    if [ "$available_space" -lt "$required_space" ]; then
        log_error "Insufficient disk space. Required: 10GB, Available: $(( available_space / 1024 / 1024 ))GB"
    fi
    
    log_success "All CI dependencies satisfied"
}

# Run pre-build checks
run_prebuild_checks() {
    log_info "Running pre-build checks..."
    
    # Check VexFS source code
    if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
        log_error "VexFS source code not found"
    fi
    
    # Run Rust syntax check
    log_info "Running Rust syntax check..."
    cd "$PROJECT_ROOT"
    if cargo check --lib --target=x86_64-unknown-linux-gnu; then
        log_success "Rust syntax check passed"
    else
        log_error "Rust syntax check failed"
    fi
    
    # Check Packer configuration
    log_info "Validating Packer configuration..."
    cd "$SCRIPT_DIR"
    if packer validate packer/vexfs-production.pkr.hcl; then
        log_success "Packer configuration is valid"
    else
        log_error "Packer configuration validation failed"
    fi
    
    log_success "Pre-build checks completed"
}

# Build images for CI
build_ci_images() {
    log_info "Building VexFS images for CI..."
    
    local build_log="$CI_LOG_DIR/build-${CI_BUILD_NUMBER}.log"
    local build_start_time=$(date +%s)
    
    # Build images using the build script
    local build_cmd=(
        "$SCRIPT_DIR/build-images.sh"
        --vexfs-version "$VEXFS_VERSION"
        --output-dir "$CI_OUTPUT_DIR"
        --log-dir "$CI_LOG_DIR"
    )
    
    if [ "$ENABLE_VALIDATION" = false ]; then
        build_cmd+=(--no-validation)
    fi
    
    # Add variants
    local variants_str=$(IFS=,; echo "${BUILD_VARIANTS[*]}")
    build_cmd+=(--variants "$variants_str")
    
    log_info "Build command: ${build_cmd[*]}"
    
    if "${build_cmd[@]}" > "$build_log" 2>&1; then
        local build_duration=$(($(date +%s) - build_start_time))
        log_success "Image build completed in ${build_duration}s"
    else
        log_error "Image build failed. Check log: $build_log"
    fi
}

# Run image validation
run_ci_validation() {
    if [ "$ENABLE_VALIDATION" = false ]; then
        log_info "Skipping image validation (disabled)"
        return
    fi
    
    log_info "Running CI image validation..."
    
    local validation_log="$CI_LOG_DIR/validation-${CI_BUILD_NUMBER}.log"
    
    # Run validation using the validation script
    local validation_cmd=(
        "$SCRIPT_DIR/validate-images.sh"
        --images-dir "$CI_OUTPUT_DIR"
        --test-type "quick"
        --boot-timeout 300
        --no-cleanup
    )
    
    if "${validation_cmd[@]}" > "$validation_log" 2>&1; then
        log_success "Image validation completed successfully"
    else
        log_error "Image validation failed. Check log: $validation_log"
    fi
}

# Generate CI artifacts
generate_ci_artifacts() {
    log_info "Generating CI artifacts..."
    
    local artifacts_dir="$CI_OUTPUT_DIR/artifacts"
    mkdir -p "$artifacts_dir"
    
    # Create build manifest
    cat > "$artifacts_dir/build-manifest.json" << EOF
{
  "build_number": "$CI_BUILD_NUMBER",
  "commit_sha": "$CI_COMMIT_SHA",
  "branch": "$CI_BRANCH",
  "tag": "$CI_TAG",
  "vexfs_version": "$VEXFS_VERSION",
  "build_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "variants": [$(printf '"%s",' "${BUILD_VARIANTS[@]}" | sed 's/,$//')]
}
EOF
    
    # Create checksums for all images
    log_info "Generating checksums..."
    find "$CI_OUTPUT_DIR" -name "*.qcow2" -o -name "*.qcow2.gz" | while read -r file; do
        sha256sum "$file" >> "$artifacts_dir/checksums.sha256"
    done
    
    # Create release notes
    cat > "$artifacts_dir/release-notes.md" << EOF
# VexFS Release $VEXFS_VERSION

## Build Information
- **Build Number**: $CI_BUILD_NUMBER
- **Commit**: $CI_COMMIT_SHA
- **Branch**: $CI_BRANCH
- **Build Date**: $(date)

## Image Variants
$(for variant in "${BUILD_VARIANTS[@]}"; do
    echo "- **$variant**: Optimized for $variant use cases"
done)

## Installation
1. Download the appropriate image variant for your use case
2. Verify the checksum using the provided checksums.sha256 file
3. Boot the image using QEMU or your preferred virtualization platform
4. VexFS will be automatically available after boot

## Usage
- VexFS module: \`modprobe vexfs\`
- VexFS control tool: \`vexctl --help\`
- Configuration: \`/etc/vexfs/vexfs.conf\`

## Support
For issues and support, please visit the VexFS project repository.
EOF
    
    # Copy logs to artifacts
    cp -r "$CI_LOG_DIR" "$artifacts_dir/logs"
    
    log_success "CI artifacts generated in: $artifacts_dir"
}

# Upload artifacts (if enabled)
upload_artifacts() {
    if [ "$UPLOAD_ARTIFACTS" = false ]; then
        log_info "Skipping artifact upload (disabled)"
        return
    fi
    
    if [ -z "$ARTIFACT_REGISTRY" ]; then
        log_warning "Artifact registry not configured, skipping upload"
        return
    fi
    
    log_info "Uploading artifacts to: $ARTIFACT_REGISTRY"
    
    # This is a placeholder for actual artifact upload logic
    # Implementation would depend on the specific artifact registry being used
    # Examples: AWS S3, Google Cloud Storage, Azure Blob Storage, etc.
    
    log_warning "Artifact upload not implemented - placeholder only"
}

# Generate CI report
generate_ci_report() {
    log_info "Generating CI build report..."
    
    local report_file="$CI_OUTPUT_DIR/ci-build-report.html"
    
    cat > "$report_file" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>VexFS CI Build Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .header { background: #f0f0f0; padding: 20px; border-radius: 5px; }
        .success { color: #28a745; }
        .error { color: #dc3545; }
        .warning { color: #ffc107; }
        .info { color: #17a2b8; }
        table { border-collapse: collapse; width: 100%; margin: 20px 0; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; }
    </style>
</head>
<body>
    <div class="header">
        <h1>VexFS CI Build Report</h1>
        <p><strong>Build Number:</strong> BUILD_NUMBER_PLACEHOLDER</p>
        <p><strong>Commit:</strong> COMMIT_SHA_PLACEHOLDER</p>
        <p><strong>Branch:</strong> BRANCH_PLACEHOLDER</p>
        <p><strong>Version:</strong> VERSION_PLACEHOLDER</p>
        <p><strong>Build Date:</strong> BUILD_DATE_PLACEHOLDER</p>
    </div>

    <h2>Build Summary</h2>
    <table>
        <tr><th>Stage</th><th>Status</th><th>Duration</th></tr>
        <tr><td>Pre-build Checks</td><td class="success">✅ Passed</td><td>-</td></tr>
        <tr><td>Image Build</td><td class="success">✅ Passed</td><td>-</td></tr>
        <tr><td>Validation</td><td class="success">✅ Passed</td><td>-</td></tr>
        <tr><td>Artifact Generation</td><td class="success">✅ Passed</td><td>-</td></tr>
    </table>

    <h2>Built Images</h2>
    <table>
        <tr><th>Variant</th><th>Image File</th><th>Size</th><th>Checksum</th></tr>
        IMAGES_TABLE_PLACEHOLDER
    </table>

    <h2>Artifacts</h2>
    <ul>
        <li>Build Manifest: <code>artifacts/build-manifest.json</code></li>
        <li>Checksums: <code>artifacts/checksums.sha256</code></li>
        <li>Release Notes: <code>artifacts/release-notes.md</code></li>
        <li>Build Logs: <code>artifacts/logs/</code></li>
    </ul>
</body>
</html>
EOF
    
    # Replace placeholders
    sed -i "s/BUILD_NUMBER_PLACEHOLDER/$CI_BUILD_NUMBER/g" "$report_file"
    sed -i "s/COMMIT_SHA_PLACEHOLDER/$CI_COMMIT_SHA/g" "$report_file"
    sed -i "s/BRANCH_PLACEHOLDER/$CI_BRANCH/g" "$report_file"
    sed -i "s/VERSION_PLACEHOLDER/$VEXFS_VERSION/g" "$report_file"
    sed -i "s/BUILD_DATE_PLACEHOLDER/$(date)/g" "$report_file"
    
    # Generate images table
    local images_table=""
    for variant in "${BUILD_VARIANTS[@]}"; do
        local image_files=$(find "$CI_OUTPUT_DIR" -name "*${variant}*.qcow2" 2>/dev/null || true)
        for image_file in $image_files; do
            if [ -f "$image_file" ]; then
                local image_name=$(basename "$image_file")
                local image_size=$(du -h "$image_file" | cut -f1)
                local checksum=$(sha256sum "$image_file" | cut -d' ' -f1 | cut -c1-16)
                images_table+="<tr><td>$variant</td><td>$image_name</td><td>$image_size</td><td>$checksum...</td></tr>"
            fi
        done
    done
    
    sed -i "s/IMAGES_TABLE_PLACEHOLDER/$images_table/g" "$report_file"
    
    log_success "CI build report generated: $report_file"
}

# Cleanup CI environment
cleanup_ci() {
    log_info "Cleaning up CI environment..."
    
    # Clean up any running QEMU processes
    pkill -f "qemu-system-x86_64.*vexfs" 2>/dev/null || true
    
    # Clean up temporary files
    rm -rf /tmp/vexfs-* 2>/dev/null || true
    
    log_success "CI cleanup completed"
}

# Main CI pipeline
main() {
    local pipeline_start_time=$(date +%s)
    
    echo "🚀 VexFS CI/CD Build Pipeline"
    echo "============================="
    echo "Build Number: $CI_BUILD_NUMBER"
    echo "Commit: $CI_COMMIT_SHA"
    echo "Branch: $CI_BRANCH"
    echo "Version: $VEXFS_VERSION"
    echo ""
    
    # Trap cleanup on exit
    trap cleanup_ci EXIT
    
    # Run CI pipeline stages
    setup_ci_environment
    check_ci_dependencies
    run_prebuild_checks
    build_ci_images
    run_ci_validation
    generate_ci_artifacts
    upload_artifacts
    generate_ci_report
    
    local pipeline_duration=$(($(date +%s) - pipeline_start_time))
    
    echo ""
    log_success "🎉 CI/CD pipeline completed successfully!"
    log_info "Total pipeline duration: ${pipeline_duration}s"
    log_info "Artifacts available in: $CI_OUTPUT_DIR"
    log_info "Build report: $CI_OUTPUT_DIR/ci-build-report.html"
    
    # Set exit code for CI system
    exit 0
}

# Handle command line arguments
case "${1:-}" in
    --help|-h)
        cat << EOF
VexFS CI/CD Build Pipeline

Usage: $0 [OPTIONS]

Environment Variables:
    CI_BUILD_NUMBER    Build number (default: timestamp)
    CI_COMMIT_SHA      Git commit SHA (default: current HEAD)
    CI_BRANCH          Git branch name (default: current branch)
    CI_TAG             Git tag (used for version if set)
    VEXFS_VERSION      VexFS version (default: 1.0.0-dev or CI_TAG)

Options:
    --help, -h         Show this help message

Examples:
    $0                                    # Run full CI pipeline
    CI_TAG=v1.0.0 $0                     # Build release version
    CI_BUILD_NUMBER=123 $0               # Set specific build number

EOF
        exit 0
        ;;
    *)
        main "$@"
        ;;
esac