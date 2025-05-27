# VexFS Entity Extraction Implementation Plan

## Overview

This document provides the detailed implementation plan for extracting entities from the monolithic VexFS files into domain-specific modules. Each entity is designed to be 200-300 lines maximum for optimal LLM processing.

## Entity Extraction Mapping

### From file_ops.rs (1,388 lines) → Multiple Domains

#### → Shared Domain
**VexfsSpinLock** (Lines 62-87)
- **Target**: `shared/utils/locking.rs`
- **Size**: ~25 lines
- **Responsibility**: Basic spinlock implementation for no_std environment

**VexfsInodeLock** (Lines 89-141)
- **Target**: `shared/utils/locking.rs`
- **Size**: ~52 lines
- **Responsibility**: Per-inode read-write lock implementation

**Permission checking logic** (Lines 260-285)
- **Target**: `fs_core/entities/permission.rs`
- **Size**: ~25 lines + additional permission logic
- **Responsibility**: Access control and permission validation

**File type constants** (Lines 41-58, 250-295)
- **Target**: `shared/types/constants.rs`
- **Size**: ~50 lines
- **Responsibility**: File type and permission constants

#### → Filesystem Core Domain
**VexfsFileHandle** (Lines 237-247)
- **Target**: `fs_core/entities/file.rs`
- **Size**: ~100-150 lines (expanded with file operations)
- **Responsibility**: File handle management and operations

**VexfsContext** (Lines 144-234)
- **Target**: `fs_core/services/filesystem_service.rs`
- **Size**: ~200-250 lines
- **Responsibility**: Filesystem context and coordination

#### → Storage Domain
**Space allocation logic** (Referenced via VexfsSpaceAllocator)
- **Target**: `storage/services/alloc_service.rs`
- **Size**: ~200-250 lines
- **Responsibility**: Block allocation and deallocation

**Journal operations** (Referenced via VexfsJournal)
- **Target**: `storage/services/journal_service.rs`
- **Size**: ~200-250 lines
- **Responsibility**: Transaction logging and recovery

### From dir_ops.rs (1,492 lines) → Multiple Domains

#### → Filesystem Core Domain
**VexfsDirHandle** (Lines 44-73)
- **Target**: `fs_core/entities/directory.rs`
- **Size**: ~150-200 lines (expanded with directory operations)
- **Responsibility**: Directory handle management and operations

**VexfsDirOps** (Lines 75-91)
- **Target**: `fs_core/services/dir_service.rs`
- **Size**: ~200-250 lines
- **Responsibility**: Directory operation coordination

**DirEntryIterator** (Lines 93-100+)
- **Target**: `fs_core/entities/directory.rs`
- **Size**: ~100-150 lines
- **Responsibility**: Directory entry iteration

#### → Shared Domain
**DirOpError** (Lines 27-42)
- **Target**: `shared/types/errors.rs`
- **Size**: ~20 lines + additional error types
- **Responsibility**: Directory operation error definitions

**Directory locking constants** (Lines 23-25)
- **Target**: `shared/types/constants.rs`
- **Size**: ~10 lines
- **Responsibility**: Directory operation constants

### From ondisk.rs (1,120 lines) → Multiple Domains

#### → Storage Domain
**VexfsSuperblock** (Defined in ondisk.rs)
- **Target**: `storage/entities/superblock.rs`
- **Size**: ~100-150 lines
- **Responsibility**: Filesystem metadata and configuration

**VexfsGroupDesc** (Defined in ondisk.rs)
- **Target**: `storage/entities/allocation.rs`
- **Size**: ~100-150 lines
- **Responsibility**: Block group allocation metadata

**VexfsDirEntry** (Defined in ondisk.rs)
- **Target**: `fs_core/entities/directory.rs`
- **Size**: ~50-100 lines
- **Responsibility**: Directory entry on-disk representation

#### → Shared Domain
**OnDiskSerialize trait** (Lines 14-22)
- **Target**: `shared/traits/common.rs`
- **Size**: ~20-30 lines
- **Responsibility**: Serialization interface for on-disk structures

**File type constants** (Lines 24-59)
- **Target**: `shared/types/constants.rs`
- **Size**: ~40 lines
- **Responsibility**: File system constants and limits

**Block and inode constants** (Lines 68-100)
- **Target**: `shared/types/constants.rs`
- **Size**: ~35 lines
- **Responsibility**: Filesystem layout constants

#### → Filesystem Core Domain
**VexfsInode** (Defined in ondisk.rs)
- **Target**: `fs_core/entities/inode.rs`
- **Size**: ~100-150 lines
- **Responsibility**: Inode metadata and operations

## Detailed Entity Specifications

### Shared Domain Entities

