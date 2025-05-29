//! Directory Management System for VexFS
//! 
//! This module implements directory entry management and lookup operations
//! for VexFS directories, including creation, deletion, and traversal.



use crate::ondisk::*;
use crate::inode_mgmt::*;
use crate::space_alloc::*;
use crate::journal::*;
use core::sync::atomic::{AtomicU32, Ordering};
use core::ffi::{c_char, c_int, c_uint, c_void};


// Lock-related constants for directory operations
const DIR_LOCK_READ: u32 = 1;
const DIR_LOCK_WRITE: u32 = 2;

/// Directory operations error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DirOpError {
    NotFound,
    AlreadyExists,
    NotDirectory,
    DirectoryNotEmpty,
    NameTooLong,
    InvalidName,
    NoSpace,
    IoError,
    PermissionDenied,
    CrossDevice,
    IsDirectory,
    TooManyLinks,
}

/// Directory handle for managing open directories
#[derive(Debug)]
pub struct VexfsDirHandle {
    /// Associated inode
    pub inode: VexfsInodeInfo,
    
    /// Current position in directory
    pub pos: u64,
    
    /// Open flags
    pub flags: u32,
    
    /// Reference count
    pub ref_count: u32,
    
    /// Cached directory entries
    pub cached_entries: [VexfsDirEntry; VEXFS_DIR_ENTRIES_PER_BLOCK],
    
    /// Number of cached entries
    pub cached_count: usize,
    
    /// Current block being read
    pub current_block: u64,
    
    /// Lock for concurrent access
    pub lock: AtomicU32,
    
    /// Journal transaction ID for consistency
    pub journal_tid: Option<u64>,
}

/// Directory operations manager
pub struct VexfsDirOps {
    /// Block size for directory operations
    pub block_size: u32,
    
    /// Maximum filename length
    pub max_name_len: u16,
    
    /// Inode manager reference
    pub inode_manager: VexfsInodeManager,
    
    /// Space allocator for block allocation
    pub space_allocator: VexfsSpaceAllocator,
    
    /// Journal manager for consistency
    pub journal_manager: VexfsJournal,
}

/// Directory entry iterator
pub struct DirEntryIterator {
    /// Directory handle
    pub dir_handle: VexfsDirHandle,
    
    /// Current position in directory
    pub pos: u64,
    
    /// End position
    pub end_pos: u64,
}

/// Directory lookup result
#[derive(Debug)]
pub struct LookupResult {
    /// Found inode number
    pub ino: u64,
    
    /// File type
    pub file_type: u8,
    
    /// Entry offset in directory
    pub offset: u32,
    
    /// Entry length
    pub rec_len: u16,
}

impl VexfsDirHandle {
    /// Create a new directory handle
    pub fn new(inode: VexfsInodeInfo, flags: u32) -> Result<Self, DirOpError> {
        if !inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        Ok(Self {
            inode,
            pos: 0,
            flags,
            ref_count: 1,
            cached_entries: [VexfsDirEntry::new(); VEXFS_DIR_ENTRIES_PER_BLOCK],
            cached_count: 0,
            current_block: 0,
            lock: AtomicU32::new(0),
            journal_handle: None,
        })
    }
    
    /// Acquire read lock on directory
    pub fn lock_read(&self) -> Result<(), DirOpError> {
        let mut current = self.lock.load(Ordering::Acquire);
        loop {
            if (current & DIR_LOCK_WRITE) != 0 {
                // Write lock held, wait
                return Err(DirOpError::IoError);
            }
            
            let new_value = current + DIR_LOCK_READ;
            match self.lock.compare_exchange_weak(current, new_value, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => return Ok(()),
                Err(val) => current = val,
            }
        }
    }
    
    /// Acquire write lock on directory
    pub fn lock_write(&self) -> Result<(), DirOpError> {
        let expected = 0;
        match self.lock.compare_exchange(expected, DIR_LOCK_WRITE, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => Ok(()),
            Err(_) => Err(DirOpError::IoError),
        }
    }
    
    /// Release read lock on directory
    pub fn unlock_read(&self) {
        self.lock.fetch_sub(DIR_LOCK_READ, Ordering::AcqRel);
    }
    
    /// Release write lock on directory
    pub fn unlock_write(&self) {
        self.lock.store(0, Ordering::Release);
    }
    
    /// Get current directory position
    pub fn position(&self) -> u64 {
        self.pos
    }
    
    /// Seek to a specific position in directory
    pub fn seek(&mut self, pos: u64) -> Result<u64, DirOpError> {
        if pos > self.inode.size() {
            return Err(DirOpError::InvalidName);
        }
        
        self.pos = pos;
        
        // Clear cache if we're moving to a different block
        let new_block = pos / self.block_size() as u64;
        if new_block != self.current_block {
            self.invalidate_cache();
        }
        
        Ok(pos)
    }
    
    /// Get block size for this directory
    pub fn block_size(&self) -> u32 {
        VEXFS_DEFAULT_BLOCK_SIZE
    }
    
    /// Invalidate cached entries
    pub fn invalidate_cache(&mut self) {
        self.cached_count = 0;
        self.current_block = u64::MAX;
    }
    
    /// Increment reference count
    pub fn get(&mut self) -> u32 {
        self.ref_count += 1;
        self.ref_count
    }
    
    /// Decrement reference count
    pub fn put(&mut self) -> u32 {
        if self.ref_count > 0 {
            self.ref_count -= 1;
        }
        self.ref_count
    }
}

impl VexfsDirOps {
    /// Create a new directory operations manager
    pub fn new(block_size: u32, inode_manager: VexfsInodeManager, space_allocator: VexfsSpaceAllocator, journal_manager: VexfsJournal) -> Self {
        Self {
            block_size,
            max_name_len: VEXFS_MAX_FILENAME_LEN,
            inode_manager,
            space_allocator,
            journal_manager,
        }
    }
    
