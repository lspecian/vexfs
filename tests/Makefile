# VexFS Test Discovery and Execution Makefile
# Provides convenient commands for selective test execution based on tags and domains

# Default Python and Rust commands
PYTHON := python3
CARGO := cargo
PYTEST := pytest
RUST_TEST := cargo test

# Test directories
TEST_DIR := .
DOMAINS_DIR := domains
LEGACY_DIR := legacy
RESULTS_DIR := test_results

# Create results directory
$(RESULTS_DIR):
	mkdir -p $(RESULTS_DIR)

# Help target
.PHONY: help
help:
	@echo "VexFS Test Discovery and Execution"
	@echo "=================================="
	@echo ""
	@echo "Test Type Commands:"
	@echo "  test-unit          Run only unit tests"
	@echo "  test-integration   Run integration tests"
	@echo "  test-performance   Run performance tests"
	@echo "  test-security      Run security tests"
	@echo ""
	@echo "Domain Commands:"
	@echo "  test-kernel        Run kernel module tests"
	@echo "  test-filesystem    Run filesystem tests"
	@echo "  test-vector        Run vector operations tests"
	@echo "  test-fuse          Run FUSE implementation tests"
	@echo ""
	@echo "Complexity Commands:"
	@echo "  test-quick         Run quick tests only (< 10s)"
	@echo "  test-slow          Run slow tests only (> 30s)"
	@echo "  test-vm            Run VM-required tests"
	@echo ""
	@echo "Safety Commands:"
	@echo "  test-safe          Run safe tests only"
	@echo "  test-monitored     Run monitored tests"
	@echo "  test-risky         Run risky tests (VM recommended)"
	@echo ""
	@echo "Language-Specific Commands:"
	@echo "  test-python        Run all Python tests"
	@echo "  test-rust          Run all Rust tests"
	@echo ""
	@echo "Combined Commands:"
	@echo "  test-all           Run all tests"
	@echo "  test-ci            Run CI-appropriate tests"
	@echo "  test-dev           Run development tests"
	@echo ""
	@echo "Utility Commands:"
	@echo "  clean              Clean test results"
	@echo "  list-tests         List all available tests"
	@echo "  list-tags          List all available tags"
	@echo "  validate-tags      Validate test tags"

# Test Type Commands
.PHONY: test-unit
test-unit: $(RESULTS_DIR)
	@echo "Running unit tests..."
	$(PYTEST) -m "unit" --tb=short

.PHONY: test-integration
test-integration: $(RESULTS_DIR)
	@echo "Running integration tests..."
	$(PYTEST) -m "integration" --tb=short

.PHONY: test-performance
test-performance: $(RESULTS_DIR)
	@echo "Running performance tests..."
	$(PYTEST) -m "performance" --tb=short

.PHONY: test-security
test-security: $(RESULTS_DIR)
	@echo "Running security tests..."
	$(PYTEST) -m "security" --tb=short

# Domain Commands
.PHONY: test-kernel
test-kernel: $(RESULTS_DIR)
	@echo "Running kernel module tests..."
	$(PYTEST) -m "kernel_module" --tb=short

.PHONY: test-filesystem
test-filesystem: $(RESULTS_DIR)
	@echo "Running filesystem tests..."
	$(PYTEST) -m "filesystem" --tb=short

.PHONY: test-vector
test-vector: $(RESULTS_DIR)
	@echo "Running vector operations tests..."
	$(PYTEST) -m "vector_operations" --tb=short

.PHONY: test-fuse
test-fuse: $(RESULTS_DIR)
	@echo "Running FUSE implementation tests..."
	$(PYTEST) -m "fuse" --tb=short

# Complexity Commands
.PHONY: test-quick
test-quick: $(RESULTS_DIR)
	@echo "Running quick tests..."
	$(PYTEST) -m "quick" --tb=short

.PHONY: test-slow
test-slow: $(RESULTS_DIR)
	@echo "Running slow tests..."
	$(PYTEST) -m "slow" --tb=short

.PHONY: test-vm
test-vm: $(RESULTS_DIR)
	@echo "Running VM-required tests..."
	$(PYTEST) -m "vm_required" --tb=short

# Safety Commands
.PHONY: test-safe
test-safe: $(RESULTS_DIR)
	@echo "Running safe tests..."
	$(PYTEST) -m "safe" --tb=short

.PHONY: test-monitored
test-monitored: $(RESULTS_DIR)
	@echo "Running monitored tests..."
	$(PYTEST) -m "monitored" --tb=short

.PHONY: test-risky
test-risky: $(RESULTS_DIR)
	@echo "Running risky tests (VM recommended)..."
	$(PYTEST) -m "risky" --tb=short

# Language-Specific Commands
.PHONY: test-python
test-python: $(RESULTS_DIR)
	@echo "Running all Python tests..."
	$(PYTEST) $(DOMAINS_DIR) --tb=short

.PHONY: test-rust
test-rust: $(RESULTS_DIR)
	@echo "Running all Rust tests..."
	cd .. && $(RUST_TEST) --workspace

