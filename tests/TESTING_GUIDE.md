# VexFS Testing Guide

This guide provides comprehensive instructions for running tests in the VexFS project using the new Domain-Driven Design (DDD) testing architecture with improved test discovery and selective execution capabilities.

## Quick Start

```bash
# Navigate to tests directory
cd tests/

# Run all tests
make test-all

# Run only unit tests
make test-unit

# Run tests for a specific domain
make test-domain DOMAIN=filesystem

# Run quick tests only
make test-quick

# List available test commands
make help
```

## Test Organization

### Domain Structure

Tests are organized by business domains following DDD principles:

```
tests/domains/
├── filesystem/          # File and directory operations
│   ├── operations/      # Basic CRUD operations
│   ├── metadata/        # File metadata and attributes
│   ├── permissions/     # Access control and permissions
│   └── vfs_integration/ # VFS layer integration
├── kernel_module/       # Kernel module functionality
│   ├── loading/         # Module loading/unloading
│   ├── syscalls/        # System call interface
│   ├── stability/       # Stability and reliability
│   └── memory_management/ # Memory allocation/deallocation
├── vector_operations/   # Vector search and ANNS
│   ├── storage/         # Vector storage and retrieval
│   ├── search/          # Search algorithms and performance
│   ├── indexing/        # Index building and maintenance
│   └── caching/         # Vector caching strategies
├── performance/         # Performance and benchmarking
│   ├── throughput/      # Throughput measurements
│   ├── latency/         # Latency measurements
│   ├── memory/          # Memory usage analysis
│   └── concurrent/      # Concurrency performance
├── security/            # Security and access control
│   ├── access_control/  # Permission enforcement
│   ├── encryption/      # Data encryption
│   ├── integrity/       # Data integrity verification
│   └── privilege/       # Privilege escalation prevention
└── integration/         # Cross-component integration
    ├── end_to_end/      # Complete workflow tests
    ├── cross_component/ # Component interaction tests
    └── system_recovery/  # Recovery and resilience tests
```

## Test Categories

### By Test Type

- **Unit Tests**: Test individual functions and components in isolation
- **Integration Tests**: Test component interactions and interfaces
- **Performance Tests**: Measure and benchmark system performance
- **Security Tests**: Verify security controls and access restrictions

### By Complexity

- **Quick** (`quick`): Fast tests that complete in seconds
- **Medium** (`medium`): Moderate tests that may take minutes
- **Slow** (`slow`): Comprehensive tests that may take significant time

### By Safety Level

- **Safe** (`safe`): Tests that don't modify system state or require privileges
- **Monitored** (`monitored`): Tests that modify state but are carefully controlled
- **Risky** (`risky`): Tests that may affect system stability
- **Dangerous** (`dangerous`): Tests requiring extreme caution (VM recommended)

### By Requirements

- **VM Required** (`vm_required`): Tests that must run in a virtual machine
- **Root Required** (`root_required`): Tests requiring root/administrator privileges

## Running Tests

### Basic Test Execution

```bash
# Run all tests
make test-all

# Run tests by type
make test-unit           # Unit tests only
make test-integration    # Integration tests only
make test-performance    # Performance tests only
make test-security       # Security tests only
```

### Domain-Specific Testing

```bash
# Test specific domains
make test-domain DOMAIN=filesystem
make test-domain DOMAIN=kernel_module
make test-domain DOMAIN=vector_operations
make test-domain DOMAIN=performance
make test-domain DOMAIN=security
make test-domain DOMAIN=integration

# Test specific features within a domain
make test-feature DOMAIN=filesystem FEATURE=operations
make test-feature DOMAIN=kernel_module FEATURE=loading
make test-feature DOMAIN=vector_operations FEATURE=search
```

### Complexity-Based Testing

```bash
# Run tests by complexity
make test-quick          # Quick tests only (< 30 seconds)
make test-medium         # Medium complexity tests
make test-slow           # Slow/comprehensive tests

# Exclude slow tests
make test-no-slow        # All tests except slow ones
```

### Safety-Based Testing

```bash
# Run tests by safety level
make test-safe           # Safe tests only
make test-monitored      # Monitored tests (careful state changes)
make test-risky          # Risky tests (may affect stability)

# Exclude dangerous tests
make test-no-dangerous   # All tests except dangerous ones
```

