//! On-disk format and layout definitions for VexFS
//!
//! This module defines the physical structure of the VexFS filesystem,
//! including the superblock, inode tables, data blocks, and free space tracking.
//!
//! The VexFS on-disk format is designed for optimal performance with vector storage
//! capabilities while maintaining compatibility with standard POSIX filesystem semantics.


// Basic type definitions for kernel module compatibility
pub type c_int = i32;
pub type umode_t = u16;

/// Serialization trait for on-disk structures
pub trait OnDiskSerialize {
    /// Serialize structure to bytes
    fn to_bytes(&self) -> &[u8];
    /// Deserialize structure from bytes
    fn from_bytes(data: &[u8]) -> Result<Self, &'static str> where Self: Sized;
    /// Get the size of the serialized structure
    fn serialized_size() -> usize where Self: Sized;
}

// File type constants (from Linux kernel)
// In userspace, define our own constants; in kernel space, import from kernel headers
#[cfg(not(feature = "kernel"))]
pub const S_IFMT: u16 = 0o170000;
#[cfg(not(feature = "kernel"))]
pub const S_IFREG: u16 = 0o100000;
#[cfg(not(feature = "kernel"))]
pub const S_IFDIR: u16 = 0o040000;
#[cfg(not(feature = "kernel"))]
pub const S_IFCHR: u16 = 0o020000;
#[cfg(not(feature = "kernel"))]
pub const S_IFBLK: u16 = 0o060000;
#[cfg(not(feature = "kernel"))]
pub const S_IFIFO: u16 = 0o010000;
#[cfg(not(feature = "kernel"))]
pub const S_IFSOCK: u16 = 0o140000;
#[cfg(not(feature = "kernel"))]
pub const S_IFLNK: u16 = 0o120000;

// In kernel space, define our own constants matching Linux kernel values
#[cfg(feature = "kernel")]
pub const S_IFMT: u16 = 0o170000;
#[cfg(feature = "kernel")]
pub const S_IFREG: u16 = 0o100000;
#[cfg(feature = "kernel")]
pub const S_IFDIR: u16 = 0o040000;
#[cfg(feature = "kernel")]
pub const S_IFCHR: u16 = 0o020000;
#[cfg(feature = "kernel")]
pub const S_IFBLK: u16 = 0o060000;
#[cfg(feature = "kernel")]
pub const S_IFIFO: u16 = 0o010000;
#[cfg(feature = "kernel")]
pub const S_IFSOCK: u16 = 0o140000;
#[cfg(feature = "kernel")]
pub const S_IFLNK: u16 = 0o120000;

/// VexFS magic number for superblock identification
pub const VEXFS_MAGIC: u64 = 0x5645584653_u64; // "VEXFS" in ASCII

/// VexFS version constants
pub const VEXFS_VERSION_MAJOR: u16 = 1;
pub const VEXFS_VERSION_MINOR: u16 = 0;

/// Block size constants (configurable 4KB-64KB)
pub const VEXFS_MIN_BLOCK_SIZE: u32 = 4096;   // 4KB
pub const VEXFS_MAX_BLOCK_SIZE: u32 = 65536;  // 64KB
pub const VEXFS_DEFAULT_BLOCK_SIZE: u32 = 4096; // 4KB default

/// Inode constants - Fixed to match actual struct size
pub const VEXFS_ROOT_INO: u64 = 1;
pub const VEXFS_FIRST_USER_INO: u64 = 11;
pub const VEXFS_INODE_SIZE: u16 = 128; // Fixed 128 bytes per inode (matches actual struct)
pub const VEXFS_INODES_PER_BLOCK: u32 = VEXFS_DEFAULT_BLOCK_SIZE / VEXFS_INODE_SIZE as u32;

// File system limits
pub const VEXFS_MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024; // 1GB
pub const VEXFS_MAX_FILENAME_LEN: usize = 255;
pub const VEXFS_DIR_ENTRIES_PER_BLOCK: usize = VEXFS_DEFAULT_BLOCK_SIZE as usize / 64; // Estimate based on entry size

// Directory entry types (from Linux kernel)
pub const DT_UNKNOWN: u8 = 0;
pub const DT_FIFO: u8 = 1;
pub const DT_CHR: u8 = 2;
pub const DT_DIR: u8 = 4;
pub const DT_BLK: u8 = 6;
pub const DT_REG: u8 = 8;
pub const DT_LNK: u8 = 10;
pub const DT_SOCK: u8 = 12;
pub const DT_WHT: u8 = 14;

/// Direct and indirect block pointer constants
pub const VEXFS_N_DIRECT: usize = 12;      // Direct block pointers
pub const VEXFS_N_INDIRECT: usize = 1;     // Single indirect
pub const VEXFS_N_DINDIRECT: usize = 1;    // Double indirect
pub const VEXFS_N_TINDIRECT: usize = 1;    // Triple indirect (for very large files)

