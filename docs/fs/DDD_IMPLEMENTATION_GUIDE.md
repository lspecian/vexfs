# VexFS DDD Implementation Guide

## Overview

This document provides the step-by-step implementation guide for executing the Domain-Driven Design refactoring of VexFS. It includes specific file creation instructions, module organization, and integration strategies.

## Domain Module Structure

### Complete File Structure
```
fs/src/
├── lib.rs                          # Updated main library file
├── shared/
│   ├── mod.rs                      # Shared domain module
│   ├── types/
│   │   ├── mod.rs
│   │   ├── constants.rs            # All VexFS constants (150 lines)
│   │   ├── errors.rs               # Unified error types (100 lines)
│   │   └── results.rs              # Common result types (50 lines)
│   ├── utils/
│   │   ├── mod.rs
│   │   ├── locking.rs              # Locking primitives (200 lines)
│   │   ├── serialization.rs        # Serialization utilities (150 lines)
│   │   └── validation.rs           # Validation utilities (100 lines)
│   └── traits/
│       ├── mod.rs
│       └── common.rs               # Common traits and interfaces (100 lines)
├── storage/
│   ├── mod.rs                      # Storage domain module
│   ├── entities/
│   │   ├── mod.rs
│   │   ├── superblock.rs           # Filesystem metadata (150 lines)
│   │   ├── allocation.rs           # Space allocation tracking (150 lines)
│   │   ├── block.rs                # Block management (150 lines)
│   │   ├── journal.rs              # Transaction logging (150 lines)
│   │   └── transaction.rs          # Transaction entities (100 lines)
│   ├── services/
│   │   ├── mod.rs
│   │   ├── block_service.rs        # Block management service (250 lines)
│   │   ├── journal_service.rs      # Journaling service (250 lines)
│   │   └── alloc_service.rs        # Allocation service (250 lines)
│   └── repositories/
│       ├── mod.rs
│       ├── block_repo.rs           # Block persistence (200 lines)
│       └── journal_repo.rs         # Journal persistence (200 lines)
├── fs_core/
│   ├── mod.rs                      # Filesystem core domain module
│   ├── entities/
│   │   ├── mod.rs
│   │   ├── file.rs                 # File entity and operations (200 lines)
│   │   ├── directory.rs            # Directory entity and operations (200 lines)
│   │   ├── inode.rs                # Inode entity and metadata (150 lines)
│   │   ├── path.rs                 # Path handling and validation (150 lines)
│   │   └── permission.rs           # Permission checking (100 lines)
│   ├── services/
│   │   ├── mod.rs
│   │   ├── file_service.rs         # File operations service (250 lines)
│   │   ├── dir_service.rs          # Directory operations service (250 lines)
│   │   └── filesystem_service.rs   # Filesystem coordination (200 lines)
│   └── repositories/
│       ├── mod.rs
│       ├── file_repo.rs            # File persistence abstraction (200 lines)
│       └── dir_repo.rs             # Directory persistence abstraction (200 lines)
├── vector_domain/
│   ├── mod.rs                      # Vector domain module
│   ├── entities/
│   │   ├── mod.rs
│   │   ├── vector.rs               # Vector entity (150 lines)
│   │   ├── index.rs                # Vector index entity (200 lines)
│   │   ├── query.rs                # Search query entity (150 lines)
│   │   ├── result.rs               # Search result entity (150 lines)
│   │   └── metrics.rs              # Distance metrics entity (200 lines)
│   ├── services/
│   │   ├── mod.rs
│   │   ├── search_service.rs       # Vector search service (250 lines)
│   │   ├── index_service.rs        # Index management service (250 lines)
│   │   └── metrics_service.rs      # Metrics calculation service (200 lines)
│   └── repositories/
│       ├── mod.rs
│       ├── vector_repo.rs          # Vector persistence (200 lines)
│       └── index_repo.rs           # Index persistence (200 lines)
├── interfaces/
│   ├── mod.rs                      # Interfaces domain module
│   ├── entities/
│   │   ├── mod.rs
│   │   ├── vfs_operation.rs        # VFS operation entity (200 lines)
│   │   ├── ffi_binding.rs          # FFI binding entity (150 lines)
│   │   ├── ioctl_command.rs        # IOCTL command entity (150 lines)
│   │   └── kernel_interface.rs     # Kernel interface entity (200 lines)
│   ├── services/
│   │   ├── mod.rs
│   │   ├── vfs_service.rs          # VFS integration service (250 lines)
│   │   ├── ffi_service.rs          # FFI service (200 lines)
│   │   └── ioctl_service.rs        # IOCTL service (200 lines)
│   └── adapters/
│       ├── mod.rs
│       ├── vfs_adapter.rs          # VFS adaptation layer (250 lines)
│       └── ffi_adapter.rs          # FFI adaptation layer (200 lines)
└── legacy/
    ├── mod.rs                      # Legacy module compatibility layer
    ├── file_ops.rs                 # Legacy file_ops (deprecated)
    ├── dir_ops.rs                  # Legacy dir_ops (deprecated)
    └── ondisk.rs                   # Legacy ondisk (deprecated)
```

