# VexFS Test Naming Conventions

This document defines the standardized naming conventions for VexFS tests to ensure consistency, discoverability, and maintainability across the testing infrastructure.

## File Naming Patterns

### Unit Tests
- **Rust**: `test_<component>_<functionality>.rs` or `<component>_test.rs`
- **Python**: `test_<component>_<functionality>.py`

Examples:
- `test_vector_storage_basic.rs`
- `test_filesystem_operations.py`
- `kernel_module_test.rs`
- `test_anns_hnsw_construction.py`

### Integration Tests
- **Pattern**: `integration_<domain>_<scenario>.rs` or `integration_<domain>_<scenario>.py`

Examples:
- `integration_filesystem_vfs_operations.rs`
- `integration_kernel_module_loading.py`
- `integration_vector_search_performance.rs`
- `integration_security_acl_validation.py`

### Performance Tests
- **Pattern**: `perf_<component>_<metric>.rs` or `perf_<component>_<metric>.py`

Examples:
- `perf_vector_search_throughput.rs`
- `perf_filesystem_latency.py`
- `perf_memory_usage_stress.rs`
- `perf_concurrent_access_benchmark.py`

### Security Tests
- **Pattern**: `security_<component>_<vulnerability>.rs` or `security_<component>_<vulnerability>.py`

Examples:
- `security_filesystem_access_control.rs`
- `security_kernel_module_privilege_escalation.py`
- `security_vector_data_encryption.rs`
- `security_memory_bounds_checking.py`

## Test Function Naming

### Unit Test Functions
- **Pattern**: `test_<functionality>_<specific_case>()`

Examples:
```rust
#[test]
fn test_vector_storage_create_index() { }

#[test]
fn test_filesystem_create_directory_success() { }

#[test]
fn test_anns_hnsw_search_k_neighbors() { }
```

### Integration Test Functions
- **Pattern**: `test_<domain>_<scenario>_<expected_outcome>()`

Examples:
```rust
#[test]
fn test_vfs_mount_unmount_success() { }

#[test]
fn test_kernel_module_load_unload_cycle() { }

#[test]
fn test_vector_search_concurrent_access() { }
```

### Performance Test Functions
- **Pattern**: `bench_<component>_<operation>_<metric>()`

Examples:
```rust
#[test]
fn bench_vector_search_throughput_1000_queries() { }

#[test]
fn bench_filesystem_write_latency_large_files() { }
```

## Directory Structure Naming

### Domain-Based Organization
```
tests/domains/
├── filesystem/
│   ├── operations/          # Basic filesystem operations
│   ├── metadata/           # Metadata handling tests
│   ├── permissions/        # Permission and ACL tests
│   └── vfs_integration/    # VFS layer integration
├── kernel_module/
│   ├── loading/            # Module loading/unloading tests
│   ├── syscalls/           # System call interface tests
│   ├── stability/          # Stability and safety tests
│   └── memory_management/  # Memory leak and management tests
├── vector_operations/
│   ├── storage/            # Vector storage tests
│   ├── search/             # Search algorithm tests
│   ├── indexing/           # Index construction and maintenance
│   └── caching/            # Vector caching tests
├── performance/
│   ├── throughput/         # Throughput benchmarks
│   ├── latency/            # Latency measurements
│   ├── memory/             # Memory usage tests
│   └── concurrent/         # Concurrency performance
├── security/
│   ├── access_control/     # Access control tests
│   ├── encryption/         # Data encryption tests
│   ├── integrity/          # Data integrity tests
│   └── privilege/          # Privilege escalation tests
└── integration/
    ├── end_to_end/         # Complete workflow tests
    ├── cross_component/    # Multi-component integration
    └── system_recovery/    # Recovery and resilience tests
```

## Test Case Naming in Code

### Test Case Identifiers
- **Pattern**: `<domain>_<component>_<test_type>_<specific_case>`

Examples:
```python
# In domain models
test_cases = [
    "kernel_module_loading_basic_success",
    "filesystem_operations_create_directory",
    "vector_search_knn_performance_1000_vectors",
    "security_acl_permission_validation",
    "integration_vfs_mount_unmount_cycle"
]
```