    /// Open a directory and create a directory handle
    pub fn opendir(&self, inode: VexfsInodeInfo, flags: u32) -> Result<VexfsDirHandle, DirOpError> {
        // Check permissions
        let uid = 0; // In real implementation, get from current task
        let gid = 0; // In real implementation, get from current task
        
        if !inode.check_permission(uid, gid, 0o4) { // Read permission
            return Err(DirOpError::PermissionDenied);
        }
        
        VexfsDirHandle::new(inode, flags)
    }
    
    /// Create a new directory
    pub fn mkdir(&mut self, parent_inode: &mut VexfsInodeInfo, name: &str, mode: u16, uid: u32, gid: u32) -> Result<VexfsInodeInfo, DirOpError> {
        if name.len() > self.max_name_len as usize {
            return Err(DirOpError::NameTooLong);
        }
        
        if !self.is_valid_name(name) {
            return Err(DirOpError::InvalidName);
        }
        
        // Check if entry already exists
        if self.lookup_entry(parent_inode, name)?.is_some() {
            return Err(DirOpError::AlreadyExists);
        }
        
        // Check permissions in parent directory
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !parent_inode.check_permission(current_uid, current_gid, 0o2) { // Write permission
            return Err(DirOpError::PermissionDenied);
        }
        
        // Start journal transaction for atomic directory creation
        let mut transaction = self.journal_manager.begin_transaction()
            .map_err(|_| DirOpError::IoError)?;
        
        // Create new inode for the directory
        let mut new_inode = self.inode_manager.create_inode(
            S_IFDIR | (mode & 0o777),
            uid as u16,
            gid as u16
        ).map_err(|_| DirOpError::NoSpace)?;
        
        // Allocate initial block for directory
        let dir_block = self.space_allocator.alloc_block()
            .map_err(|_| DirOpError::NoSpace)?;
        
        // Set the directory block in inode
        new_inode.set_block(0, dir_block);
        new_inode.set_size(self.block_size as u64);
        
        // Log inode creation
        transaction.log_inode_create(new_inode.ino, &new_inode.disk_inode)
            .map_err(|_| DirOpError::IoError)?;
        
        // Initialize directory with "." and ".." entries
        self.init_directory(&mut new_inode, parent_inode.ino)?;
        
        // Add entry to parent directory
        self.add_entry(parent_inode, name, new_inode.ino, DT_DIR)?;
        
        // Log parent directory modification
        transaction.log_inode_update(parent_inode.ino, &parent_inode.disk_inode)
            .map_err(|_| DirOpError::IoError)?;
        
        // Update parent's link count and times
        parent_inode.disk_inode.i_links_count += 1;
        parent_inode.touch_mtime(0); // In real implementation, use current time
        parent_inode.touch_ctime(0);
        
        // Commit the transaction
        transaction.commit().map_err(|_| DirOpError::IoError)?;
        
        Ok(new_inode)
    }
    
    /// Remove a directory
    pub fn rmdir(&mut self, parent_inode: &mut VexfsInodeInfo, name: &str) -> Result<(), DirOpError> {
        if !self.is_valid_name(name) {
            return Err(DirOpError::InvalidName);
        }
        
        // Find the directory entry
        let lookup_result = self.lookup_entry(parent_inode, name)?
            .ok_or(DirOpError::NotFound)?;
        
        // Check permissions in parent directory
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !parent_inode.check_permission(current_uid, current_gid, 0o2) { // Write permission
            return Err(DirOpError::PermissionDenied);
        }
        
        // Read the target inode
        let target_inode = self.inode_manager.read_inode(lookup_result.ino)
            .map_err(|_| DirOpError::NotFound)?;
        
        if !target_inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        // Check if directory is empty (only contains "." and "..")
        if !self.is_empty_directory(&target_inode)? {
            return Err(DirOpError::DirectoryNotEmpty);
        }
        
        // Start journal transaction for atomic directory removal
        let mut transaction = self.journal_manager.begin_transaction()
            .map_err(|_| DirOpError::IoError)?;
        
        // Log directory removal
        transaction.log_inode_delete(lookup_result.ino, &target_inode.disk_inode)
            .map_err(|_| DirOpError::IoError)?;
        
        // Free directory blocks
        for block_idx in 0..target_inode.get_block_count() {
            if let Some(block_num) = target_inode.get_block(block_idx) {
                self.space_allocator.free_block(block_num)
                    .map_err(|_| DirOpError::IoError)?;
            }
        }
        
        // Remove entry from parent directory
        self.remove_entry(parent_inode, name)?;
        
        // Log parent directory modification
        transaction.log_inode_update(parent_inode.ino, &parent_inode.disk_inode)
            .map_err(|_| DirOpError::IoError)?;
        
        // Free the inode
        self.inode_manager.free_inode(lookup_result.ino)
            .map_err(|_| DirOpError::IoError)?;
        
        // Update parent's link count and times
        if parent_inode.disk_inode.i_links_count > 0 {
            parent_inode.disk_inode.i_links_count -= 1;
        }
        parent_inode.touch_mtime(0);
        parent_inode.touch_ctime(0);
        
        // Commit the transaction
        transaction.commit().map_err(|_| DirOpError::IoError)?;
        
        Ok(())
    }
    