/// Journal constants
pub const VEXFS_JOURNAL_BLOCKS: u32 = 1024; // Default journal size in blocks
pub const VEXFS_JOURNAL_MAGIC: u32 = 0x56455846; // "VEXF" for journal

/// Feature flags for superblock
pub const VEXFS_FEATURE_COMPAT_DIR_INDEX: u32     = 0x00000001; // Hash directory indexing
pub const VEXFS_FEATURE_COMPAT_RESIZE_INODE: u32  = 0x00000002; // Online resizing
pub const VEXFS_FEATURE_COMPAT_JOURNAL: u32       = 0x00000004; // Has journal

pub const VEXFS_FEATURE_INCOMPAT_COMPRESSION: u32 = 0x00000001; // File compression
pub const VEXFS_FEATURE_INCOMPAT_FILETYPE: u32    = 0x00000002; // File type in dir entries
pub const VEXFS_FEATURE_INCOMPAT_64BIT: u32       = 0x00000004; // 64-bit block addresses
pub const VEXFS_FEATURE_INCOMPAT_EXTENTS: u32     = 0x00000008; // Extent-based allocation

pub const VEXFS_FEATURE_RO_COMPAT_SPARSE_SUPER: u32 = 0x00000001; // Sparse superblocks
pub const VEXFS_FEATURE_RO_COMPAT_LARGE_FILE: u32   = 0x00000002; // Large files (>2GB)
pub const VEXFS_FEATURE_RO_COMPAT_BTREE_DIR: u32    = 0x00000004; // B-tree directories

/// VexFS on-disk superblock structure
/// Located at the beginning of the filesystem (block 0)
/// Size: 1024 bytes (fits in quarter of 4KB block with room for expansion)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VexfsSuperblock {
    /// Magic number (VEXFS_MAGIC)
    pub s_magic: u64,
    
    /// Total number of blocks in filesystem
    pub s_blocks_count: u64,
    
    /// Number of free blocks
    pub s_free_blocks_count: u64,
    
    /// Total number of inodes
    pub s_inodes_count: u32,
    
    /// Number of free inodes
    pub s_free_inodes_count: u32,
    
    /// Block size in bytes
    pub s_block_size: u32,
    
    /// Inode size in bytes
    pub s_inode_size: u16,
    
    /// Major version
    pub s_version_major: u16,
    
    /// Minor version
    pub s_version_minor: u16,
    
    /// Creation timestamp
    pub s_mkfs_time: u64,
    
    /// Last mount timestamp
    pub s_mount_time: u64,
    
    /// Last write timestamp
    pub s_wtime: u64,
    
    /// Mount count since last fsck
    pub s_mount_count: u16,
    
    /// Maximum mount count before fsck
    pub s_max_mount_count: u16,
    
    /// Filesystem state (clean/error)
    pub s_state: u16,
    
    /// Error behavior
    pub s_errors: u16,
    
    /// Compatible feature flags
    pub s_feature_compat: u32,
    
    /// Incompatible feature flags
    pub s_feature_incompat: u32,
    
    /// Read-only compatible feature flags
    pub s_feature_ro_compat: u32,
    
    /// Filesystem UUID
    pub s_uuid: [u8; 16],
    
    /// Volume name
    pub s_volume_name: [u8; 64],
    
    /// First data block
    pub s_first_data_block: u64,
    
    /// Blocks per group
    pub s_blocks_per_group: u32,
    
    /// Inodes per group
    pub s_inodes_per_group: u32,
    
    /// Number of block groups
    pub s_group_count: u32,
    
    /// Journal inode number
    pub s_journal_inum: u32,
    
    /// Journal block count
    pub s_journal_blocks: u32,
    
    /// First journal block
    pub s_journal_first_block: u64,
    
    // Vector Storage Metadata Extensions
    /// Vector index magic number for validation
    pub s_vector_magic: u32,
    
    /// Vector storage format version
    pub s_vector_version: u16,
    
    /// Vector dimension size (0 = no vectors)
    pub s_vector_dimensions: u16,
    
    /// Vector index algorithm (0=none, 1=HNSW, 2=IVF, etc.)
    pub s_vector_algorithm: u8,
    
    /// Vector distance metric (0=L2, 1=cosine, 2=dot, etc.)
    pub s_vector_metric: u8,
    
    /// Vector index parameters (algorithm-specific)
    pub s_vector_params: [u16; 4],
    
    /// Block containing vector index metadata
    pub s_vector_index_block: u64,
    
    /// Number of blocks used for vector indices
    pub s_vector_index_blocks: u32,
    
    /// Total number of vectors stored
    pub s_vector_count: u64,
    
    /// Vector storage feature flags
    pub s_vector_features: u32,
    
    /// Checksum for superblock integrity
    pub s_checksum: u32,
    
    /// Reserved padding for future extensions
    pub s_reserved: [u32; 126],
}

