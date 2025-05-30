# VexFS Testing Infrastructure

This directory contains the comprehensive testing infrastructure for VexFS, organized using Domain-Driven Design (DDD) principles with advanced test discovery and selective execution capabilities.

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

# List available commands
make help
```

## Test Discovery and Execution

### New Test Organization Features

- **🏷️ Comprehensive Tagging System**: Tests are tagged by type, domain, complexity, and safety level
- **🎯 Selective Test Execution**: Run specific subsets of tests based on multiple criteria
- **📁 Domain-Driven Structure**: Tests organized by business domains with consistent naming
- **⚡ Performance Optimized**: Quick vs. slow test categorization for efficient development
- **🔒 Safety Levels**: Safe, monitored, risky, and dangerous test classifications

### Test Execution Examples

```bash
# By test type
make test-unit           # Unit tests only
make test-integration    # Integration tests only
make test-performance    # Performance tests only
make test-security       # Security tests only

# By domain
make test-domain DOMAIN=filesystem
make test-domain DOMAIN=kernel_module
make test-domain DOMAIN=vector_operations

# By complexity
make test-quick          # Quick tests (< 30 seconds)
make test-medium         # Medium complexity tests
make test-slow           # Comprehensive tests

# By safety level
make test-safe           # Safe tests only
make test-monitored      # Monitored tests
make test-no-dangerous   # Exclude dangerous tests

# Combined filters
make test-unit-safe      # Unit tests that are safe
make test-integration-quick  # Quick integration tests
```

### Python Test Tags

Tests use decorators for automatic categorization:

```python
from tests.domains.shared.test_tags import unit_test, integration_test, tag

@unit_test("filesystem", "quick", "safe")
def test_file_creation(self):
    """Test basic file creation functionality."""
    pass

@integration_test("kernel_module", "medium", "monitored")
def test_module_loading(self):
    """Test kernel module loading integration."""
    pass

@tag("performance", "vector_operations", "slow", "safe")
def test_search_performance(self):
    """Test vector search performance benchmarks."""
    pass
```

## Directory Structure

```
tests/
├── domains/                 # Domain-driven test organization
│   ├── filesystem/         # File system operations and VFS integration
│   │   ├── operations/     # Basic CRUD operations
│   │   ├── metadata/       # File metadata and attributes
│   │   ├── permissions/    # Access control and permissions
│   │   └── vfs_integration/ # VFS layer integration
│   ├── kernel_module/      # Kernel module functionality
│   │   ├── loading/        # Module loading/unloading
│   │   ├── syscalls/       # System call interface
│   │   ├── stability/      # Stability and reliability
│   │   └── memory_management/ # Memory allocation/deallocation
│   ├── vector_operations/  # Vector search and ANNS operations
│   │   ├── storage/        # Vector storage and retrieval
│   │   ├── search/         # Search algorithms and performance
│   │   ├── indexing/       # Index building and maintenance
│   │   └── caching/        # Vector caching strategies
│   ├── performance/        # Performance and benchmarking tests
│   │   ├── throughput/     # Throughput measurements
│   │   ├── latency/        # Latency measurements
│   │   ├── memory/         # Memory usage analysis
│   │   └── concurrent/     # Concurrency performance
│   ├── security/          # Security and access control tests
│   │   ├── access_control/ # Permission enforcement
│   │   ├── encryption/     # Data encryption
│   │   ├── integrity/      # Data integrity verification
│   │   └── privilege/      # Privilege escalation prevention
│   ├── integration/       # Cross-component integration tests
│   │   ├── end_to_end/     # Complete workflow tests
│   │   ├── cross_component/ # Component interaction tests
│   │   └── system_recovery/ # Recovery and resilience tests
│   └── shared/            # Shared utilities and test framework
│       ├── test_tags.py    # Test tagging system
│       ├── fixtures.py     # Common test fixtures
│       └── utils.py        # Test utilities
├── infrastructure/        # Infrastructure-as-Code for test environments
│   ├── terraform/         # Terraform configurations
│   ├── ansible/          # Ansible playbooks
│   └── docker/           # Docker configurations
├── legacy/               # Legacy test scripts and configurations
│   ├── vm_management/    # VM setup and management
│   ├── shell_scripts/    # Shell-based test scripts
│   └── QUICK_START.md   # Legacy testing quick start
├── Makefile              # Test execution commands
├── pytest.ini           # pytest configuration
├── NAMING_CONVENTIONS.md # Test naming standards
└── TESTING_GUIDE.md     # Comprehensive testing guide
```

## Test Categories and Tags

### Test Types
- **Unit** (`unit`): Test individual functions and components
- **Integration** (`integration`): Test component interactions
- **Performance** (`performance`): Measure and benchmark performance
- **Security** (`security`): Verify security controls

### Test Domains
- **Filesystem** (`filesystem`): File and directory operations
- **Kernel Module** (`kernel_module`): Kernel module functionality
- **Vector Operations** (`vector_operations`): Vector search and ANNS
- **Performance** (`performance`): Performance and benchmarking
- **Security** (`security`): Security and access control
- **Integration** (`integration`): Cross-component integration

### Test Complexity
- **Quick** (`quick`): Fast tests (< 30 seconds)
- **Medium** (`medium`): Moderate tests (< 5 minutes)
- **Slow** (`slow`): Comprehensive tests (> 5 minutes)

### Safety Levels
- **Safe** (`safe`): No system modifications, no privileges required
- **Monitored** (`monitored`): Controlled system modifications
- **Risky** (`risky`): May affect system stability
- **Dangerous** (`dangerous`): Requires extreme caution (VM recommended)

### Special Requirements
- **VM Required** (`vm_required`): Must run in virtual machine
- **Root Required** (`root_required`): Requires root/administrator privileges

## Testing Approach

### Domain-Driven Design

Tests are organized by business domains rather than technical layers:

- **Filesystem Domain**: File operations, metadata, permissions, VFS integration
- **Kernel Module Domain**: Module loading, syscalls, memory management, stability
- **Vector Operations Domain**: Storage, search, indexing, caching of vector data
- **Performance Domain**: Throughput, latency, memory usage, concurrency testing
- **Security Domain**: Access control, encryption, integrity, privilege management
- **Integration Domain**: End-to-end workflows, cross-component interactions

### Test Development Guidelines

1. **Follow naming conventions** (see [`NAMING_CONVENTIONS.md`](NAMING_CONVENTIONS.md))
2. **Use appropriate tags** for test categorization
3. **Start with unit tests**, then integration tests
4. **Keep tests fast** when possible (prefer `quick` over `slow`)
5. **Use safe tests** for frequent execution
6. **Document test purpose** with clear docstrings

## Infrastructure as Code

### Terraform

Automated provisioning of test environments:

```bash
cd infrastructure/terraform/
terraform init
terraform plan
terraform apply
```

### Ansible

Configuration management for test environments:

```bash
cd infrastructure/ansible/
ansible-playbook -i inventory setup-test-environment.yml
```

### Docker

Containerized test environments:

```bash
cd infrastructure/docker/
docker-compose up -d
```

## Legacy Testing

The `legacy/` directory contains the original testing infrastructure:

- **VM Management**: QEMU-based virtual machine setup
- **Shell Scripts**: Bash-based test automation
- **Quick Start**: Rapid testing setup for development

See [`legacy/QUICK_START.md`](legacy/QUICK_START.md) for legacy testing instructions.

## Development Workflow

### Local Development
```bash
# Quick feedback during development
make test-quick-safe

