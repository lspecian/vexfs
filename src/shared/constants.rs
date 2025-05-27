/*
 * VexFS - Vector Extended File System
 * Copyright 2025 VexFS Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * Note: Kernel module components are licensed under GPL v2.
 * See LICENSE.kernel for kernel-specific licensing terms.
 */

//! Constants for VexFS Shared Domain
//!
//! This module contains all shared constants used throughout the VexFS codebase.
//! Constants are organized by category: filesystem magic numbers, sizes, limits,
//! and configuration defaults.

// =======================
// Filesystem Magic Numbers
// =======================

/// VexFS filesystem magic number for superblock identification
pub const VEXFS_MAGIC: u32 = 0x56455846; // "VEXF" in ASCII

/// VexFS superblock magic number for validation
pub const VEXFS_SUPER_MAGIC: u64 = 0x5645584653555045; // "VEXFSUPE" in ASCII

/// VexFS inode magic number for validation
pub const VEXFS_INODE_MAGIC: u32 = 0x56455849; // "VEXI" in ASCII

/// VexFS directory entry magic number
pub const VEXFS_DIR_MAGIC: u32 = 0x56455844; // "VEXD" in ASCII

/// VexFS vector header magic number
pub const VEXFS_VECTOR_MAGIC: u32 = 0x56455856; // "VEXV" in ASCII

/// VexFS journal magic number
pub const VEXFS_JOURNAL_MAGIC: u32 = 0x5645584A; // "VEXJ" in ASCII

// =======================
// Block and Size Constants
// =======================

/// Default block size (4KB)
pub const VEXFS_DEFAULT_BLOCK_SIZE: usize = 4096;

/// Block size alias for compatibility
pub const VEXFS_BLOCK_SIZE: usize = VEXFS_DEFAULT_BLOCK_SIZE;

/// Minimum block size (1KB)
pub const VEXFS_MIN_BLOCK_SIZE: usize = 1024;

/// Maximum block size (64KB)
pub const VEXFS_MAX_BLOCK_SIZE: usize = 65536;

/// Default block group size (128MB)
pub const VEXFS_DEFAULT_BLOCK_GROUP_SIZE: u64 = 128 * 1024 * 1024;

/// Default inode size in bytes
pub const VEXFS_INODE_SIZE: usize = 256;

/// Maximum filename length
pub const VEXFS_MAX_NAME_LEN: usize = 255;
pub const VEXFS_MAX_FILENAME_LEN: usize = 255;
pub const VEXFS_MAX_NAME_LENGTH: usize = 255;

// Write buffer constants
pub const VEXFS_MAX_WRITE_BUFFERS: usize = 16;

// Inode size constants
pub const VEXFS_MIN_INODE_SIZE: u32 = 128;
pub const VEXFS_MAX_INODE_SIZE: u32 = 1024;
pub const VEXFS_DEFAULT_INODE_SIZE: u32 = 256;

// Mount count constants
pub const VEXFS_DEFAULT_MAX_MOUNT_COUNT: u32 = 100;

// Journal block constants
pub const VEXFS_DEFAULT_JOURNAL_BLOCKS: u32 = 1024;

/// Maximum path length
pub const VEXFS_MAX_PATH_LEN: usize = 4096;

/// Maximum path length alias for compatibility
pub const VEXFS_MAX_PATH_LENGTH: usize = VEXFS_MAX_PATH_LEN;

/// Directory entry size
pub const VEXFS_DIRENT_SIZE: usize = 264; // name(255) + inode(8) + type(1)

/// Superblock size
pub const VEXFS_SUPERBLOCK_SIZE: usize = 1024;

// =======================
// Filesystem Limits
// =======================

/// Maximum number of inodes per filesystem
pub const VEXFS_MAX_INODES: u64 = 1 << 32; // 4 billion

/// Maximum file size (1TB)
pub const VEXFS_MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024 * 1024;

/// Maximum number of blocks per file
pub const VEXFS_MAX_BLOCKS_PER_FILE: u32 = 16 * 1024 * 1024; // 16M blocks

/// Maximum number of direct blocks in inode
pub const VEXFS_DIRECT_BLOCKS: usize = 12;

/// Number of direct block pointers in inode
pub const VEXFS_INODE_DIRECT_BLOCKS: usize = 12;

/// Number of indirect block pointers
pub const VEXFS_INDIRECT_BLOCKS: usize = 3; // single, double, triple

/// Maximum symlink length
pub const VEXFS_MAX_SYMLINK_LEN: usize = 1024;

