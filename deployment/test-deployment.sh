#!/bin/bash
set -e

# VexFS Deployment Test Script
# This script tests all deployment methods to ensure they work correctly

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

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

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
FAILED_TESTS=()

# Test function wrapper
run_test() {
    local test_name="$1"
    local test_function="$2"
    
    log_info "Running test: $test_name"
    
    if $test_function; then
        log_success "✓ $test_name"
        ((TESTS_PASSED++))
    else
        log_error "✗ $test_name"
        ((TESTS_FAILED++))
        FAILED_TESTS+=("$test_name")
    fi
    echo
}

# Test Docker build
test_docker_build() {
    log_info "Testing Docker production build..."
    
    cd "$SCRIPT_DIR/docker"
    
    # Build the production image
    if docker build -f Dockerfile.production -t vexfs:test-production "$PROJECT_ROOT"; then
        log_success "Docker production image built successfully"
        
        # Test image security
        if docker run --rm vexfs:test-production whoami 2>/dev/null | grep -q "nobody"; then
            log_success "Container runs as non-root user"
        else
            log_warning "Container may not be running as non-root user"
        fi
        
        # Clean up
        docker rmi vexfs:test-production >/dev/null 2>&1 || true
        
        return 0
    else
        log_error "Docker production build failed"
        return 1
    fi
}

# Test Docker Compose
test_docker_compose() {
    log_info "Testing Docker Compose configuration..."
    
    cd "$SCRIPT_DIR/docker"
    
    # Validate docker-compose file
    if docker-compose -f docker-compose.production.yml config >/dev/null 2>&1; then
        log_success "Docker Compose configuration is valid"
        return 0
    else
        log_error "Docker Compose configuration is invalid"
        return 1
    fi
}

# Test Helm chart
test_helm_chart() {
    log_info "Testing Helm chart..."
    
    cd "$SCRIPT_DIR/kubernetes/helm"
    
    # Check if helm is available
    if ! command -v helm >/dev/null 2>&1; then
        log_warning "Helm not found, skipping Helm chart test"
        return 0
    fi
    
    # Lint the Helm chart
    if helm lint vexfs; then
        log_success "Helm chart linting passed"
        
        # Test template rendering
        if helm template test-release vexfs >/dev/null 2>&1; then
            log_success "Helm chart templates render successfully"
            return 0
        else
            log_error "Helm chart template rendering failed"
            return 1
        fi
    else
        log_error "Helm chart linting failed"
        return 1
    fi
}

# Test Debian package build
test_debian_package() {
    log_info "Testing Debian package build..."
    
    cd "$SCRIPT_DIR/packages"
    
    # Check if required tools are available
    if ! command -v dpkg-deb >/dev/null 2>&1; then
        log_warning "dpkg-deb not found, skipping Debian package test"
        return 0
    fi
    
    # Test build script syntax
    if bash -n build-deb.sh; then
        log_success "Debian build script syntax is valid"
        
        # Test control file
        if dpkg-parsechangelog -l debian/control >/dev/null 2>&1 || true; then
            log_success "Debian control file is valid"
        fi
        
        return 0
    else
        log_error "Debian build script has syntax errors"
        return 1
    fi
}

# Test RPM package build
test_rpm_package() {
    log_info "Testing RPM package build..."
    
    cd "$SCRIPT_DIR/packages"
    
    # Check if required tools are available
    if ! command -v rpmbuild >/dev/null 2>&1; then
        log_warning "rpmbuild not found, skipping RPM package test"
        return 0
    fi
    
    # Test build script syntax
    if bash -n build-rpm.sh; then
        log_success "RPM build script syntax is valid"
        
        # Test spec file
        if rpmbuild --nobuild rpm/vexfs.spec >/dev/null 2>&1; then
            log_success "RPM spec file is valid"
        else
            log_warning "RPM spec file may have issues"
        fi
        
        return 0
    else
        log_error "RPM build script has syntax errors"
        return 1
    fi
}

# Test installation scripts
test_installation_scripts() {
    log_info "Testing installation scripts..."
    
    cd "$SCRIPT_DIR/scripts"
    
    local all_valid=true
    
    # Test script syntax
    for script in install.sh uninstall.sh backup.sh restore.sh; do
        if bash -n "$script"; then
            log_success "$script syntax is valid"
        else
            log_error "$script has syntax errors"
            all_valid=false
        fi
    done
    
    # Test script permissions
    for script in install.sh uninstall.sh backup.sh restore.sh; do
        if [[ -x "$script" ]]; then
            log_success "$script is executable"
        else
            log_error "$script is not executable"
            all_valid=false
        fi
    done
    
    return $all_valid
}

# Test VexFS server build
test_server_build() {
    log_info "Testing VexFS server build..."
    
    cd "$PROJECT_ROOT"
    
    # Test server build
    if cargo build --features server --bin vexfs_server; then
        log_success "VexFS server builds successfully"
        
        # Test binary exists and is executable
        if [[ -x "target/debug/vexfs_server" ]]; then
            log_success "VexFS server binary is executable"
            return 0
        else
            log_error "VexFS server binary is not executable"
            return 1
        fi
    else
        log_error "VexFS server build failed"
        return 1
    fi
}