## Implementation Phases

### Phase 1: Foundation Setup (Day 1-2)

#### Step 1.1: Create Domain Module Structure
```bash
# Create domain directories (already done)
mkdir -p fs/src/{shared,storage,fs_core,vector_domain,interfaces,legacy}

# Create subdirectories
mkdir -p fs/src/shared/{types,utils,traits}
mkdir -p fs/src/storage/{entities,services,repositories}
mkdir -p fs/src/fs_core/{entities,services,repositories}
mkdir -p fs/src/vector_domain/{entities,services,repositories}
mkdir -p fs/src/interfaces/{entities,services,adapters}
```

#### Step 1.2: Create Module Declaration Files
Each `mod.rs` file needs to be created with proper module declarations:

**fs/src/shared/mod.rs**
```rust
//! Shared utilities and types across all VexFS domains

pub mod types;
pub mod utils;
pub mod traits;

// Re-export commonly used types
pub use types::{constants::*, errors::*, results::*};
pub use traits::common::*;
```

**fs/src/storage/mod.rs**
```rust
//! Storage domain for block management, journaling, and persistence

pub mod entities;
pub mod services;
pub mod repositories;

// Re-export key entities and services
pub use entities::{superblock::*, allocation::*, block::*, journal::*};
pub use services::{block_service::*, journal_service::*, alloc_service::*};
```

#### Step 1.3: Move Legacy Files
```bash
# Move existing monolithic files to legacy directory
mv fs/src/file_ops.rs fs/src/legacy/
mv fs/src/dir_ops.rs fs/src/legacy/
mv fs/src/ondisk.rs fs/src/legacy/
```

### Phase 2: Shared Domain Implementation (Day 2-3)

#### Step 2.1: Extract Constants
Create `fs/src/shared/types/constants.rs` with all constants from:
- `ondisk.rs` lines 24-100 (filesystem constants)
- `file_ops.rs` lines 41-58, 250-295 (file type and operation constants)
- `dir_ops.rs` lines 23-25 (directory constants)

#### Step 2.2: Unify Error Types
Create `fs/src/shared/types/errors.rs` consolidating error types from:
- `ffi.rs` VexfsError enum
- `dir_ops.rs` DirOpError enum
- Add vector domain error types
- Add storage domain error types

#### Step 2.3: Extract Locking Primitives
Create `fs/src/shared/utils/locking.rs` with:
- VexfsSpinLock from `file_ops.rs` lines 62-87
- VexfsInodeLock from `file_ops.rs` lines 89-141
- Additional domain coordination locks

#### Step 2.4: Define Common Traits
Create `fs/src/shared/traits/common.rs` with:
- OnDiskSerialize trait from `ondisk.rs`
- Entity trait for domain entities
- Repository trait for data access
- Service trait for business logic

### Phase 3: Storage Domain Implementation (Day 3-5)

#### Step 3.1: Extract Superblock Entity
Create `fs/src/storage/entities/superblock.rs`:
- VexfsSuperblock from `ondisk.rs`
- Superblock operations and validation
- On-disk serialization implementation

#### Step 3.2: Extract Allocation Entities
Create `fs/src/storage/entities/allocation.rs`:
- VexfsGroupDesc from `ondisk.rs`
- Block and inode allocation tracking
- Space management utilities

#### Step 3.3: Implement Storage Services
Create storage services that coordinate entity operations:
- `block_service.rs` for block management
- `journal_service.rs` for transaction logging
- `alloc_service.rs` for space allocation

### Phase 4: Filesystem Core Implementation (Day 5-7)

#### Step 4.1: Extract Inode Entity
Create `fs/src/fs_core/entities/inode.rs`:
- VexfsInode from `ondisk.rs`
- Inode operations and metadata management
- Block pointer handling

#### Step 4.2: Extract File Entity
Create `fs/src/fs_core/entities/file.rs`:
- VexfsFileHandle from `file_ops.rs` lines 237-247
- File operations (read, write, truncate)
- File metadata management

#### Step 4.3: Extract Directory Entity
Create `fs/src/fs_core/entities/directory.rs`:
- VexfsDirHandle from `dir_ops.rs` lines 44-73
- VexfsDirEntry from `ondisk.rs`
- Directory operations and entry management

#### Step 4.4: Extract Permission Entity
Create `fs/src/fs_core/entities/permission.rs`:
- Permission checking logic from `file_ops.rs` lines 260-285
- Access control implementation
- Security validation