### Requirement-Based Testing

```bash
# Run tests with specific requirements
make test-vm-required    # Tests that require VM environment
make test-root-required  # Tests requiring root privileges
make test-no-root        # Tests that don't require root
```

### Combined Filters

```bash
# Combine multiple filters
make test-unit-safe      # Unit tests that are safe
make test-integration-quick  # Quick integration tests
make test-performance-monitored  # Monitored performance tests

# Custom combinations using pytest directly
pytest -m "unit and filesystem and quick and safe"
pytest -m "integration and not slow"
pytest -m "performance and vector_operations"
```

## Python Test Examples

### Using Test Tags

```python
from tests.domains.shared.test_tags import tag, unit_test, integration_test

class TestExample:
    @unit_test("filesystem", "quick", "safe")
    def test_file_creation(self):
        """Test file creation functionality."""
        pass
    
    @integration_test("filesystem", "medium", "monitored")
    def test_vfs_integration(self):
        """Test VFS layer integration."""
        pass
    
    @tag("performance", "vector_operations", "slow", "safe")
    def test_search_performance(self):
        """Test vector search performance."""
        pass
```

### Running Specific Python Tests

```bash
# Run specific test file
pytest tests/domains/filesystem/operations/test_directory_operations.py

# Run specific test class
pytest tests/domains/filesystem/operations/test_directory_operations.py::TestDirectoryOperations

# Run specific test method
pytest tests/domains/filesystem/operations/test_directory_operations.py::TestDirectoryOperations::test_directory_create_success

# Run with specific markers
pytest -m "unit and filesystem"
pytest -m "performance and not slow"
pytest -m "safe and quick"
```

## Rust Test Examples

### Test Naming Convention

Rust tests follow the naming pattern:
`test_<domain>_<feature>_<type>_<complexity>_<safety>`

```rust
#[test]
fn test_kernel_module_loading_unit_quick_safe() {
    // Unit test for kernel module loading
}

#[test]
#[ignore] // For tests requiring special conditions
fn test_kernel_module_stress_integration_slow_risky() {
    // Stress test for kernel module
}
```

### Running Specific Rust Tests

```bash
# Run all Rust tests
cargo test

# Run specific test
cargo test test_kernel_module_loading_unit_quick_safe

# Run tests matching pattern
cargo test kernel_module
cargo test loading
cargo test unit

# Run ignored tests (requires explicit flag)
cargo test -- --ignored

# Run with specific features
cargo test --features "test-utils"
```

## Test Configuration

### pytest Configuration

The `pytest.ini` file configures test discovery and execution:

```ini
[tool:pytest]
testpaths = domains
python_files = test_*.py
python_classes = Test*
python_functions = test_*
markers =
    unit: Unit tests
    integration: Integration tests
    performance: Performance tests
    security: Security tests
    # ... (see pytest.ini for complete list)
```

### Rust Test Configuration

Rust tests are configured in `Cargo.toml`:

```toml
[package.metadata.test]
categories = ["unit", "integration", "performance", "security"]
domains = ["filesystem", "kernel_module", "vector_operations", "performance", "security", "integration"]
complexity = ["quick", "medium", "slow"]
safety = ["safe", "monitored", "risky", "dangerous"]
requirements = ["vm_required", "root_required"]
```

## Test Development Guidelines

### Naming Conventions

Follow the established naming patterns documented in [`NAMING_CONVENTIONS.md`](NAMING_CONVENTIONS.md):

- **Python**: `test_<domain>_<feature>_<specific_test>.py`
- **Rust**: `test_<domain>_<feature>.rs`
- **Test functions**: Include domain, type, complexity, and safety in the name

### Using Test Tags

Always tag your tests appropriately:

```python
# Python example
@unit_test("filesystem", "quick", "safe")
def test_file_operations_basic(self):
    pass

@integration_test("kernel_module", "medium", "monitored")
def test_module_loading_integration(self):
    pass
```

```rust
// Rust example - use descriptive names
#[test]
fn test_filesystem_operations_unit_quick_safe() {
    // Test implementation
}

#[test]
#[ignore] // For tests requiring special setup
fn test_kernel_module_stress_integration_slow_risky() {
    // Test implementation
}
```

### Test Structure

