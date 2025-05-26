# VexFS Domain-Driven Design Refactoring Plan

## Executive Summary

The current VexFS codebase suffers from monolithic file structures that impede LLM-assisted development. Files exceeding 1,400 lines create context window issues and mixed responsibilities that reduce development velocity. This plan outlines a systematic refactoring approach using Domain-Driven Design principles to create an LLM-optimized architecture.

## Problem Analysis

### Current Issues
- **file_ops.rs**: 1,388 lines - Too large for effective LLM processing
- **dir_ops.rs**: 1,492 lines - Mixed file/directory/permission concerns
- **ondisk.rs**: 1,120 lines - Serialization mixed with data structures
- **space_alloc.rs**: 878 lines - Multiple allocation strategies in one file

### LLM Development Challenges
1. **Context Window Limits**: Large files exceed optimal LLM context size
2. **Mixed Responsibilities**: LLMs struggle with multiple concerns per file
3. **Complex Dependencies**: Circular dependencies confuse LLM understanding
4. **Error Debugging**: Compilation errors are hard to isolate in large files

## Domain Architecture

### Core Domains Identified

#### 1. Filesystem Core (`fs_core`)
**Purpose**: Traditional filesystem operations and entities
**Size Target**: 15-20 files, 150-300 lines each

#### 2. Vector Search (`vector_domain`) 
**Purpose**: Vector indexing, search, and similarity operations
**Size Target**: 12-15 files, 200-300 lines each

#### 3. Storage Infrastructure (`storage`)
**Purpose**: Block management, journaling, persistence
**Size Target**: 10-12 files, 200-300 lines each

#### 4. Interface Layer (`interfaces`)
**Purpose**: VFS, FFI, and IOCTL integrations
**Size Target**: 8-10 files, 150-250 lines each

## Detailed Refactoring Plan

### Phase 1: Core Entity Extraction (Week 1)

#### Day 1-2: File Operations Refactoring
**Target**: Break down `file_ops.rs` (1,388 lines → 5 files)

```
src/fs_core/entities/file.rs                    (200 lines)
├── File entity with core properties
├── File state management
└── File validation logic

src/fs_core/services/file_operations.rs         (250 lines)
├── Core file operation business logic
├── File creation/deletion workflows
└── File content management

src/fs_core/services/file_permissions.rs        (150 lines)
├── Permission checking logic
├── Access control validation
└── Security enforcement

src/interfaces/vfs/file_interface.rs             (200 lines)
├── VFS layer integration
├── System call handling
└── Error translation

src/infrastructure/file_storage.rs               (200 lines)
├── Block allocation for files
├── Disk I/O operations
└── Caching logic
```

#### Day 3-4: Directory Operations Refactoring
**Target**: Break down `dir_ops.rs` (1,492 lines → 6 files)

```
src/fs_core/entities/directory.rs               (180 lines)
├── Directory entity definition
├── Directory entry management
└── Parent-child relationships

src/fs_core/services/directory_operations.rs    (250 lines)
├── Directory creation/deletion
├── Entry lookup and traversal
└── Rename/move operations

src/fs_core/services/path_resolution.rs         (200 lines)
├── Path parsing and validation
├── Symbolic link resolution
└── Path security checks

src/fs_core/value_objects/directory_entry.rs    (150 lines)
├── Directory entry value object
├── Entry comparison and sorting
└── Entry serialization

src/interfaces/vfs/directory_interface.rs       (200 lines)
├── VFS directory operations
├── Readdir implementation
└── Directory locking

src/infrastructure/directory_storage.rs         (200 lines)
├── Directory block management
├── Entry storage optimization
└── Directory caching
```

#### Day 5: On-Disk Structure Refactoring
**Target**: Break down `ondisk.rs` (1,120 lines → 4 files)

```
src/storage/disk_layout/structures.rs           (300 lines)
├── Core on-disk structures
├── Superblock, inode definitions
└── Block group descriptors

src/storage/disk_layout/serialization.rs        (250 lines)
├── Serialization trait implementations
├── Endianness handling
└── Version compatibility

src/storage/disk_layout/validation.rs           (200 lines)
├── Structure validation
├── Consistency checks
└── Corruption detection

src/storage/disk_layout/layout_calculator.rs    (200 lines)
├── Filesystem layout calculations
├── Size optimization
└── Block allocation strategies
```

