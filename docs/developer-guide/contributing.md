# Contributing to VexFS v2.0

Welcome to VexFS v2.0! This guide will help you contribute to the world's first production-ready vector-extended filesystem.

## 🚀 Getting Started

### Prerequisites

Before contributing, ensure you have:

- **Linux development environment** (Ubuntu 20.04+ recommended)
- **Kernel headers** for your kernel version
- **Rust 1.70+** for userspace components
- **GCC/Clang** for kernel module development
- **Git** for version control
- **Docker** (optional, for testing)

### Development Setup

```bash
# Clone the repository
git clone https://github.com/lspecian/vexfs.git
cd vexfs

# Install development dependencies
sudo apt update
sudo apt install -y \
    build-essential \
    linux-headers-$(uname -r) \
    pkg-config \
    libssl-dev \
    libfuse-dev \
    git \
    curl

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build the project
make all

# Run tests
make test
```

## 📋 Project Structure

Understanding the codebase organization:

```
vexfs/
├── kernel/vexfs_v2_build/     # Kernel module implementation
│   ├── vexfs_v2_main.c        # Main kernel module
│   ├── vexfs_v2_hnsw.c        # HNSW algorithm
│   ├── vexfs_v2_lsh.c         # LSH algorithm
│   ├── vexfs_v2_uapi.h        # User API definitions
│   └── Makefile               # Kernel build system
├── rust/                      # Rust userspace components
│   ├── src/fuse_impl.rs       # FUSE implementation
│   ├── src/lib.rs             # Core library
│   └── Cargo.toml             # Rust dependencies
├── bindings/                  # Language bindings
│   ├── python/                # Python SDK
│   └── typescript/            # TypeScript SDK
├── vexctl/                    # CLI tool
├── docs/                      # Documentation
├── tests/                     # Test suites
└── examples/                  # Usage examples
```

## 🎯 Contribution Areas

### 1. Kernel Module Development

**Skills needed**: C, Linux kernel development, filesystems

**Areas to contribute**:
- ANNS algorithm improvements
- Performance optimizations
- Memory management enhancements
- New distance metrics
- Security features

**Getting started**:
```bash
cd kernel/vexfs_v2_build

# Build kernel module
make clean && make

# Load for testing
sudo insmod vexfs_v2.ko

# Check kernel logs
dmesg | grep vexfs
```

### 2. Userspace Components

**Skills needed**: Rust, FUSE, systems programming

**Areas to contribute**:
- FUSE implementation improvements
- Cross-platform compatibility
- Performance benchmarking
- Error handling
- Testing infrastructure

**Getting started**:
```bash
cd rust

# Build FUSE implementation
cargo build --release --bin vexfs_fuse

# Run tests
cargo test

# Check code quality
cargo clippy
cargo fmt
```

### 3. Language Bindings

**Skills needed**: Python, TypeScript, FFI

**Areas to contribute**:
- SDK feature completeness
- Documentation and examples
- Performance optimizations
- Additional language bindings
- Integration tests

**Python SDK**:
```bash
cd bindings/python

# Setup development environment
python -m venv venv
source venv/bin/activate
pip install -r requirements-dev.txt

# Install in development mode
pip install -e .

# Run tests
pytest tests/
```

**TypeScript SDK**:
```bash
cd bindings/typescript

# Install dependencies
npm install

# Build
npm run build

# Run tests
npm test

# Lint
npm run lint
```

### 4. Documentation

**Skills needed**: Technical writing, Markdown

**Areas to contribute**:
- User guides and tutorials
- API documentation
- Architecture documentation
- Performance guides
- Troubleshooting guides

### 5. Testing and Quality Assurance

**Skills needed**: Testing frameworks, CI/CD

**Areas to contribute**:
- Unit tests
- Integration tests
- Performance benchmarks
- Stress tests
- Compatibility tests

## 🔄 Development Workflow

### 1. Issue Selection

1. **Browse open issues** on GitHub
2. **Look for "good first issue"** labels for beginners
3. **Comment on the issue** to express interest
4. **Wait for assignment** before starting work

### 2. Branch Strategy

```bash
# Create feature branch
git checkout -b feature/your-feature-name

# Or for bug fixes
git checkout -b fix/issue-description

# Keep branch up to date
git fetch origin
git rebase origin/main
```

### 3. Development Process

#### For Kernel Module Changes

```bash
# 1. Make changes to kernel code
vim kernel/vexfs_v2_build/vexfs_v2_main.c

# 2. Build and test
cd kernel/vexfs_v2_build
make clean && make

# 3. Test loading
sudo rmmod vexfs_v2  # Remove old version
sudo insmod vexfs_v2.ko

# 4. Run tests
./test_hnsw_functionality
./standalone_phase3_test

# 5. Check for memory leaks
sudo dmesg | grep -i "memory leak\|slab\|kmem"
```

#### For Rust Components

