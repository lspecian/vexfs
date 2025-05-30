"""
VexFS Test Tagging System

Provides decorators and utilities for tagging tests to enable selective
test execution based on type, domain, complexity, and safety requirements.
"""

import functools
from typing import Set, List, Callable, Any
from enum import Enum


class TestType(Enum):
    """Test type categories"""
    UNIT = "unit"
    INTEGRATION = "integration"
    PERFORMANCE = "performance"
    SECURITY = "security"


class TestDomain(Enum):
    """Test domain categories"""
    KERNEL_MODULE = "kernel_module"
    FILESYSTEM = "filesystem"
    VECTOR_OPERATIONS = "vector_operations"
    FUSE = "fuse"
    SECURITY = "security"
    PERFORMANCE = "performance"
    INTEGRATION = "integration"


class TestComplexity(Enum):
    """Test complexity/duration categories"""
    QUICK = "quick"          # < 10 seconds
    MEDIUM = "medium"        # 10-30 seconds
    SLOW = "slow"           # > 30 seconds
    VM_REQUIRED = "vm_required"
    ROOT_REQUIRED = "root_required"


class TestSafety(Enum):
    """Test safety levels"""
    SAFE = "safe"           # Safe to run anywhere
    MONITORED = "monitored" # Requires monitoring
    RISKY = "risky"        # May affect system stability
    DANGEROUS = "dangerous" # High risk, VM-only


# Global test registry for tag-based filtering
_test_registry = {}


def tag(*tags: str):
    """
    Decorator to tag test functions for selective execution.
    
    Args:
        *tags: Variable number of tag strings
        
    Example:
        @tag("unit", "filesystem", "quick", "safe")
        def test_directory_creation(self):
            pass
    """
    def decorator(func: Callable) -> Callable:
        # Store tags as function attribute
        if not hasattr(func, '_test_tags'):
            func._test_tags = set()
        func._test_tags.update(tags)
        
        # Register test in global registry
        test_name = f"{func.__module__}.{func.__qualname__}"
        _test_registry[test_name] = {
            'function': func,
            'tags': func._test_tags,
            'module': func.__module__,
            'name': func.__name__
        }
        
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            return func(*args, **kwargs)
        
        # Preserve tags on wrapper
        wrapper._test_tags = func._test_tags
        return wrapper
    
    return decorator


def get_tests_by_tags(include_tags: List[str] = None, exclude_tags: List[str] = None) -> List[dict]:
    """
    Get tests filtered by tags.
    
    Args:
        include_tags: List of tags that tests must have (OR logic)
        exclude_tags: List of tags that tests must not have
        
    Returns:
        List of test dictionaries matching the criteria
    """
    include_tags = set(include_tags or [])
    exclude_tags = set(exclude_tags or [])
    
    filtered_tests = []
    
    for test_name, test_info in _test_registry.items():
        test_tags = test_info['tags']
        
        # Check include criteria (OR logic - test must have at least one included tag)
        if include_tags and not include_tags.intersection(test_tags):
            continue
            
        # Check exclude criteria (test must not have any excluded tags)
        if exclude_tags and exclude_tags.intersection(test_tags):
            continue
            
        filtered_tests.append(test_info)
    
    return filtered_tests


def get_tests_by_domain(domain: str) -> List[dict]:
    """Get all tests for a specific domain."""
    return get_tests_by_tags(include_tags=[domain])


def get_tests_by_type(test_type: str) -> List[dict]:
    """Get all tests of a specific type."""
    return get_tests_by_tags(include_tags=[test_type])


def get_quick_tests() -> List[dict]:
    """Get all quick-running tests."""
    return get_tests_by_tags(include_tags=["quick"])


def get_safe_tests() -> List[dict]:
    """Get all safe tests (no system impact)."""
    return get_tests_by_tags(include_tags=["safe"])


def get_vm_tests() -> List[dict]:
    """Get all tests that require VM environment."""
    return get_tests_by_tags(include_tags=["vm_required"])


def list_all_tags() -> Set[str]:
    """Get all unique tags used across all registered tests."""
    all_tags = set()
    for test_info in _test_registry.values():
        all_tags.update(test_info['tags'])
    return all_tags


def list_tests_with_tags() -> dict:
    """Get a summary of all tests with their tags."""
    return {
        test_name: list(test_info['tags'])
        for test_name, test_info in _test_registry.items()
    }