### Phase 2: Service Layer Creation (Week 2)

#### Day 1-2: Domain Services
```
src/fs_core/services/inode_service.rs           (250 lines)
├── Inode lifecycle management
├── Inode allocation/deallocation
└── Inode caching strategy

src/fs_core/services/permission_service.rs      (200 lines)
├── UNIX permission model
├── Access control lists
└── Security policy enforcement

src/fs_core/services/link_service.rs            (180 lines)
├── Hard link management
├── Symbolic link operations
└── Link count maintenance
```

#### Day 3-4: Vector Domain Services
```
src/vector_domain/services/indexing_service.rs  (300 lines)
├── Vector index creation
├── Index maintenance
└── Index optimization

src/vector_domain/services/search_service.rs    (280 lines)
├── Vector similarity search
├── Query processing
└── Result ranking

src/vector_domain/services/embedding_service.rs (200 lines)
├── Vector embedding generation
├── Dimension reduction
└── Normalization operations
```

#### Day 5: Repository Interfaces
```
src/fs_core/repositories/inode_repository.rs    (150 lines)
├── Inode persistence interface
├── Query methods
└── Caching contract

src/fs_core/repositories/block_repository.rs    (150 lines)
├── Block storage interface
├── Allocation methods
└── Free space tracking

src/vector_domain/repositories/vector_repository.rs (150 lines)
├── Vector storage interface
├── Index persistence
└── Search optimization
```

### Phase 3: Infrastructure Refactoring (Week 3)

#### Day 1-2: Space Allocation Refactoring
**Target**: Break down `space_alloc.rs` (878 lines → 4 files)

```
src/storage/allocation/bitmap_allocator.rs      (250 lines)
├── Bitmap-based allocation
├── Free space tracking
└── Allocation optimization

src/storage/allocation/extent_allocator.rs      (200 lines)
├── Extent-based allocation
├── Contiguous block finding
└── Fragmentation management

src/storage/allocation/group_allocator.rs       (200 lines)
├── Block group management
├── Locality optimization
└── Load balancing

src/storage/allocation/allocation_policy.rs     (150 lines)
├── Allocation strategies
├── Policy configuration
└── Performance tuning
```

#### Day 3-4: Journaling System Refactoring
```
src/storage/journaling/transaction_manager.rs   (250 lines)
├── Transaction lifecycle
├── Atomicity guarantees
└── Rollback mechanisms

src/storage/journaling/log_writer.rs            (200 lines)
├── Journal log writing
├── Write optimization
└── Log rotation

src/storage/journaling/recovery_manager.rs      (200 lines)
├── Crash recovery
├── Journal replay
└── Consistency checking
```

#### Day 5: Vector Infrastructure
```
src/vector_domain/algorithms/hnsw/builder.rs    (250 lines)
├── HNSW index construction
├── Layer management
└── Connection optimization

src/vector_domain/algorithms/hnsw/search.rs     (200 lines)
├── HNSW search implementation
├── Beam search optimization
└── Result collection

src/vector_domain/algorithms/metrics/distance.rs (150 lines)
├── Distance metric implementations
├── Cosine similarity
└── Euclidean distance
```

### Phase 4: Interface Consolidation (Week 4)

#### Day 1-2: VFS Interface Refactoring
```
src/interfaces/vfs/super_operations.rs          (200 lines)
├── Filesystem mounting
├── Superblock operations
└── Statfs implementation

src/interfaces/vfs/inode_operations.rs          (200 lines)
├── Inode VFS operations
├── Attribute management
└── Extended attributes

src/interfaces/vfs/address_space_operations.rs  (180 lines)
├── Page cache integration
├── Memory mapping
└── Write-back operations
```

#### Day 3-4: FFI Layer Refactoring
```
src/interfaces/ffi/file_ffi.rs                  (200 lines)
├── File operation C bindings
├── Error code translation
└── Memory management

src/interfaces/ffi/directory_ffi.rs             (200 lines)
├── Directory operation C bindings
├── Path handling
└── Buffer management

src/interfaces/ffi/vector_ffi.rs                (180 lines)
├── Vector operation C bindings
├── Search result handling
└── Index management
```