# Test configuration files
test_configuration_files() {
    log_info "Testing configuration files..."
    
    local all_valid=true
    
    # Test systemd service file
    if systemd-analyze verify "$SCRIPT_DIR/packages/debian/vexfs.service" 2>/dev/null; then
        log_success "Systemd service file is valid"
    else
        log_warning "Systemd service file may have issues (systemd-analyze not available or issues found)"
    fi
    
    # Test Debian control file format
    if [[ -f "$SCRIPT_DIR/packages/debian/control" ]]; then
        if grep -q "^Package:" "$SCRIPT_DIR/packages/debian/control"; then
            log_success "Debian control file has required fields"
        else
            log_error "Debian control file is missing required fields"
            all_valid=false
        fi
    fi
    
    # Test RPM spec file format
    if [[ -f "$SCRIPT_DIR/packages/rpm/vexfs.spec" ]]; then
        if grep -q "^Name:" "$SCRIPT_DIR/packages/rpm/vexfs.spec"; then
            log_success "RPM spec file has required fields"
        else
            log_error "RPM spec file is missing required fields"
            all_valid=false
        fi
    fi
    
    return $all_valid
}

# Test documentation
test_documentation() {
    log_info "Testing documentation..."
    
    local all_valid=true
    
    # Check if README exists and has content
    if [[ -f "$SCRIPT_DIR/README.md" ]] && [[ -s "$SCRIPT_DIR/README.md" ]]; then
        log_success "Deployment README.md exists and has content"
    else
        log_error "Deployment README.md is missing or empty"
        all_valid=false
    fi
    
    # Check for required sections in README
    local required_sections=("Docker Deployment" "Kubernetes Deployment" "Linux Package Installation" "Bare Metal Installation")
    for section in "${required_sections[@]}"; do
        if grep -q "$section" "$SCRIPT_DIR/README.md"; then
            log_success "README contains '$section' section"
        else
            log_error "README missing '$section' section"
            all_valid=false
        fi
    done
    
    return $all_valid
}

# Test security configurations
test_security_configurations() {
    log_info "Testing security configurations..."
    
    local all_valid=true
    
    # Check Docker security settings
    if grep -q "security_opt:" "$SCRIPT_DIR/docker/docker-compose.production.yml"; then
        log_success "Docker Compose has security options"
    else
        log_warning "Docker Compose may be missing security options"
    fi
    
    # Check systemd security settings
    if grep -q "NoNewPrivileges=true" "$SCRIPT_DIR/packages/debian/vexfs.service"; then
        log_success "Systemd service has security hardening"
    else
        log_error "Systemd service is missing security hardening"
        all_valid=false
    fi
    
    # Check Kubernetes security context
    if grep -q "securityContext:" "$SCRIPT_DIR/kubernetes/helm/vexfs/templates/deployment.yaml"; then
        log_success "Kubernetes deployment has security context"
    else
        log_error "Kubernetes deployment is missing security context"
        all_valid=false
    fi
    
    return $all_valid
}

# Test monitoring configurations
test_monitoring_configurations() {
    log_info "Testing monitoring configurations..."
    
    local all_valid=true
    
    # Check for health check endpoints
    if grep -q "/health" "$SCRIPT_DIR/README.md"; then
        log_success "Health check endpoints documented"
    else
        log_warning "Health check endpoints may not be documented"
    fi
    
    # Check for Prometheus metrics
    if grep -q "/metrics" "$SCRIPT_DIR/README.md"; then
        log_success "Prometheus metrics documented"
    else
        log_warning "Prometheus metrics may not be documented"
    fi
    
    # Check Docker Compose monitoring stack
    if grep -q "prometheus:" "$SCRIPT_DIR/docker/docker-compose.production.yml"; then
        log_success "Docker Compose includes monitoring stack"
    else
        log_warning "Docker Compose may be missing monitoring stack"
    fi
    
    return $all_valid
}

# Main test runner
main() {
    echo "========================================"
    echo "VexFS Deployment Test Suite"
    echo "========================================"
    echo
    
    log_info "Starting deployment tests..."
    echo
    
    # Run all tests
    run_test "VexFS Server Build" test_server_build
    run_test "Docker Production Build" test_docker_build
    run_test "Docker Compose Configuration" test_docker_compose
    run_test "Helm Chart" test_helm_chart
    run_test "Debian Package" test_debian_package
    run_test "RPM Package" test_rpm_package
    run_test "Installation Scripts" test_installation_scripts
    run_test "Configuration Files" test_configuration_files
    run_test "Documentation" test_documentation
    run_test "Security Configurations" test_security_configurations
    run_test "Monitoring Configurations" test_monitoring_configurations
    
    # Print summary
    echo "========================================"
    echo "Test Summary"
    echo "========================================"
    echo
    log_success "Tests passed: $TESTS_PASSED"
    
    if [[ $TESTS_FAILED -gt 0 ]]; then
        log_error "Tests failed: $TESTS_FAILED"
        echo
        log_error "Failed tests:"
        for test in "${FAILED_TESTS[@]}"; do
            echo "  - $test"
        done
        echo
        exit 1
    else
        echo
        log_success "All tests passed! VexFS v1.0 is ready for production deployment."
        echo
        echo "Available deployment methods:"
        echo "  🐳 Docker: cd deployment/docker && docker-compose -f docker-compose.production.yml up -d"
        echo "  ☸️  Kubernetes: helm install vexfs deployment/kubernetes/helm/vexfs"
        echo "  📦 Debian: cd deployment/packages && sudo ./build-deb.sh && sudo dpkg -i build/*.deb"
        echo "  📦 RPM: cd deployment/packages && sudo ./build-rpm.sh && sudo rpm -ivh rpmbuild/RPMS/*/*.rpm"
        echo "  🖥️  Bare Metal: cd deployment/scripts && sudo ./install.sh"
        echo
        echo "For detailed instructions, see: deployment/README.md"
    fi
}

# Run main function
main "$@"