/// Filesystem states
pub const VEXFS_VALID_FS: u16 = 0x0001; // Cleanly unmounted
pub const VEXFS_ERROR_FS: u16 = 0x0002; // Errors detected

/// Error handling behavior
pub const VEXFS_ERRORS_CONTINUE: u16 = 1; // Continue on errors
pub const VEXFS_ERRORS_RO: u16 = 2;       // Remount read-only on errors
pub const VEXFS_ERRORS_PANIC: u16 = 3;    // Panic on errors

/// Block group descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VexfsGroupDesc {
    /// Block bitmap block
    pub bg_block_bitmap: u64,
    
    /// Inode bitmap block
    pub bg_inode_bitmap: u64,
    
    /// Inode table start block
    pub bg_inode_table: u64,
    
    /// Number of free blocks in group
    pub bg_free_blocks_count: u16,
    
    /// Number of free inodes in group
    pub bg_free_inodes_count: u16,
    
    /// Number of directories in group
    pub bg_used_dirs_count: u16,
    
    /// Padding
    pub bg_pad: u16,
    
    /// Reserved
    pub bg_reserved: [u32; 3],
}

/// On-disk inode structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VexfsInode {
    /// File mode and type
    pub i_mode: u16,
    
    /// Owner UID
    pub i_uid: u16,
    
    /// Size in bytes (lower 32 bits)
    pub i_size_lo: u32,
    
    /// Access time
    pub i_atime: u32,
    
    /// Creation time
    pub i_ctime: u32,
    
    /// Modification time
    pub i_mtime: u32,
    
    /// Deletion time
    pub i_dtime: u32,
    
    /// Group ID
    pub i_gid: u16,
    
    /// Hard link count
    pub i_links_count: u16,
    
    /// Number of 512-byte sectors used
    pub i_blocks: u32,
    
    /// File flags
    pub i_flags: u32,
    
    /// OS specific field 1
    pub l_i_version: u32,
    
    /// Direct block pointers
    pub i_block: [u32; 15],
    
    /// File version (for NFS)
    pub i_generation: u32,
    
    /// Extended attributes block
    pub i_file_acl: u32,
    
    /// Size in bytes (upper 32 bits) or directory ACL
    pub i_size_high: u32,
    
    /// Fragment address (obsolete)
    pub i_faddr: u32,
    
    /// Fragment number (obsolete)
    pub l_i_frag: u8,
    
    /// Fragment size (obsolete)
    pub l_i_fsize: u8,
    
    /// Padding
    pub i_pad1: u16,
    
    /// High 16-bits of UID
    pub l_i_uid_high: u16,
    
    /// High 16-bits of GID
    pub l_i_gid_high: u16,
    
    /// Reserved
    pub l_i_reserved2: u32,
    
    /// Extra inode size
    pub i_extra_isize: u16,
    
    /// Padding to 256 bytes
    pub i_pad: [u8; 2],
}

/// Directory entry structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VexfsDirEntry {
    /// Inode number
    pub inode: u32,
    
    /// Record length
    pub rec_len: u16,
    
    /// Name length
    pub name_len: u8,
    
    /// File type
    pub file_type: u8,
    
    // Name follows here (variable length)
    // Use VexfsDirEntry::name() to access safely
}

/// File types for directory entries
pub const VEXFS_FT_UNKNOWN: u8 = 0;
pub const VEXFS_FT_REG_FILE: u8 = 1;
pub const VEXFS_FT_DIR: u8 = 2;
pub const VEXFS_FT_CHRDEV: u8 = 3;
pub const VEXFS_FT_BLKDEV: u8 = 4;
pub const VEXFS_FT_FIFO: u8 = 5;
pub const VEXFS_FT_SOCK: u8 = 6;
pub const VEXFS_FT_SYMLINK: u8 = 7;

/// Journal superblock structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VexfsJournalSuperblock {
    /// Journal magic number
    pub s_magic: u32,
    
    /// Journal block size
    pub s_blocksize: u32,
    
    /// Maximum transaction length
    pub s_maxlen: u32,
    
    /// First block of journal
    pub s_first: u32,
    
    /// First transaction sequence number
    pub s_sequence: u32,
    
    /// First block of current transaction
    pub s_start: u32,
    
    /// Error value
    pub s_errno: i32,
    
    /// Compatible feature flags
    pub s_feature_compat: u32,
    
    /// Incompatible feature flags
    pub s_feature_incompat: u32,
    
    /// Read-only compatible feature flags
    pub s_feature_ro_compat: u32,
    
    /// Journal UUID
    pub s_uuid: [u8; 16],
    
    /// Number of filesystem users
    pub s_nr_users: u32,
    
    /// Dynamic superblock start block
    pub s_dynsuper: u32,
    
    /// Maximum transaction blocks
    pub s_max_transaction: u32,
    
    /// Maximum batch transaction blocks
    pub s_max_trans_data: u32,
    
    /// Padding
    pub s_padding: [u32; 44],
    
    /// Users of this journal
    pub s_users: [u8; 768],
}

