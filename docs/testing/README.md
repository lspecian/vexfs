# VexFS Testing Documentation

This directory contains comprehensive documentation for VexFS testing infrastructure, which has been consolidated into a unified `tests/` directory with Domain-Driven Design (DDD) principles and Infrastructure-as-Code (IaC) approach.

## 🚀 Quick Start

### 1. Navigate to Testing Directory
```bash
cd tests/
```

### 2. Run Quick Tests
```bash
# Run all quick tests (< 30 seconds)
make test-quick

# Run safe tests only
make test-safe

# Run unit tests for rapid iteration
make test-unit
```

### 3. Domain-Specific Testing
```bash
# Test filesystem operations
make test-domain DOMAIN=filesystem

# Test kernel module functionality
make test-domain DOMAIN=kernel_module

# Test vector operations
make test-domain DOMAIN=vector_operations
```

## 📁 Testing Infrastructure Overview

VexFS testing infrastructure has been **consolidated and modernized** from scattered components into a unified, sophisticated system:

### **Unified Testing Directory Structure**
```
tests/
├── domains/                 # Domain-Driven Design test organization
│   ├── filesystem/         # File system operations and VFS integration
│   ├── kernel_module/      # Kernel module functionality
│   ├── vector_operations/  # Vector search and ANNS operations
│   ├── performance/        # Performance and benchmarking tests
│   ├── security/          # Security and access control tests
│   ├── integration/       # Cross-component integration tests
│   └── shared/            # Shared utilities and test framework
├── infrastructure/        # Infrastructure-as-Code (Terraform + Ansible)
│   ├── terraform/         # VM provisioning and environment setup
│   ├── ansible/          # Configuration management and test orchestration
│   └── README.md         # Infrastructure setup guide
├── legacy/               # Legacy test scripts (migrated from test_env/)
│   ├── shell_scripts/    # 35+ shell scripts for VM testing
│   ├── vm_management/    # QEMU VM setup and management
│   └── QUICK_START.md   # Legacy testing quick start
├── Makefile              # Unified test execution commands
└── README.md            # Complete testing guide
```

### **Key Improvements from Legacy Structure**

| Aspect | Legacy (test_env/) | Current (tests/) | Improvement |
|--------|-------------------|------------------|-------------|
| **Organization** | Scattered scripts | Domain-driven structure | **Logical organization** |
| **Test Discovery** | Manual execution | Tagged test system | **Intelligent selection** |
| **Infrastructure** | Manual VM setup | Infrastructure-as-Code | **Automated provisioning** |
| **Test Execution** | Individual scripts | Unified Makefile | **Consistent interface** |
| **Development** | Static VM images | Live source mounting | **Rapid iteration** |

## 🧪 Testing Approaches

### 1. **Domain-Driven Testing**
Tests organized by business domains rather than technical layers:

- **Filesystem Domain**: File operations, metadata, permissions, VFS integration
- **Kernel Module Domain**: Module loading, syscalls, memory management, stability
- **Vector Operations Domain**: Storage, search, indexing, caching of vector data
- **Performance Domain**: Throughput, latency, memory usage, concurrency testing
- **Security Domain**: Access control, encryption, integrity, privilege management
- **Integration Domain**: End-to-end workflows, cross-component interactions

### 2. **Infrastructure-as-Code Testing**
Sophisticated test environment provisioning using:

- **Terraform**: Automated VM provisioning and infrastructure setup
- **Ansible**: Configuration management and test orchestration
- **Docker**: Containerized test environments for isolation

### 3. **Legacy Shell-Based Testing**
Preserved 35+ shell scripts for:

- **VM Management**: QEMU-based virtual machine lifecycle
- **Kernel Testing**: Module loading, FFI integration, system stability
- **Performance Testing**: Benchmarking and validation scripts

## 🏷️ Test Categorization System

### **Test Types**
- **Unit** (`unit`): Test individual functions and components
- **Integration** (`integration`): Test component interactions
- **Performance** (`performance`): Measure and benchmark performance
- **Security** (`security`): Verify security controls

### **Test Complexity**
- **Quick** (`quick`): Fast tests (< 30 seconds)
- **Medium** (`medium`): Moderate tests (< 5 minutes)
- **Slow** (`slow`): Comprehensive tests (> 5 minutes)

### **Safety Levels**
- **Safe** (`safe`): No system modifications, no privileges required
- **Monitored** (`monitored`): Controlled system modifications
- **Risky** (`risky`): May affect system stability
- **Dangerous** (`dangerous`): Requires extreme caution (VM recommended)

## 🛠️ Development Workflows

### **Local Development**
```bash
# Quick feedback during development
make test-quick-safe

# Test specific domain you're working on
make test-domain DOMAIN=filesystem

# Run unit tests for rapid iteration
make test-unit
```

### **Pre-Commit Testing**
```bash
# Run safe tests before committing
make test-safe

# Run unit and integration tests
make test-unit test-integration-no-root
```

### **Comprehensive Testing**
```bash
# Full test suite (use in VM for safety)
make test-all

# Performance validation
make test-performance

# Security validation
make test-security
```

## 🏗️ Infrastructure Setup

### **Terraform (Automated VM Provisioning)**
```bash
cd tests/infrastructure/terraform/
terraform init
terraform plan
terraform apply
```

### **Ansible (Configuration Management)**
```bash
cd tests/infrastructure/ansible/
ansible-playbook -i inventory setup-test-environment.yml
```

### **Legacy VM Setup (Quick Start)**
```bash
cd tests/legacy/shell_scripts/
./setup_vm.sh                    # One-time VM setup
./vm_control.sh start            # Start VM
./test_module_vm.sh              # Run comprehensive tests
```

## 🔧 Migration from Legacy test_env/

