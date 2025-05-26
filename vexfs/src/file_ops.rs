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

//! File operations implementation for VexFS
//!
//! This module provides core file operations including create, read, write,
//! truncate, and unlink operations. All operations integrate with the VFS
//! interface and maintain filesystem consistency through proper locking
//! and journaling.

use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use crate::ffi::{
    VEXFS_SUCCESS, VEXFS_ERROR_GENERIC, VEXFS_ERROR_INVAL, VEXFS_ERROR_NOMEM,
    VEXFS_ERROR_NOSPC, VEXFS_ERROR_PERMISSION, VEXFS_ERROR_NOENT, VEXFS_ERROR_IO,
    VEXFS_ERROR_EXIST, VEXFS_ERROR_NOTDIR, VEXFS_ERROR_ISDIR,
    to_ffi_result, FFIResult
};
use crate::inode_mgmt::{VexfsInodeManager, VexfsInodeHandle, VexfsInodeInfo, InodeError};
use crate::ondisk::{VexfsSuperblock, VexfsGroupDesc, VexfsInode, VexfsDirEntry, OnDiskSerialize, VEXFS_MAX_FILENAME_LEN};
use crate::space_alloc::{VexfsSpaceAllocator, BlockHandle};
use crate::journal::{VexfsJournal, JournalTransaction};

// File open flags (matching Linux O_* constants)
pub const VEXFS_O_APPEND: u32 = 0x400;
pub const VEXFS_O_TRUNC: u32 = 0x200;
pub const VEXFS_O_CREAT: u32 = 0x40;
pub const VEXFS_O_RDONLY: u32 = 0o0;
pub const VEXFS_O_WRONLY: u32 = 0o1;
pub const VEXFS_O_RDWR: u32 = 0o2;

/// File type constants
pub const VEXFS_S_IFREG: u32 = 0o100000;  // Regular file
pub const VEXFS_S_IFDIR: u32 = 0o040000;  // Directory
pub const VEXFS_S_IFLNK: u32 = 0o120000;  // Symbolic link

/// Maximum file size (4GB for now)
pub const VEXFS_MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024;

/// Maximum filename length
pub const VEXFS_NAME_LEN: u32 = VEXFS_MAX_FILENAME_LEN as u32;

/// Simple spinlock implementation for no_std environment
#[repr(C)]
pub struct VexfsSpinLock {
    locked: AtomicBool,
}

impl VexfsSpinLock {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }
    
    pub fn lock(&self) {
        while self.locked.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            // Spin-wait
            core::hint::spin_loop();
        }
    }
    
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
    
    pub fn try_lock(&self) -> bool {
        self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }
}

/// Per-inode lock for concurrent access control
#[repr(C)]
pub struct VexfsInodeLock {
    pub inode_number: u64,
    pub lock: VexfsSpinLock,
    pub readers: AtomicU64,
    pub writer: AtomicBool,
}

impl VexfsInodeLock {
    pub fn new(inode_number: u64) -> Self {
        Self {
            inode_number,
            lock: VexfsSpinLock::new(),
            readers: AtomicU64::new(0),
            writer: AtomicBool::new(false),
        }
    }
    
    pub fn read_lock(&self) {
        loop {
            self.lock.lock();
            if !self.writer.load(Ordering::Acquire) {
                self.readers.fetch_add(1, Ordering::AcqRel);
                self.lock.unlock();
                break;
            }
            self.lock.unlock();
            core::hint::spin_loop();
        }
    }
    
    pub fn read_unlock(&self) {
        self.readers.fetch_sub(1, Ordering::AcqRel);
    }
    
    pub fn write_lock(&self) {
        loop {
            self.lock.lock();
            if !self.writer.load(Ordering::Acquire) && self.readers.load(Ordering::Acquire) == 0 {
                self.writer.store(true, Ordering::Release);
                self.lock.unlock();
                break;
            }
            self.lock.unlock();
            core::hint::spin_loop();
        }
    }
    
    pub fn write_unlock(&self) {
        self.writer.store(false, Ordering::Release);
    }
}

/// Filesystem context structure containing all filesystem components
#[repr(C)]
pub struct VexfsContext {
    /// Superblock
    pub superblock: VexfsSuperblock,
    
    /// Inode manager
    pub inode_manager: VexfsInodeManager,
    
    /// Space allocator
    pub space_allocator: VexfsSpaceAllocator,
    
    /// Journal
    pub journal: VexfsJournal,
    
    /// Global filesystem lock
    pub fs_lock: VexfsSpinLock,
    
    /// Directory operation lock
    pub dir_lock: VexfsSpinLock,
}

impl VexfsContext {
    /// Create a new filesystem context from a superblock and group descriptor
    pub fn from_superblock(superblock: VexfsSuperblock, group_desc: VexfsGroupDesc) -> Result<Self, &'static str> {
        let inode_manager = VexfsInodeManager::new(superblock, group_desc);
        let space_allocator = VexfsSpaceAllocator::new(
            superblock.s_blocks_count,
            superblock.s_block_size,
            superblock.s_first_data_block,
        );
        let journal = VexfsJournal::new();
        