```bash
# 1. Make changes
vim rust/src/fuse_impl.rs

# 2. Check formatting
cargo fmt

# 3. Run linter
cargo clippy -- -D warnings

# 4. Run tests
cargo test

# 5. Build release
cargo build --release
```

#### For Documentation

```bash
# 1. Edit documentation
vim docs/user-guide/installation.md

# 2. Check links and formatting
# (Use markdown linter if available)

# 3. Test examples
# Run any code examples to ensure they work
```

### 4. Testing Requirements

All contributions must include appropriate tests:

#### Kernel Module Tests

```c
// Example test function
static int test_hnsw_insertion(void)
{
    struct vexfs_hnsw_index *index;
    int32_t test_vector[384];
    int result;
    
    // Initialize test data
    for (int i = 0; i < 384; i++) {
        test_vector[i] = i % 1000;
    }
    
    // Create index
    index = vexfs_hnsw_create(384, 1000, 16, 200);
    if (!index) {
        return -ENOMEM;
    }
    
    // Test insertion
    result = vexfs_hnsw_insert(index, test_vector, 1);
    if (result != 0) {
        vexfs_hnsw_destroy(index);
        return result;
    }
    
    // Cleanup
    vexfs_hnsw_destroy(index);
    return 0;
}
```

#### Rust Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vector_insertion() {
        let mut collection = Collection::new("test", 384, Algorithm::HNSW);
        let vector = vec![0.1; 384];
        let metadata = HashMap::new();
        
        let result = collection.insert(vector, metadata);
        assert!(result.is_ok());
        
        let vector_id = result.unwrap();
        assert!(vector_id > 0);
    }
    
    #[test]
    fn test_vector_search() {
        let mut collection = Collection::new("test", 384, Algorithm::HNSW);
        
        // Insert test vectors
        for i in 0..100 {
            let vector = vec![i as f32 / 100.0; 384];
            collection.insert(vector, HashMap::new()).unwrap();
        }
        
        // Search
        let query = vec![0.5; 384];
        let results = collection.search(query, 10).unwrap();
        
        assert!(!results.is_empty());
        assert!(results.len() <= 10);
    }
}
```

#### Python Tests

```python
import pytest
import numpy as np
import vexfs

class TestVexFSCollection:
    @pytest.fixture
    def client(self):
        return vexfs.Client('/tmp/test_vexfs')
    
    @pytest.fixture
    def collection(self, client):
        return client.create_collection(
            name="test_collection",
            dimension=384,
            algorithm="hnsw"
        )
    
    def test_vector_insertion(self, collection):
        vector = np.random.random(384).astype(np.float32)
        metadata = {"test": True}
        
        result = collection.insert(vector=vector, metadata=metadata)
        
        assert result.success
        assert result.id > 0
    
    def test_vector_search(self, collection):
        # Insert test data
        vectors = np.random.random((100, 384)).astype(np.float32)
        collection.insert_batch(vectors)
        
        # Search
        query = np.random.random(384).astype(np.float32)
        results = collection.search(query, limit=10)
        
        assert len(results) <= 10
        assert all(r.distance >= 0 for r in results)
```

### 5. Code Quality Standards

#### C Code Standards

```c
// Use consistent naming
static int vexfs_function_name(struct vexfs_data *data);

// Document functions
/**
 * vexfs_hnsw_insert - Insert vector into HNSW index
 * @index: HNSW index structure
 * @vector: Vector data to insert
 * @id: Vector identifier
 *
 * Returns: 0 on success, negative error code on failure
 */
int vexfs_hnsw_insert(struct vexfs_hnsw_index *index,
                      const int32_t *vector,
                      uint64_t id);

// Error handling
if (!vector) {
    pr_err("vexfs: NULL vector pointer\n");
    return -EINVAL;
}

// Memory management
data = kmalloc(sizeof(*data), GFP_KERNEL);
if (!data) {
    return -ENOMEM;
}
// ... use data ...
kfree(data);
```

#### Rust Code Standards

```rust
// Use idiomatic Rust
pub fn insert_vector(&mut self, vector: Vec<f32>) -> Result<u64, VexFSError> {
    // Validate input
    if vector.len() != self.dimension {
        return Err(VexFSError::DimensionMismatch {
            expected: self.dimension,
            actual: vector.len(),
        });
    }
    
    // Implementation
    let id = self.next_id();
    self.vectors.insert(id, vector);
    Ok(id)
}

