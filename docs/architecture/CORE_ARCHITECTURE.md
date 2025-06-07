# VexFS v2.0 Core Module

## Overview

The `core/` directory contains the fundamental filesystem implementation for VexFS v2.0. This includes the main VFS operations, filesystem structures, and Phase 3 advanced indexing definitions.

## Files

### `vexfs_v2_main.c`
**Purpose**: Main filesystem implementation with VFS operations

**Key Components**:
- **VFS Operations**: File operations, inode operations, super operations
- **Filesystem Registration**: Module initialization and cleanup
- **IOCTL Handlers**: Core filesystem control operations
- **Memory Management**: Inode and dentry management
- **SIMD Support**: Optimized vector operations for x86_64

**Key Functions**:
- `vexfs_init_module()` - Module initialization
- `vexfs_cleanup_module()` - Module cleanup
- `vexfs_fill_super()` - Superblock initialization
- `vexfs_file_ioctl()` - IOCTL operation handler
- `vexfs_create()` - File creation
- `vexfs_lookup()` - Directory lookup

**Dependencies**:
- `../search/vexfs_v2_search.h` - Search algorithm interfaces
- `vexfs_v2_phase3.h` - Phase 3 definitions

### `vexfs_v2_phase3.h`
**Purpose**: Phase 3 advanced indexing definitions and structures

**Key Components**:
- **Data Structures**: Advanced indexing structures for Phase 3
- **Constants**: Configuration constants for advanced features
- **Type Definitions**: Phase 3 specific types and enums
- **Function Declarations**: Phase 3 function prototypes

**Key Definitions**:
- Phase 3 configuration structures
- Advanced indexing parameters
- Multi-model search definitions
- Performance monitoring structures

**Dependencies**:
- `../uapi/vexfs_v2_uapi.h` - UAPI definitions
- `../search/vexfs_v2_search.h` - Search interfaces

## Architecture

### Module Initialization Flow
1. **Register Filesystem**: Register VexFS with the kernel VFS
2. **Initialize Search**: Set up search algorithm subsystems
3. **Setup SIMD**: Configure SIMD capabilities
4. **Create Proc Entries**: Set up /proc interface for monitoring

### VFS Integration
- **Superblock Operations**: Mount, unmount, statfs
- **Inode Operations**: Create, lookup, unlink, mkdir, rmdir
- **File Operations**: Open, read, write, ioctl, mmap
- **Dentry Operations**: Validation and caching

### IOCTL Interface
The core module provides the main IOCTL entry point that dispatches to:
- Search operations (delegated to `search/` modules)
- Phase 3 operations (delegated to Phase 3 integration)
- Core filesystem operations (handled locally)

## Performance Features

### SIMD Optimization
- **AVX2 Support**: Vectorized operations for compatible CPUs
- **Fallback Implementation**: Standard operations for older CPUs
- **Runtime Detection**: Automatic capability detection

### Memory Management
- **Efficient Allocation**: Optimized memory allocation patterns
- **Caching**: Intelligent caching of frequently accessed data
- **Cleanup**: Proper resource cleanup and leak prevention

## Integration Points

### Search Module Integration
- Provides filesystem context to search operations
- Manages search result storage and retrieval
- Coordinates between different search algorithms

### Phase 3 Integration
- Enables advanced indexing features
- Supports multi-model search capabilities
- Provides performance monitoring infrastructure

## Development Guidelines

### Adding New VFS Operations
1. Add function declaration to appropriate operations structure
2. Implement function following VFS conventions
3. Update error handling and logging
4. Add appropriate tests

### Modifying IOCTL Interface
1. Update `../uapi/vexfs_v2_uapi.h` first
2. Add handler in `vexfs_file_ioctl()`
3. Implement operation logic
4. Add validation and error handling

### Performance Considerations
- Use SIMD operations where possible
- Minimize memory allocations in hot paths
- Implement proper caching strategies
- Monitor performance impact of changes

## Testing

### Unit Testing
- Test individual VFS operations
- Verify IOCTL command handling
- Validate memory management

### Integration Testing
- Test with search modules
- Verify Phase 3 integration
- Test filesystem mounting and operations

### Performance Testing
- Benchmark SIMD operations
- Measure filesystem operation latency
- Monitor memory usage patterns

## Debugging

### Kernel Logs
- Use `dmesg` to view kernel log messages
- Enable debug logging with appropriate kernel parameters
- Monitor for memory leaks and errors

### Proc Interface
- Check `/proc/vexfs/` for runtime statistics
- Monitor performance counters
- Verify configuration settings

---

The core module provides the foundation for VexFS v2.0, implementing essential filesystem functionality while maintaining clean interfaces to the search and utility modules.