/// Journal block types
pub const VEXFS_JOURNAL_DESCRIPTOR_BLOCK: u32 = 1;
pub const VEXFS_JOURNAL_COMMIT_BLOCK: u32 = 2;
pub const VEXFS_JOURNAL_SUPERBLOCK_V1: u32 = 3;
pub const VEXFS_JOURNAL_SUPERBLOCK_V2: u32 = 4;
pub const VEXFS_JOURNAL_REVOKE_BLOCK: u32 = 5;

/// Vector storage metadata structure
/// Used for storing vector index metadata and configuration
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VexfsVectorMetadata {
    /// Magic number for vector metadata validation
    pub vm_magic: u32,
    
    /// Vector metadata version
    pub vm_version: u16,
    
    /// Vector dimension count
    pub vm_dimensions: u16,
    
    /// Vector algorithm type (HNSW=1, IVF=2, etc.)
    pub vm_algorithm: u8,
    
    /// Distance metric (L2=0, Cosine=1, Dot=2)
    pub vm_metric: u8,
    
    /// Algorithm parameters
    pub vm_max_connections: u16,      // HNSW M parameter
    pub vm_ef_construction: u16,      // HNSW efConstruction
    pub vm_ml: f32,                   // HNSW level generation factor
    pub vm_num_clusters: u32,         // IVF cluster count
    
    /// Index layout information
    pub vm_entry_point: u32,          // Entry point node for HNSW
    pub vm_level_count: u16,          // Number of levels in HNSW
    pub vm_node_count: u32,           // Total nodes in index
    
    /// Storage layout
    pub vm_nodes_block: u64,          // Block containing node storage
    pub vm_nodes_blocks: u32,         // Number of blocks for nodes
    pub vm_edges_block: u64,          // Block containing edge storage
    pub vm_edges_blocks: u32,         // Number of blocks for edges
    pub vm_vectors_block: u64,        // Block containing raw vectors
    pub vm_vectors_blocks: u32,       // Number of blocks for vectors
    
    /// Statistics
    pub vm_total_vectors: u64,        // Total vectors stored
    pub vm_last_update: u64,          // Last modification timestamp
    
    /// Reserved for future use
    pub vm_reserved: [u32; 16],
}

/// Vector metadata magic number
pub const VEXFS_VECTOR_MAGIC: u32 = 0x56455856; // "VEXV"

/// Extent structure for efficient large file allocation
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VexfsExtent {
    /// Starting logical block
    pub ee_block: u32,
    
    /// Number of blocks covered by extent
    pub ee_len: u16,
    
    /// Upper 16 bits of physical block start
    pub ee_start_hi: u16,
    
    /// Lower 32 bits of physical block start
    pub ee_start_lo: u32,
}

/// Extent tree node header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VexfsExtentHeader {
    /// Magic number
    pub eh_magic: u16,
    
    /// Number of valid entries following the header
    pub eh_entries: u16,
    
    /// Maximum number of entries that could fit
    pub eh_max: u16,
    
    /// Depth of this extent node (0 = leaf)
    pub eh_depth: u16,
    
    /// Generation of the tree
    pub eh_generation: u32,
}

/// Extent magic number
pub const VEXFS_EXT_MAGIC: u16 = 0xf30a;

/// Free space tracking structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VexfsFreeSpace {
    /// Starting block of free region
    pub fs_start_block: u64,
    
    /// Length of free region in blocks
    pub fs_length: u32,
    
    /// Next free space entry (0 = end of list)
    pub fs_next: u32,
}

impl VexfsSuperblock {
    /// Create a new superblock with default values
    pub fn new(blocks_count: u64, block_size: u32) -> Self {
        let inodes_count = (blocks_count / 4) as u32; // 1 inode per 4 blocks heuristic
        
        Self {
            s_magic: VEXFS_MAGIC,
            s_blocks_count: blocks_count,
            s_free_blocks_count: blocks_count - 100, // Reserve some blocks
            s_inodes_count: inodes_count,
            s_free_inodes_count: inodes_count - 1, // Root inode used
            s_block_size: block_size,
            s_inode_size: VEXFS_INODE_SIZE,
            s_version_major: VEXFS_VERSION_MAJOR,
            s_version_minor: VEXFS_VERSION_MINOR,
            s_mkfs_time: 0, // Will be set during format
            s_mount_time: 0,
            s_wtime: 0,
            s_mount_count: 0,
            s_max_mount_count: 32,
            s_state: VEXFS_VALID_FS,
            s_errors: VEXFS_ERRORS_CONTINUE,
            s_feature_compat: VEXFS_FEATURE_COMPAT_DIR_INDEX,
            s_feature_incompat: VEXFS_FEATURE_INCOMPAT_FILETYPE,
            s_feature_ro_compat: 0,
            s_uuid: [0; 16], // Will be generated
            s_volume_name: [0; 64],
            s_first_data_block: if block_size == 1024 { 1 } else { 0 },
            s_blocks_per_group: block_size * 8, // 8 blocks per bit in bitmap
            s_inodes_per_group: inodes_count / 8, // Distribute across groups
            s_group_count: 1, // Start with single group
            s_journal_inum: 0, // No journal initially
            s_journal_blocks: 0,
            s_journal_first_block: 0,
            // Vector Storage Metadata - Initialize as disabled
            s_vector_magic: 0, // No vector support initially
            s_vector_version: 0,
            s_vector_dimensions: 0,
            s_vector_algorithm: 0,
            s_vector_metric: 0,
            s_vector_params: [0; 4],
            s_vector_index_block: 0,
            s_vector_index_blocks: 0,
            s_vector_count: 0,
            s_vector_features: 0,
            s_checksum: 0, // Will be calculated when written
            s_reserved: [0; 126], // Updated to match new struct size
        }
    }
    
