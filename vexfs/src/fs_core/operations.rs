//! Core Filesystem Operations for VexFS
//! 
//! This module provides high-level filesystem operations that coordinate
//! between file, directory, inode, and storage components. It serves as
//! the main interface for filesystem operations and handles transaction
//! coordination and error recovery.

use crate::shared::errors::VexfsError;
use crate::shared::types::{
    InodeNumber, FileType, FileSize, BlockNumber, Result, Timestamp
};
use crate::shared::constants::VEXFS_ROOT_INODE;
use crate::fs_core::{
    file::{File, FileHandle},
    directory::{Directory, DirectoryEntry},
    inode::{Inode, create_inode, get_inode, put_inode, delete_inode},
    path::{resolve_path, resolve_parent, ResolutionContext},
    permissions::{UserContext, check_read_permission, check_write_permission, 
                 check_create_permission, check_delete_permission},
    locking::{acquire_inode_lock, release_inode_lock, LockType}
};
use crate::storage::StorageManager;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, collections::BTreeMap};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, collections::BTreeMap};

/// Operation result with optional data
#[derive(Debug, Clone)]
pub enum OperationResult {
    /// Operation completed successfully
    Success,
    /// File operation with handle
    FileHandle(FileHandle),
    /// Directory listing
    DirectoryListing(Vec<DirectoryEntry>),
    /// File data
    FileData(Vec<u8>),
    /// File metadata
    FileMetadata {
        size: FileSize,
        file_type: FileType,
        mtime: Timestamp,
        permissions: u32,
    },
    /// Inode number
    InodeNumber(InodeNumber),
}

/// Filesystem operation context
#[derive(Debug, Clone)]
pub struct OperationContext {
    /// User context for permission checking
    pub user: UserContext,
    /// Current working directory
    pub cwd_inode: InodeNumber,
    /// Default umask for new files
    pub umask: u32,
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
}

impl OperationContext {
    /// Create a new operation context
    pub fn new(user: UserContext, cwd_inode: InodeNumber) -> Self {
        Self {
            user,
            cwd_inode,
            umask: 0o022,
            follow_symlinks: true,
        }
    }
    
    /// Create a root operation context
    pub fn root() -> Self {
        Self::new(UserContext::root(), VEXFS_ROOT_INODE)
    }
    
    /// Get resolution context for path operations
    pub fn resolution_context(&self) -> ResolutionContext {
        let mut ctx = ResolutionContext::new(self.cwd_inode, self.user.clone());
        ctx.follow_symlinks = self.follow_symlinks;
        ctx
    }
}

/// Filesystem Operations Manager
/// 
/// Provides high-level filesystem operations with proper coordination,
/// locking, and error handling.
pub struct FilesystemOperations;

impl FilesystemOperations {
    /// Create a new file
    /// 
    /// Creates a new regular file at the specified path with given permissions.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path where to create the file
    /// * `mode` - Permission bits for the new file
    /// * `context` - Operation context
    /// 
    /// # Returns
    /// 
    /// Returns the new file's inode number or an error.
    pub fn create_file(path: &str, mode: u32, context: &OperationContext) -> Result<InodeNumber> {
        // Resolve parent directory and filename
        let (parent_inode, filename) = resolve_parent(path, context.resolution_context())?;
        
        // Lock parent directory
        let _parent_lock = acquire_inode_lock(parent_inode, LockType::Write)?;
        
        // Check permissions to create in parent directory
        let parent = get_inode(parent_inode)?;
        check_create_permission(&parent, &context.user)?;
        
        // Check if file already exists
        if let Ok(_) = resolve_path(path, context.resolution_context()) {
            return Err(VexfsError::FileExists);
        }
        
        // Create new inode
        let mut file_inode = create_inode(FileType::Regular)?;
        file_inode.mode = crate::fs_core::permissions::apply_umask(mode, context.umask);
        file_inode.uid = context.user.uid;
        file_inode.gid = context.user.gid;
        file_inode.link_count = 1;
        
        let inode_number = file_inode.number;
        
        // Save the inode
        put_inode(file_inode)?;
        
        // Create directory entry
        let entry = DirectoryEntry {
            inode_number,
            name: filename,
            file_type: FileType::Regular,
        };
        
        // Add entry to parent directory
        crate::fs_core::directory::add_entry(parent_inode, entry, &context.user)?;
        
        Ok(inode_number)
    }
    