// Use proper error types
#[derive(Debug, thiserror::Error)]
pub enum VexFSError {
    #[error("Vector dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    
    #[error("Collection not found: {name}")]
    CollectionNotFound { name: String },
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

#### Python Code Standards

```python
# Type hints
from typing import List, Dict, Optional, Union
import numpy as np

def insert_vector(
    self,
    vector: Union[np.ndarray, List[float]],
    metadata: Optional[Dict[str, Any]] = None
) -> InsertResult:
    """Insert a vector into the collection.
    
    Args:
        vector: Vector data with correct dimension
        metadata: Optional metadata dictionary
        
    Returns:
        InsertResult with assigned vector ID
        
    Raises:
        VectorDimensionError: If vector dimension doesn't match
        VexFSError: For other errors
    """
    # Validate input
    if len(vector) != self.dimension:
        raise VectorDimensionError(
            f"Expected dimension {self.dimension}, got {len(vector)}"
        )
    
    # Implementation
    return self._insert_vector_impl(vector, metadata)

# Docstrings for all public functions
# Use numpy-style docstrings
```

## 📝 Pull Request Process

### 1. Pre-submission Checklist

- [ ] **Code compiles** without warnings
- [ ] **All tests pass** (unit, integration, performance)
- [ ] **Code follows style guidelines**
- [ ] **Documentation updated** (if applicable)
- [ ] **Changelog updated** (for significant changes)
- [ ] **No memory leaks** (for kernel code)
- [ ] **Performance impact assessed**

### 2. Pull Request Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] Performance testing performed (if applicable)

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests added for new functionality
- [ ] All tests pass
- [ ] No new compiler warnings

## Performance Impact
Describe any performance implications

## Breaking Changes
List any breaking changes and migration path
```

### 3. Review Process

1. **Automated checks** run (CI/CD pipeline)
2. **Code review** by maintainers
3. **Testing** on multiple platforms
4. **Performance validation** (if applicable)
5. **Documentation review**
6. **Final approval** and merge

## 🐛 Bug Reports

### Bug Report Template

```markdown
## Bug Description
Clear description of the bug

## Environment
- OS: Ubuntu 22.04
- Kernel: 5.15.0
- VexFS Version: v2.0.0
- Hardware: x86_64, 16GB RAM

## Steps to Reproduce
1. Step one
2. Step two
3. Step three

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Error Messages
```
Paste any error messages or logs
```

## Additional Context
Any other relevant information
```

### Debugging Information

When reporting bugs, include:

```bash
# System information
uname -a
cat /etc/os-release
free -h
lscpu

# VexFS status
lsmod | grep vexfs
cat /proc/filesystems | grep vexfs
mount | grep vexfs

# Recent logs
dmesg | grep vexfs | tail -20
journalctl -u vexfs -n 50
```

## 🎯 Feature Requests

### Feature Request Template

```markdown
## Feature Description
Clear description of the proposed feature

## Use Case
Why is this feature needed?

## Proposed Solution
How should this feature work?

## Alternatives Considered
Other approaches considered

## Implementation Notes
Technical considerations

## Breaking Changes
Any potential breaking changes
```

## 🏆 Recognition

Contributors are recognized through:

- **GitHub contributors page**
- **Changelog mentions**
- **Release notes**
- **Special recognition** for significant contributions

## 📞 Getting Help

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and ideas
- **Code Reviews**: Technical discussions on PRs

### Mentorship

New contributors can get help through:

- **Good first issue** labels
- **Detailed issue descriptions**
- **Code review feedback**
- **Documentation and examples**

## 📚 Learning Resources

### Kernel Development

- [Linux Kernel Development (3rd Edition)](https://www.amazon.com/Linux-Kernel-Development-Robert-Love/dp/0672329468)
- [Linux Device Drivers](https://lwn.net/Kernel/LDD3/)
- [Kernel Newbies](https://kernelnewbies.org/)

### Rust Development

- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/)

### Vector Databases

- [Approximate Nearest Neighbor Search](https://en.wikipedia.org/wiki/Nearest_neighbor_search#Approximate_nearest_neighbor)
- [HNSW Paper](https://arxiv.org/abs/1603.09320)
- [LSH Tutorial](https://web.stanford.edu/class/cs246/slides/03-lsh.pdf)

## 🔒 Security

### Reporting Security Issues

**DO NOT** open public issues for security vulnerabilities.

Instead:
1. **Email**: security@vexfs.org
2. **Include**: Detailed description and reproduction steps
3. **Wait**: For acknowledgment before public disclosure

### Security Guidelines

- **Validate all inputs** from userspace
- **Use safe memory operations** (no buffer overflows)
- **Check permissions** before operations
- **Sanitize file paths** and metadata
- **Limit resource usage** to prevent DoS

## 📄 License

By contributing to VexFS v2.0, you agree that your contributions will be licensed under:

- **GPL v2** for kernel module components
- **Apache 2.0** for userspace components

See [LICENSE](../../LICENSE) for details.

---

**Thank you for contributing to VexFS v2.0!** 🚀

Your contributions help build the future of vector-extended filesystems. Every bug fix, feature, test, and documentation improvement makes VexFS better for everyone.

**Questions?** Open a [discussion](https://github.com/lspecian/vexfs/discussions) or check our [documentation](../README.md).