    /// Validate superblock magic and basic consistency
    pub fn is_valid(&self) -> bool {
        self.s_magic == VEXFS_MAGIC &&
        self.s_block_size >= VEXFS_MIN_BLOCK_SIZE &&
        self.s_block_size <= VEXFS_MAX_BLOCK_SIZE &&
        self.s_inode_size == VEXFS_INODE_SIZE &&
        self.s_blocks_count > 0 &&
        self.s_inodes_count > 0
    }
    
    /// Calculate the number of block groups needed
    pub fn calculate_group_count(&self) -> u32 {
        ((self.s_blocks_count + self.s_blocks_per_group as u64 - 1) / self.s_blocks_per_group as u64) as u32
    }
    
    /// Calculate checksum for superblock integrity
    pub fn calculate_checksum(&self) -> u32 {
        let mut checksum: u32 = 0;
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<VexfsSuperblock>() - 4 // Exclude checksum field itself
            )
        };
        
        for &byte in bytes {
            checksum = checksum.wrapping_add(byte as u32);
        }
        checksum
    }
    
    /// Update checksum field with calculated value
    pub fn update_checksum(&mut self) {
        self.s_checksum = self.calculate_checksum();
    }
    
    /// Verify checksum integrity
    pub fn verify_checksum(&self) -> bool {
        let stored_checksum = self.s_checksum;
        let mut temp_sb = *self;
        temp_sb.s_checksum = 0;
        stored_checksum == temp_sb.calculate_checksum()
    }
}

impl OnDiskSerialize for VexfsSuperblock {
    fn to_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<VexfsSuperblock>()
            )
        }
    }
    
    fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < core::mem::size_of::<VexfsSuperblock>() {
            return Err("Insufficient data for VexfsSuperblock");
        }
        
        let sb = unsafe {
            *(data.as_ptr() as *const VexfsSuperblock)
        };
        
        if !sb.is_valid() {
            return Err("Invalid superblock magic or data");
        }
        
        if !sb.verify_checksum() {
            return Err("Superblock checksum mismatch");
        }
        
        Ok(sb)
    }
    
    fn serialized_size() -> usize {
        core::mem::size_of::<VexfsSuperblock>()
    }
}

impl VexfsInode {
    /// Create a new inode with basic initialization
    pub fn new(mode: u16, uid: u16, gid: u16) -> Self {
        Self {
            i_mode: mode,
            i_uid: uid,
            i_size_lo: 0,
            i_atime: 0, // Will be set to current time
            i_ctime: 0,
            i_mtime: 0,
            i_dtime: 0,
            i_gid: gid,
            i_links_count: 1,
            i_blocks: 0,
            i_flags: 0,
            l_i_version: 0,
            i_block: [0; 15],
            i_generation: 0,
            i_file_acl: 0,
            i_size_high: 0,
            i_faddr: 0,
            l_i_frag: 0,
            l_i_fsize: 0,
            i_pad1: 0,
            l_i_uid_high: 0,
            l_i_gid_high: 0,
            l_i_reserved2: 0,
            i_extra_isize: 0,
            i_pad: [0; 2],
        }
    }
    
    /// Get the full file size (combining lo and hi parts)
    pub fn get_size(&self) -> u64 {
        (self.i_size_high as u64) << 32 | self.i_size_lo as u64
    }
    
    /// Set the full file size (splitting into lo and hi parts)
    pub fn set_size(&mut self, size: u64) {
        self.i_size_lo = size as u32;
        self.i_size_high = (size >> 32) as u32;
    }
    
    /// Check if inode represents a directory
    pub fn is_dir(&self) -> bool {
        (self.i_mode & S_IFMT) == S_IFDIR
    }
    
    /// Check if inode represents a regular file
    pub fn is_file(&self) -> bool {
        (self.i_mode & S_IFMT) == S_IFREG
    }
}