    /// Rename a directory entry
    pub fn rename(&mut self, old_parent: &mut VexfsInodeInfo, old_name: &str,
                  new_parent: &mut VexfsInodeInfo, new_name: &str) -> Result<(), DirOpError> {
        if !self.is_valid_name(old_name) || !self.is_valid_name(new_name) {
            return Err(DirOpError::InvalidName);
        }
        
        if new_name.len() > self.max_name_len as usize {
            return Err(DirOpError::NameTooLong);
        }
        
        // Check permissions
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !old_parent.check_permission(current_uid, current_gid, 0o2) {
            return Err(DirOpError::PermissionDenied);
        }
        
        if old_parent.ino != new_parent.ino && !new_parent.check_permission(current_uid, current_gid, 0o2) {
            return Err(DirOpError::PermissionDenied);
        }
        
        // Find the old entry
        let old_entry = self.lookup_entry(old_parent, old_name)?
            .ok_or(DirOpError::NotFound)?;
        
        // Check if new name already exists
        if self.lookup_entry(new_parent, new_name)?.is_some() {
            return Err(DirOpError::AlreadyExists);
        }
        
        // Start journal transaction for atomic rename
        let mut transaction = self.journal_manager.begin_transaction()
            .map_err(|_| DirOpError::IoError)?;
        
        // Add entry with new name
        self.add_entry(new_parent, new_name, old_entry.ino, old_entry.file_type)?;
        
        // Remove old entry
        self.remove_entry(old_parent, old_name)?;
        
        // Update timestamps
        old_parent.touch_mtime(0);
        old_parent.touch_ctime(0);
        
        if old_parent.ino != new_parent.ino {
            new_parent.touch_mtime(0);
            new_parent.touch_ctime(0);
            
            // Log new parent modification
            transaction.log_inode_update(new_parent.ino, &new_parent.disk_inode)
                .map_err(|_| DirOpError::IoError)?;
        }
        
        // Log old parent modification
        transaction.log_inode_update(old_parent.ino, &old_parent.disk_inode)
            .map_err(|_| DirOpError::IoError)?;
        
        // Commit the transaction
        transaction.commit().map_err(|_| DirOpError::IoError)?;
        
        Ok(())
    }
    
    /// Create a regular file in a directory
    pub fn create_file(&mut self, parent_inode: &mut VexfsInodeInfo, name: &str, mode: u16, uid: u32, gid: u32) -> Result<VexfsInodeInfo, DirOpError> {
        if name.len() > self.max_name_len as usize {
            return Err(DirOpError::NameTooLong);
        }
        
        if !self.is_valid_name(name) {
            return Err(DirOpError::InvalidName);
        }
        
        // Check if entry already exists
        if self.lookup_entry(parent_inode, name)?.is_some() {
            return Err(DirOpError::AlreadyExists);
        }
        
        // Check permissions in parent directory
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !parent_inode.check_permission(current_uid, current_gid, 0o2) { // Write permission
            return Err(DirOpError::PermissionDenied);
        }
        
        // Start journal transaction for atomic file creation
        let mut transaction = self.journal_manager.begin_transaction()
            .map_err(|_| DirOpError::IoError)?;
        
        // Create new inode for the file
        let new_inode = self.inode_manager.create_inode(
            S_IFREG | (mode & 0o777),
            uid as u16,
            gid as u16
        ).map_err(|_| DirOpError::NoSpace)?;
        
        // Log inode creation
        transaction.log_inode_create(new_inode.ino, &new_inode.disk_inode)
            .map_err(|_| DirOpError::IoError)?;
        
        // Add entry to parent directory
        self.add_entry(parent_inode, name, new_inode.ino, DT_REG)?;
        
        // Log parent directory modification
        transaction.log_inode_update(parent_inode.ino, &parent_inode.disk_inode)
            .map_err(|_| DirOpError::IoError)?;
        
        // Update parent's times
        parent_inode.touch_mtime(0);
        parent_inode.touch_ctime(0);
        
        // Commit the transaction
        transaction.commit().map_err(|_| DirOpError::IoError)?;
        
        Ok(new_inode)
    }
    
    /// Remove a file from a directory
    pub fn unlink(&mut self, parent_inode: &mut VexfsInodeInfo, name: &str) -> Result<(), DirOpError> {
        if !self.is_valid_name(name) {
            return Err(DirOpError::InvalidName);
        }
        
        // Find the file entry
        let lookup_result = self.lookup_entry(parent_inode, name)?
            .ok_or(DirOpError::NotFound)?;
        
        // Check permissions in parent directory
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !parent_inode.check_permission(current_uid, current_gid, 0o2) { // Write permission
            return Err(DirOpError::PermissionDenied);
        }
        
        // Read the target inode
        let mut target_inode = self.inode_manager.read_inode(lookup_result.ino)
            .map_err(|_| DirOpError::NotFound)?;
        
        if target_inode.is_dir() {
            return Err(DirOpError::IsDirectory);
        }
        
        // Start journal transaction for atomic file removal
        let mut transaction = self.journal_manager.begin_transaction()
            .map_err(|_| DirOpError::IoError)?;
        
        // Remove entry from parent directory
        self.remove_entry(parent_inode, name)?;
        
        // Decrease link count
        if target_inode.disk_inode.i_links_count > 0 {
            target_inode.disk_inode.i_links_count -= 1;
        }
        
        // If no more links, free the inode and its blocks
        if target_inode.disk_inode.i_links_count == 0 {
            // Free all data blocks
            for block_idx in 0..target_inode.get_block_count() {
                if let Some(block_num) = target_inode.get_block(block_idx) {
                    self.space_allocator.free_block(block_num)
                        .map_err(|_| DirOpError::IoError)?;
                }
            }
            
            // Log inode deletion
            transaction.log_inode_delete(lookup_result.ino, &target_inode.disk_inode)
                .map_err(|_| DirOpError::IoError)?;
            
            // Free the inode
            self.inode_manager.free_inode(lookup_result.ino)
                .map_err(|_| DirOpError::IoError)?;
        } else {
            // Log inode update (link count change)
            transaction.log_inode_update(lookup_result.ino, &target_inode.disk_inode)
                .map_err(|_| DirOpError::IoError)?;
        }
        
        // Log parent directory modification
        transaction.log_inode_update(parent_inode.ino, &parent_inode.disk_inode)
            .map_err(|_| DirOpError::IoError)?;
        
        // Update parent's times
        parent_inode.touch_mtime(0);
        parent_inode.touch_ctime(0);
        
        // Commit the transaction
        transaction.commit().map_err(|_| DirOpError::IoError)?;
        
        Ok(())
    }
    