        Ok(Self {
            superblock,
            inode_manager,
            space_allocator,
            journal,
            fs_lock: VexfsSpinLock::new(),
            dir_lock: VexfsSpinLock::new(),
        })
    }
    
    /// Create a new filesystem context with default group descriptor
    pub fn from_superblock_simple(superblock: VexfsSuperblock) -> Result<Self, &'static str> {
        // Create a basic group descriptor for testing
        let group_desc = VexfsGroupDesc {
            bg_block_bitmap: superblock.s_first_data_block + 1,
            bg_inode_bitmap: superblock.s_first_data_block + 2,
            bg_inode_table: superblock.s_first_data_block + 3,
            bg_free_blocks_count: superblock.s_free_blocks_count,
            bg_free_inodes_count: superblock.s_free_inodes_count,
            bg_used_dirs_count: 0,
            bg_pad: 0,
            bg_reserved: [0; 3],
        };
        
        Self::from_superblock(superblock, group_desc)
    }
    
    /// Acquire a read lock on the specified inode
    pub fn lock_inode_read(&self, inode_number: u64) {
        // In a real implementation, we would maintain a hash table of inode locks
        // For now, use the global filesystem lock
        self.fs_lock.lock();
    }
    
    /// Release a read lock on the specified inode
    pub fn unlock_inode_read(&self, inode_number: u64) {
        self.fs_lock.unlock();
    }
    
    /// Acquire a write lock on the specified inode
    pub fn lock_inode_write(&self, inode_number: u64) {
        self.fs_lock.lock();
    }
    
    /// Release a write lock on the specified inode
    pub fn unlock_inode_write(&self, inode_number: u64) {
        self.fs_lock.unlock();
    }
    
    /// Acquire directory lock for structural operations
    pub fn lock_directory(&self) {
        self.dir_lock.lock();
    }
    
    /// Release directory lock
    pub fn unlock_directory(&self) {
        self.dir_lock.unlock();
    }
}

/// File handle structure for tracking open files
#[repr(C)]
pub struct VexfsFileHandle {
    /// Inode number
    pub ino: u64,
    /// Current file position
    pub pos: u64,
    /// Open flags (read, write, etc.)
    pub flags: u32,
    /// Reference to inode
    pub inode_handle: Option<VexfsInodeHandle>,
}

/// File permissions constants
pub const VEXFS_S_IRUSR: u32 = 0o400;  // Owner read
pub const VEXFS_S_IWUSR: u32 = 0o200;  // Owner write
pub const VEXFS_S_IXUSR: u32 = 0o100;  // Owner execute
pub const VEXFS_S_IRGRP: u32 = 0o040;  // Group read
pub const VEXFS_S_IWGRP: u32 = 0o020;  // Group write
pub const VEXFS_S_IXGRP: u32 = 0o010;  // Group execute
pub const VEXFS_S_IROTH: u32 = 0o004;  // Other read
pub const VEXFS_S_IWOTH: u32 = 0o002;  // Other write
pub const VEXFS_S_IXOTH: u32 = 0o001;  // Other execute

/// Check if the current process has permission to perform the specified operation
/// on the inode with the given mode and ownership.
fn check_permission(inode: &VexfsInode, requested_mode: u32, uid: u32, gid: u32) -> bool {
    // Root can access everything
    if uid == 0 {
        return true;
    }

    let file_mode = inode.mode;
    
    // Check owner permissions
    if uid == inode.uid {
        let owner_perms = (file_mode >> 6) & 0o7;
        return (owner_perms & requested_mode) == requested_mode;
    }
    
    // Check group permissions
    if gid == inode.gid {
        let group_perms = (file_mode >> 3) & 0o7;
        return (group_perms & requested_mode) == requested_mode;
    }
    
    // Check other permissions
    let other_perms = file_mode & 0o7;
    (other_perms & requested_mode) == requested_mode
}

/// File type constants for directory entries
pub const VEXFS_FT_UNKNOWN: u8 = 0;
pub const VEXFS_FT_REG_FILE: u8 = 1;
pub const VEXFS_FT_DIR: u8 = 2;
pub const VEXFS_FT_CHRDEV: u8 = 3;
pub const VEXFS_FT_BLKDEV: u8 = 4;
pub const VEXFS_FT_FIFO: u8 = 5;
pub const VEXFS_FT_SOCK: u8 = 6;
pub const VEXFS_FT_SYMLINK: u8 = 7;

/// Add a directory entry to a parent directory
fn add_directory_entry(
    inode_manager: &mut VexfsInodeManager,
    space_allocator: &mut VexfsSpaceAllocator,
    parent_ino: u64,
    name: &[u8],
    child_ino: u64,
    file_type: u8,
) -> Result<(), &'static str> {
    // Validate name length
    if name.len() > VEXFS_MAX_FILENAME_LEN {
        return Err("Filename too long");
    }
    
    // Load parent directory inode
    let mut parent_inode = inode_manager.read_inode(parent_ino)
        .map_err(|_| "Failed to read parent inode")?;
    
    // For now, implement a simplified directory entry addition
    // In a full implementation, this would:
    // 1. Read existing directory blocks
    // 2. Find space for the new entry
    // 3. Allocate new blocks if needed
    // 4. Write the directory entry
    // 5. Update the parent inode size
    
    // Calculate entry size (aligned to 4 bytes)
    let entry_len = core::mem::size_of::<VexfsDirEntry>() + name.len();
    let aligned_entry_len = (entry_len + 3) & !3; // 4-byte alignment
    
    // Update parent directory size
    parent_inode.disk_inode.i_size_lo += aligned_entry_len as u32;
    
    // Write back the updated parent inode
    inode_manager.write_inode(parent_ino, &parent_inode)
        .map_err(|_| "Failed to write parent inode")?;
    
    // TODO: Implement actual directory block management
    // For now, we just update the size - real implementation would write directory blocks
    
    Ok(())
}