impl OnDiskSerialize for VexfsInode {
    fn to_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<VexfsInode>()
            )
        }
    }
    
    fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < core::mem::size_of::<VexfsInode>() {
            return Err("Insufficient data for VexfsInode");
        }
        
        let inode = unsafe {
            *(data.as_ptr() as *const VexfsInode)
        };
        
        // Basic validation - check if mode is reasonable
        if inode.i_mode == 0 {
            return Err("Invalid inode mode");
        }
        
        Ok(inode)
    }
    
    fn serialized_size() -> usize {
        core::mem::size_of::<VexfsInode>()
    }
}

impl VexfsDirEntry {
    /// Get the name from a directory entry
    /// This is unsafe because it reads past the struct boundary
    pub unsafe fn name(&self) -> &[u8] {
        let name_ptr = (self as *const Self as *const u8).add(8); // Skip fixed fields
        core::slice::from_raw_parts(name_ptr, self.name_len as usize)
    }
    
    /// Calculate the total size needed for a directory entry with given name
    pub fn calc_rec_len(name_len: u8) -> u16 {
        let base_len = 8; // Fixed fields: inode(4) + rec_len(2) + name_len(1) + file_type(1)
        let total = base_len + name_len as u16;
        // Align to 4-byte boundary
        (total + 3) & !3
    }
    
    /// Convert file mode to directory entry file type
    pub fn mode_to_file_type(mode: u16) -> u8 {
        match mode & S_IFMT {
            S_IFREG => VEXFS_FT_REG_FILE,
            S_IFDIR => VEXFS_FT_DIR,
            S_IFCHR => VEXFS_FT_CHRDEV,
            S_IFBLK => VEXFS_FT_BLKDEV,
            S_IFIFO => VEXFS_FT_FIFO,
            S_IFSOCK => VEXFS_FT_SOCK,
            S_IFLNK => VEXFS_FT_SYMLINK,
            _ => VEXFS_FT_UNKNOWN,
        }
    }
}