/// Maximum path traversal depth
pub const VEXFS_MAX_PATH_DEPTH: usize = 64;

/// Root inode number (alternative name for VEXFS_ROOT_INO)
pub const VEXFS_ROOT_INODE: u64 = VEXFS_ROOT_INO;

// File operation flags (POSIX compatibility)
pub const O_RDONLY: i32 = 0;
pub const O_WRONLY: i32 = 1;
pub const O_RDWR: i32 = 2;
pub const O_CREAT: i32 = 64;
pub const O_EXCL: i32 = 128;
pub const O_TRUNC: i32 = 512;
pub const O_APPEND: i32 = 1024;

/// Maximum number of hard links per file
pub const VEXFS_MAX_LINKS: u32 = 65000;

/// Maximum number of open files
pub const VEXFS_MAX_OPEN_FILES: usize = 1024;

// =======================
// Vector Storage Constants
// =======================

/// Maximum vector dimensions
pub const VEXFS_MAX_VECTOR_DIMS: usize = 4096;

/// Default vector dimensions
pub const VEXFS_DEFAULT_VECTOR_DIMS: usize = 512;

/// Maximum number of vectors per file
pub const VEXFS_MAX_VECTORS_PER_FILE: usize = 16;

/// Vector alignment requirement (16 bytes for SIMD)
pub const VEXFS_VECTOR_ALIGNMENT: usize = 16;

/// Default alignment for general use (alias for compatibility)
pub const VEXFS_DEFAULT_ALIGNMENT: usize = 16;

/// Vector header size
pub const VEXFS_VECTOR_HEADER_SIZE: usize = 64;

/// Maximum vector metadata size
pub const VEXFS_MAX_VECTOR_METADATA_SIZE: usize = 1024;

// =======================
// Index and Search Constants
// =======================

/// Default K for KNN search
pub const VEXFS_DEFAULT_K: usize = 10;

/// Maximum K for KNN search
pub const VEXFS_MAX_K: usize = 1000;

/// Default ef parameter for HNSW
pub const VEXFS_DEFAULT_EF: usize = 200;

/// Maximum ef parameter for HNSW
pub const VEXFS_MAX_EF: usize = 10000;

/// Default M parameter for HNSW (connections per node)
pub const VEXFS_DEFAULT_M: usize = 16;

/// Default HNSW M parameter (alias for compatibility)
pub const VEXFS_DEFAULT_HNSW_M: usize = 16;

/// Default HNSW M_L parameter (connections for layer 0)
pub const VEXFS_DEFAULT_HNSW_M_L: usize = 32;

/// Default HNSW ML parameter (level factor)
pub const VEXFS_DEFAULT_HNSW_ML: f64 = 1.0 / 0.6931471805599453; // 1.0 / ln(2.0)

/// Default EF construction parameter for HNSW
pub const VEXFS_DEFAULT_EF_CONSTRUCTION: usize = 200;

/// Default random seed for HNSW
pub const VEXFS_DEFAULT_RANDOM_SEED: u64 = 42;

/// Maximum HNSW M parameter
pub const VEXFS_MAX_HNSW_M: usize = 64;

/// Maximum HNSW M_L parameter
pub const VEXFS_MAX_HNSW_M_L: usize = 128;

/// Maximum EF construction parameter
pub const VEXFS_MAX_EF_CONSTRUCTION: usize = 1000;

/// Default cache line size
pub const VEXFS_DEFAULT_CACHE_LINE_SIZE: usize = 64;

/// Default IO queue depth
pub const VEXFS_DEFAULT_IO_QUEUE_DEPTH: usize = 32;

/// Default readahead size
pub const VEXFS_DEFAULT_READAHEAD_SIZE: usize = 128 * 1024;

/// Default max batch size
pub const VEXFS_DEFAULT_MAX_BATCH_SIZE: usize = 64;

/// Default IO timeout in milliseconds
pub const VEXFS_DEFAULT_IO_TIMEOUT_MS: u64 = 30000;

/// Default IO threads
pub const VEXFS_DEFAULT_IO_THREADS: usize = 4;

/// Maximum M parameter for HNSW
pub const VEXFS_MAX_M: usize = 256;

/// Default maximum layer for HNSW
pub const VEXFS_DEFAULT_MAX_LAYER: usize = 16;

/// Search result cache size
pub const VEXFS_SEARCH_CACHE_SIZE: usize = 1024;