#### shared/types/constants.rs (~150 lines)
```rust
// File type constants from ondisk.rs
pub const S_IFMT: u16 = 0o170000;
pub const S_IFREG: u16 = 0o100000;
// ... (all file type constants)

// VexFS specific constants
pub const VEXFS_MAGIC: u64 = 0x5645584653_u64;
pub const VEXFS_VERSION_MAJOR: u16 = 1;
pub const VEXFS_VERSION_MINOR: u16 = 0;

// Block and inode constants
pub const VEXFS_MIN_BLOCK_SIZE: u32 = 4096;
pub const VEXFS_MAX_BLOCK_SIZE: u32 = 65536;
// ... (all block/inode constants)

// File operation constants from file_ops.rs
pub const VEXFS_O_APPEND: u32 = 0x400;
pub const VEXFS_O_TRUNC: u32 = 0x200;
// ... (all file operation constants)

// Permission constants
pub const VEXFS_S_IRUSR: u32 = 0o400;
pub const VEXFS_S_IWUSR: u32 = 0o200;
// ... (all permission constants)
```

#### shared/types/errors.rs (~100 lines)
```rust
// Common error types across domains
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VexfsError {
    // From ffi.rs
    Generic,
    InvalidArgument,
    NoMemory,
    NoSpace,
    Permission,
    NotFound,
    IoError,
    Exists,
    NotDirectory,
    IsDirectory,
    
    // From dir_ops.rs
    DirectoryNotEmpty,
    NameTooLong,
    InvalidName,
    CrossDevice,
    TooManyLinks,
}

// Result type for all domains
pub type VexfsResult<T> = Result<T, VexfsError>;
```

#### shared/utils/locking.rs (~200 lines)
```rust
// VexfsSpinLock from file_ops.rs
#[repr(C)]
pub struct VexfsSpinLock {
    locked: AtomicBool,
}

impl VexfsSpinLock {
    // Implementation from file_ops.rs lines 67-87
}

// VexfsInodeLock from file_ops.rs
#[repr(C)]
pub struct VexfsInodeLock {
    // Implementation from file_ops.rs lines 89-141
}

// Additional locking utilities for domain coordination
```

#### shared/traits/common.rs (~100 lines)
```rust
// OnDiskSerialize trait from ondisk.rs
pub trait OnDiskSerialize {
    fn to_bytes(&self) -> &[u8];
    fn from_bytes(data: &[u8]) -> Result<Self, &'static str> where Self: Sized;
    fn serialized_size() -> usize where Self: Sized;
}

// Common traits for domain entities
pub trait Entity {
    type Id;
    fn id(&self) -> Self::Id;
}

pub trait Repository<T: Entity> {
    fn save(&mut self, entity: T) -> VexfsResult<()>;
    fn load(&self, id: T::Id) -> VexfsResult<T>;
    fn delete(&mut self, id: T::Id) -> VexfsResult<()>;
}
```

### Filesystem Core Domain Entities

#### fs_core/entities/file.rs (~200 lines)
```rust
use crate::shared::types::{VexfsResult, VexfsError};
use crate::shared::utils::locking::VexfsInodeLock;

// VexfsFileHandle from file_ops.rs with expanded operations
#[repr(C)]
pub struct VexfsFile {
    pub handle: VexfsFileHandle,
    pub metadata: FileMetadata,
    pub lock: VexfsInodeLock,
}

#[repr(C)]
pub struct VexfsFileHandle {
    pub ino: u64,
    pub pos: u64,
    pub flags: u32,
    pub inode_handle: Option<VexfsInodeHandle>,
}

pub struct FileMetadata {
    pub size: u64,
    pub permissions: u32,
    pub created_time: u64,
    pub modified_time: u64,
    pub accessed_time: u64,
}

impl VexfsFile {
    pub fn new(ino: u64, flags: u32) -> Self { /* ... */ }
    pub fn read(&mut self, buf: &mut [u8], offset: u64) -> VexfsResult<usize> { /* ... */ }
    pub fn write(&mut self, buf: &[u8], offset: u64) -> VexfsResult<usize> { /* ... */ }
    pub fn truncate(&mut self, size: u64) -> VexfsResult<()> { /* ... */ }
    pub fn sync(&mut self) -> VexfsResult<()> { /* ... */ }
}
```