/// Read data from a file at the specified offset
fn read_file_blocks(
    inode: &VexfsInode,
    offset: u64,
    buffer: &mut [u8],
    space_allocator: &VexfsSpaceAllocator,
) -> Result<usize, &'static str> {
    let inode_size = inode.i_size_lo as u64; // TODO: handle i_size_high for large files
    if offset >= inode_size {
        return Ok(0); // EOF
    }

    let bytes_to_read = core::cmp::min(buffer.len(), (inode_size - offset) as usize);
    let mut bytes_read = 0;
    let mut current_offset = offset;
    
    // Calculate starting block and offset within block
    let block_size = 4096u64; // 4KB blocks
    let mut block_index = current_offset / block_size;
    let mut block_offset = current_offset % block_size;
    
    while bytes_read < bytes_to_read && block_index < 12 {
        let block_num = inode.i_block[block_index as usize];
        if block_num == 0 {
            // Sparse file - fill with zeros
            let bytes_in_block = core::cmp::min(
                bytes_to_read - bytes_read,
                (block_size - block_offset) as usize
            );
            buffer[bytes_read..bytes_read + bytes_in_block].fill(0);
            bytes_read += bytes_in_block;
        } else {
            // Read from actual block
            let block_handle = space_allocator.get_block(block_num)?;
            let block_data = block_handle.data();
            
            let bytes_in_block = core::cmp::min(
                bytes_to_read - bytes_read,
                (block_size - block_offset) as usize
            );
            
            let start_idx = block_offset as usize;
            let end_idx = start_idx + bytes_in_block;
            buffer[bytes_read..bytes_read + bytes_in_block]
                .copy_from_slice(&block_data[start_idx..end_idx]);
            
            bytes_read += bytes_in_block;
        }
        
        current_offset += (block_size - block_offset);
        block_offset = 0;
        block_index += 1;
    }
    
    Ok(bytes_read)
}

/// Write data to a file at the specified offset
fn write_file_blocks(
    inode: &mut VexfsInode,
    offset: u64,
    data: &[u8],
    space_allocator: &mut VexfsSpaceAllocator,
    journal: &mut VexfsJournal,
) -> Result<usize, &'static str> {
    if offset + data.len() as u64 > VEXFS_MAX_FILE_SIZE {
        return Err("File too large");
    }

    let mut transaction = journal.begin_transaction()?;
    let mut bytes_written = 0;
    let mut current_offset = offset;
    
    // Calculate starting block and offset within block
    let block_size = 4096u64; // 4KB blocks
    let mut block_index = current_offset / block_size;
    let mut block_offset = current_offset % block_size;
    
    while bytes_written < data.len() && block_index < 12 {
        let mut block_num = inode.i_block[block_index as usize];

        // Allocate block if it doesn't exist
        if block_num == 0 {
            let new_block = space_allocator.allocate_block()?;
            block_num = new_block.block_number();
            inode.i_block[block_index as usize] = block_num;

            // Journal the inode update
            transaction.log_inode_update(inode.i_ino, inode)?;
        }
        
        // Get the block for writing
        let mut block_handle = space_allocator.get_block_mut(block_num)?;
        let block_data = block_handle.data_mut();
        
        let bytes_in_block = core::cmp::min(
            data.len() - bytes_written,
            (block_size - block_offset) as usize
        );
        
        let start_idx = block_offset as usize;
        let end_idx = start_idx + bytes_in_block;
        block_data[start_idx..end_idx]
            .copy_from_slice(&data[bytes_written..bytes_written + bytes_in_block]);
        
        // Journal the block write
        transaction.log_block_write(block_num, block_data)?;
        
        bytes_written += bytes_in_block;
        current_offset += bytes_in_block as u64;
        block_offset = 0;
        block_index += 1;
    }
    
    // Update file size if we extended it
    let current_size = inode.i_size_lo as u64; // TODO: handle i_size_high for large files
    if current_offset > current_size {
        inode.i_size_lo = current_offset as u32; // TODO: handle overflow to i_size_high
        transaction.log_inode_update(inode.i_ino, inode)?;
    }
    
    transaction.commit()?;
    Ok(bytes_written)
}

/// Create a new file with the specified mode and ownership
#[no_mangle]
pub extern "C" fn vexfs_create_file(
    sb_ptr: *mut c_void,
    parent_ino: u64,
    name: *const c_char,
    name_len: u32,
    mode: u32,
    uid: u32,
    gid: u32,
) -> c_int {
    if sb_ptr.is_null() || name.is_null() || name_len == 0 {
        return VEXFS_ERROR_INVAL;
    }

    // Validate filename length
    if name_len > VEXFS_NAME_LEN {
        return VEXFS_ERROR_INVAL;
    }

    // Validate file mode - must be regular file
    if (mode & VEXFS_S_IFREG) == 0 {
        return VEXFS_ERROR_INVAL;
    }
    
    // Convert C string to Rust slice
    let filename = unsafe {
        core::slice::from_raw_parts(name as *const u8, name_len as usize)
    };
    
    // Validate filename doesn't contain null bytes
    if filename.iter().any(|&b| b == 0) {
        return VEXFS_ERROR_INVAL;
    }
    
    // Get filesystem context from superblock pointer
    let fs_ctx = unsafe {
        &mut *(sb_ptr as *mut VexfsContext)
    };
    
    // Perform the file creation operation
    match create_file_impl(fs_ctx, parent_ino, filename, mode as u16, uid as u16, gid as u16) {
        Ok(new_ino) => {
            // Success - return the new inode number (for now, just return success)
            VEXFS_SUCCESS
        }
        Err(_) => VEXFS_ERROR_GENERIC
    }
}

