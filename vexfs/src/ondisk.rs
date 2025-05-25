//! On-disk format and layout definitions for VexFS
//!
//! This module defines the physical structure of the VexFS filesystem,
//! including the superblock, inode tables, data blocks, and free space tracking.



// Basic type definitions for kernel module compatibility
pub type c_int = i32;
pub type umode_t = u16;

// File type constants (from Linux kernel)
pub const S_IFMT: u16 = 0o170000;
pub const S_IFREG: u16 = 0o100000;
pub const S_IFDIR: u16 = 0o040000;
pub const S_IFCHR: u16 = 0o020000;
pub const S_IFBLK: u16 = 0o060000;
pub const S_IFIFO: u16 = 0o010000;
pub const S_IFSOCK: u16 = 0o140000;
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

/// Inode constants
pub const VEXFS_ROOT_INO: u64 = 1;
pub const VEXFS_FIRST_USER_INO: u64 = 11;
pub const VEXFS_INODE_SIZE: u16 = 256; // Fixed 256 bytes per inode
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
    
    /// Reserved padding
    pub s_reserved: [u32; 158],
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
#[derive(Debug)]
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
            s_reserved: [0; 158],
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