    /// Lookup an entry in a directory
    pub fn lookup_entry(&self, dir_inode: &VexfsInodeInfo, name: &str) -> Result<Option<LookupResult>, DirOpError> {
        if !dir_inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        if name.len() > self.max_name_len as usize {
            return Err(DirOpError::NameTooLong);
        }
        
        // Search through directory blocks
        let file_size = dir_inode.size();
        let mut offset = 0u64;
        
        while offset < file_size {
            let block_offset = offset / self.block_size as u64;
            
            // Get block for this offset
            if let Some(block_num) = dir_inode.get_block(block_offset) {
                // In a real implementation, we would read the block from disk
                // For now, simulate reading directory entries
                
                // Parse directory entries in this block
                let entries_in_block = self.parse_directory_block(block_num, offset % self.block_size as u64)?;
                
                for entry in entries_in_block.iter() {
                    if entry.name_len > 0 && entry.matches_name(name) {
                        return Ok(Some(LookupResult {
                            ino: entry.inode as u64,
                            file_type: entry.file_type,
                            offset: offset as u32,
                            rec_len: entry.rec_len,
                        }));
                    }
                    
                    offset += entry.rec_len as u64;
                    if offset >= file_size {
                        break;
                    }
                }
            } else {
                // Sparse directory block - skip
                offset += self.block_size as u64;
            }
        }
        
        Ok(None)
    }
    
    /// Add an entry to a directory
    pub fn add_entry(&mut self, dir_inode: &mut VexfsInodeInfo, name: &str, ino: u64, file_type: u8) -> Result<(), DirOpError> {
        if !dir_inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        if name.len() > self.max_name_len as usize {
            return Err(DirOpError::NameTooLong);
        }
        
        let entry_size = VexfsDirEntry::calc_size(name.len() as u8);
        
        // Find space for the new entry
        let insertion_point = self.find_insertion_point(dir_inode, entry_size)?;
        
        // Create the new directory entry
        let mut new_entry = VexfsDirEntry::new();
        new_entry.inode = ino as u32;
        new_entry.file_type = file_type;
        new_entry.name_len = name.len() as u8;
        new_entry.rec_len = entry_size;
        
        // In a real implementation, we would:
        // 1. Read the block containing the insertion point
        // 2. Insert the new entry
        // 3. Adjust the previous entry's rec_len
        // 4. Write the block back to disk
        
        // Update directory size if we added at the end
        let new_offset = insertion_point.offset + entry_size as u64;
        if new_offset > dir_inode.size() {
            dir_inode.set_size(new_offset);
        }
        
        // Update directory modification time
        dir_inode.touch_mtime(0);
        dir_inode.touch_ctime(0);
        
        Ok(())
    }
    
    /// Remove an entry from a directory
    pub fn remove_entry(&mut self, dir_inode: &mut VexfsInodeInfo, name: &str) -> Result<(), DirOpError> {
        let lookup_result = self.lookup_entry(dir_inode, name)?
            .ok_or(DirOpError::NotFound)?;
        
        // In a real implementation, we would:
        // 1. Read the block containing the entry
        // 2. Remove the entry by adjusting the previous entry's rec_len
        // 3. Write the block back to disk
        
        // Update directory modification time
        dir_inode.touch_mtime(0);
        dir_inode.touch_ctime(0);
        
        Ok(())
    }
    
    /// Read directory entries
    pub fn readdir(&self, dir_handle: &mut VexfsDirHandle, buffer: &mut [VexfsDirEntry], count: usize) -> Result<usize, DirOpError> {
        if !dir_handle.inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        let file_size = dir_handle.inode.size();
        if dir_handle.pos >= file_size {
            return Ok(0); // EOF
        }
        
        let mut entries_read = 0;
        let mut current_pos = dir_handle.pos;
        
        while entries_read < count && current_pos < file_size {
            let block_offset = current_pos / self.block_size as u64;
            
            // Get block for this offset
            if let Some(block_num) = dir_handle.inode.get_block(block_offset) {
                // Parse directory entries in this block
                let entries_in_block = self.parse_directory_block(block_num, current_pos % self.block_size as u64)?;
                
                for entry in entries_in_block.iter() {
                    if entries_read >= count {
                        break;
                    }
                    
                    if entry.name_len > 0 {
                        buffer[entries_read] = *entry;
                        entries_read += 1;
                    }
                    
                    current_pos += entry.rec_len as u64;
                    if current_pos >= file_size {
                        break;
                    }
                }
            } else {
                // Sparse directory block - skip
                current_pos += self.block_size as u64;
            }
        }
        
        dir_handle.pos = current_pos;
        Ok(entries_read)
    }
    
    /// Initialize a new directory with "." and ".." entries
    fn init_directory(&mut self, dir_inode: &mut VexfsInodeInfo, parent_ino: u64) -> Result<(), DirOpError> {
        // Create "." entry (self)
        self.add_entry(dir_inode, ".", dir_inode.ino, DT_DIR)?;
        
        // Create ".." entry (parent)
        self.add_entry(dir_inode, "..", parent_ino, DT_DIR)?;
        
        // Set initial link count (. and .. entries)
        dir_inode.disk_inode.i_links_count = 2;
        
        Ok(())
    }
    