# Combined Commands
.PHONY: test-all
test-all: $(RESULTS_DIR)
	@echo "Running all tests..."
	$(PYTEST) --tb=short
	cd .. && $(RUST_TEST) --workspace

.PHONY: test-ci
test-ci: $(RESULTS_DIR)
	@echo "Running CI-appropriate tests..."
	$(PYTEST) -m "safe and quick" --tb=short

.PHONY: test-dev
test-dev: $(RESULTS_DIR)
	@echo "Running development tests..."
	$(PYTEST) -m "unit or (integration and safe)" --tb=short

# Domain-specific test commands with parameters
.PHONY: test-domain
test-domain: $(RESULTS_DIR)
	@if [ -z "$(DOMAIN)" ]; then \
		echo "Usage: make test-domain DOMAIN=<domain_name>"; \
		echo "Available domains: kernel_module, filesystem, vector_operations, fuse, security, performance, integration"; \
		exit 1; \
	fi
	@echo "Running tests for domain: $(DOMAIN)"
	$(PYTEST) -m "$(DOMAIN)" --tb=short

# Custom test execution with tags
.PHONY: test-tags
test-tags: $(RESULTS_DIR)
	@if [ -z "$(TAGS)" ]; then \
		echo "Usage: make test-tags TAGS='tag1 and tag2'"; \
		echo "Example: make test-tags TAGS='unit and filesystem'"; \
		exit 1; \
	fi
	@echo "Running tests with tags: $(TAGS)"
	$(PYTEST) -m "$(TAGS)" --tb=short

# Utility Commands
.PHONY: clean
clean:
	@echo "Cleaning test results..."
	rm -rf $(RESULTS_DIR)
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true
	find . -type f -name ".coverage" -delete 2>/dev/null || true

.PHONY: list-tests
list-tests:
	@echo "Listing all available tests..."
	$(PYTEST) --collect-only -q

.PHONY: list-tags
list-tags:
	@echo "Available test tags:"
	@echo "==================="
	@echo "Test Types: unit, integration, performance, security"
	@echo "Domains: kernel_module, filesystem, vector_operations, fuse"
	@echo "Complexity: quick, medium, slow, vm_required, root_required"
	@echo "Safety: safe, monitored, risky, dangerous"
	@echo "Features: cow_snapshots, vector_search, anns, memory_management, concurrency, error_handling"

.PHONY: validate-tags
validate-tags:
	@echo "Validating test tags..."
	$(PYTHON) -c "from domains.shared.test_tags import validate_tags, list_all_tags; \
		tags = list_all_tags(); \
		errors = validate_tags(tags); \
		print('All tags valid!' if not errors else f'Errors: {errors}')"

# Test runner with specific patterns
.PHONY: test-pattern
test-pattern: $(RESULTS_DIR)
	@if [ -z "$(PATTERN)" ]; then \
		echo "Usage: make test-pattern PATTERN=<pattern>"; \
		echo "Example: make test-pattern PATTERN='test_*filesystem*'"; \
		exit 1; \
	fi
	@echo "Running tests matching pattern: $(PATTERN)"
	$(PYTEST) -k "$(PATTERN)" --tb=short

# Parallel test execution
.PHONY: test-parallel
test-parallel: $(RESULTS_DIR)
	@echo "Running tests in parallel..."
	$(PYTEST) -n auto --tb=short

# Test with coverage
.PHONY: test-coverage
test-coverage: $(RESULTS_DIR)
	@echo "Running tests with coverage..."
	$(PYTEST) --cov=domains --cov-report=html:$(RESULTS_DIR)/coverage_html --cov-report=term-missing

# Stress testing
.PHONY: test-stress
test-stress: $(RESULTS_DIR)
	@echo "Running stress tests..."
	$(PYTEST) -m "performance or slow" --tb=short

# Test specific file
.PHONY: test-file
test-file: $(RESULTS_DIR)
	@if [ -z "$(FILE)" ]; then \
		echo "Usage: make test-file FILE=<file_path>"; \
		echo "Example: make test-file FILE=domains/kernel_module/test_loading.py"; \
		exit 1; \
	fi
	@echo "Running tests in file: $(FILE)"
	$(PYTEST) $(FILE) --tb=short

# Generate test report
.PHONY: test-report
test-report: $(RESULTS_DIR)
	@echo "Generating comprehensive test report..."
	$(PYTEST) --html=$(RESULTS_DIR)/comprehensive_report.html --self-contained-html --tb=short

# Examples of complex test combinations
.PHONY: test-examples
test-examples:
	@echo "Example test combinations:"
	@echo "========================="
	@echo "make test-tags TAGS='unit and filesystem and safe'"
	@echo "make test-tags TAGS='integration and kernel_module'"
	@echo "make test-tags TAGS='performance and not dangerous'"
	@echo "make test-tags TAGS='quick and safe'"
	@echo "make test-pattern PATTERN='test_vector_*'"
	@echo "make test-domain DOMAIN=filesystem"

# Default target
.DEFAULT_GOAL := help