### Phase 5: Vector Domain Implementation (Day 7-9)

#### Step 5.1: Migrate Existing Vector Code
Move existing vector-related modules to vector_domain:
- Consolidate `vector_search.rs`, `vector_storage.rs`, `vector_metrics.rs`
- Move `anns/` subdirectory to `vector_domain/anns/`
- Extract vector entities from existing code

#### Step 5.2: Create Vector Entities
- Vector entity with metadata
- VectorIndex entity with search capabilities
- SearchQuery and SearchResult entities
- DistanceMetric entity

### Phase 6: Interface Domain Implementation (Day 9-10)

#### Step 6.1: Extract VFS Interface
Create `fs/src/interfaces/entities/vfs_operation.rs`:
- VFS operation abstractions
- POSIX compatibility layer
- Kernel interface definitions

#### Step 6.2: Extract FFI Bindings
Create `fs/src/interfaces/entities/ffi_binding.rs`:
- C FFI interface definitions from `ffi.rs`
- Type marshalling and conversion
- Error code translation

#### Step 6.3: Extract IOCTL Interface
Create `fs/src/interfaces/entities/ioctl_command.rs`:
- IOCTL command definitions from `ioctl.rs`
- Command processing and validation
- User-kernel communication

### Phase 7: Integration and Wiring (Day 10-12)

#### Step 7.1: Update lib.rs
Update the main library file to use the new domain structure:

```rust
// Domain modules
pub mod shared;
pub mod storage;
pub mod fs_core;
pub mod vector_domain;
pub mod interfaces;

// Legacy compatibility (temporary)
pub mod legacy;

// Re-export key types for backward compatibility
pub use shared::types::*;
pub use fs_core::entities::{VexfsFile, VexfsDirectory, VexfsInode};
pub use storage::entities::VexfsSuperblock;
pub use interfaces::entities::VfsOperation;

// Conditional compilation for userspace vs kernel
#[cfg(not(feature = "kernel"))]
pub use vector_domain::entities::*;
```

#### Step 7.2: Create Domain Facades
Create high-level domain facades that coordinate cross-domain operations:

**fs/src/fs_core/filesystem_facade.rs**
```rust
use crate::storage::services::*;
use crate::fs_core::entities::*;
use crate::shared::types::*;

pub struct FilesystemFacade {
    block_service: BlockService,
    journal_service: JournalService,
    alloc_service: AllocService,
}

impl FilesystemFacade {
    pub fn create_file(&mut self, path: &str, mode: u32) -> VexfsResult<VexfsFile> {
        // Coordinate between storage and fs_core domains
    }
    
    pub fn read_file(&mut self, file: &mut VexfsFile, buf: &mut [u8]) -> VexfsResult<usize> {
        // Coordinate file read with storage layer
    }
}
```

#### Step 7.3: Implement Domain Events
Create event system for cross-domain communication:

**fs/src/shared/events/mod.rs**
```rust
pub enum DomainEvent {
    FileCreated { inode: u64, path: String },
    FileDeleted { inode: u64 },
    BlockAllocated { block_id: u64, size: u32 },
    VectorIndexed { vector_id: u64, index_id: u64 },
}

pub trait EventHandler {
    fn handle(&mut self, event: DomainEvent) -> VexfsResult<()>;
}
```

## Domain Interaction Patterns

### Cross-Domain Communication

#### 1. Repository Pattern for Data Access
```rust
// Storage repository interface
pub trait BlockRepository {
    fn allocate(&mut self, size: u32) -> VexfsResult<u64>;
    fn deallocate(&mut self, block_id: u64) -> VexfsResult<()>;
    fn read(&self, block_id: u64, buf: &mut [u8]) -> VexfsResult<()>;
    fn write(&mut self, block_id: u64, data: &[u8]) -> VexfsResult<()>;
}

// Filesystem core using storage repository
impl FileService {
    pub fn new(block_repo: Box<dyn BlockRepository>) -> Self {
        Self { block_repo }
    }
    
    pub fn read_file_data(&self, file: &VexfsFile, offset: u64, buf: &mut [u8]) -> VexfsResult<usize> {
        // Use block_repo to read file data
    }
}
```

#### 2. Service Layer for Business Logic
```rust
// Cross-domain service coordination
pub struct VexfsService {
    filesystem_service: FilesystemService,
    storage_service: StorageService,
    vector_service: VectorService,
}

impl VexfsService {
    pub fn create_vector_file(&mut self, path: &str, vectors: Vec<Vector>) -> VexfsResult<VexfsFile> {
        // 1. Create file using filesystem service
        let file = self.filesystem_service.create_file(path)?;
        
        // 2. Allocate storage using storage service
        let blocks = self.storage_service.allocate_blocks(vectors.len())?;
        
        // 3. Index vectors using vector service
        self.vector_service.index_vectors(&vectors, &blocks)?;
        
        Ok(file)
    }
}
```