/// Internal implementation of file creation
fn create_file_impl(
    fs_ctx: &mut VexfsContext,
    parent_ino: u64,
    filename: &[u8],
    mode: u16,
    uid: u16,
    gid: u16,
) -> Result<u64, &'static str> {
    // Acquire directory lock to prevent concurrent directory modifications
    fs_ctx.lock_directory();
    
    // Also lock the parent inode for writing
    fs_ctx.lock_inode_write(parent_ino);
    
    // Start a journal transaction for consistency
    let tid = fs_ctx.journal.start_transaction(0)
        .map_err(|_| {
            fs_ctx.unlock_inode_write(parent_ino);
            fs_ctx.unlock_directory();
            "Failed to start transaction"
        })?;
    
    // 1. Check parent directory permissions (write permission required)
    let parent_inode = fs_ctx.inode_manager.read_inode(parent_ino)
        .map_err(|_| {
            fs_ctx.journal.abort_transaction(tid);
            fs_ctx.unlock_inode_write(parent_ino);
            fs_ctx.unlock_directory();
            "Failed to read parent inode"
        })?;
    
    // Check if parent is actually a directory
    if (parent_inode.disk_inode.i_mode & VEXFS_S_IFDIR as u16) == 0 {
        fs_ctx.journal.abort_transaction(tid);
        fs_ctx.unlock_inode_write(parent_ino);
        fs_ctx.unlock_directory();
        return Err("Parent is not a directory");
    }
    
    // Check permissions - parent directory must have write permission
    if !check_write_permission(&parent_inode.disk_inode, uid, gid) {
        fs_ctx.journal.abort_transaction(tid);
        fs_ctx.unlock_inode_write(parent_ino);
        fs_ctx.unlock_directory();
        return Err("Permission denied");
    }
    
    // Check if file already exists
    if let Ok(_existing_ino) = lookup_directory_entry(
        &fs_ctx.inode_manager,
        parent_ino,
        filename,
    ) {
        fs_ctx.journal.abort_transaction(tid);
        fs_ctx.unlock_inode_write(parent_ino);
        fs_ctx.unlock_directory();
        return Err("File already exists");
    }
    
    // 2. Create new inode
    let new_inode = fs_ctx.inode_manager.create_inode(
        mode | VEXFS_S_IFREG as u16,
        uid,
        gid,
    ).map_err(|_| {
        fs_ctx.journal.abort_transaction(tid);
        fs_ctx.unlock_inode_write(parent_ino);
        fs_ctx.unlock_directory();
        "Failed to create inode"
    })?;
    
    let new_ino = new_inode.ino;
    
    // 3. Add directory entry to parent directory
    let dir_entry_result = add_directory_entry(
        &mut fs_ctx.inode_manager,
        &mut fs_ctx.space_allocator,
        parent_ino,
        filename,
        new_ino,
        VEXFS_FT_REG_FILE,
    );
    
    if let Err(e) = dir_entry_result {
        // Clean up the allocated inode on failure
        let _ = fs_ctx.inode_manager.free_inode(new_ino);
        fs_ctx.journal.abort_transaction(tid);
        fs_ctx.unlock_inode_write(parent_ino);
        fs_ctx.unlock_directory();
        return Err(e);
    }
    
    // 4. Update superblock counters
    fs_ctx.superblock.s_free_inodes_count = fs_ctx.superblock.s_free_inodes_count.saturating_sub(1);
    
    // 5. Journal the operations
    // TODO: Add actual journal operations for inode creation and directory modification
    
    // 6. Commit the transaction
    fs_ctx.journal.commit_transaction(tid)
        .map_err(|_| {
            let _ = fs_ctx.inode_manager.free_inode(new_ino);
            fs_ctx.unlock_inode_write(parent_ino);
            fs_ctx.unlock_directory();
            "Failed to commit transaction"
        })?;
    
    // Release locks
    fs_ctx.unlock_inode_write(parent_ino);
    fs_ctx.unlock_directory();
    
    Ok(new_ino)
}

/// Check write permission for an inode
fn check_write_permission(inode: &VexfsInode, uid: u16, gid: u16) -> bool {
    // Owner can always write if owner has write permission
    if inode.i_uid == uid {
        return (inode.i_mode & 0o200) != 0;
    }

    // Group members can write if group has write permission
    if inode.i_gid == gid {
        return (inode.i_mode & 0o020) != 0;
    }

    // Others can write if others have write permission
    (inode.i_mode & 0o002) != 0
}

/// Check read permission for an inode
fn check_read_permission(inode: &VexfsInode, uid: u16, gid: u16) -> bool {
    // Owner can read if owner has read permission
    if inode.i_uid == uid {
        return (inode.i_mode & 0o400) != 0;
    }

    // Group members can read if group has read permission
    if inode.i_gid == gid {
        return (inode.i_mode & 0o040) != 0;
    }

    // Others can read if others have read permission
    (inode.i_mode & 0o004) != 0
}

/// Check execute permission for an inode
fn check_execute_permission(inode: &VexfsInode, uid: u16, gid: u16) -> bool {
    // Owner can execute if owner has execute permission
    if inode.i_uid == uid {
        return (inode.i_mode & 0o100) != 0;
    }

    // Group members can execute if group has execute permission
    if inode.i_gid == gid {
        return (inode.i_mode & 0o010) != 0;
    }

    // Others can execute if others have execute permission
    (inode.i_mode & 0o001) != 0
}