    /// Check if a directory is empty (only contains "." and "..")
    fn is_empty_directory(&self, dir_inode: &VexfsInodeInfo) -> Result<bool, DirOpError> {
        let file_size = dir_inode.size();
        let mut offset = 0u64;
        let mut entry_count = 0;
        
        while offset < file_size {
            let block_offset = offset / self.block_size as u64;
            
            if let Some(block_num) = dir_inode.get_block(block_offset) {
                let entries_in_block = self.parse_directory_block(block_num, offset % self.block_size as u64)?;
                
                for entry in entries_in_block.iter() {
                    if entry.name_len > 0 {
                        entry_count += 1;
                        
                        // Skip "." and ".." entries
                        if entry_count > 2 {
                            return Ok(false); // Directory is not empty
                        }
                    }
                    
                    offset += entry.rec_len as u64;
                    if offset >= file_size {
                        break;
                    }
                }
            } else {
                offset += self.block_size as u64;
            }
        }
        
        Ok(entry_count <= 2)
    }
    
    /// Validate a filename
    fn is_valid_name(&self, name: &str) -> bool {
        if name.is_empty() || name == "." || name == ".." {
            return false;
        }
        
        // Check for invalid characters
        for ch in name.chars() {
            if ch == '/' || ch == '\0' {
                return false;
            }
        }
        
        true
    }
    
    /// Find insertion point for a new directory entry
    fn find_insertion_point(&self, dir_inode: &VexfsInodeInfo, entry_size: u16) -> Result<InsertionPoint, DirOpError> {
        let file_size = dir_inode.size();
        let mut offset = 0u64;
        
        // Search for free space in existing blocks
        while offset < file_size {
            let block_offset = offset / self.block_size as u64;
            
            if let Some(block_num) = dir_inode.get_block(block_offset) {
                // In a real implementation, we would check for free space in this block
                // For now, assume we need to append at the end
            }
            
            offset += self.block_size as u64;
        }
        
        // Append at the end of the directory
        Ok(InsertionPoint {
            block_num: (file_size / self.block_size as u64) + 1,
            offset: file_size,
            available_space: entry_size,
        })
    }
    
    /// Parse directory entries from a block (stub implementation)
    fn parse_directory_block(&self, block_num: u64, start_offset: u64) -> Result<[VexfsDirEntry; VEXFS_DIR_ENTRIES_PER_BLOCK], DirOpError> {
        // In a real implementation, this would:
        // 1. Read the block from disk
        // 2. Parse directory entries from the block data
        // 3. Return an array of parsed entries
        
        // For now, return empty entries
        Ok([VexfsDirEntry::new(); VEXFS_DIR_ENTRIES_PER_BLOCK])
    }
/// Create a hard link
    pub fn link(&mut self, target_inode: &mut VexfsInodeInfo, dir_inode: &mut VexfsInodeInfo, name: &str) -> Result<(), DirOpError> {
        if target_inode.is_dir() {
            return Err(DirOpError::IsDirectory);
        }
        
        if !self.is_valid_name(name) {
            return Err(DirOpError::InvalidName);
        }
        
        // Check if entry already exists
        if self.lookup_entry(dir_inode, name)?.is_some() {
            return Err(DirOpError::AlreadyExists);
        }
        
        // Check permissions in target directory
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !dir_inode.check_permission(current_uid, current_gid, 0o2) { // Write permission
            return Err(DirOpError::PermissionDenied);
        }
        
        // Check for too many links
        if target_inode.disk_inode.i_links_count >= 32000 {
            return Err(DirOpError::TooManyLinks);
        }
        
        // Add entry to directory
        self.add_entry(dir_inode, name, target_inode.ino, DT_REG)?;
        
        // Increment link count
        target_inode.disk_inode.i_links_count += 1;
        target_inode.touch_ctime(0);
        
        // Update directory times
        dir_inode.touch_mtime(0);
        dir_inode.touch_ctime(0);
        
        Ok(())
    }
    
    /// Create a symbolic link
    pub fn symlink(&mut self, target: &str, dir_inode: &mut VexfsInodeInfo, name: &str, uid: u32, gid: u32) -> Result<VexfsInodeInfo, DirOpError> {
        if !self.is_valid_name(name) {
            return Err(DirOpError::InvalidName);
        }
        
        // Check if entry already exists
        if self.lookup_entry(dir_inode, name)?.is_some() {
            return Err(DirOpError::AlreadyExists);
        }
        
        // Check permissions in parent directory
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !dir_inode.check_permission(current_uid, current_gid, 0o2) { // Write permission
            return Err(DirOpError::PermissionDenied);
        }
        
        // Create new inode for the symbolic link
        let mut new_inode = self.inode_manager.create_inode(
            S_IFLNK | 0o777,
            uid as u16,
            gid as u16
        ).map_err(|_| DirOpError::NoSpace)?;
        
        // Set the size to the length of the target path
        new_inode.set_size(target.len() as u64);
        
        // In a real implementation, we would store the target path in the inode
        // For now, just mark it as a symbolic link
        
        // Add entry to parent directory
        self.add_entry(dir_inode, name, new_inode.ino, DT_LNK)?;
        
        // Update parent's times
        dir_inode.touch_mtime(0);
        dir_inode.touch_ctime(0);
        
        Ok(new_inode)
    }
    
    /// Read the target of a symbolic link
    pub fn readlink(&self, link_inode: &VexfsInodeInfo, buffer: &mut [u8]) -> Result<usize, DirOpError> {
        if !link_inode.is_symlink() {
            return Err(DirOpError::InvalidName);
        }
        
        let link_size = link_inode.size() as usize;
        if buffer.len() < link_size {
            return Err(DirOpError::NoSpace);
        }
        
        // In a real implementation, we would read the target path from the inode's data
        // For now, return a placeholder
        let placeholder = b"placeholder_target";
        let copy_len = core::cmp::min(placeholder.len(), buffer.len());
        buffer[..copy_len].copy_from_slice(&placeholder[..copy_len]);
        
        Ok(copy_len)
    }
}