def validate_tags(tags: Set[str]) -> List[str]:
    """
    Validate that tags follow the expected patterns.
    
    Returns:
        List of validation errors (empty if all tags are valid)
    """
    errors = []
    
    valid_tags = set()
    
    # Add enum values to valid tags
    for enum_class in [TestType, TestDomain, TestComplexity, TestSafety]:
        valid_tags.update(member.value for member in enum_class)
    
    # Check for invalid tags
    invalid_tags = tags - valid_tags
    if invalid_tags:
        errors.append(f"Invalid tags found: {invalid_tags}")
    
    return errors


# Convenience decorators for common tag combinations
def unit_test(domain: str = None, complexity: str = "quick", safety: str = "safe"):
    """Convenience decorator for unit tests."""
    tags = ["unit", complexity, safety]
    if domain:
        tags.append(domain)
    return tag(*tags)


def integration_test(domain: str = None, complexity: str = "medium", safety: str = "monitored"):
    """Convenience decorator for integration tests."""
    tags = ["integration", complexity, safety]
    if domain:
        tags.append(domain)
    return tag(*tags)


def performance_test(domain: str = None, complexity: str = "slow", safety: str = "safe"):
    """Convenience decorator for performance tests."""
    tags = ["performance", complexity, safety]
    if domain:
        tags.append(domain)
    return tag(*tags)


def security_test(domain: str = None, complexity: str = "medium", safety: str = "monitored"):
    """Convenience decorator for security tests."""
    tags = ["security", complexity, safety]
    if domain:
        tags.append(domain)
    return tag(*tags)


def vm_test(test_type: str = "integration", domain: str = None, safety: str = "risky"):
    """Convenience decorator for VM-required tests."""
    tags = [test_type, "vm_required", safety]
    if domain:
        tags.append(domain)
    return tag(*tags)


# Test discovery utilities
class TestFilter:
    """Utility class for building complex test filters."""
    
    def __init__(self):
        self.include_tags = set()
        self.exclude_tags = set()
    
    def include(self, *tags: str) -> 'TestFilter':
        """Add tags to include filter."""
        self.include_tags.update(tags)
        return self
    
    def exclude(self, *tags: str) -> 'TestFilter':
        """Add tags to exclude filter."""
        self.exclude_tags.update(tags)
        return self
    
    def domain(self, domain: str) -> 'TestFilter':
        """Filter by domain."""
        return self.include(domain)
    
    def type(self, test_type: str) -> 'TestFilter':
        """Filter by test type."""
        return self.include(test_type)
    
    def quick_only(self) -> 'TestFilter':
        """Include only quick tests."""
        return self.include("quick").exclude("slow", "medium")
    
    def safe_only(self) -> 'TestFilter':
        """Include only safe tests."""
        return self.include("safe").exclude("risky", "dangerous")
    
    def vm_only(self) -> 'TestFilter':
        """Include only VM tests."""
        return self.include("vm_required")
    
    def get_tests(self) -> List[dict]:
        """Get tests matching the current filter."""
        return get_tests_by_tags(
            include_tags=list(self.include_tags) if self.include_tags else None,
            exclude_tags=list(self.exclude_tags) if self.exclude_tags else None
        )


# Example usage and testing
if __name__ == "__main__":
    # Example test functions with tags
    
    @tag("unit", "filesystem", "quick", "safe")
    def example_unit_test():
        """Example unit test"""
        pass
    
    @tag("integration", "kernel_module", "slow", "vm_required", "risky")
    def example_integration_test():
        """Example integration test"""
        pass
    
    @performance_test("vector_operations")
    def example_performance_test():
        """Example performance test"""
        pass
    
    # Demonstrate filtering
    print("All registered tests:")
    for name, info in _test_registry.items():
        print(f"  {name}: {list(info['tags'])}")
    
    print("\nQuick tests:")
    quick_tests = get_quick_tests()
    for test in quick_tests:
        print(f"  {test['name']}: {list(test['tags'])}")
    
    print("\nFilesystem domain tests:")
    fs_tests = get_tests_by_domain("filesystem")
    for test in fs_tests:
        print(f"  {test['name']}: {list(test['tags'])}")
    
    print("\nUsing TestFilter:")
    filter_tests = (TestFilter()
                   .domain("kernel_module")
                   .type("integration")
                   .exclude("dangerous")
                   .get_tests())
    for test in filter_tests:
        print(f"  {test['name']}: {list(test['tags'])}")