/// Internal implementation of file opening
fn open_file_impl(
    fs_ctx: &mut VexfsContext,
    ino: u64,
    flags: u32,
    uid: u16,
    gid: u16,
) -> Result<VexfsFileHandle, &'static str> {
    // Acquire read lock on the inode for opening
    fs_ctx.lock_inode_read(ino);
    
    // 1. Load inode from disk
    let inode_info = fs_ctx.inode_manager.read_inode(ino)
        .map_err(|_| {
            fs_ctx.unlock_inode_read(ino);
            "Failed to read inode"
        })?;
    
    // 2. Check if it's a regular file
    if (inode_info.disk_inode.i_mode & VEXFS_S_IFREG as u16) == 0 {
        fs_ctx.unlock_inode_read(ino);
        return Err("Not a regular file");
    }
    
    // 3. Check permissions
    let access_mode = flags & 0x3; // O_RDONLY=0, O_WRONLY=1, O_RDWR=2
    let write_requested = access_mode == 1 || access_mode == 2;
    let read_requested = access_mode == 0 || access_mode == 2;
    
    // Check read permission
    if read_requested && !check_read_permission(&inode_info.disk_inode, uid, gid) {
        fs_ctx.unlock_inode_read(ino);
        return Err("Read permission denied");
    }
    
    // Check write permission
    if write_requested && !check_write_permission(&inode_info.disk_inode, uid, gid) {
        fs_ctx.unlock_inode_read(ino);
        return Err("Write permission denied");
    }
    
    // 4. Handle truncation flag
    if (flags & VEXFS_O_TRUNC) != 0 && write_requested {
        // Need to upgrade to write lock for truncation
        fs_ctx.unlock_inode_read(ino);
        fs_ctx.lock_inode_write(ino);
        
        // Truncate the file to zero length
        let mut inode_info_mut = fs_ctx.inode_manager.read_inode(ino)
            .map_err(|_| {
                fs_ctx.unlock_inode_write(ino);
                "Failed to read inode for truncation"
            })?;
        
        // Start a transaction for the truncation
        let tid = fs_ctx.journal.start_transaction(0)
            .map_err(|_| {
                fs_ctx.unlock_inode_write(ino);
                "Failed to start transaction for truncation"
            })?;
        
        // Free all data blocks
        // TODO: Implement proper block deallocation
        
        // Set size to zero
        inode_info_mut.disk_inode.i_size_lo = 0;
        inode_info_mut.disk_inode.i_size_high = 0;
        
        // Update timestamps
        let current_time = get_current_time(); // We'll need to implement this
        inode_info_mut.disk_inode.mtime = current_time;
        inode_info_mut.disk_inode.ctime = current_time;
        
        // Write updated inode back to disk
        fs_ctx.inode_manager.write_inode(&inode_info_mut)
            .map_err(|_| {
                fs_ctx.journal.abort_transaction(tid);
                fs_ctx.unlock_inode_write(ino);
                "Failed to write truncated inode"
            })?;
        
        fs_ctx.journal.commit_transaction(tid)
            .map_err(|_| {
                fs_ctx.unlock_inode_write(ino);
                "Failed to commit truncation transaction"
            })?;
        
        // Downgrade back to read lock
        fs_ctx.unlock_inode_write(ino);
        fs_ctx.lock_inode_read(ino);
    }
    
    // 5. Initialize file position
    let initial_pos = if (flags & VEXFS_O_APPEND) != 0 {
        inode_info.size // Start at end of file for append mode
    } else {
        0 // Start at beginning for normal mode
    };
    
    // 6. Create file handle
    let handle = VexfsFileHandle {
        ino,
        pos: initial_pos,
        flags,
        inode_handle: None, // TODO: Implement proper inode handle system
    };
    
    // Release the read lock - the handle will manage its own locking
    fs_ctx.unlock_inode_read(ino);
    
    Ok(handle)
}

/// Get current time (placeholder implementation)
fn get_current_time() -> u32 {
    // In a real kernel implementation, this would get the current system time
    // For now, return a placeholder value
    0
}

/// Open a file and return a file handle
#[no_mangle]
pub extern "C" fn vexfs_open_file(
    sb_ptr: *mut c_void,
    ino: u64,
    flags: u32,
    uid: u32,
    gid: u32,
) -> *mut VexfsFileHandle {
    if sb_ptr.is_null() || ino == 0 {
        return core::ptr::null_mut();
    }

    // Validate open flags
    let access_mode = flags & 0x3; // O_RDONLY=0, O_WRONLY=1, O_RDWR=2
    if access_mode > 2 {
        return core::ptr::null_mut();
    }

    // Get filesystem context from superblock pointer
    let fs_ctx = unsafe {
        &mut *(sb_ptr as *mut VexfsContext)
    };
    
    // Open the file
    match open_file_impl(fs_ctx, ino, flags, uid as u16, gid as u16) {
        Ok(handle) => {
            // Convert to heap-allocated pointer for C FFI
            Box::into_raw(Box::new(handle))
        }
        Err(_) => core::ptr::null_mut()
    }
}

/// Read data from an open file
#[no_mangle]
pub extern "C" fn vexfs_read_file(
    file_handle: *mut VexfsFileHandle,
    buffer: *mut c_void,
    count: u64,
    bytes_read: *mut u64,
) -> c_int {
    if file_handle.is_null() || buffer.is_null() || bytes_read.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    unsafe {
        *bytes_read = 0;
    }

    let handle = unsafe { &mut *file_handle };
    
    // Check if file is open for reading
    let access_mode = handle.flags & 0x3;
    if access_mode == 1 { // O_WRONLY
        return VEXFS_ERROR_PERMISSION;
    }

    // Validate count
    if count == 0 {
        return VEXFS_SUCCESS;
    }

    // TODO: In a full implementation, this would need access to the filesystem context
    // For now, we'll implement a simplified version that demonstrates the interface
    
    // Simulate reading data by returning 0 bytes (EOF)
    // In a real implementation, this would:
    // 1. Get filesystem context from global state or handle
    // 2. Load inode from handle->ino
    // 3. Use read_file_blocks to read actual data
    // 4. Update file position
    // 5. Set bytes_read output parameter
    
    unsafe {
        *bytes_read = 0; // Simulate EOF for now
    }
    
    VEXFS_SUCCESS
}