#### fs_core/entities/directory.rs (~200 lines)
```rust
use crate::shared::types::{VexfsResult, VexfsError};

// VexfsDirHandle from dir_ops.rs with expanded operations
#[derive(Debug)]
pub struct VexfsDirectory {
    pub handle: VexfsDirHandle,
    pub entries: DirectoryEntries,
}

#[derive(Debug)]
pub struct VexfsDirHandle {
    pub inode: VexfsInodeInfo,
    pub pos: u64,
    pub flags: u32,
    pub ref_count: u32,
    pub cached_entries: [VexfsDirEntry; VEXFS_DIR_ENTRIES_PER_BLOCK],
    pub cached_count: usize,
    pub current_block: u64,
    pub lock: AtomicU32,
    pub journal_handle: Option<JournalHandle>,
}

// DirEntryIterator from dir_ops.rs
pub struct DirEntryIterator {
    pub dir_handle: VexfsDirHandle,
    pub pos: u64,
}

impl VexfsDirectory {
    pub fn new(inode: VexfsInodeInfo) -> Self { /* ... */ }
    pub fn create_entry(&mut self, name: &str, inode: u64) -> VexfsResult<()> { /* ... */ }
    pub fn remove_entry(&mut self, name: &str) -> VexfsResult<()> { /* ... */ }
    pub fn lookup(&self, name: &str) -> VexfsResult<Option<VexfsDirEntry>> { /* ... */ }
    pub fn iter(&self) -> DirEntryIterator { /* ... */ }
}
```

#### fs_core/entities/inode.rs (~150 lines)
```rust
use crate::shared::types::{VexfsResult, VexfsError};
use crate::shared::traits::common::{Entity, OnDiskSerialize};

// VexfsInode from ondisk.rs with expanded operations
#[repr(C)]
pub struct VexfsInode {
    pub ino: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub created_time: u64,
    pub modified_time: u64,
    pub accessed_time: u64,
    pub links_count: u32,
    pub blocks_count: u32,
    pub direct_blocks: [u64; VEXFS_N_DIRECT],
    pub indirect_block: u64,
    pub double_indirect_block: u64,
    pub triple_indirect_block: u64,
}

impl Entity for VexfsInode {
    type Id = u64;
    fn id(&self) -> Self::Id { self.ino }
}

impl VexfsInode {
    pub fn new(ino: u64, mode: u32, uid: u32, gid: u32) -> Self { /* ... */ }
    pub fn is_file(&self) -> bool { /* ... */ }
    pub fn is_directory(&self) -> bool { /* ... */ }
    pub fn is_symlink(&self) -> bool { /* ... */ }
    pub fn get_block(&self, block_index: u64) -> VexfsResult<u64> { /* ... */ }
    pub fn allocate_block(&mut self, block: u64) -> VexfsResult<()> { /* ... */ }
}

impl OnDiskSerialize for VexfsInode {
    // Implementation for serialization
}
```

#### fs_core/entities/permission.rs (~100 lines)
```rust
use crate::shared::types::{VexfsResult, VexfsError};

pub struct Permission {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

impl Permission {
    pub fn new(mode: u32, uid: u32, gid: u32) -> Self { /* ... */ }
    
    // check_permission function from file_ops.rs
    pub fn check_access(&self, requested_mode: u32, current_uid: u32, current_gid: u32) -> bool {
        // Root can access everything
        if current_uid == 0 {
            return true;
        }

        // Check owner permissions
        if current_uid == self.uid {
            let owner_perms = (self.mode >> 6) & 0o7;
            return (owner_perms & requested_mode) == requested_mode;
        }
        
        // Check group permissions
        if current_gid == self.gid {
            let group_perms = (self.mode >> 3) & 0o7;
            return (group_perms & requested_mode) == requested_mode;
        }
        
        // Check other permissions
        let other_perms = self.mode & 0o7;
        (other_perms & requested_mode) == requested_mode
    }
}
```

### Storage Domain Entities

#### storage/entities/superblock.rs (~150 lines)
```rust
use crate::shared::types::{VexfsResult, VexfsError};
use crate::shared::traits::common::{Entity, OnDiskSerialize};

// VexfsSuperblock from ondisk.rs
#[repr(C)]
pub struct VexfsSuperblock {
    pub magic: u64,
    pub version_major: u16,
    pub version_minor: u16,
    pub block_size: u32,
    pub blocks_count: u64,
    pub free_blocks_count: u64,
    pub inodes_count: u64,
    pub free_inodes_count: u64,
    pub first_data_block: u64,
    pub inode_table_block: u64,
    pub block_bitmap_block: u64,
    pub inode_bitmap_block: u64,
    pub root_inode: u64,
    pub max_filename_len: u32,
    pub created_time: u64,
    pub modified_time: u64,
    pub mount_count: u32,
    pub max_mount_count: u32,
    pub state: u16,
    pub errors: u16,
    pub minor_rev_level: u16,
    pub lastcheck: u64,
    pub checkinterval: u64,
    pub creator_os: u32,
    pub rev_level: u32,
    pub def_resuid: u16,
    pub def_resgid: u16,
}

impl Entity for VexfsSuperblock {
    type Id = u64;
    fn id(&self) -> Self::Id { self.magic }
}

impl VexfsSuperblock {
    pub fn new(block_size: u32, blocks_count: u64, inodes_count: u64) -> Self { /* ... */ }
    pub fn is_valid(&self) -> bool { /* ... */ }
    pub fn allocate_block(&mut self) -> VexfsResult<u64> { /* ... */ }
    pub fn deallocate_block(&mut self, block: u64) -> VexfsResult<()> { /* ... */ }
}

impl OnDiskSerialize for VexfsSuperblock {
    // Implementation for serialization
}
```