/// Directory operations function table for integration with VFS
#[repr(C)]
pub struct VexfsDirOperations {
    pub mkdir: unsafe extern "C" fn(u64, *const c_char, c_uint, c_uint, c_uint, *mut u64) -> c_int,
    pub rmdir: unsafe extern "C" fn(u64, *const c_char) -> c_int,
    pub opendir: unsafe extern "C" fn(u64, *mut *mut c_void) -> c_int,
    pub readdir: unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *mut usize) -> c_int,
    pub closedir: unsafe extern "C" fn(*mut c_void) -> c_int,
    pub lookup: unsafe extern "C" fn(u64, *const c_char, *mut u64, *mut c_uint) -> c_int,
    pub rename: unsafe extern "C" fn(u64, *const c_char, u64, *const c_char) -> c_int,
    pub link: unsafe extern "C" fn(u64, u64, *const c_char) -> c_int,
    pub symlink: unsafe extern "C" fn(*const c_char, u64, *const c_char, c_uint, c_uint, *mut u64) -> c_int,
    pub readlink: unsafe extern "C" fn(u64, *mut c_char, usize, *mut usize) -> c_int,
    pub getdents: unsafe extern "C" fn(*mut VexfsDirHandle, *mut c_void, u32, *mut u32) -> c_int,
}

/// Global directory operations table for VFS integration
#[no_mangle]
pub static VEXFS_DIR_OPERATIONS: VexfsDirOperations = VexfsDirOperations {
    mkdir: vexfs_mkdir,
    rmdir: vexfs_rmdir,
    opendir: vexfs_opendir,
    readdir: vexfs_readdir,
    closedir: vexfs_closedir,
    lookup: vexfs_lookup,
    rename: vexfs_rename,
    link: vexfs_link,
    symlink: vexfs_symlink,
    readlink: vexfs_readlink,
    getdents: vexfs_getdents,
};

/// Insertion point for new directory entries
#[derive(Debug)]
struct InsertionPoint {
    pub block_num: u64,
    pub offset: u64,
    pub available_space: u16,
}

impl DirEntryIterator {
    /// Create a new directory entry iterator
    pub fn new(dir_handle: VexfsDirHandle) -> Self {
        let end_pos = dir_handle.inode.size();
        
        Self {
            dir_handle,
            pos: 0,
            end_pos,
        }
    }
    
    /// Get the next directory entry
    pub fn next(&mut self) -> Option<VexfsDirEntry> {
        if self.pos >= self.end_pos {
            return None;
        }
        
        // In a real implementation, this would read and parse the next entry
        // For now, return None to indicate end of iteration
        None
    }
    
    /// Reset iterator to beginning
    pub fn reset(&mut self) {
        self.pos = 0;
    }
    
    /// Seek to a specific position
    pub fn seek(&mut self, pos: u64) -> Result<(), DirOpError> {
        if pos > self.end_pos {
            return Err(DirOpError::InvalidName);
        }
        
        self.pos = pos;
        Ok(())
    }
}

impl VexfsDirEntry {
    /// Check if this entry matches a given name
    pub fn matches_name(&self, name: &str) -> bool {
        if self.name_len as usize != name.len() {
            return false;
        }
        
        // In a real implementation, we would compare the actual name data
        // For now, assume names don't match (since we don't have real data)
        false
    }
    
    /// Calculate the size needed for an entry with a given name length
    pub fn calc_size(name_len: u8) -> u16 {
        let base_size = core::mem::size_of::<VexfsDirEntry>() as u16;
        let name_size = name_len as u16;
        
        // Align to 4-byte boundary
        ((base_size + name_size + 3) / 4) * 4
    }
}

/// Directory statistics
#[derive(Debug, Clone, Copy)]
pub struct DirStats {
    pub total_entries: u32,
    pub total_size: u64,
    pub blocks_used: u32,
    pub free_space: u64,
}

impl VexfsDirOps {
    /// Get directory statistics
    pub fn get_dir_stats(&self, dir_inode: &VexfsInodeInfo) -> Result<DirStats, DirOpError> {
        if !dir_inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        let file_size = dir_inode.size();
        let blocks_used = ((file_size + self.block_size as u64 - 1) / self.block_size as u64) as u32;
        
        Ok(DirStats {
            total_entries: 0, // Would count entries in real implementation
            total_size: file_size,
            blocks_used,
            free_space: (blocks_used as u64 * self.block_size as u64) - file_size,
        })
    }
    
    /// Rename an entry within a directory
    pub fn rename(&mut self, old_dir: &mut VexfsInodeInfo, old_name: &str, new_dir: &mut VexfsInodeInfo, new_name: &str) -> Result<(), DirOpError> {
        // Check if source exists
        let lookup_result = self.lookup_entry(old_dir, old_name)?
            .ok_or(DirOpError::NotFound)?;
        
        // Check if destination already exists
        if self.lookup_entry(new_dir, new_name)?.is_some() {
            return Err(DirOpError::AlreadyExists);
        }
        
        // Add entry to new directory
        self.add_entry(new_dir, new_name, lookup_result.ino, lookup_result.file_type)?;
        
        // Remove entry from old directory
        self.remove_entry(old_dir, old_name)?;
        
        Ok(())
    }
}

//=============================================================================
// C FFI Exports for VFS Integration
//=============================================================================

use crate::ffi::{FFIResult, to_ffi_result};
use core::ptr;