    /// Open a file
    /// 
    /// Opens an existing file and returns a file handle.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the file
    /// * `flags` - Open flags (read, write, etc.)
    /// * `context` - Operation context
    /// 
    /// # Returns
    /// 
    /// Returns a file handle or an error.
    pub fn open_file(path: &str, flags: u32, context: &OperationContext) -> Result<FileHandle> {
        // Resolve the path
        let result = resolve_path(path, context.resolution_context())?;
        let inode_number = result.inode;
        
        // Get and check the inode
        let inode = get_inode(inode_number)?;
        
        if !inode.is_regular_file() && !inode.is_vector_file() {
            return Err(VexfsError::IsDirectory);
        }
        
        // Check permissions based on flags
        let read_flag = 0x01;
        let write_flag = 0x02;
        
        if flags & read_flag != 0 {
            check_read_permission(&inode, &context.user)?;
        }
        
        if flags & write_flag != 0 {
            check_write_permission(&inode, &context.user)?;
        }
        
        // Create file handle
        let file = File::from_inode(inode);
        let handle = FileHandle::new(file, flags);
        
        Ok(handle)
    }
    
    /// Read data from a file
    /// 
    /// Reads data from a file at the specified offset.
    /// 
    /// # Arguments
    /// 
    /// * `handle` - File handle
    /// * `offset` - Offset to read from
    /// * `size` - Number of bytes to read
    /// 
    /// # Returns
    /// 
    /// Returns the read data or an error.
    pub fn read_file(handle: &FileHandle, offset: u64, size: usize) -> Result<Vec<u8>> {
        handle.file.read(offset, size)
    }
    
    /// Write data to a file
    /// 
    /// Writes data to a file at the specified offset.
    /// 
    /// # Arguments
    /// 
    /// * `handle` - File handle
    /// * `offset` - Offset to write at
    /// * `data` - Data to write
    /// 
    /// # Returns
    /// 
    /// Returns the number of bytes written or an error.
    pub fn write_file(handle: &mut FileHandle, offset: u64, data: &[u8]) -> Result<usize> {
        handle.file.write(offset, data)
    }
    
    /// Close a file
    /// 
    /// Closes a file handle and flushes any pending data.
    /// 
    /// # Arguments
    /// 
    /// * `handle` - File handle to close
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn close_file(handle: FileHandle) -> Result<()> {
        handle.file.flush()?;
        // File handle is dropped automatically
        Ok(())
    }
    
    /// Delete a file
    /// 
    /// Removes a file from the filesystem.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the file to delete
    /// * `context` - Operation context
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn delete_file(path: &str, context: &OperationContext) -> Result<()> {
        // Resolve parent directory and filename
        let (parent_inode, filename) = resolve_parent(path, context.resolution_context())?;
        
        // Resolve the file itself
        let file_result = resolve_path(path, context.resolution_context())?;
        let file_inode_number = file_result.inode;
        
        // Lock both parent and file
        let _parent_lock = acquire_inode_lock(parent_inode, LockType::Write)?;
        let _file_lock = acquire_inode_lock(file_inode_number, LockType::Write)?;
        
        // Get inodes
        let parent = get_inode(parent_inode)?;
        let file_inode = get_inode(file_inode_number)?;
        
        // Check permissions
        check_delete_permission(&parent, &file_inode, &context.user)?;
        
        // Can't delete directories with this function
        if file_inode.is_directory() {
            return Err(VexfsError::IsDirectory);
        }
        
        // Remove directory entry
        crate::fs_core::directory::remove_entry(parent_inode, &filename, &context.user)?;
        
        // Decrease link count
        let mut updated_inode = file_inode.clone();
        updated_inode.link_count -= 1;
        
        // If no more links, delete the inode and its data
        if updated_inode.link_count == 0 {
            // TODO: Deallocate file data blocks
            crate::fs_core::inode::deallocate_inode_blocks(file_inode_number)?;
            delete_inode(file_inode_number)?;
        } else {
            // Just update the link count
            put_inode(updated_inode)?;
        }
        
        Ok(())
    }
    
