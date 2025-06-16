#!/bin/bash
#
# VexFS CI/CD Configuration and Management Script
# Provides utilities for managing the GitHub Actions CI/CD pipeline
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
    exit 1
}

header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

show_help() {
    cat << EOF
VexFS CI/CD Configuration and Management Script

USAGE:
    $0 [COMMAND] [OPTIONS]

COMMANDS:
    validate        Validate CI/CD workflow configurations
    status          Show current CI/CD pipeline status
    setup           Setup CI/CD environment and dependencies
    test-local      Run local CI/CD simulation
    clean           Clean CI/CD artifacts and caches
    monitor         Monitor running workflows
    report          Generate CI/CD performance report

OPTIONS:
    -h, --help      Show this help message
    -v, --verbose   Enable verbose output
    --dry-run       Show what would be done without executing

EXAMPLES:
    $0 validate                    # Validate all workflow files
    $0 setup                       # Setup CI/CD environment
    $0 test-local --dry-run        # Simulate local testing
    $0 monitor                     # Monitor active workflows
    $0 report --output-dir ./reports

EOF
}

# Validate workflow configurations
validate_workflows() {
    header "Validating CI/CD Workflow Configurations"
    
    local workflows_dir="$PROJECT_ROOT/.github/workflows"
    local errors=0
    
    if [ ! -d "$workflows_dir" ]; then
        error "Workflows directory not found: $workflows_dir"
    fi
    
    log "Checking workflow files..."
    
    # Check main CI workflow
    if [ -f "$workflows_dir/vexfs-ci.yml" ]; then
        success "Main CI workflow found"
        
        # Validate YAML syntax
        if command -v yamllint &> /dev/null; then
            if yamllint "$workflows_dir/vexfs-ci.yml" &> /dev/null; then
                success "Main CI workflow YAML is valid"
            else
                error "Main CI workflow YAML has syntax errors"
                ((errors++))
            fi
        else
            warning "yamllint not found, skipping YAML validation"
        fi
    else
        error "Main CI workflow not found"
        ((errors++))
    fi
    
    # Check quick validation workflow
    if [ -f "$workflows_dir/quick-validation.yml" ]; then
        success "Quick validation workflow found"
    else
        error "Quick validation workflow not found"
        ((errors++))
    fi
    
    # Check nightly workflow
    if [ -f "$workflows_dir/nightly-comprehensive.yml" ]; then
        success "Nightly comprehensive workflow found"
    else
        error "Nightly comprehensive workflow not found"
        ((errors++))
    fi
    
    # Check release workflow
    if [ -f "$workflows_dir/release-testing.yml" ]; then
        success "Release testing workflow found"
    else
        error "Release testing workflow not found"
        ((errors++))
    fi
    
    # Validate workflow dependencies
    log "Checking workflow dependencies..."
    
    local required_scripts=(
        "tests/vm_testing/run_complete_test_suite.sh"
        "tests/vm_testing/vexfs_performance_benchmark.sh"
        "tests/vm_testing/manage_alpine_vm.sh"
        "tests/vm_testing/setup_xfstests.sh"
        "tests/vm_testing/setup_syzkaller_auto.sh"
        "tests/vm_testing/setup_ebpf_tracing.sh"
        "tests/vm_testing/avocado_vt/run_vexfs_orchestration.sh"
    )
    
    for script in "${required_scripts[@]}"; do
        if [ -f "$PROJECT_ROOT/$script" ]; then
            success "Required script found: $script"
        else
            error "Required script missing: $script"
            ((errors++))
        fi
    done
    
    if [ $errors -eq 0 ]; then
        success "All CI/CD workflow validations passed"
        return 0
    else
        error "CI/CD workflow validation failed with $errors errors"
        return 1
    fi
}