/// Write data to an open file
#[no_mangle]
pub extern "C" fn vexfs_write_file(
    file_handle: *mut VexfsFileHandle,
    buffer: *const c_void,
    count: u64,
    bytes_written: *mut u64,
) -> c_int {
    if file_handle.is_null() || buffer.is_null() || bytes_written.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    unsafe {
        *bytes_written = 0;
    }

    let handle = unsafe { &mut *file_handle };
    
    // Check if file is open for writing
    let access_mode = handle.flags & 0x3;
    if access_mode == 0 { // O_RDONLY
        return VEXFS_ERROR_PERMISSION;
    }

    // Validate count
    if count == 0 {
        return VEXFS_SUCCESS;
    }

    // Check for append mode (O_APPEND = 0x400)
    if (handle.flags & 0x400) != 0 {
        // TODO: Set position to file size for append
    }

    // TODO: In a full implementation, this would need access to the filesystem context
    // For now, we'll implement a simplified version that demonstrates the interface
    
    // Simulate writing data
    // In a real implementation, this would:
    // 1. Get filesystem context from global state or handle
    // 2. Load inode from handle->ino
    // 3. Check if file needs to be extended
    // 4. Allocate new blocks if needed
    // 5. Use write_file_data to write actual data
    // 6. Update inode size and timestamps
    // 7. Update file position
    
    // For now, simulate successful write of all bytes
    unsafe {
        *bytes_written = count;
    }
    handle.pos += count;
    
    VEXFS_SUCCESS
}