// =======================
// Memory and Buffer Constants
// =======================

/// Default page cache size (16MB)
pub const VEXFS_DEFAULT_PAGE_CACHE_SIZE: usize = 16 * 1024 * 1024;

/// Minimum memory pool size (1MB)
pub const VEXFS_MIN_MEMORY_POOL_SIZE: usize = 1024 * 1024;

/// Default memory pool size (64MB)
pub const VEXFS_DEFAULT_MEMORY_POOL_SIZE: usize = 64 * 1024 * 1024;

/// Buffer alignment for DMA (4KB)
pub const VEXFS_BUFFER_ALIGNMENT: usize = 4096;

/// Default I/O buffer size
pub const VEXFS_DEFAULT_IO_BUFFER_SIZE: usize = 64 * 1024;

/// Maximum I/O request size (1MB)
pub const VEXFS_MAX_IO_SIZE: usize = 1024 * 1024;

// =======================
// Journal and Transaction Constants
// =======================

/// Default journal size (64MB)
pub const VEXFS_DEFAULT_JOURNAL_SIZE: u64 = 64 * 1024 * 1024;

/// Minimum journal size (1MB)
pub const VEXFS_MIN_JOURNAL_SIZE: u64 = 1024 * 1024;

/// Maximum journal size (1GB)
pub const VEXFS_MAX_JOURNAL_SIZE: u64 = 1024 * 1024 * 1024;

/// Maximum transaction size
pub const VEXFS_MAX_TRANSACTION_SIZE: usize = 1024 * 1024; // 1MB

/// Default transaction size
pub const VEXFS_DEFAULT_MAX_TRANSACTION_SIZE: usize = 512 * 1024; // 512KB

/// Default journal commit interval (milliseconds)
pub const VEXFS_DEFAULT_JOURNAL_COMMIT_INTERVAL: u64 = 5000; // 5 seconds

/// Journal record header size
pub const VEXFS_JOURNAL_RECORD_HEADER_SIZE: usize = 32;

/// Maximum number of concurrent transactions
pub const VEXFS_MAX_CONCURRENT_TRANSACTIONS: usize = 256;

/// Maximum operations per transaction
pub const VEXFS_MAX_TRANSACTION_OPS: usize = 64;

/// Transaction buffer size
pub const VEXFS_TRANSACTION_BUFFER_SIZE: usize = 64 * 1024; // 64KB

/// Journal version
pub const VEXFS_JOURNAL_VERSION: u32 = 1;
pub const VEXFS_JOURNAL_BUFFER_SIZE: usize = 4096;

/// Journal record types
pub const JOURNAL_RECORD_COMMIT: u32 = 1;
pub const JOURNAL_RECORD_OPERATION: u32 = 2;

/// Journal states
pub const JOURNAL_STATE_CLEAN: u32 = 0;

// =======================
// Cache and Performance Constants
// =======================

/// Default metadata cache size (8MB)
pub const VEXFS_DEFAULT_METADATA_CACHE_SIZE: usize = 8 * 1024 * 1024;

/// Default inode cache size (entries)
pub const VEXFS_DEFAULT_INODE_CACHE_SIZE: usize = 10000;

/// Default directory cache size (entries)
pub const VEXFS_DEFAULT_DIR_CACHE_SIZE: usize = 5000;

/// Default vector cache size
pub const VEXFS_DEFAULT_VECTOR_CACHE_SIZE: usize = 5000;

/// Cache line size for optimization
pub const VEXFS_CACHE_LINE_SIZE: usize = 64;

/// Default prefetch size
pub const VEXFS_DEFAULT_PREFETCH_SIZE: usize = 8;

// =======================
// Error and Retry Constants
// =======================

/// Maximum retry attempts for I/O operations
pub const VEXFS_MAX_IO_RETRIES: u32 = 3;

/// Default timeout for operations (seconds)
pub const VEXFS_DEFAULT_TIMEOUT_SECS: u32 = 30;

/// Maximum error log entries
pub const VEXFS_MAX_ERROR_LOG_ENTRIES: usize = 1000;

// =======================
// Memory Pool Size Constants
// =======================

/// Default small memory pool size
pub const VEXFS_DEFAULT_SMALL_POOL_SIZE: usize = 16 * 1024; // 16KB

/// Default medium memory pool size
pub const VEXFS_DEFAULT_MEDIUM_POOL_SIZE: usize = 64 * 1024; // 64KB

/// Default large memory pool size
pub const VEXFS_DEFAULT_LARGE_POOL_SIZE: usize = 256 * 1024; // 256KB

