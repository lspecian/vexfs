# VexFS Domain-Driven Design Architecture

## Overview

This document outlines the Domain-Driven Design (DDD) refactoring architecture for VexFS, extracting monolithic files into well-defined domain boundaries with clear entities and responsibilities.

## Current Monolithic Structure Analysis

### Identified Issues
- **file_ops.rs**: 1,388 lines - Contains file operations, locking primitives, context management, and permissions
- **dir_ops.rs**: 1,492 lines - Contains directory operations, entry management, and traversal logic
- **ondisk.rs**: 1,120 lines - Contains on-disk format definitions, serialization, and layout structures

### Key Responsibilities Identified

#### From file_ops.rs:
- File operations (create, read, write, truncate, unlink)
- Locking primitives (VexfsSpinLock, VexfsInodeLock)
- Filesystem context management (VexfsContext)
- Permission checking
- File handle management
- File type constants and flags

#### From dir_ops.rs:
- Directory operations (create, delete, traverse)
- Directory entry management
- Directory handle management
- Directory locking
- Entry iteration

#### From ondisk.rs:
- On-disk format definitions
- Serialization traits
- Block and inode layout
- Constants and limits
- Type definitions

## Proposed Domain Structure

### 1. Filesystem Core Domain (`fs_core/`)
**Purpose**: Traditional filesystem operations and POSIX compliance
**Bounded Context**: File and directory operations, POSIX semantics

#### Entities:
- **File** - Represents a regular file with metadata and content
- **Directory** - Represents a directory with entries and metadata
- **Inode** - Core filesystem metadata structure
- **Path** - Filesystem path operations and validation
- **Permission** - Access control and permission checking

#### Modules:
```
fs_core/
├── mod.rs              # Domain module definition
├── entities/
│   ├── mod.rs
│   ├── file.rs         # File entity (150-200 lines)
│   ├── directory.rs    # Directory entity (150-200 lines)
│   ├── inode.rs        # Inode entity (100-150 lines)
│   ├── path.rs         # Path entity (100-150 lines)
│   └── permission.rs   # Permission entity (100-150 lines)
├── services/
│   ├── mod.rs
│   ├── file_service.rs # File operations service (200-250 lines)
│   └── dir_service.rs  # Directory operations service (200-250 lines)
└── repositories/
    ├── mod.rs
    ├── file_repo.rs    # File persistence abstraction (150-200 lines)
    └── dir_repo.rs     # Directory persistence abstraction (150-200 lines)
```

### 2. Vector Search Domain (`vector_domain/`)
**Purpose**: Vector indexing, similarity search, and embedding operations
**Bounded Context**: Vector operations, search algorithms, metrics

#### Entities:
- **Vector** - Represents a vector embedding with metadata
- **VectorIndex** - Manages vector indices and search structures
- **SearchQuery** - Encapsulates search parameters and constraints
- **SearchResult** - Represents search results with scores and metadata
- **DistanceMetric** - Defines distance calculation methods

#### Modules:
```
vector_domain/
├── mod.rs              # Domain module definition
├── entities/
│   ├── mod.rs
│   ├── vector.rs       # Vector entity (100-150 lines)
│   ├── index.rs        # Vector index entity (150-200 lines)
│   ├── query.rs        # Search query entity (100-150 lines)
│   ├── result.rs       # Search result entity (100-150 lines)
│   └── metrics.rs      # Distance metrics entity (150-200 lines)
├── services/
│   ├── mod.rs
│   ├── search_service.rs   # Vector search service (200-250 lines)
│   ├── index_service.rs    # Index management service (200-250 lines)
│   └── metrics_service.rs  # Metrics calculation service (150-200 lines)
└── repositories/
    ├── mod.rs
    ├── vector_repo.rs  # Vector persistence abstraction (150-200 lines)
    └── index_repo.rs   # Index persistence abstraction (150-200 lines)
```

### 3. Storage Domain (`storage/`)
**Purpose**: Block management, journaling, persistence, space allocation
**Bounded Context**: Physical storage, consistency, allocation

#### Entities:
- **Block** - Represents a storage block with metadata
- **Journal** - Transaction logging and recovery
- **Transaction** - Atomic operation unit
- **Allocation** - Space allocation tracking
- **Superblock** - Filesystem metadata and configuration

#### Modules:
```
storage/
├── mod.rs              # Domain module definition
├── entities/
│   ├── mod.rs
│   ├── block.rs        # Block entity (100-150 lines)
│   ├── journal.rs      # Journal entity (150-200 lines)
│   ├── transaction.rs  # Transaction entity (100-150 lines)
│   ├── allocation.rs   # Allocation entity (150-200 lines)
│   └── superblock.rs   # Superblock entity (100-150 lines)
├── services/
│   ├── mod.rs
│   ├── block_service.rs    # Block management service (200-250 lines)
│   ├── journal_service.rs  # Journaling service (200-250 lines)
│   └── alloc_service.rs    # Allocation service (200-250 lines)
└── repositories/
    ├── mod.rs
    ├── block_repo.rs   # Block persistence abstraction (150-200 lines)
    └── journal_repo.rs # Journal persistence abstraction (150-200 lines)
```

### 4. Interface Domain (`interfaces/`)
**Purpose**: VFS integration, FFI bindings, IOCTL operations
**Bounded Context**: External system integration, API boundaries

#### Entities:
- **VfsOperation** - VFS interface operations
- **FfiBinding** - C FFI interface definitions
- **IoctlCommand** - IOCTL command processing
- **KernelInterface** - Kernel-specific operations

