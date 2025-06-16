#!/bin/bash
#
# VexFS CI/CD Setup Validation Script
# Quick validation that the CI/CD pipeline is properly configured
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Validate CI/CD setup
validate_cicd_setup() {
    header "VexFS CI/CD Pipeline Setup Validation"
    
    local errors=0
    
    # Check workflow files
    info "Checking GitHub Actions workflow files..."
    
    local workflows=(
        "vexfs-ci.yml"
        "quick-validation.yml"
        "nightly-comprehensive.yml"
        "release-testing.yml"
    )
    
    for workflow in "${workflows[@]}"; do
        if [ -f "$PROJECT_ROOT/.github/workflows/$workflow" ]; then
            success "Workflow found: $workflow"
        else
            error "Workflow missing: $workflow"
            ((errors++))
        fi
    done
    
    # Check CI scripts
    info "Checking CI/CD management scripts..."
    
    if [ -f "$PROJECT_ROOT/.github/scripts/ci-config.sh" ]; then
        success "CI configuration script found"
        if [ -x "$PROJECT_ROOT/.github/scripts/ci-config.sh" ]; then
            success "CI configuration script is executable"
        else
            warning "CI configuration script is not executable"
        fi
    else
        warning "CI configuration script missing (optional)"
    fi
    
    # Check test infrastructure integration
    info "Checking test infrastructure integration..."
    
    local test_scripts=(
        "tests/vm_testing/run_complete_test_suite.sh"
        "tests/vm_testing/vexfs_performance_benchmark.sh"
        "tests/vm_testing/manage_alpine_vm.sh"
        "tests/vm_testing/setup_xfstests.sh"
        "tests/vm_testing/setup_syzkaller_auto.sh"
        "tests/vm_testing/setup_ebpf_tracing.sh"
        "tests/vm_testing/avocado_vt/run_vexfs_orchestration.sh"
    )
    
    for script in "${test_scripts[@]}"; do
        if [ -f "$PROJECT_ROOT/$script" ]; then
            success "Test script found: $script"
        else
            error "Test script missing: $script"
            ((errors++))
        fi
    done
    
    # Check Task 33 subtask integration
    info "Checking Task 33 subtask integration..."
    
    local subtask_components=(
        "Performance Benchmarking (33.2): tests/vm_testing/vexfs_performance_benchmark.sh"
        "xfstests Integration (33.3): tests/vm_testing/setup_xfstests.sh"
        "Syzkaller Fuzzing (33.4): tests/vm_testing/setup_syzkaller_auto.sh"
        "eBPF Tracing (33.5): tests/vm_testing/setup_ebpf_tracing.sh"
        "Avocado-VT Orchestration (33.6): tests/vm_testing/avocado_vt/"
    )
    
    for component in "${subtask_components[@]}"; do
        local name=$(echo "$component" | cut -d':' -f1)
        local path=$(echo "$component" | cut -d':' -f2 | xargs)
        
        if [ -e "$PROJECT_ROOT/$path" ]; then
            success "$name integrated"
        else
            error "$name not found at $path"
            ((errors++))
        fi
    done
    
    # Check documentation
    info "Checking CI/CD documentation..."
    
    if [ -f "$PROJECT_ROOT/.github/README.md" ]; then
        success "CI/CD documentation found"
    else
        error "CI/CD documentation missing"
        ((errors++))
    fi
    
    # Summary
    header "Validation Summary"
    
    if [ $errors -eq 0 ]; then
        success "All CI/CD setup validations passed!"
        success "The pipeline is ready for use"
        
        info "Next steps:"
        echo "  1. Push changes to trigger quick validation"
        echo "  2. Create a pull request to test the full pipeline"
        echo "  3. Monitor workflow execution in GitHub Actions"
        echo "  4. Review artifacts and results"
        
        return 0
    else
        error "CI/CD setup validation failed with $errors errors"
        error "Please fix the issues above before using the pipeline"
        return 1
    fi
}

# Show pipeline overview
show_pipeline_overview() {
    header "VexFS CI/CD Pipeline Overview"
    
    cat << EOF

🚀 VexFS CI/CD Pipeline Components:

📋 WORKFLOWS:
  • Quick Validation      - Fast feedback on every push/PR (~20 min)
  • Main CI Pipeline      - Comprehensive testing (~3-4 hours)
  • Nightly Comprehensive - Extended testing and regression (~6-8 hours)
  • Release Testing       - Pre-release validation (~4-6 hours)

🧪 TEST INTEGRATION:
  • Task 33.2: Performance Benchmarking ✅
  • Task 33.3: xfstests Integration ✅
  • Task 33.4: Syzkaller Fuzzing ✅
  • Task 33.5: eBPF Tracing ✅
  • Task 33.6: Avocado-VT Orchestration ✅

⚡ PARALLEL EXECUTION:
  • VM testing levels run in parallel
  • Test suites execute concurrently
  • Optimized for performance and resource efficiency

📊 RESULTS & REPORTING:
  • Automated PR comments with test results
  • Comprehensive artifact collection
  • Performance regression analysis
  • Security vulnerability reports

🔧 MANAGEMENT:
  • Use .github/scripts/ci-config.sh for pipeline management
  • Local testing with 'act' for GitHub Actions simulation
  • Comprehensive monitoring and alerting

EOF
}

# Main execution
main() {
    local command="${1:-validate}"
    
    case "$command" in
        validate)
            validate_cicd_setup
            ;;
        overview)
            show_pipeline_overview
            ;;
        all)
            validate_cicd_setup
            show_pipeline_overview
            ;;
        *)
            echo "Usage: $0 [validate|overview|all]"
            echo ""
            echo "Commands:"
            echo "  validate  - Validate CI/CD setup and integration"
            echo "  overview  - Show pipeline overview"
            echo "  all       - Run all validations and show overview"
            exit 1
            ;;
    esac
}

# Execute main function
main "$@"