/// Layout of the filesystem on disk:
/// 
/// Block 0: Superblock
/// Block 1: Group descriptor table
/// Block 2-N: Block bitmap (1 block per 8*block_size blocks)
/// Block N+1-M: Inode bitmap (1 block per 8*block_size inodes)
/// Block M+1-K: Inode table
/// Block K+1-J: Journal (if enabled)
/// Block J+1-End: Data blocks
///
/// Each block group contains:
/// - Block bitmap (1 block)
/// - Inode bitmap (1 block) 
/// - Inode table (variable blocks)
/// - Data blocks (remaining blocks in group)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structure_sizes() {
        // Verify that our structures have the expected sizes for kernel compatibility
        assert_eq!(core::mem::size_of::<VexfsSuperblock>(), 1024, "Superblock should be exactly 1KB");
        assert_eq!(core::mem::size_of::<VexfsInode>(), 128, "Inode should be exactly 128 bytes");
        assert_eq!(core::mem::size_of::<VexfsDirEntry>(), 8, "DirEntry base should be 8 bytes");
        assert_eq!(core::mem::size_of::<VexfsGroupDesc>(), 32, "GroupDesc should be 32 bytes");
    }

    #[test]
    fn test_structure_alignment() {
        // Verify proper alignment for cache efficiency
        assert_eq!(core::mem::align_of::<VexfsSuperblock>(), 8, "Superblock should be 8-byte aligned");
        assert_eq!(core::mem::align_of::<VexfsInode>(), 8, "Inode should be 8-byte aligned");
        assert_eq!(core::mem::align_of::<VexfsGroupDesc>(), 4, "GroupDesc should be 4-byte aligned");
    }

    #[test]
    fn test_superblock_creation() {
        let sb = VexfsSuperblock::new(1000000, 4096);
        assert!(sb.is_valid());
    }

    #[test]
    fn test_superblock_serialization() {
        let mut sb = VexfsSuperblock::new(100000, 4096);
        sb.update_checksum();

        // Serialize
        let bytes = sb.to_bytes();
        assert_eq!(bytes.len(), VexfsSuperblock::serialized_size());

        // Deserialize
        let sb2 = VexfsSuperblock::from_bytes(bytes).unwrap();
        assert!(sb2.is_valid());
    }

    #[test]
    fn test_superblock_checksum() {
        let mut sb = VexfsSuperblock::new(100000, 4096);
        
        // Initially no checksum
        assert!(!sb.verify_checksum());
        
        // Update checksum
        sb.update_checksum();
        assert!(sb.verify_checksum());
    }

    #[test]
    fn test_inode_creation() {
        let inode = VexfsInode::new(S_IFREG | 0o644, 1000, 1000);
        
        assert_eq!(inode.get_size(), 0);
        assert!(inode.is_file());
        assert!(!inode.is_dir());
    }

    #[test]
    fn test_inode_size_handling() {
        let mut inode = VexfsInode::new(S_IFREG | 0o644, 0, 0);
        
        // Test small size
        inode.set_size(12345);
        assert_eq!(inode.get_size(), 12345);
        
        // Test large size (> 4GB)
        let large_size = 5_000_000_000u64; // 5GB
        inode.set_size(large_size);
        assert_eq!(inode.get_size(), large_size);
    }

    #[test]
    fn test_inode_serialization() {
        let inode = VexfsInode::new(S_IFDIR | 0o755, 0, 0);
        
        // Serialize
        let bytes = inode.to_bytes();
        assert_eq!(bytes.len(), VexfsInode::serialized_size());
        
        // Deserialize
        let inode2 = VexfsInode::from_bytes(bytes).unwrap();
        assert!(inode2.is_dir());
    }

    #[test]
    fn test_dir_entry_calculations() {
        // Test record length calculation with alignment
        assert_eq!(VexfsDirEntry::calc_rec_len(1), 12);  // 8 + 1 = 9, aligned to 12
        assert_eq!(VexfsDirEntry::calc_rec_len(4), 12);  // 8 + 4 = 12, already aligned
        assert_eq!(VexfsDirEntry::calc_rec_len(5), 16);  // 8 + 5 = 13, aligned to 16
        assert_eq!(VexfsDirEntry::calc_rec_len(255), 264); // 8 + 255 = 263, aligned to 264
    }

    #[test]
    fn test_file_type_conversion() {
        assert_eq!(VexfsDirEntry::mode_to_file_type(S_IFREG), VEXFS_FT_REG_FILE);
        assert_eq!(VexfsDirEntry::mode_to_file_type(S_IFDIR), VEXFS_FT_DIR);
        assert_eq!(VexfsDirEntry::mode_to_file_type(S_IFLNK), VEXFS_FT_SYMLINK);
        assert_eq!(VexfsDirEntry::mode_to_file_type(S_IFCHR), VEXFS_FT_CHRDEV);
        assert_eq!(VexfsDirEntry::mode_to_file_type(S_IFBLK), VEXFS_FT_BLKDEV);
        assert_eq!(VexfsDirEntry::mode_to_file_type(0), VEXFS_FT_UNKNOWN);
    }

    #[test]
    fn test_superblock_validation() {
        // Test valid superblock
        let sb = VexfsSuperblock::new(1000, 4096);
        assert!(sb.is_valid());
        
        // Test creation and basic functionality
        let mut sb2 = VexfsSuperblock::new(50000, 4096);
        sb2.update_checksum();
        assert!(sb2.verify_checksum());
    }

    #[test]
    fn test_group_calculations() {
        let sb = VexfsSuperblock::new(100000, 4096);
        
        // With default blocks per group, calculate expected groups
        let group_count = sb.calculate_group_count();
        assert!(group_count > 0);
        assert!(group_count <= 10); // Reasonable upper bound
    }

    #[test]
    fn test_error_conditions() {
        // Test invalid superblock deserialization
        let short_data = vec![0u8; 100]; // Too short
        assert!(VexfsSuperblock::from_bytes(&short_data).is_err());
        
        let mut bad_magic_data = vec![0u8; 1024];
        bad_magic_data[0] = 0xFF; // Wrong magic
        assert!(VexfsSuperblock::from_bytes(&bad_magic_data).is_err());
        
        // Test invalid inode deserialization
        let short_inode_data = vec![0u8; 50]; // Too short
        assert!(VexfsInode::from_bytes(&short_inode_data).is_err());
        
        let zero_mode_data = vec![0u8; 128]; // Valid size but zero mode
        assert!(VexfsInode::from_bytes(&zero_mode_data).is_err());
    }

    #[test]
    fn test_basic_constants() {
        // Test that our magic number is defined
        assert_eq!(VEXFS_MAGIC, 0x5645584653); // "VEXFS" in little endian
        
        // Test that inode size constant is reasonable
        assert_eq!(VEXFS_INODE_SIZE, 128);
        
        // Test version constants
        assert!(VEXFS_VERSION_MAJOR > 0);
        assert!(VEXFS_VERSION_MINOR >= 0);
    }

    #[test]
    fn test_vector_metadata_structure() {
        // Vector metadata should be cache-line aligned
        let size = size_of::<VexfsVectorMetadata>();
        println!("VexfsVectorMetadata size: {} bytes", size);
        
        // Should be reasonable size (not too large)
        assert!(size <= 256);
        
        // Test magic number
        assert_eq!(VEXFS_VECTOR_MAGIC, 0x56455856);
        
        // Test initialization
        let vm = VexfsVectorMetadata {
            vm_magic: VEXFS_VECTOR_MAGIC,
            vm_version: 1,
            vm_dimensions: 128,
            vm_algorithm: 1, // HNSW
            vm_metric: 0,    // L2
            vm_max_connections: 16,
            vm_ef_construction: 200,
            vm_ml: 1.0 / (2.0_f32).ln(),
            vm_num_clusters: 0,
            vm_entry_point: 0,
            vm_level_count: 0,
            vm_node_count: 0,
            vm_nodes_block: 0,
            vm_nodes_blocks: 0,
            vm_edges_block: 0,
            vm_edges_blocks: 0,
            vm_vectors_block: 0,
            vm_vectors_blocks: 0,
            vm_total_vectors: 0,
            vm_last_update: 0,
            vm_reserved: [0; 16],
        };
        
        // Use addr_of! to safely access packed struct fields
        use core::ptr::addr_of;
        let magic = unsafe { addr_of!(vm.vm_magic).read_unaligned() };
        let dimensions = unsafe { addr_of!(vm.vm_dimensions).read_unaligned() };
        assert_eq!(magic, VEXFS_VECTOR_MAGIC);
        assert_eq!(dimensions, 128);
    }

    #[test]
    fn test_extent_structures() {
        // Extent should be small and efficient
        let extent_size = size_of::<VexfsExtent>();
        println!("VexfsExtent size: {} bytes", extent_size);
        assert_eq!(extent_size, 12); // Should be exactly 12 bytes
        
        // Extent header should be small
        let header_size = size_of::<VexfsExtentHeader>();
        println!("VexfsExtentHeader size: {} bytes", header_size);
        assert_eq!(header_size, 12); // Should be exactly 12 bytes
        
        // Test extent magic constant
        assert_eq!(VEXFS_EXT_MAGIC, 0xf30a);
    }

    #[test]
    fn test_free_space_structure() {
        let size = size_of::<VexfsFreeSpace>();
        println!("VexfsFreeSpace size: {} bytes", size);
        assert_eq!(size, 16); // Should be exactly 16 bytes
    }

    #[test]
    fn test_layout_sizes() {
        // All critical structures should be properly aligned
        println!("Structure sizes:");
        println!("  VexfsSuperblock: {} bytes", size_of::<VexfsSuperblock>());
        println!("  VexfsInode: {} bytes", size_of::<VexfsInode>());
        println!("  VexfsDirEntry: {} bytes", size_of::<VexfsDirEntry>());
        println!("  VexfsVectorMetadata: {} bytes", size_of::<VexfsVectorMetadata>());
        println!("  VexfsExtent: {} bytes", size_of::<VexfsExtent>());
        println!("  VexfsExtentHeader: {} bytes", size_of::<VexfsExtentHeader>());
        println!("  VexfsFreeSpace: {} bytes", size_of::<VexfsFreeSpace>());
        
        // Verify superblock fits in one block
        assert!(size_of::<VexfsSuperblock>() <= VEXFS_DEFAULT_BLOCK_SIZE as usize);
        
        // Verify inode size matches constant
        assert_eq!(size_of::<VexfsInode>(), VEXFS_INODE_SIZE as usize);
        
        // Verify structures are reasonably sized
        assert!(size_of::<VexfsDirEntry>() <= 256);
        assert!(size_of::<VexfsVectorMetadata>() <= 256);
    }
}