#### storage/entities/allocation.rs (~150 lines)
```rust
use crate::shared::types::{VexfsResult, VexfsError};

// VexfsGroupDesc from ondisk.rs
#[repr(C)]
pub struct VexfsGroupDesc {
    pub bg_block_bitmap: u64,
    pub bg_inode_bitmap: u64,
    pub bg_inode_table: u64,
    pub bg_free_blocks_count: u32,
    pub bg_free_inodes_count: u32,
    pub bg_used_dirs_count: u32,
    pub bg_pad: u16,
    pub bg_reserved: [u32; 3],
}

pub struct BlockAllocation {
    pub block_id: u64,
    pub size: u32,
    pub allocated: bool,
    pub group: u32,
}

pub struct InodeAllocation {
    pub inode_id: u64,
    pub allocated: bool,
    pub group: u32,
}

impl VexfsGroupDesc {
    pub fn new(
        block_bitmap: u64,
        inode_bitmap: u64,
        inode_table: u64,
        free_blocks: u32,
        free_inodes: u32,
    ) -> Self { /* ... */ }
    
    pub fn allocate_block(&mut self) -> VexfsResult<u64> { /* ... */ }
    pub fn deallocate_block(&mut self, block: u64) -> VexfsResult<()> { /* ... */ }
    pub fn allocate_inode(&mut self) -> VexfsResult<u64> { /* ... */ }
    pub fn deallocate_inode(&mut self, inode: u64) -> VexfsResult<()> { /* ... */ }
}
```

### Vector Domain Entities

#### vector_domain/entities/vector.rs (~150 lines)
```rust
use crate::shared::types::{VexfsResult, VexfsError};

pub struct Vector {
    pub id: u64,
    pub data: Vec<f32>,
    pub dimensions: usize,
    pub metadata: VectorMetadata,
}

pub struct VectorMetadata {
    pub file_inode: u64,
    pub offset: u64,
    pub created_time: u64,
    pub data_type: VectorDataType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VectorDataType {
    Float32,
    Float64,
    Int8,
    Int16,
    Int32,
}

impl Vector {
    pub fn new(id: u64, data: Vec<f32>, file_inode: u64, offset: u64) -> Self { /* ... */ }
    pub fn distance_to(&self, other: &Vector, metric: DistanceMetric) -> f32 { /* ... */ }
    pub fn normalize(&mut self) { /* ... */ }
    pub fn magnitude(&self) -> f32 { /* ... */ }
}
```

## Implementation Priority

### Phase 1: Shared Domain (Week 1)
1. `shared/types/constants.rs` - Extract all constants
2. `shared/types/errors.rs` - Unify error handling
3. `shared/traits/common.rs` - Define common interfaces
4. `shared/utils/locking.rs` - Extract locking primitives

### Phase 2: Storage Domain (Week 1-2)
1. `storage/entities/superblock.rs` - Core filesystem metadata
2. `storage/entities/allocation.rs` - Space allocation tracking
3. `storage/entities/block.rs` - Block management
4. `storage/entities/journal.rs` - Transaction logging

### Phase 3: Filesystem Core Domain (Week 2-3)
1. `fs_core/entities/inode.rs` - Core filesystem entities
2. `fs_core/entities/permission.rs` - Access control
3. `fs_core/entities/file.rs` - File operations
4. `fs_core/entities/directory.rs` - Directory operations

### Phase 4: Vector Domain (Week 3-4)
1. `vector_domain/entities/vector.rs` - Vector representations
2. `vector_domain/entities/index.rs` - Vector indexing
3. `vector_domain/entities/query.rs` - Search queries
4. `vector_domain/entities/result.rs` - Search results

### Phase 5: Interface Domain (Week 4)
1. `interfaces/entities/vfs_operation.rs` - VFS integration
2. `interfaces/entities/ffi_binding.rs` - C FFI bindings
3. `interfaces/entities/ioctl_command.rs` - IOCTL operations

## Validation Criteria

### Code Quality
- Each entity file ≤ 300 lines
- Clear single responsibility
- No circular dependencies between domains
- Proper error handling using shared error types

### Functionality
- All existing functionality preserved
- Compilation succeeds after each phase
- Integration tests pass
- FFI interface remains compatible

### Architecture
- Clear domain boundaries maintained
- Entity relationships properly modeled
- Cross-domain communication through well-defined interfaces
- Future extensibility enabled

This entity extraction plan provides a concrete roadmap for implementing the DDD refactoring while maintaining system integrity and development velocity.