    /// Create a directory
    /// 
    /// Creates a new directory at the specified path.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path where to create the directory
    /// * `mode` - Permission bits for the new directory
    /// * `context` - Operation context
    /// 
    /// # Returns
    /// 
    /// Returns the new directory's inode number or an error.
    pub fn create_directory(path: &str, mode: u32, context: &OperationContext) -> Result<InodeNumber> {
        // Resolve parent directory and dirname
        let (parent_inode, dirname) = resolve_parent(path, context.resolution_context())?;
        
        // Lock parent directory
        let _parent_lock = acquire_inode_lock(parent_inode, LockType::Write)?;
        
        // Check permissions to create in parent directory
        let parent = get_inode(parent_inode)?;
        check_create_permission(&parent, &context.user)?;
        
        // Check if directory already exists
        if let Ok(_) = resolve_path(path, context.resolution_context()) {
            return Err(VexfsError::FileExists);
        }
        
        // Create new inode for directory
        let mut dir_inode = create_inode(FileType::Directory)?;
        dir_inode.mode = crate::fs_core::permissions::apply_umask(mode, context.umask);
        dir_inode.uid = context.user.uid;
        dir_inode.gid = context.user.gid;
        dir_inode.link_count = 2; // . and entry from parent
        
        let inode_number = dir_inode.number;
        
        // Save the inode
        put_inode(dir_inode)?;
        
        // Initialize directory with . and .. entries
        let current_entry = DirectoryEntry {
            inode_number,
            name: ".".to_string(),
            file_type: FileType::Directory,
        };
        
        let parent_entry = DirectoryEntry {
            inode_number: parent_inode,
            name: "..".to_string(),
            file_type: FileType::Directory,
        };
        
        crate::fs_core::directory::add_entry(inode_number, current_entry, &context.user)?;
        crate::fs_core::directory::add_entry(inode_number, parent_entry, &context.user)?;
        
        // Add entry to parent directory
        let entry = DirectoryEntry {
            inode_number,
            name: dirname,
            file_type: FileType::Directory,
        };
        
        crate::fs_core::directory::add_entry(parent_inode, entry, &context.user)?;
        
        // Update parent directory link count (for the .. entry we added)
        let mut parent_updated = get_inode(parent_inode)?;
        parent_updated.link_count += 1;
        put_inode(parent_updated)?;
        
        Ok(inode_number)
    }
    
    /// List directory contents
    /// 
    /// Returns a list of entries in a directory.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the directory
    /// * `context` - Operation context
    /// 
    /// # Returns
    /// 
    /// Returns a vector of directory entries or an error.
    pub fn list_directory(path: &str, context: &OperationContext) -> Result<Vec<DirectoryEntry>> {
        // Resolve the directory path
        let result = resolve_path(path, context.resolution_context())?;
        let inode_number = result.inode;
        
        // Get and check the inode
        let inode = get_inode(inode_number)?;
        
        if !inode.is_directory() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Check read permission
        check_read_permission(&inode, &context.user)?;
        
        // Read directory entries
        crate::fs_core::directory::read_entries(inode_number, &context.user)
    }
    