/// C FFI: Create a new directory
#[no_mangle]
pub unsafe extern "C" fn vexfs_mkdir(
    dir_ino: u64,
    name: *const c_char,
    mode: c_uint,
    uid: c_uint,
    gid: c_uint,
    new_ino: *mut u64,
) -> c_int {
    if name.is_null() || new_ino.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    // Convert C string to Rust string
    let name_str = match core::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return to_ffi_result(Err("Invalid name encoding")),
    };
    
    // In a real implementation, we would:
    // 1. Get the directory inode manager instance
    // 2. Look up the parent directory inode
    // 3. Create the new directory
    // 4. Add it to the parent directory
    
    // For now, return a placeholder success with a fake inode number
    *new_ino = 42; // Placeholder inode number
    to_ffi_result(Ok(()))
}

/// C FFI: Remove a directory
#[no_mangle]
pub unsafe extern "C" fn vexfs_rmdir(
    dir_ino: u64,
    name: *const c_char,
) -> c_int {
    if name.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    let name_str = match core::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return to_ffi_result(Err("Invalid name encoding")),
    };
    
    // In a real implementation, we would:
    // 1. Look up the parent directory inode
    // 2. Look up the target directory
    // 3. Check if it's empty
    // 4. Remove the directory entry
    // 5. Free the directory inode
    
    to_ffi_result(Ok(()))
}

/// C FFI: Look up a directory entry
#[no_mangle]
pub unsafe extern "C" fn vexfs_lookup(
    dir_ino: u64,
    name: *const c_char,
    result_ino: *mut u64,
    result_type: *mut c_uint,
) -> c_int {
    if name.is_null() || result_ino.is_null() || result_type.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    let name_str = match core::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return to_ffi_result(Err("Invalid name encoding")),
    };
    
    // In a real implementation, we would:
    // 1. Look up the directory inode
    // 2. Search for the entry by name
    // 3. Return the inode number and type
    
    // For now, return not found
    to_ffi_result(Err("Not found"))
}

/// C FFI: Open a directory for reading
#[no_mangle]
pub unsafe extern "C" fn vexfs_opendir(
    dir_ino: u64,
    handle: *mut *mut c_void,
) -> c_int {
    if handle.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    // In a real implementation, we would:
    // 1. Look up the directory inode
    // 2. Create a directory handle
    // 3. Return the handle
    
    // For now, return a null handle
    *handle = ptr::null_mut();
    to_ffi_result(Ok(()))
}

/// C FFI: Read directory entries
#[no_mangle]
pub unsafe extern "C" fn vexfs_readdir(
    handle: *mut c_void,
    buffer: *mut c_void,
    buffer_size: usize,
    entries_read: *mut usize,
) -> c_int {
    if handle.is_null() || buffer.is_null() || entries_read.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    // In a real implementation, we would:
    // 1. Cast handle to VexfsDirHandle
    // 2. Read entries into the buffer
    // 3. Return the number of entries read
    
    *entries_read = 0;
    to_ffi_result(Ok(()))
}

/// C FFI: Close a directory handle
#[no_mangle]
pub unsafe extern "C" fn vexfs_closedir(handle: *mut c_void) -> c_int {
    if handle.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    // In a real implementation, we would:
    // 1. Cast handle to VexfsDirHandle
    // 2. Free the handle and associated resources
    
    to_ffi_result(Ok(()))
}

/// C FFI: Rename a file or directory
#[no_mangle]
pub unsafe extern "C" fn vexfs_rename(
    old_dir_ino: u64,
    old_name: *const c_char,
    new_dir_ino: u64,
    new_name: *const c_char,
) -> c_int {
    if old_name.is_null() || new_name.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    let old_name_str = match core::ffi::CStr::from_ptr(old_name).to_str() {
        Ok(s) => s,
        Err(_) => return to_ffi_result(Err("Invalid old name encoding")),
    };
    
    let new_name_str = match core::ffi::CStr::from_ptr(new_name).to_str() {
        Ok(s) => s,
        Err(_) => return to_ffi_result(Err("Invalid new name encoding")),
    };
    
    // In a real implementation, we would:
    // 1. Look up both directory inodes
    // 2. Look up the source entry
    // 3. Check if destination exists
    // 4. Move the entry between directories
    
    to_ffi_result(Ok(()))
}

/// C FFI: Create a hard link
#[no_mangle]
pub unsafe extern "C" fn vexfs_link(
    target_ino: u64,
    dir_ino: u64,
    name: *const c_char,
) -> c_int {
    if name.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    let name_str = match core::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return to_ffi_result(Err("Invalid name encoding")),
    };
    
    // In a real implementation, we would:
    // 1. Look up the target inode
    // 2. Look up the directory inode
    // 3. Create the hard link
    // 4. Increment the link count
    
    to_ffi_result(Ok(()))
}

/// C FFI: Create a symbolic link
#[no_mangle]
pub unsafe extern "C" fn vexfs_symlink(
    target: *const c_char,
    dir_ino: u64,
    name: *const c_char,
    uid: c_uint,
    gid: c_uint,
    new_ino: *mut u64,
) -> c_int {
    if target.is_null() || name.is_null() || new_ino.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    let target_str = match core::ffi::CStr::from_ptr(target).to_str() {
        Ok(s) => s,
        Err(_) => return to_ffi_result(Err("Invalid target encoding")),
    };
    
    let name_str = match core::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return to_ffi_result(Err("Invalid name encoding")),
    };
    
    // In a real implementation, we would:
    // 1. Look up the directory inode
    // 2. Create a new inode for the symlink
    // 3. Store the target path in the inode
    // 4. Add the entry to the directory
    
    *new_ino = 43; // Placeholder inode number
    to_ffi_result(Ok(()))
}