Organize tests with clear setup, execution, and verification phases:

```python
def test_example(self):
    # Setup
    test_data = self.create_test_data()
    
    # Execute
    result = perform_operation(test_data)
    
    # Verify
    assert result.success
    assert result.data == expected_data
    
    # Cleanup (if needed)
    self.cleanup_test_data(test_data)
```

## Continuous Integration

### GitHub Actions Integration

Tests are automatically run in CI with different configurations:

```yaml
# Example CI configuration
- name: Run Quick Tests
  run: make test-quick

- name: Run Safe Tests
  run: make test-safe

- name: Run Unit Tests
  run: make test-unit

- name: Run Integration Tests (Non-Root)
  run: make test-integration-no-root
```

### Local Development Workflow

1. **Before committing**: Run quick, safe tests
   ```bash
   make test-quick-safe
   ```

2. **Before pushing**: Run comprehensive safe tests
   ```bash
   make test-safe
   ```

3. **Before releases**: Run full test suite in VM
   ```bash
   make test-all  # In VM environment
   ```

## Troubleshooting

### Common Issues

1. **Permission Errors**
   ```bash
   # Run tests that don't require root
   make test-no-root
   ```

2. **Slow Test Execution**
   ```bash
   # Skip slow tests during development
   make test-no-slow
   ```

3. **Missing Dependencies**
   ```bash
   # Install test dependencies
   pip install -r requirements-test.txt
   ```

4. **Rust Test Failures**
   ```bash
   # Run with verbose output
   cargo test -- --nocapture
   ```

### Test Environment Setup

For comprehensive testing, especially kernel module tests:

1. **Use VM Environment**
   ```bash
   # Set up test VM (see infrastructure/README.md)
   cd infrastructure/
   terraform apply
   ```

2. **Install Dependencies**
   ```bash
   # Run setup script
   ../scripts/setup_dev_environment.sh
   ```

3. **Verify Environment**
   ```bash
   # Run environment validation tests
   make test-environment
   ```

## Performance Testing

### Benchmarking

Performance tests include benchmarking capabilities:

```python
@performance_test("vector_operations", "slow", "safe")
def test_search_performance_benchmark(self):
    # Benchmark vector search operations
    results = benchmark_vector_search(dataset_size=100000)
    assert results.avg_query_time < 0.05  # 50ms
    assert results.throughput > 1000      # 1000 QPS
```

### Performance Monitoring

Monitor performance trends over time:

```bash
# Run performance tests with reporting
make test-performance REPORT=true

# Generate performance report
make performance-report
```

## Security Testing

### Access Control Tests

```python
@security_test("filesystem", "medium", "monitored")
def test_permission_enforcement(self):
    # Test file permission enforcement
    pass
```

### Privilege Escalation Tests

```python
@tag("security", "kernel_module", "slow", "risky")
def test_privilege_escalation_prevention(self):
    # Test prevention of privilege escalation
    pass
```

## Integration with Development Tools

### IDE Integration

Most IDEs support pytest markers for selective test execution:

- **VS Code**: Use pytest extension with marker support
- **PyCharm**: Configure test runners with marker filters
- **Vim/Neovim**: Use test runner plugins with marker support

### Git Hooks

Set up pre-commit hooks to run appropriate tests:

```bash
# .git/hooks/pre-commit
#!/bin/bash
make test-quick-safe
```

## Best Practices

1. **Start with unit tests**: Write unit tests first, then integration tests
2. **Use appropriate tags**: Tag tests correctly for proper categorization
3. **Keep tests fast**: Prefer quick tests for frequent execution
4. **Isolate dangerous tests**: Use VM for risky or dangerous tests
5. **Document test purpose**: Include clear docstrings explaining test goals
6. **Clean up resources**: Ensure tests clean up after themselves
7. **Use realistic data**: Use representative test data for meaningful results

## Getting Help

- **Documentation**: See [`README.md`](README.md) for overview
- **Naming Conventions**: See [`NAMING_CONVENTIONS.md`](NAMING_CONVENTIONS.md)
- **Infrastructure**: See [`infrastructure/README.md`](infrastructure/README.md)
- **Legacy Tests**: See [`legacy/QUICK_START.md`](legacy/QUICK_START.md)

For questions or issues, consult the project documentation or create an issue in the project repository.