    /// Remove a directory
    /// 
    /// Removes an empty directory from the filesystem.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the directory to remove
    /// * `context` - Operation context
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn remove_directory(path: &str, context: &OperationContext) -> Result<()> {
        // Can't remove root directory
        if path == "/" {
            return Err(VexfsError::InvalidArgument);
        }
        
        // Resolve parent directory and dirname
        let (parent_inode, dirname) = resolve_parent(path, context.resolution_context())?;
        
        // Resolve the directory itself
        let dir_result = resolve_path(path, context.resolution_context())?;
        let dir_inode_number = dir_result.inode;
        
        // Lock both parent and directory
        let _parent_lock = acquire_inode_lock(parent_inode, LockType::Write)?;
        let _dir_lock = acquire_inode_lock(dir_inode_number, LockType::Write)?;
        
        // Get inodes
        let parent = get_inode(parent_inode)?;
        let dir_inode = get_inode(dir_inode_number)?;
        
        // Check that it's actually a directory
        if !dir_inode.is_directory() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Check permissions
        check_delete_permission(&parent, &dir_inode, &context.user)?;
        
        // Check that directory is empty (should only contain . and ..)
        let entries = crate::fs_core::directory::read_entries(dir_inode_number, &context.user)?;
        let non_special_entries: Vec<_> = entries.iter()
            .filter(|e| e.name != "." && e.name != "..")
            .collect();
        
        if !non_special_entries.is_empty() {
            return Err(VexfsError::DirectoryNotEmpty);
        }
        
        // Remove directory entry from parent
        crate::fs_core::directory::remove_entry(parent_inode, &dirname, &context.user)?;
        
        // Update parent directory link count (removing the .. reference)
        let mut parent_updated = get_inode(parent_inode)?;
        parent_updated.link_count -= 1;
        put_inode(parent_updated)?;
        
        // Delete the directory inode
        delete_inode(dir_inode_number)?;
        
        Ok(())
    }
    
    /// Rename a file or directory
    /// 
    /// Moves a file or directory from one path to another.
    /// 
    /// # Arguments
    /// 
    /// * `old_path` - Current path
    /// * `new_path` - New path
    /// * `context` - Operation context
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn rename(old_path: &str, new_path: &str, context: &OperationContext) -> Result<()> {
        // Resolve old file
        let old_result = resolve_path(old_path, context.resolution_context())?;
        let old_inode_number = old_result.inode;
        
        // Resolve old parent
        let (old_parent_inode, old_filename) = resolve_parent(old_path, context.resolution_context())?;
        
        // Resolve new parent
        let (new_parent_inode, new_filename) = resolve_parent(new_path, context.resolution_context())?;
        
        // Lock all involved inodes
        let _old_parent_lock = acquire_inode_lock(old_parent_inode, LockType::Write)?;
        let _new_parent_lock = if new_parent_inode != old_parent_inode {
            Some(acquire_inode_lock(new_parent_inode, LockType::Write)?)
        } else {
            None
        };
        let _file_lock = acquire_inode_lock(old_inode_number, LockType::Write)?;
        
        // Get inodes
        let old_parent = get_inode(old_parent_inode)?;
        let new_parent = get_inode(new_parent_inode)?;
        let file_inode = get_inode(old_inode_number)?;
        
        // Check permissions
        check_delete_permission(&old_parent, &file_inode, &context.user)?;
        check_create_permission(&new_parent, &context.user)?;
        
        // Check if destination exists
        if let Ok(_) = resolve_path(new_path, context.resolution_context()) {
            return Err(VexfsError::FileExists);
        }
        
        // Remove from old location
        crate::fs_core::directory::remove_entry(old_parent_inode, &old_filename, &context.user)?;
        
        // Add to new location
        let new_entry = DirectoryEntry {
            inode_number: old_inode_number,
            name: new_filename,
            file_type: file_inode.file_type,
        };
        
        crate::fs_core::directory::add_entry(new_parent_inode, new_entry, &context.user)?;
        
        // Update directory link counts if moving a directory between different parents
        if file_inode.is_directory() && old_parent_inode != new_parent_inode {
            // Decrease old parent link count
            let mut old_parent_updated = old_parent.clone();
            old_parent_updated.link_count -= 1;
            put_inode(old_parent_updated)?;
            
            // Increase new parent link count
            let mut new_parent_updated = new_parent.clone();
            new_parent_updated.link_count += 1;
            put_inode(new_parent_updated)?;
            
            // Update .. entry in the moved directory
            crate::fs_core::directory::remove_entry(old_inode_number, "..", &context.user)?;
            let parent_entry = DirectoryEntry {
                inode_number: new_parent_inode,
                name: "..".to_string(),
                file_type: FileType::Directory,
            };
            crate::fs_core::directory::add_entry(old_inode_number, parent_entry, &context.user)?;
        }
        
        Ok(())
    }
    