/// C FFI: Get directory entries (getdents syscall)
#[no_mangle]
pub unsafe extern "C" fn vexfs_getdents(
    dir_handle: *mut VexfsDirHandle,
    buffer: *mut c_void,
    buffer_size: u32,
    bytes_read: *mut u32,
) -> c_int {
    if dir_handle.is_null() || buffer.is_null() || bytes_read.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    let handle = &mut *dir_handle;
    let output_buffer = core::slice::from_raw_parts_mut(buffer as *mut u8, buffer_size as usize);
    
    // In a real implementation, we would:
    // 1. Read directory entries from the current position
    // 2. Convert them to the appropriate format (dirent structure)
    // 3. Copy to the output buffer
    // 4. Update the directory position
    // 5. Return the number of bytes written
    
    // For now, return empty directory
    *bytes_read = 0;
    to_ffi_result(Ok(()))
}

/// C FFI: Read the target of a symbolic link
#[no_mangle]
pub unsafe extern "C" fn vexfs_readlink(
    link_ino: u64,
    buffer: *mut c_char,
    buffer_size: usize,
    bytes_read: *mut usize,
) -> c_int {
    if buffer.is_null() || bytes_read.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    // In a real implementation, we would:
    // 1. Look up the symlink inode
    // 2. Read the target path from the inode
    // 3. Copy it to the buffer
    
    *bytes_read = 0;
    to_ffi_result(Ok(()))
}

/// C FFI: Remove a file or directory entry
#[no_mangle]
pub unsafe extern "C" fn vexfs_unlink(
    dir_ino: u64,
    name: *const c_char,
) -> c_int {
    if name.is_null() {
        return to_ffi_result(Err("Invalid arguments"));
    }
    
    let name_str = match core::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return to_ffi_result(Err("Invalid name encoding")),
    };
    
    // In a real implementation, we would:
    // 1. Look up the directory inode
    // 2. Look up the target entry
    // 3. Remove the directory entry
    // 4. Decrement link count or free inode if last link
    
    to_ffi_result(Ok(()))
}

//=============================================================================
// Testing and Development Support
//=============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inode_mgmt::VexfsInodeManager;
    
    fn create_test_dir_ops() -> VexfsDirOps {
        let inode_manager = VexfsInodeManager::new(4096, 1000);
        VexfsDirOps::new(4096, 255, inode_manager)
    }
    
    #[test]
    fn test_dir_creation() {
        let mut dir_ops = create_test_dir_ops();
        let mut parent_inode = dir_ops.inode_manager.create_inode(
            S_IFDIR | 0o755,
            0,
            0
        ).unwrap();
        
        let result = dir_ops.create_directory(&mut parent_inode, "test_dir", 0o755, 0, 0);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_file_name_validation() {
        let dir_ops = create_test_dir_ops();
        
        assert!(dir_ops.is_valid_name("valid_name"));
        assert!(!dir_ops.is_valid_name(""));
        assert!(!dir_ops.is_valid_name("."));
        assert!(!dir_ops.is_valid_name(".."));
        assert!(!dir_ops.is_valid_name("name/with/slash"));
        assert!(!dir_ops.is_valid_name("name\0with\0null"));
    }
    
    #[test]
    fn test_directory_entry_creation() {
        let entry = VexfsDirEntry::new();
        assert_eq!(entry.ino, 0);
        assert_eq!(entry.name_len, 0);
        assert_eq!(entry.file_type, 0);
    }
    
    #[test]
    fn test_entry_size_calculation() {
        let size_1 = VexfsDirEntry::calc_size(1);
        let size_10 = VexfsDirEntry::calc_size(10);
        let size_255 = VexfsDirEntry::calc_size(255);
        
        // All sizes should be 4-byte aligned
        assert_eq!(size_1 % 4, 0);
        assert_eq!(size_10 % 4, 0);
        assert_eq!(size_255 % 4, 0);
        
        // Longer names should require more space
        assert!(size_10 > size_1);
        assert!(size_255 > size_10);
    }
    
    #[test]
    fn test_dir_stats() {
        let dir_ops = create_test_dir_ops();
        let dir_inode = dir_ops.inode_manager.create_inode(
            S_IFDIR | 0o755,
            0,
            0
        ).unwrap();
        
        let stats = dir_ops.get_dir_stats(&dir_inode);
        assert!(stats.is_ok());
        
        let stats = stats.unwrap();
        assert_eq!(stats.total_size, 0); // Empty directory
        assert_eq!(stats.blocks_used, 0);
    }
    
    #[test]
    fn test_dir_operations_table() {
        // Verify that the operations table is properly initialized
        assert!(!(VEXFS_DIR_OPERATIONS.mkdir as *const _ as *const u8).is_null());
        assert!(!(VEXFS_DIR_OPERATIONS.rmdir as *const _ as *const u8).is_null());
        assert!(!(VEXFS_DIR_OPERATIONS.opendir as *const _ as *const u8).is_null());
        assert!(!(VEXFS_DIR_OPERATIONS.readdir as *const _ as *const u8).is_null());
        assert!(!(VEXFS_DIR_OPERATIONS.closedir as *const _ as *const u8).is_null());
        assert!(!(VEXFS_DIR_OPERATIONS.lookup as *const _ as *const u8).is_null());
        assert!(!(VEXFS_DIR_OPERATIONS.rename as *const _ as *const u8).is_null());
        assert!(!(VEXFS_DIR_OPERATIONS.link as *const _ as *const u8).is_null());
        assert!(!(VEXFS_DIR_OPERATIONS.symlink as *const _ as *const u8).is_null());
        assert!(!(VEXFS_DIR_OPERATIONS.readlink as *const _ as *const u8).is_null());
        assert!(!(VEXFS_DIR_OPERATIONS.getdents as *const _ as *const u8).is_null());
    }
}