/// Default memory pressure threshold
pub const VEXFS_DEFAULT_MEMORY_PRESSURE_THRESHOLD: f32 = 0.8; // 80%

/// Default debug buffer size
pub const VEXFS_DEFAULT_DEBUG_BUFFER_SIZE: usize = 32 * 1024; // 32KB

/// Vector dimensions alias (fixing name mismatch)
pub const VEXFS_MAX_VECTOR_DIMENSIONS: usize = VEXFS_MAX_VECTOR_DIMS;

// =======================
// Versioning Constants
// =======================

/// VexFS version major
pub const VEXFS_VERSION_MAJOR: u16 = 0;

/// VexFS version minor
pub const VEXFS_VERSION_MINOR: u16 = 1;

/// VexFS version patch
pub const VEXFS_VERSION_PATCH: u16 = 0;

/// VexFS version string
pub const VEXFS_VERSION_STRING: &str = "0.1.0";

/// Minimum supported version for compatibility
pub const VEXFS_MIN_SUPPORTED_VERSION: u32 = 0x00010000; // 0.1.0

/// Current on-disk format version
pub const VEXFS_DISK_FORMAT_VERSION: u32 = 1;

// =======================
// Checksum and Validation Constants
// =======================

/// CRC32 polynomial for checksums
pub const VEXFS_CRC32_POLYNOMIAL: u32 = 0xEDB88320;

/// Checksum seed value
pub const VEXFS_CHECKSUM_SEED: u32 = 0x5645584E; // "VEXN" in ASCII

/// Validation magic for data integrity
pub const VEXFS_VALIDATION_MAGIC: u64 = 0x5645584656414C49; // "VEXFVALI" in ASCII

// =======================
// Additional Missing Constants
// =======================

/// Default EF search parameter
pub const VEXFS_DEFAULT_EF_SEARCH: usize = 200;

/// Default index cache size
pub const VEXFS_DEFAULT_INDEX_CACHE_SIZE: usize = 32 * 1024 * 1024; // 32MB

/// Default page size
pub const VEXFS_DEFAULT_PAGE_SIZE: usize = 4096;

// =======================
// Test and Development Constants
// =======================

/// Test vector dimensions
pub const VEXFS_TEST_VECTOR_DIMS: u32 = 128;

/// Test data size
pub const VEXFS_TEST_DATA_SIZE: usize = 1024;

// =======================
// Special Inode Numbers
// =======================

/// Root directory inode number
pub const VEXFS_ROOT_INO: u64 = 2;

/// Invalid inode number
pub const VEXFS_INVALID_INO: u64 = 0;

/// First user inode number
pub const VEXFS_FIRST_USER_INO: u64 = 11;

/// Journal inode number
pub const VEXFS_JOURNAL_INO: u64 = 8;

/// Metadata inode number
pub const VEXFS_METADATA_INO: u64 = 9;

/// Vector index inode number
pub const VEXFS_VECTOR_INDEX_INO: u64 = 10;

// Vector version constants
pub const VEXFS_VECTOR_VERSION: u32 = 1;

// Vector feature flags
pub const VEXFS_VECTOR_FEATURE_HNSW: u32 = 0x01;

// Filesystem size constraints
pub const VEXFS_MIN_FILESYSTEM_BLOCKS: u64 = 1024;

// MAX_FILENAME_LEN alias for compatibility
pub const MAX_FILENAME_LEN: usize = VEXFS_MAX_NAME_LEN;

// =======================
// Permission and Mode Constants
// =======================

/// Default file permissions (644)
pub const VEXFS_DEFAULT_FILE_MODE: u32 = 0o644;

/// Default directory permissions (755)
pub const VEXFS_DEFAULT_DIR_MODE: u32 = 0o755;

/// Maximum permission value
pub const VEXFS_MAX_MODE: u32 = 0o777;

// Standard POSIX file type constants
pub const S_IFMT: u32 = 0o170000;   // File type mask
pub const S_IFREG: u32 = 0o100000;  // Regular file
pub const S_IFDIR: u32 = 0o040000;  // Directory
pub const S_IFLNK: u32 = 0o120000;  // Symbolic link
pub const S_IFBLK: u32 = 0o060000;  // Block device
pub const S_IFCHR: u32 = 0o020000;  // Character device
pub const S_IFIFO: u32 = 0o010000;  // FIFO/pipe
pub const S_IFSOCK: u32 = 0o140000; // Socket