    /// Get file/directory metadata
    /// 
    /// Returns metadata for a file or directory.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to get metadata for
    /// * `context` - Operation context
    /// 
    /// # Returns
    /// 
    /// Returns metadata or an error.
    pub fn get_metadata(path: &str, context: &OperationContext) -> Result<OperationResult> {
        // Resolve the path
        let result = resolve_path(path, context.resolution_context())?;
        let inode = get_inode(result.inode)?;
        
        // No special permission check needed for metadata (just needs to be able to access the path)
        
        Ok(OperationResult::FileMetadata {
            size: inode.size,
            file_type: inode.file_type,
            mtime: inode.mtime,
            permissions: inode.mode,
        })
    }
    
    /// Change file permissions
    /// 
    /// Changes the permission bits of a file or directory.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the file/directory
    /// * `mode` - New permission bits
    /// * `context` - Operation context
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn change_permissions(path: &str, mode: u32, context: &OperationContext) -> Result<()> {
        // Resolve the path
        let result = resolve_path(path, context.resolution_context())?;
        let inode_number = result.inode;
        
        // Lock the inode
        let _lock = acquire_inode_lock(inode_number, LockType::Write)?;
        
        // Get and check the inode
        let mut inode = get_inode(inode_number)?;
        
        // Check permissions to change mode
        crate::fs_core::permissions::PermissionChecker::check_change_permissions(&inode, &context.user)?;
        
        // Update permissions
        inode.mode = (inode.mode & !0o777) | (mode & 0o777);
        inode.touch_ctime();
        
        put_inode(inode)?;
        
        Ok(())
    }
    
    /// Sync filesystem changes to storage
    /// 
    /// Forces synchronization of all pending changes to storage.
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn sync_filesystem() -> Result<()> {
        // Flush all dirty inodes
        crate::fs_core::inode::InodeManager::flush_dirty_inodes()?;
        
        // Sync storage layer
        // TODO: Add storage sync call
        
        Ok(())
    }
}

// Public API functions

/// Create a new file
pub fn create_file(path: &str, mode: u32, context: &OperationContext) -> Result<InodeNumber> {
    FilesystemOperations::create_file(path, mode, context)
}

/// Open a file
pub fn open_file(path: &str, flags: u32, context: &OperationContext) -> Result<FileHandle> {
    FilesystemOperations::open_file(path, flags, context)
}

/// Read from a file
pub fn read_file(handle: &FileHandle, offset: u64, size: usize) -> Result<Vec<u8>> {
    FilesystemOperations::read_file(handle, offset, size)
}

/// Write to a file
pub fn write_file(handle: &mut FileHandle, offset: u64, data: &[u8]) -> Result<usize> {
    FilesystemOperations::write_file(handle, offset, data)
}

/// Close a file
pub fn close_file(handle: FileHandle) -> Result<()> {
    FilesystemOperations::close_file(handle)
}

/// Delete a file
pub fn delete_file(path: &str, context: &OperationContext) -> Result<()> {
    FilesystemOperations::delete_file(path, context)
}

/// Create a directory
pub fn create_directory(path: &str, mode: u32, context: &OperationContext) -> Result<InodeNumber> {
    FilesystemOperations::create_directory(path, mode, context)
}

/// List directory contents
pub fn list_directory(path: &str, context: &OperationContext) -> Result<Vec<DirectoryEntry>> {
    FilesystemOperations::list_directory(path, context)
}

/// Remove a directory
pub fn remove_directory(path: &str, context: &OperationContext) -> Result<()> {
    FilesystemOperations::remove_directory(path, context)
}

/// Rename a file or directory
pub fn rename(old_path: &str, new_path: &str, context: &OperationContext) -> Result<()> {
    FilesystemOperations::rename(old_path, new_path, context)
}

/// Get metadata
pub fn get_metadata(path: &str, context: &OperationContext) -> Result<OperationResult> {
    FilesystemOperations::get_metadata(path, context)
}

/// Change permissions
pub fn change_permissions(path: &str, mode: u32, context: &OperationContext) -> Result<()> {
    FilesystemOperations::change_permissions(path, mode, context)
}

/// Sync filesystem
pub fn sync_filesystem() -> Result<()> {
    FilesystemOperations::sync_filesystem()
}