#### 3. Event-Driven Communication
```rust
// Event bus for domain coordination
pub struct EventBus {
    handlers: HashMap<TypeId, Vec<Box<dyn EventHandler>>>,
}

impl EventBus {
    pub fn publish(&mut self, event: DomainEvent) -> VexfsResult<()> {
        // Notify all registered handlers
    }
    
    pub fn subscribe<T: EventHandler + 'static>(&mut self, handler: T) {
        // Register handler for specific event types
    }
}
```

## Compilation Strategy

### Maintaining Compilation During Refactoring

#### 1. Legacy Compatibility Layer
```rust
// fs/src/legacy/mod.rs
//! Legacy compatibility layer for gradual migration

pub mod file_ops;
pub mod dir_ops;
pub mod ondisk;

// Re-export legacy functions for backward compatibility
pub use file_ops::*;
pub use dir_ops::*;
pub use ondisk::*;
```

#### 2. Gradual Migration Pattern
```rust
// fs/src/fs_core/entities/file.rs
use crate::legacy::file_ops as legacy;

impl VexfsFile {
    pub fn read(&mut self, buf: &mut [u8]) -> VexfsResult<usize> {
        // New implementation
        if cfg!(feature = "new_implementation") {
            self.read_new_impl(buf)
        } else {
            // Delegate to legacy implementation
            legacy::vexfs_file_read(self.handle.ino, buf)
        }
    }
}
```

#### 3. Feature Flag Strategy
```toml
# Cargo.toml
[features]
default = ["legacy_compat"]
legacy_compat = []
new_domains = []
full_ddd = ["new_domains"]
```

### Testing Strategy

#### 1. Domain Unit Tests
```rust
// fs/src/fs_core/entities/tests/file_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_file_creation() {
        let file = VexfsFile::new(1, 0o644);
        assert_eq!(file.handle.ino, 1);
    }
    
    #[test]
    fn test_file_read_write() {
        // Test file operations
    }
}
```

#### 2. Integration Tests
```rust
// fs/tests/integration/domain_integration.rs
#[test]
fn test_cross_domain_file_creation() {
    let mut facade = FilesystemFacade::new();
    let file = facade.create_file("/test/file.txt", 0o644).unwrap();
    assert!(file.handle.ino > 0);
}
```

#### 3. Regression Tests
```rust
// fs/tests/regression/legacy_compatibility.rs
#[test]
fn test_legacy_file_ops_compatibility() {
    // Ensure new implementation produces same results as legacy
}
```

## Validation Checklist

### Phase Completion Criteria

#### Phase 1: Foundation
- [ ] All domain directories created
- [ ] All module declaration files created
- [ ] Legacy files moved to legacy directory
- [ ] Project compiles with legacy compatibility

#### Phase 2: Shared Domain
- [ ] Constants extracted and consolidated
- [ ] Error types unified across domains
- [ ] Locking primitives extracted
- [ ] Common traits defined
- [ ] Shared domain compiles independently

#### Phase 3: Storage Domain
- [ ] Superblock entity extracted
- [ ] Allocation entities extracted
- [ ] Storage services implemented
- [ ] Repository interfaces defined
- [ ] Storage domain compiles independently

#### Phase 4: Filesystem Core
- [ ] Inode entity extracted
- [ ] File entity extracted
- [ ] Directory entity extracted
- [ ] Permission entity extracted
- [ ] Filesystem core compiles independently

#### Phase 5: Vector Domain
- [ ] Vector entities created
- [ ] Existing vector code migrated
- [ ] Vector services implemented
- [ ] Vector domain compiles independently

#### Phase 6: Interface Domain
- [ ] VFS interface extracted
- [ ] FFI bindings extracted
- [ ] IOCTL interface extracted
- [ ] Interface domain compiles independently

#### Phase 7: Integration
- [ ] lib.rs updated with new structure
- [ ] Domain facades implemented
- [ ] Cross-domain communication working
- [ ] Full project compiles
- [ ] All tests pass
- [ ] FFI interface remains compatible

### Quality Assurance

#### Code Quality Metrics
- Each entity file ≤ 300 lines
- Cyclomatic complexity ≤ 10 per function
- Test coverage ≥ 80% per domain
- No circular dependencies between domains
- Clear separation of concerns

#### Functional Verification
- All existing functionality preserved
- Performance characteristics maintained
- Memory usage patterns unchanged
- Error handling behavior consistent

#### Architecture Validation
- Domain boundaries clearly defined
- Entity relationships properly modeled
- Cross-domain interfaces well-defined
- Future extensibility enabled

This implementation guide provides a concrete, step-by-step approach to executing the DDD refactoring while maintaining system integrity and ensuring successful completion of the architectural transformation.