// Permission bits
pub const S_IRWXU: u32 = 0o700;     // User read, write, execute
pub const S_IRUSR: u32 = 0o400;     // User read
pub const S_IWUSR: u32 = 0o200;     // User write
pub const S_IXUSR: u32 = 0o100;     // User execute

pub const S_IRWXG: u32 = 0o070;     // Group read, write, execute
pub const S_IRGRP: u32 = 0o040;     // Group read
pub const S_IWGRP: u32 = 0o020;     // Group write
pub const S_IXGRP: u32 = 0o010;     // Group execute

pub const S_IRWXO: u32 = 0o007;     // Other read, write, execute
pub const S_IROTH: u32 = 0o004;     // Other read
pub const S_IWOTH: u32 = 0o002;     // Other write
pub const S_IXOTH: u32 = 0o001;     // Other execute

// Special permission bits
pub const S_ISUID: u32 = 0o4000;    // Set user ID
pub const S_ISGID: u32 = 0o2000;    // Set group ID
pub const S_ISVTX: u32 = 0o1000;    // Sticky bit

// =======================
// Time Constants
// =======================

/// Timestamp precision (nanoseconds)
pub const VEXFS_TIMESTAMP_PRECISION: u64 = 1_000_000_000;

/// Default atime update threshold (seconds)
pub const VEXFS_ATIME_UPDATE_THRESHOLD: u64 = 3600;

// =======================
// Feature Flags
// =======================

/// Feature flag: Vector indexing enabled
pub const VEXFS_FEATURE_VECTOR_INDEX: u64 = 1 << 0;

/// Feature flag: Compression enabled
pub const VEXFS_FEATURE_COMPRESSION: u64 = 1 << 1;

/// Feature flag: Encryption enabled
pub const VEXFS_FEATURE_ENCRYPTION: u64 = 1 << 2;

/// Feature flag: Journaling enabled
pub const VEXFS_FEATURE_JOURNAL: u64 = 1 << 3;

/// Feature flag: Extended attributes enabled
pub const VEXFS_FEATURE_XATTR: u64 = 1 << 4;

/// All supported features
pub const VEXFS_SUPPORTED_FEATURES: u64 = VEXFS_FEATURE_VECTOR_INDEX 
    | VEXFS_FEATURE_COMPRESSION 
    | VEXFS_FEATURE_ENCRYPTION 
    | VEXFS_FEATURE_JOURNAL 
    | VEXFS_FEATURE_XATTR;

/// Required features for mounting
pub const VEXFS_REQUIRED_FEATURES: u64 = VEXFS_FEATURE_VECTOR_INDEX;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_numbers() {
        assert_eq!(VEXFS_MAGIC, 0x56455846);
        assert_eq!(VEXFS_SUPER_MAGIC, 0x5645584653555045);
        assert_eq!(VEXFS_INODE_MAGIC, 0x56455849);
    }

    #[test]
    fn test_size_constraints() {
        assert!(VEXFS_MIN_BLOCK_SIZE <= VEXFS_DEFAULT_BLOCK_SIZE);
        assert!(VEXFS_DEFAULT_BLOCK_SIZE <= VEXFS_MAX_BLOCK_SIZE);
        assert!(VEXFS_MAX_NAME_LEN < VEXFS_MAX_PATH_LEN);
    }

    #[test]
    fn test_vector_constraints() {
        assert!(VEXFS_DEFAULT_VECTOR_DIMS <= VEXFS_MAX_VECTOR_DIMS);
        assert!(VEXFS_DEFAULT_K <= VEXFS_MAX_K);
        assert!(VEXFS_DEFAULT_EF <= VEXFS_MAX_EF);
    }

    #[test]
    fn test_feature_flags() {
        assert!(VEXFS_REQUIRED_FEATURES & VEXFS_SUPPORTED_FEATURES == VEXFS_REQUIRED_FEATURES);
    }

    #[test]
    fn test_version_format() {
        assert_eq!(VEXFS_VERSION_STRING, "0.1.0");
        let version = (VEXFS_VERSION_MAJOR as u32) << 16 
            | (VEXFS_VERSION_MINOR as u32) << 8 
            | (VEXFS_VERSION_PATCH as u32);
        assert_eq!(version, VEXFS_MIN_SUPPORTED_VERSION);
    }
}
// =======================
// Environment and Configuration Constants
// =======================

/// Environment variable for enabling debug mode
pub const ENV_VEXFS_DEBUG: &str = "VEXFS_DEBUG";