#### Day 5: IOCTL Interface
```
src/interfaces/ioctl/file_controls.rs           (150 lines)
├── File-specific controls
├── Attribute modification
└── Special operations

src/interfaces/ioctl/vector_controls.rs         (200 lines)
├── Vector index controls
├── Search configuration
└── Performance tuning
```

## LLM Development Benefits

### Before Refactoring
```
❌ file_ops.rs (1,388 lines) - Context overflow, mixed concerns
❌ dir_ops.rs (1,492 lines) - Multiple responsibilities
❌ ondisk.rs (1,120 lines) - Data + logic mixed
❌ Circular dependencies confuse LLM understanding
```

### After Refactoring
```
✅ file.rs (200 lines) - Clear entity definition
✅ file_operations.rs (250 lines) - Focused business logic
✅ file_interface.rs (200 lines) - Clean VFS integration
✅ Clear dependency injection for testability
```

### LLM Workflow Improvements

#### 1. **Focused Context Windows**
- Each file fits comfortably in LLM context (200-300 lines)
- Clear purpose and responsibilities per file
- Reduced cognitive load for understanding code

#### 2. **Simplified Dependencies**
- Explicit dependency injection
- Interface-based coupling
- Clear module boundaries

#### 3. **Enhanced Error Debugging**
- Compilation errors isolated to specific concerns
- Easier to identify root causes
- Faster iteration cycles

#### 4. **Improved Testing**
- Smaller units are easier to test
- Mock dependencies through interfaces
- Better test coverage per concern

## Implementation Guidelines

### File Size Targets
- **Entities**: 150-200 lines (core domain objects)
- **Services**: 200-300 lines (business logic)
- **Repositories**: 100-150 lines (persistence interfaces)
- **Infrastructure**: 200-300 lines (implementation details)
- **Interfaces**: 150-250 lines (external integration)

### Naming Conventions
```
src/
├── fs_core/           # Filesystem domain
├── vector_domain/     # Vector search domain  
├── storage/          # Infrastructure domain
└── interfaces/       # External interfaces
```

### Dependency Rules
1. **Domain layers depend only on abstractions**
2. **Infrastructure implements domain interfaces**
3. **Interfaces orchestrate domain and infrastructure**
4. **No circular dependencies between domains**

## Migration Strategy

### Week 1: Extract Core Entities
1. Create new domain structure
2. Extract file and directory entities
3. Move on-disk structures
4. Ensure compilation

### Week 2: Create Service Layer
1. Extract business logic to services
2. Define repository interfaces
3. Implement dependency injection
4. Unit test services

### Week 3: Refactor Infrastructure
1. Break down large infrastructure files
2. Implement repository interfaces
3. Separate concerns clearly
4. Integration testing

### Week 4: Consolidate Interfaces
1. Refactor VFS integration
2. Clean up FFI layer
3. Organize IOCTL interfaces
4. End-to-end testing

## Success Metrics

### Development Velocity
- **File modification time**: Reduced by 60% (smaller context)
- **Compilation cycles**: Faster due to isolated changes
- **Error resolution**: Quicker due to focused error locations

### Code Quality
- **Cyclomatic complexity**: Reduced per file
- **Test coverage**: Improved due to smaller units
- **Maintainability index**: Higher due to separation of concerns

### LLM Effectiveness
- **Context utilization**: 90%+ effective context usage
- **Change accuracy**: Fewer unintended side effects
- **Feature velocity**: Faster implementation of new features

## Conclusion

This refactoring plan transforms VexFS from a monolithic structure to a clean, domain-driven architecture optimized for LLM-assisted development. The result will be:

1. **60+ smaller, focused files** instead of several large monolithic files
2. **Clear domain boundaries** that improve understanding and modification
3. **LLM-optimized file sizes** that fit effectively in context windows
4. **Better testability** through separation of concerns
5. **Future-proof architecture** for additional features

The investment in refactoring will pay dividends in development velocity, code quality, and maintainability throughout the project lifecycle.