/// Calculate filesystem layout parameters
pub struct VexfsLayout {
    pub superblock_block: u64,
    pub group_desc_block: u64,
    pub block_bitmap_blocks: u32,
    pub inode_bitmap_blocks: u32,
    pub inode_table_blocks: u32,
    pub journal_start_block: u64,
    pub journal_blocks: u32,
    pub data_start_block: u64,
}

impl VexfsLayout {
    pub fn calculate(sb: &VexfsSuperblock) -> Self {
        let superblock_block = 0;
        let group_desc_block = 1;
        
        // Calculate blocks needed for bitmaps and tables
        let blocks_per_bitmap = sb.s_block_size * 8; // 8 bits per byte
        let block_bitmap_blocks = (sb.s_blocks_count + blocks_per_bitmap as u64 - 1) / blocks_per_bitmap as u64;
        let inode_bitmap_blocks = (sb.s_inodes_count + blocks_per_bitmap - 1) / blocks_per_bitmap;
        
        let inodes_per_block = sb.s_block_size / sb.s_inode_size as u32;
        let inode_table_blocks = (sb.s_inodes_count + inodes_per_block - 1) / inodes_per_block;
        
        // Journal starts after metadata
        let metadata_blocks = 2 + block_bitmap_blocks + inode_bitmap_blocks as u64 + inode_table_blocks as u64;
        let journal_start_block = metadata_blocks;
        let journal_blocks = if sb.s_journal_blocks > 0 { sb.s_journal_blocks } else { 0 };
        
        let data_start_block = journal_start_block + journal_blocks as u64;
        
        VexfsLayout {
            superblock_block,
            group_desc_block,
            block_bitmap_blocks: block_bitmap_blocks as u32,
            inode_bitmap_blocks,
            inode_table_blocks,
            journal_start_block,
            journal_blocks,
            data_start_block,
        }
    }
}