#### Modules:
```
interfaces/
├── mod.rs              # Domain module definition
├── entities/
│   ├── mod.rs
│   ├── vfs_operation.rs    # VFS operation entity (150-200 lines)
│   ├── ffi_binding.rs      # FFI binding entity (100-150 lines)
│   ├── ioctl_command.rs    # IOCTL command entity (100-150 lines)
│   └── kernel_interface.rs # Kernel interface entity (150-200 lines)
├── services/
│   ├── mod.rs
│   ├── vfs_service.rs      # VFS integration service (200-250 lines)
│   ├── ffi_service.rs      # FFI service (150-200 lines)
│   └── ioctl_service.rs    # IOCTL service (150-200 lines)
└── adapters/
    ├── mod.rs
    ├── vfs_adapter.rs      # VFS adaptation layer (200-250 lines)
    └── ffi_adapter.rs      # FFI adaptation layer (150-200 lines)
```

### 5. Shared Domain (`shared/`)
**Purpose**: Cross-domain utilities, common types, error handling
**Bounded Context**: Common infrastructure, utilities

#### Entities:
- **ErrorType** - Common error definitions
- **ResultType** - Common result types
- **Constants** - Shared constants and limits
- **Utilities** - Common utility functions

#### Modules:
```
shared/
├── mod.rs              # Domain module definition
├── types/
│   ├── mod.rs
│   ├── errors.rs       # Error types (100-150 lines)
│   ├── results.rs      # Result types (50-100 lines)
│   └── constants.rs    # Constants and limits (100-150 lines)
├── utils/
│   ├── mod.rs
│   ├── locking.rs      # Locking primitives (150-200 lines)
│   ├── serialization.rs # Serialization utilities (100-150 lines)
│   └── validation.rs   # Validation utilities (100-150 lines)
└── traits/
    ├── mod.rs
    └── common.rs       # Common traits (100-150 lines)
```

## Domain Boundaries and Interactions

### Bounded Contexts

1. **Filesystem Core** ↔ **Storage**: File/directory operations require block allocation and journaling
2. **Filesystem Core** ↔ **Vector Domain**: Files may contain vector data requiring indexing
3. **Vector Domain** ↔ **Storage**: Vector indices require persistent storage
4. **Interface Domain** ↔ **All Domains**: Provides external access to all domain functionality
5. **Shared** → **All Domains**: Provides common infrastructure to all domains

### Domain Events

#### From Filesystem Core:
- `FileCreated(inode, path, metadata)`
- `FileDeleted(inode, path)`
- `DirectoryCreated(inode, path, metadata)`
- `DirectoryDeleted(inode, path)`

#### From Vector Domain:
- `VectorIndexed(vector_id, index_id, metadata)`
- `SearchCompleted(query, results, metrics)`

#### From Storage:
- `BlockAllocated(block_id, size)`
- `BlockDeallocated(block_id)`
- `TransactionCommitted(transaction_id)`
- `TransactionRolledBack(transaction_id)`

### Cross-Domain Dependencies

#### Dependency Direction (avoiding cycles):
```
Interfaces → Filesystem Core → Vector Domain → Storage → Shared
            ↓                                    ↓
            → Storage ← Shared               → Shared
```

## Entity Extraction Strategy

### Phase 1: Extract Core Entities
1. **Move constants and types** from monolithic files to appropriate domains
2. **Extract entity definitions** with clear boundaries
3. **Create domain module structure** with proper visibility

### Phase 2: Extract Services
1. **Move business logic** from monolithic files to domain services
2. **Define service interfaces** for cross-domain communication
3. **Implement repository abstractions** for persistence

### Phase 3: Wire Domains
1. **Update lib.rs** to use new domain structure
2. **Ensure compilation** maintains existing functionality
3. **Document domain interactions** and event flows

## Implementation Guidelines

### Entity Design Principles
- **Single Responsibility**: Each entity has one clear purpose
- **Encapsulation**: Hide internal state, expose behavior through methods
- **Immutability**: Prefer immutable structures where possible
- **Domain Focus**: Entities reflect domain concepts, not technical implementation

### Service Design Principles
- **Stateless**: Services should not maintain state between calls
- **Dependency Injection**: Services receive dependencies through constructor
- **Interface Segregation**: Small, focused service interfaces
- **Domain Events**: Use events for cross-domain communication

### Repository Design Principles
- **Abstraction**: Hide persistence details from domain logic
- **Testability**: Enable easy mocking for unit tests
- **Consistency**: Maintain data consistency within bounded contexts

## Migration Strategy

### Maintaining Compilation During Refactoring
1. **Create new domain structure** alongside existing code
2. **Gradually move functionality** to new domains
3. **Use re-exports** in old modules to maintain API compatibility
4. **Remove old code** only after new implementation is complete

### Testing Strategy
- **Unit tests** for each domain entity and service
- **Integration tests** for cross-domain interactions
- **Regression tests** to ensure existing functionality remains intact

## Expected Benefits

### Development Velocity
- **Smaller files** (200-300 lines) fit within LLM context windows
- **Clear boundaries** make code changes more predictable
- **Domain focus** reduces cognitive load when working on specific features

### Code Quality
- **Single Responsibility** at the domain level
- **Testability** through clear interfaces and dependency injection
- **Maintainability** through well-defined boundaries

### Future Extensibility
- **New domains** can be added without affecting existing code
- **Domain evolution** can happen independently
- **Clear integration points** for new features

## Next Steps

1. **Create domain module files** with basic structure
2. **Extract shared types** and constants first
3. **Move entities** to appropriate domains
4. **Implement services** and repositories
5. **Update integration points** and tests
6. **Validate compilation** and functionality

This architecture provides a solid foundation for implementing the remaining VexFS features while maintaining code quality and development velocity.