# Test specific domain you're working on
make test-domain DOMAIN=filesystem

# Run unit tests for rapid iteration
make test-unit
```

### Pre-Commit Testing
```bash
# Run safe tests before committing
make test-safe

# Run unit and integration tests
make test-unit test-integration-no-root
```

### Comprehensive Testing
```bash
# Full test suite (use in VM for safety)
make test-all

# Performance validation
make test-performance

# Security validation
make test-security
```

## Getting Started

1. **Set up environment**:
   ```bash
   ../scripts/setup_dev_environment.sh
   ```

2. **Run quick tests**:
   ```bash
   make test-quick
   ```

3. **Run domain-specific tests**:
   ```bash
   make test-domain DOMAIN=filesystem
   make test-domain DOMAIN=kernel_module
   ```

4. **Set up infrastructure** (for comprehensive testing):
   ```bash
   cd infrastructure/
   terraform apply
   ```

5. **Run legacy tests** (if needed):
   ```bash
   cd legacy/
   # See QUICK_START.md for instructions
   ```

## Documentation

- **📖 Testing Guide**: [`TESTING_GUIDE.md`](TESTING_GUIDE.md) - Comprehensive testing instructions
- **📝 Naming Conventions**: [`NAMING_CONVENTIONS.md`](NAMING_CONVENTIONS.md) - Test naming standards
- **🏗️ Infrastructure**: [`infrastructure/README.md`](infrastructure/README.md) - Infrastructure setup
- **🔧 Legacy**: [`legacy/QUICK_START.md`](legacy/QUICK_START.md) - Legacy testing
- **🏛️ Architecture**: `../docs/architecture/` - Testing architecture documentation

## Example Test Files

- **Python**: [`domains/filesystem/operations/test_directory_operations.py`](domains/filesystem/operations/test_directory_operations.py)
- **Rust**: [`domains/kernel_module/loading/test_module_lifecycle.rs`](domains/kernel_module/loading/test_module_lifecycle.rs)
- **Performance**: [`domains/vector_operations/search/test_vector_search_performance.py`](domains/vector_operations/search/test_vector_search_performance.py)

## Migration Status

- ✅ **Test Discovery System**: Comprehensive tagging and selective execution
- ✅ **Domain Structure**: Organized by business domains with feature areas
- ✅ **Naming Conventions**: Consistent patterns across Python and Rust tests
- ✅ **Test Execution**: Makefile with selective test running capabilities
- ✅ **Example Tests**: Demonstrating new tagging and organization system
- ✅ **Documentation**: Complete testing guide and conventions
- ✅ **Infrastructure-as-Code**: Moved from `infrastructure/` to `tests/infrastructure/`
- ✅ **Legacy Scripts**: Moved from `test_env/` to `tests/legacy/`

## Next Steps

1. **Migrate Existing Tests**: Update existing test files to follow new naming conventions
2. **Expand Test Coverage**: Add more comprehensive tests in each domain
3. **CI/CD Integration**: Connect with automated build and deployment pipelines
4. **Performance Baselines**: Establish performance benchmarks and regression testing
5. **Security Validation**: Implement comprehensive security test coverage

This testing infrastructure supports the VexFS development lifecycle from rapid local development to comprehensive integration testing and performance validation, with intelligent test discovery and selective execution capabilities.