The testing infrastructure has been **completely reorganized** from the legacy `test_env/` structure:

### **Path Migrations**
| Legacy Path | Current Path | Purpose |
|-------------|--------------|---------|
| `test_env/setup_vm.sh` | `tests/legacy/shell_scripts/setup_vm.sh` | VM setup |
| `test_env/vm_control.sh` | `tests/legacy/shell_scripts/vm_control.sh` | VM lifecycle |
| `test_env/test_module.sh` | `tests/legacy/shell_scripts/test_module_vm.sh` | Module testing |
| `test_env/run_qemu_simple.sh` | `tests/legacy/shell_scripts/run_qemu_simple.sh` | Quick VM |
| `test_env/run_qemu_fast.sh` | `tests/legacy/shell_scripts/run_qemu_fast.sh` | Fast VM |
| `infrastructure/` | `tests/infrastructure/` | Infrastructure-as-Code |

### **Updated Commands**
```bash
# Legacy commands (deprecated)
./test_env/setup_vm.sh
./test_env/vm_control.sh start
./test_env/test_module.sh

# Current commands
cd tests/
make test-quick                           # Quick tests
make test-domain DOMAIN=kernel_module     # Domain-specific
./legacy/shell_scripts/setup_vm.sh        # Legacy VM setup (if needed)
```

## 📊 Test Execution Examples

### **By Test Type**
```bash
make test-unit           # Unit tests only
make test-integration    # Integration tests only
make test-performance    # Performance tests only
make test-security       # Security tests only
```

### **By Domain**
```bash
make test-domain DOMAIN=filesystem
make test-domain DOMAIN=kernel_module
make test-domain DOMAIN=vector_operations
```

### **By Complexity**
```bash
make test-quick          # Quick tests (< 30 seconds)
make test-medium         # Medium complexity tests
make test-slow           # Comprehensive tests
```

### **By Safety Level**
```bash
make test-safe           # Safe tests only
make test-monitored      # Monitored tests
make test-no-dangerous   # Exclude dangerous tests
```

### **Combined Filters**
```bash
make test-unit-safe      # Unit tests that are safe
make test-integration-quick  # Quick integration tests
```

## 🐛 Debugging and Troubleshooting

### **VM-Based Testing**
```bash
# Start VM for manual testing
cd tests/legacy/shell_scripts/
./vm_control.sh start

# SSH into VM
./vm_control.sh ssh

# In VM: Manual kernel module testing
cd /mnt/vexfs/kernel
make && sudo insmod vexfs.ko
./test_ffi_integration
```

### **Infrastructure Debugging**
```bash
# Check Terraform state
cd tests/infrastructure/terraform/
terraform show

# Run Ansible in verbose mode
cd tests/infrastructure/ansible/
ansible-playbook -vvv setup-test-environment.yml
```

### **Test Result Analysis**
```bash
# View test results
ls test_results/

# Check specific test logs
cat test_results/test_log_TIMESTAMP.txt
```

## 📚 Documentation References

### **Core Testing Documentation**
- **[`tests/README.md`](../../tests/README.md)** - Complete testing infrastructure guide
- **[`tests/NAMING_CONVENTIONS.md`](../../tests/NAMING_CONVENTIONS.md)** - Test naming standards
- **[`tests/infrastructure/README.md`](../../tests/infrastructure/README.md)** - Infrastructure setup
- **[`tests/legacy/QUICK_START.md`](../../tests/legacy/QUICK_START.md)** - Legacy testing

### **Architecture Documentation**
- **[`docs/architecture/DOMAIN_DRIVEN_TESTING_ARCHITECTURE.md`](../architecture/DOMAIN_DRIVEN_TESTING_ARCHITECTURE.md)** - DDD testing approach
- **[`docs/architecture/INFRASTRUCTURE_AS_CODE_ANALYSIS.md`](../architecture/INFRASTRUCTURE_AS_CODE_ANALYSIS.md)** - IaC analysis
- **[`docs/architecture/VEXFS_TESTING_EVOLUTION_PLAN.md`](../architecture/VEXFS_TESTING_EVOLUTION_PLAN.md)** - Testing evolution

### **Specific Testing Guides**
- **[`COMPREHENSIVE_TESTING_FRAMEWORK.md`](COMPREHENSIVE_TESTING_FRAMEWORK.md)** - Framework overview
- **[`VM_TESTING_STRATEGY.md`](VM_TESTING_STRATEGY.md)** - VM testing approach
- **[`QEMU_SETUP_GUIDE.md`](QEMU_SETUP_GUIDE.md)** - QEMU configuration

## 🎯 Key Benefits of New Testing Infrastructure

### **For Developers**
- ✅ **Unified Interface**: Single `make` command system for all testing
- ✅ **Rapid Iteration**: Quick tests for fast feedback during development
- ✅ **Domain Focus**: Test specific areas you're working on
- ✅ **Safety Levels**: Choose appropriate risk level for your environment

### **For CI/CD**
- ✅ **Automated Provisioning**: Infrastructure-as-Code for consistent environments
- ✅ **Selective Execution**: Run only relevant tests based on changes
- ✅ **Scalable Infrastructure**: From local development to production-scale testing
- ✅ **Comprehensive Coverage**: Unit, integration, performance, and security testing

### **For Project Maintenance**
- ✅ **Organized Structure**: Domain-driven organization reduces complexity
- ✅ **Preserved Legacy**: All existing functionality maintained in `tests/legacy/`
- ✅ **Modern Tooling**: Infrastructure-as-Code with Terraform and Ansible
- ✅ **Comprehensive Documentation**: Clear guides for all testing approaches

---

**🎯 Result: A sophisticated, unified testing infrastructure that supports VexFS development from rapid local iteration to comprehensive production validation, with intelligent test discovery and Infrastructure-as-Code automation.**