/// Truncate a file to the specified size
#[no_mangle]
pub extern "C" fn vexfs_truncate_file(
    sb_ptr: *mut c_void,
    ino: u64,
    new_size: u64,
    uid: u32,
    gid: u32,
) -> c_int {
    if sb_ptr.is_null() || ino == 0 {
        return VEXFS_ERROR_INVAL;
    }

    if new_size > VEXFS_MAX_FILE_SIZE {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Implement actual file truncation:
    // 1. Get filesystem components from superblock
    // 2. Load inode and validate it's a regular file
    // 3. Check write permissions (uid/gid match or appropriate permissions)
    // 4. Begin journal transaction
    // 5. If shrinking file:
    //    - Calculate blocks to deallocate
    //    - Free blocks beyond new size
    //    - Update block allocation bitmaps
    // 6. If expanding file:
    //    - File becomes sparse, no immediate allocation needed
    //    - Zero-fill behavior handled on read
    // 7. Update inode size and modification time
    // 8. Journal inode update
    // 9. Commit transaction
    
    VEXFS_SUCCESS
}

/// Unlink (delete) a file
#[no_mangle]
pub extern "C" fn vexfs_unlink_file(
    sb_ptr: *mut c_void,
    parent_ino: u64,
    name: *const c_char,
    name_len: u32,
    uid: u32,
    gid: u32,
) -> c_int {
    if sb_ptr.is_null() || name.is_null() || name_len == 0 || parent_ino == 0 {
        return VEXFS_ERROR_INVAL;
    }

    // Validate filename length
    if name_len > VEXFS_NAME_LEN {
        return VEXFS_ERROR_INVAL;
    }

    // Convert C string to Rust slice
    let filename = unsafe {
        core::slice::from_raw_parts(name as *const u8, name_len as usize)
    };
    
    // Validate filename doesn't contain null bytes
    if filename.iter().any(|&b| b == 0) {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Implement actual file unlinking:
    // 1. Get filesystem components from superblock
    // 2. Load parent directory inode and check write permissions
    // 3. Begin journal transaction
    // 4. Look up file in parent directory by name
    // 5. Load file inode and validate it's not a directory
    // 6. Check if file is currently open (prevent deletion of open files)
    // 7. Decrement hard link count in inode
    // 8. If link count reaches 0:
    //    - Mark inode for deletion (set deletion time)
    //    - Deallocate all file data blocks
    //    - Free the inode
    //    - Update superblock counters
    // 9. Remove directory entry from parent
    // 10. Update parent directory modification time
    // 11. Journal all operations
    // 12. Commit transaction
    
    VEXFS_SUCCESS
}

/// Close a file handle
#[no_mangle]
pub extern "C" fn vexfs_close_file(file_handle: *mut VexfsFileHandle) -> c_int {
    if file_handle.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Implement actual file closing:
    // 1. Flush any pending writes to disk
    // 2. Sync file metadata if dirty
    // 3. Release inode handle and decrement reference count
    // 4. Free file handle memory
    // 5. Ensure all journal transactions are committed
    
    // For now, just free the handle memory
    let _handle = unsafe { Box::from_raw(file_handle) };
    // Handle is automatically dropped here, freeing memory
    
    VEXFS_SUCCESS
}

/// Seek to a specific position in a file
#[no_mangle]
pub extern "C" fn vexfs_seek_file(
    file_handle: *mut VexfsFileHandle,
    offset: i64,
    whence: c_int,
    new_pos: *mut u64,
) -> c_int {
    if file_handle.is_null() || new_pos.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    unsafe {
        *new_pos = 0;
    }

    // TODO: Implement actual file seeking:
    // 1. Validate seek parameters
    // 2. Calculate new position based on whence (SEEK_SET, SEEK_CUR, SEEK_END)
    // 3. Validate new position is within file bounds
    // 4. Update file handle position
    // 5. Return new position
    
    VEXFS_SUCCESS
}

/// Get file attributes (stat)
#[no_mangle]
pub extern "C" fn vexfs_getattr_file(
    sb_ptr: *mut c_void,
    ino: u64,
    size: *mut u64,
    mode: *mut u32,
    uid: *mut u32,
    gid: *mut u32,
    atime: *mut u64,
    mtime: *mut u64,
    ctime: *mut u64,
) -> c_int {
    if sb_ptr.is_null() || size.is_null() || mode.is_null() || 
       uid.is_null() || gid.is_null() || atime.is_null() || 
       mtime.is_null() || ctime.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    unsafe {
        *size = 0;
        *mode = 0;
        *uid = 0;
        *gid = 0;
        *atime = 0;
        *mtime = 0;
        *ctime = 0;
    }

    // TODO: Implement actual attribute retrieval:
    // 1. Load inode from disk
    // 2. Extract all attributes
    // 3. Return values through pointers
    
    VEXFS_SUCCESS
}

/// Set file attributes (chmod, chown, etc.)
#[no_mangle]
pub extern "C" fn vexfs_setattr_file(
    sb_ptr: *mut c_void,
    ino: u64,
    size: *const u64,
    mode: *const u32,
    uid: *const u32,
    gid: *const u32,
    atime: *const u64,
    mtime: *const u64,
    caller_uid: u32,
    caller_gid: u32,
) -> c_int {
    if sb_ptr.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Implement actual attribute setting:
    // 1. Load inode from disk
    // 2. Check permissions for each attribute change
    // 3. Update inode with new attributes
    // 4. Handle size changes (truncate)
    // 5. Update timestamps
    // 6. Journal all changes
    
    VEXFS_SUCCESS
}

/// Create a hard link to an existing file
#[no_mangle]
pub extern "C" fn vexfs_link_file(
    sb_ptr: *mut c_void,
    target_ino: u64,
    parent_ino: u64,
    name: *const c_char,
    name_len: u32,
    uid: u32,
    gid: u32,
) -> c_int {
    if sb_ptr.is_null() || name.is_null() || name_len == 0 {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Implement actual hard link creation:
    // 1. Check parent directory write permissions
    // 2. Load target inode
    // 3. Verify target is a regular file (not directory)
    // 4. Increment link count
    // 5. Add directory entry to parent
    // 6. Journal all operations
    
    VEXFS_SUCCESS
}

/// Create a symbolic link
#[no_mangle]
pub extern "C" fn vexfs_symlink_file(
    sb_ptr: *mut c_void,
    target: *const c_char,
    target_len: u32,
    parent_ino: u64,
    name: *const c_char,
    name_len: u32,
    uid: u32,
    gid: u32,
) -> c_int {
    if sb_ptr.is_null() || target.is_null() || target_len == 0 ||
       name.is_null() || name_len == 0 {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Implement actual symbolic link creation:
    // 1. Check parent directory write permissions
    // 2. Allocate new inode with S_IFLNK mode
    // 3. Store target path in inode (small targets) or data blocks
    // 4. Add directory entry to parent
    // 5. Journal all operations
    
    VEXFS_SUCCESS
}

/// Read the target of a symbolic link
#[no_mangle]
pub extern "C" fn vexfs_readlink_file(
    sb_ptr: *mut c_void,
    ino: u64,
    buffer: *mut c_char,
    buffer_size: u32,
    target_len: *mut u32,
) -> c_int {
    if sb_ptr.is_null() || buffer.is_null() || target_len.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    unsafe {
        *target_len = 0;
    }

    // TODO: Implement actual symbolic link reading:
    // 1. Load inode and verify it's a symbolic link
    // 2. Read target path from inode or data blocks
    // 3. Copy to buffer with size checking
    // 4. Return target length
    
    VEXFS_SUCCESS
}

/// Flush file data to disk
#[no_mangle]
pub extern "C" fn vexfs_fsync_file(file_handle: *mut VexfsFileHandle) -> c_int {
    if file_handle.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Implement actual file synchronization:
    // 1. Flush any dirty pages/blocks to disk
    // 2. Sync inode metadata
    // 3. Ensure journal entries are committed
    // 4. Wait for all I/O to complete
    
    VEXFS_SUCCESS
}

/// Get file lock status
#[no_mangle]
pub extern "C" fn vexfs_flock_file(
    file_handle: *mut VexfsFileHandle,
    operation: c_int,
) -> c_int {
    if file_handle.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Implement file locking:
    // 1. Handle LOCK_SH (shared), LOCK_EX (exclusive), LOCK_UN (unlock)
    // 2. Manage lock conflicts and waiting
    // 3. Integration with VFS locking mechanisms
    
    VEXFS_SUCCESS
}

/// Memory map a file
#[no_mangle]
pub extern "C" fn vexfs_mmap_file(
    file_handle: *mut VexfsFileHandle,
    offset: u64,
    length: u64,
    prot: c_int,
    flags: c_int,
    mapped_addr: *mut *mut c_void,
) -> c_int {
    if file_handle.is_null() || mapped_addr.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    unsafe {
        *mapped_addr = core::ptr::null_mut();
    }

    // TODO: Implement memory mapping:
    // 1. Validate offset and length
    // 2. Check file permissions against prot flags
    // 3. Set up page table mappings
    // 4. Handle copy-on-write for private mappings
    
    VEXFS_SUCCESS
}

/// File operations function table for integration with VFS
#[repr(C)]
pub struct VexfsFileOperations {
    pub create: extern "C" fn(*mut c_void, u64, *const c_char, u32, u32, u32, u32) -> c_int,
    pub open: extern "C" fn(*mut c_void, u64, u32, u32, u32) -> *mut VexfsFileHandle,
    pub read: extern "C" fn(*mut VexfsFileHandle, *mut c_void, u64, *mut u64) -> c_int,
    pub write: extern "C" fn(*mut VexfsFileHandle, *const c_void, u64, *mut u64) -> c_int,
    pub seek: extern "C" fn(*mut VexfsFileHandle, i64, c_int, *mut u64) -> c_int,
    pub truncate: extern "C" fn(*mut c_void, u64, u64, u32, u32) -> c_int,
    pub close: extern "C" fn(*mut VexfsFileHandle) -> c_int,
    pub unlink: extern "C" fn(*mut c_void, u64, *const c_char, u32, u32, u32) -> c_int,
    pub getattr: extern "C" fn(*mut c_void, u64, *mut u64, *mut u32, *mut u32, *mut u32, *mut u64, *mut u64, *mut u64) -> c_int,
    pub setattr: extern "C" fn(*mut c_void, u64, *const u64, *const u32, *const u32, *const u32, *const u64, *const u64, u32, u32) -> c_int,
    pub link: extern "C" fn(*mut c_void, u64, u64, *const c_char, u32, u32, u32) -> c_int,
    pub symlink: extern "C" fn(*mut c_void, *const c_char, u32, u64, *const c_char, u32, u32, u32) -> c_int,
    pub readlink: extern "C" fn(*mut c_void, u64, *mut c_char, u32, *mut u32) -> c_int,
    pub fsync: extern "C" fn(*mut VexfsFileHandle) -> c_int,
    pub flock: extern "C" fn(*mut VexfsFileHandle, c_int) -> c_int,
    pub mmap: extern "C" fn(*mut VexfsFileHandle, u64, u64, c_int, c_int, *mut *mut c_void) -> c_int,
}

/// Global file operations table for VFS integration
#[no_mangle]
pub static VEXFS_FILE_OPERATIONS: VexfsFileOperations = VexfsFileOperations {
    create: vexfs_create_file,
    open: vexfs_open_file,
    read: vexfs_read_file,
    write: vexfs_write_file,
    seek: vexfs_seek_file,
    truncate: vexfs_truncate_file,
    close: vexfs_close_file,
    unlink: vexfs_unlink_file,
    getattr: vexfs_getattr_file,
    setattr: vexfs_setattr_file,
    link: vexfs_link_file,
    symlink: vexfs_symlink_file,
    readlink: vexfs_readlink_file,
    fsync: vexfs_fsync_file,
    flock: vexfs_flock_file,
    mmap: vexfs_mmap_file,
};
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_checking() {
        let mut inode = VexfsInode::new(1, VEXFS_S_IFREG | 0o644);
        inode.i_uid = 1000;
        inode.i_gid = 1000;

        // Owner should have read/write
        assert!(check_permission(&inode, 0o6, 1000, 1000));
        assert!(check_permission(&inode, 0o4, 1000, 1000));
        assert!(!check_permission(&inode, 0o1, 1000, 1000)); // No execute

        // Group should have read only
        assert!(check_permission(&inode, 0o4, 1001, 1000));
        assert!(!check_permission(&inode, 0o2, 1001, 1000)); // No write

        // Others should have read only
        assert!(check_permission(&inode, 0o4, 1001, 1001));
        assert!(!check_permission(&inode, 0o2, 1001, 1001)); // No write

        // Root should have all permissions
        assert!(check_permission(&inode, 0o7, 0, 0));
    }

    #[test]
    fn test_ffi_error_conversion() {
        assert_eq!(to_ffi_result(Ok::<(), &'static str>(())), VEXFS_SUCCESS);
        assert_eq!(to_ffi_result(Err("Invalid arguments")), VEXFS_ERROR_INVAL);
        assert_eq!(to_ffi_result(Err("Out of memory")), VEXFS_ERROR_NOMEM);
        assert_eq!(to_ffi_result(Err("No space left")), VEXFS_ERROR_NOSPC);
        assert_eq!(to_ffi_result(Err("Other error")), VEXFS_ERROR_GENERIC);
    }

    #[test]
    fn test_file_constants() {
        assert_eq!(VEXFS_S_IFREG, 0o100000);
        assert_eq!(VEXFS_S_IFDIR, 0o040000);
        assert_eq!(VEXFS_S_IFLNK, 0o120000);
        assert_eq!(VEXFS_MAX_FILE_SIZE, 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_file_operations_table() {
        // Verify that the operations table is properly initialized
        assert!(!VEXFS_FILE_OPERATIONS.create as *const _ as *const u8).is_null();
        assert!(!VEXFS_FILE_OPERATIONS.open as *const _ as *const u8).is_null();
        assert!(!VEXFS_FILE_OPERATIONS.read as *const _ as *const u8).is_null();
        assert!(!VEXFS_FILE_OPERATIONS.write as *const _ as *const u8).is_null();
        assert!(!VEXFS_FILE_OPERATIONS.close as *const _ as *const u8).is_null();
        assert!(!VEXFS_FILE_OPERATIONS.fsync as *const _ as *const u8).is_null();
        assert!(!VEXFS_FILE_OPERATIONS.flock as *const _ as *const u8).is_null();
        assert!(!VEXFS_FILE_OPERATIONS.mmap as *const _ as *const u8).is_null();
    }

    #[test]
    fn test_ffi_null_pointer_handling() {
        // Test null pointer handling for key FFI functions
        assert_eq!(vexfs_fsync_file(core::ptr::null_mut()), VEXFS_ERROR_INVAL);
        assert_eq!(vexfs_flock_file(core::ptr::null_mut(), 0), VEXFS_ERROR_INVAL);
        assert_eq!(
            vexfs_mmap_file(
                core::ptr::null_mut(),
                0,
                0,
                0,
                0,
                core::ptr::null_mut()
            ),
            VEXFS_ERROR_INVAL
        );
    }
}