# Show CI/CD pipeline status
show_status() {
    header "VexFS CI/CD Pipeline Status"
    
    log "Checking GitHub Actions integration..."
    
    # Check if we're in a git repository
    if ! git rev-parse --git-dir &> /dev/null; then
        error "Not in a git repository"
    fi
    
    # Check current branch
    local current_branch=$(git branch --show-current)
    log "Current branch: $current_branch"
    
    # Check recent commits
    log "Recent commits:"
    git log --oneline -5
    
    # Check workflow files
    local workflows_dir="$PROJECT_ROOT/.github/workflows"
    local workflow_count=$(find "$workflows_dir" -name "*.yml" -o -name "*.yaml" | wc -l)
    log "Workflow files: $workflow_count"
    
    # List workflows
    log "Available workflows:"
    for workflow in "$workflows_dir"/*.yml; do
        if [ -f "$workflow" ]; then
            local name=$(basename "$workflow" .yml)
            echo "  - $name"
        fi
    done
    
    # Check test infrastructure
    log "Test infrastructure status:"
    if [ -d "$PROJECT_ROOT/tests/vm_testing" ]; then
        success "VM testing infrastructure available"
    else
        warning "VM testing infrastructure not found"
    fi
    
    if [ -d "$PROJECT_ROOT/tests/kernel_module" ]; then
        success "Kernel module tests available"
    else
        warning "Kernel module tests not found"
    fi
    
    success "CI/CD pipeline status check completed"
}

# Setup CI/CD environment
setup_cicd() {
    header "Setting Up CI/CD Environment"
    
    log "Installing CI/CD dependencies..."
    
    # Install yamllint for workflow validation
    if ! command -v yamllint &> /dev/null; then
        log "Installing yamllint..."
        pip3 install yamllint || sudo apt-get install -y yamllint
        success "yamllint installed"
    else
        success "yamllint already available"
    fi
    
    # Install act for local GitHub Actions testing
    if ! command -v act &> /dev/null; then
        log "Installing act (GitHub Actions local runner)..."
        curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash
        success "act installed"
    else
        success "act already available"
    fi
    
    # Setup local testing environment
    log "Setting up local testing environment..."
    
    # Create local CI directories
    mkdir -p "$PROJECT_ROOT/.ci"
    mkdir -p "$PROJECT_ROOT/.ci/cache"
    mkdir -p "$PROJECT_ROOT/.ci/artifacts"
    mkdir -p "$PROJECT_ROOT/.ci/logs"
    
    # Create act configuration
    cat > "$PROJECT_ROOT/.ci/act.yml" << EOF
# Act configuration for local GitHub Actions testing
version: 1

container-architecture: linux/amd64
container-daemon-socket: /var/run/docker.sock

# Use larger runner for VM testing
platforms:
  ubuntu-latest: catthehacker/ubuntu:act-latest

# Environment variables for local testing
env:
  VEXFS_CI: true
  VEXFS_LOCAL_TEST: true
  OUTPUT_DIR: .ci/artifacts
  
# Secrets (add your own as needed)
secrets:
  # Add any required secrets here
  
# Artifact path
artifact-server-path: .ci/artifacts
EOF
    
    success "CI/CD environment setup completed"
}

# Run local CI/CD simulation
test_local() {
    header "Running Local CI/CD Simulation"
    
    local dry_run=false
    if [[ "${1:-}" == "--dry-run" ]]; then
        dry_run=true
        log "Running in dry-run mode"
    fi
    
    # Check if act is available
    if ! command -v act &> /dev/null; then
        error "act not found. Run '$0 setup' first."
    fi
    
    # Check if Docker is running
    if ! docker info &> /dev/null; then
        error "Docker is not running. Please start Docker first."
    fi
    
    log "Running quick validation workflow locally..."
    
    if [ "$dry_run" = true ]; then
        log "Would run: act -W .github/workflows/quick-validation.yml --artifact-server-path .ci/artifacts"
    else
        cd "$PROJECT_ROOT"
        act -W .github/workflows/quick-validation.yml \
            --artifact-server-path .ci/artifacts \
            --env-file .ci/act.yml \
            --verbose
    fi
    
    success "Local CI/CD simulation completed"
}

# Clean CI/CD artifacts
clean_cicd() {
    header "Cleaning CI/CD Artifacts"
    
    log "Removing local CI artifacts..."
    rm -rf "$PROJECT_ROOT/.ci/artifacts"/*
    rm -rf "$PROJECT_ROOT/.ci/cache"/*
    rm -rf "$PROJECT_ROOT/.ci/logs"/*
    
    log "Cleaning test results..."
    rm -rf "$PROJECT_ROOT/test-results"
    rm -rf "$PROJECT_ROOT/performance-results"
    rm -rf "$PROJECT_ROOT/unified_test_results"
    
    log "Cleaning build artifacts..."
    if [ -d "$PROJECT_ROOT/rust" ]; then
        cd "$PROJECT_ROOT/rust"
        cargo clean
    fi
    
    if [ -d "$PROJECT_ROOT/tests/kernel_module" ]; then
        cd "$PROJECT_ROOT/tests/kernel_module"
        cargo clean
    fi
    
    success "CI/CD cleanup completed"
}

# Monitor workflows
monitor_workflows() {
    header "Monitoring CI/CD Workflows"
    
    log "Checking for GitHub CLI..."
    if ! command -v gh &> /dev/null; then
        warning "GitHub CLI not found. Install with: sudo apt install gh"
        log "Showing local status instead..."
        show_status
        return
    fi
    
    log "Fetching workflow runs..."
    gh run list --limit 10
    
    log "Checking workflow status..."
    gh run list --status in_progress
    
    success "Workflow monitoring completed"
}

# Generate CI/CD report
generate_report() {
    header "Generating CI/CD Performance Report"
    
    local output_dir="${1:-./ci-reports}"
    mkdir -p "$output_dir"
    
    local report_file="$output_dir/cicd-report-$(date +%Y%m%d_%H%M%S).md"
    
    cat > "$report_file" << EOF
# VexFS CI/CD Pipeline Report

**Generated:** $(date)
**Project:** VexFS Vector-Enhanced Filesystem
**Repository:** $(git remote get-url origin 2>/dev/null || echo "Local repository")

## Pipeline Overview

The VexFS CI/CD pipeline consists of multiple workflows designed to ensure code quality,
performance, and reliability across all components.

### Workflows

1. **Quick Validation** (\`quick-validation.yml\`)
   - Triggers: Push to any branch, Pull requests
   - Duration: ~20 minutes
   - Purpose: Fast feedback on basic build and test validation

2. **Main CI Pipeline** (\`vexfs-ci.yml\`)
   - Triggers: Push to main/develop, Pull requests, Manual dispatch
   - Duration: ~3-4 hours
   - Purpose: Comprehensive testing including VM-based kernel module testing

3. **Nightly Comprehensive** (\`nightly-comprehensive.yml\`)
   - Triggers: Scheduled (2 AM UTC), Manual dispatch
   - Duration: ~6-8 hours
   - Purpose: Extended testing, performance regression, fuzzing

4. **Release Testing** (\`release-testing.yml\`)
   - Triggers: Tags, Releases, Manual dispatch
   - Duration: ~4-6 hours
   - Purpose: Pre-release validation and packaging

## Test Coverage

### Kernel Module Testing
- ✅ Basic load/unload validation
- ✅ VM-based mount operations
- ✅ Stress testing and stability
- ✅ Memory safety validation

### Performance Testing
- ✅ Kernel vs FUSE performance comparison
- ✅ Regression testing against baselines
- ✅ Large-scale data handling

### Security Testing
- ✅ Syzkaller fuzzing
- ✅ Memory safety with Valgrind
- ✅ Static analysis with Clippy

### Integration Testing
- ✅ xfstests compatibility
- ✅ eBPF tracing analysis
- ✅ Avocado-VT orchestration

## Artifacts and Results

All test runs generate comprehensive artifacts including:
- Build artifacts (kernel module, Rust binaries)
- Test results (JSON, HTML reports)
- Performance data and benchmarks
- Security analysis reports
- Documentation and release packages

## Recommendations

1. Monitor nightly test results for performance regressions
2. Review fuzzing results for potential security issues
3. Keep baseline performance metrics updated
4. Ensure all prerequisite subtasks remain functional

---
*Generated by VexFS CI/CD Management Script*
EOF
    
    success "CI/CD report generated: $report_file"
}

# Main function
main() {
    local command="${1:-help}"
    shift || true
    
    case "$command" in
        validate)
            validate_workflows "$@"
            ;;
        status)
            show_status "$@"
            ;;
        setup)
            setup_cicd "$@"
            ;;
        test-local)
            test_local "$@"
            ;;
        clean)
            clean_cicd "$@"
            ;;
        monitor)
            monitor_workflows "$@"
            ;;
        report)
            generate_report "$@"
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            error "Unknown command: $command"
            show_help
            exit 1
            ;;
    esac
}

# Execute main function with all arguments
main "$@"