### Test Result Identifiers
- **Pattern**: `<timestamp>_<domain>_<test_case>_<result>`

Examples:
```
20250530_kernel_module_loading_basic_success_passed
20250530_filesystem_operations_create_directory_failed
20250530_vector_search_knn_performance_1000_vectors_passed
```

## Legacy Test Migration

### Current Files to Rename
Based on existing test files, the following should be renamed:

#### Rust Tests (tests/ root level)
- `unit_tests.rs` → `test_vexfs_unit_suite.rs`
- `integration_tests.rs` → `integration_vfs_system_operations.rs`
- `performance_tests.rs` → `perf_vexfs_comprehensive_benchmark.rs`
- `cow_snapshot_integration.rs` → `integration_cow_snapshot_operations.rs`
- `cow_snapshot_performance.rs` → `perf_cow_snapshot_throughput.rs`
- `vector_cache_integration.rs` → `integration_vector_cache_operations.rs`

#### Python Tests
- `run_kernel_tests.py` → `test_kernel_module_domain_runner.py`

## Tag-Based Categorization

### Test Tags for Selective Execution
Tests should be tagged with appropriate categories:

#### Type Tags
- `@tag("unit")` - Unit tests
- `@tag("integration")` - Integration tests
- `@tag("performance")` - Performance/benchmark tests
- `@tag("security")` - Security-focused tests

#### Domain Tags
- `@tag("kernel_module")` - Kernel module tests
- `@tag("filesystem")` - Filesystem operation tests
- `@tag("vector_operations")` - Vector storage/search tests
- `@tag("fuse")` - FUSE implementation tests

#### Complexity Tags
- `@tag("quick")` - Fast tests (< 10 seconds)
- `@tag("slow")` - Slower tests (> 30 seconds)
- `@tag("vm_required")` - Tests requiring VM environment
- `@tag("root_required")` - Tests requiring root privileges

#### Safety Tags
- `@tag("safe")` - Safe to run in any environment
- `@tag("monitored")` - Requires monitoring during execution
- `@tag("risky")` - May affect system stability
- `@tag("dangerous")` - High risk, VM-only execution

## Examples

### Complete Test File Example (Rust)
```rust
// File: tests/domains/filesystem/operations/test_directory_operations.rs

//! Filesystem Directory Operations Unit Tests
//! 
//! Tests for basic directory operations including creation, deletion,
//! listing, and metadata management.

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[tag("unit")]
    #[tag("filesystem")]
    #[tag("quick")]
    #[tag("safe")]
    fn test_directory_create_success() {
        // Test implementation
    }
    
    #[test]
    #[tag("unit")]
    #[tag("filesystem")]
    #[tag("quick")]
    #[tag("safe")]
    fn test_directory_delete_success() {
        // Test implementation
    }
}
```

### Complete Test File Example (Python)
```python
# File: tests/domains/kernel_module/loading/test_module_lifecycle.py

"""
Kernel Module Lifecycle Tests

Tests for kernel module loading, unloading, and lifecycle management
including safety checks and error handling.
"""

import pytest
from tests.domains.shared.test_tags import tag

class TestModuleLifecycle:
    
    @tag("unit")
    @tag("kernel_module")
    @tag("monitored")
    @tag("vm_required")
    def test_module_load_basic_success(self):
        """Test basic module loading functionality"""
        pass
    
    @tag("integration")
    @tag("kernel_module")
    @tag("slow")
    @tag("vm_required")
    def test_module_load_unload_cycle(self):
        """Test complete load/unload cycle"""
        pass
```

## Validation Rules

1. **File names MUST follow the specified patterns**
2. **Test functions MUST be properly tagged**
3. **Directory structure MUST match domain organization**
4. **Legacy tests MUST be migrated to new naming**
5. **All new tests MUST include appropriate tags**

This naming convention ensures consistency, improves test discoverability, and enables efficient selective test execution across the VexFS testing infrastructure.