/// Environment variable for log level
pub const ENV_VEXFS_LOG_LEVEL: &str = "VEXFS_LOG_LEVEL";

/// Environment variable for cache size
pub const ENV_VEXFS_CACHE_SIZE: &str = "VEXFS_CACHE_SIZE";

/// Environment variable for block device path
pub const ENV_VEXFS_DEVICE: &str = "VEXFS_DEVICE";

// =======================
// File Type Constants (for persistence module)
// =======================

/// Unknown file type
pub const VEXFS_FT_UNKNOWN: u8 = 0;

/// Regular file
pub const VEXFS_FT_REG_FILE: u8 = 1;

/// Directory
pub const VEXFS_FT_DIR: u8 = 2;

/// Character device
pub const VEXFS_FT_CHRDEV: u8 = 3;

/// Block device
pub const VEXFS_FT_BLKDEV: u8 = 4;

/// FIFO
pub const VEXFS_FT_FIFO: u8 = 5;

/// Socket
pub const VEXFS_FT_SOCK: u8 = 6;

/// Symbolic link
pub const VEXFS_FT_SYMLINK: u8 = 7;

// =======================
// Filesystem State Constants (for superblock module)
// =======================

/// Valid filesystem state
pub const VEXFS_VALID_FS: u16 = 1;

/// Error filesystem state
pub const VEXFS_ERROR_FS: u16 = 2;

/// Continue on errors
pub const VEXFS_ERRORS_CONTINUE: u16 = 1;

/// Remount read-only on errors
pub const VEXFS_ERRORS_RO: u16 = 2;

/// Panic on errors
pub const VEXFS_ERRORS_PANIC: u16 = 3;

// =======================
// Feature Constants (for superblock module)
// =======================

/// Compatible feature: directory index
pub const VEXFS_FEATURE_COMPAT_DIR_INDEX: u32 = 1 << 0;

/// Compatible feature: resize inode
pub const VEXFS_FEATURE_COMPAT_RESIZE_INODE: u32 = 1 << 1;

/// Compatible feature: journal
pub const VEXFS_FEATURE_COMPAT_JOURNAL: u32 = 1 << 2;

/// Incompatible feature: compression
pub const VEXFS_FEATURE_INCOMPAT_COMPRESSION: u32 = 1 << 0;

/// Incompatible feature: file type
pub const VEXFS_FEATURE_INCOMPAT_FILETYPE: u32 = 1 << 1;

/// Incompatible feature: 64-bit
pub const VEXFS_FEATURE_INCOMPAT_64BIT: u32 = 1 << 2;

/// Incompatible feature: extents
pub const VEXFS_FEATURE_INCOMPAT_EXTENTS: u32 = 1 << 3;

/// Read-only compatible feature: sparse super
pub const VEXFS_FEATURE_RO_COMPAT_SPARSE_SUPER: u32 = 1 << 0;

/// Read-only compatible feature: large file
pub const VEXFS_FEATURE_RO_COMPAT_LARGE_FILE: u32 = 1 << 1;

/// Read-only compatible feature: btree directory
pub const VEXFS_FEATURE_RO_COMPAT_BTREE_DIR: u32 = 1 << 2;

// =======================
// Active Transactions Constants
// =======================

/// Maximum number of active transactions
pub const VEXFS_MAX_ACTIVE_TRANSACTIONS: usize = 64;

// =======================
// Cache Constants
// =======================

/// Maximum number of blocks to cache
pub const VEXFS_MAX_CACHED_BLOCKS: usize = 1024;

// =======================
// Journal Constants
// =======================

/// Magic number for journal records
pub const VEXFS_JOURNAL_RECORD_MAGIC: u32 = 0x4A524E4C; // "JRNL"

/// Special file type (device, socket, etc.)
pub const VEXFS_FT_SPECIAL: u8 = 8;

// =======================
// Additional Compatibility Constants
// =======================

/// Compatibility alias for path length
pub const MAX_PATH_LENGTH: usize = VEXFS_MAX_PATH_LEN;

/// Compatibility alias for filename length
pub const MAX_FILENAME_LENGTH: usize = VEXFS_MAX_NAME_LEN;

/// Compatibility alias for max block number
pub const VEXFS_MAX_BLOCK_NUM: u64 = 1 << 48; // Maximum block number

/// Compatibility alias for max inode number
pub const VEXFS_MAX_INODE_NUM: u64 = VEXFS_MAX_INODES;
