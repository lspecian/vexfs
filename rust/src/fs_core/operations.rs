//! Core Filesystem Operations for VexFS
//! 
//! This module provides high-level filesystem operations that coordinate
//! between file, directory, inode, and storage components. It serves as
//! the main interface for filesystem operations and handles transaction
//! coordination and error recovery.

use crate::shared::errors::VexfsError;
use crate::shared::types::{
    InodeNumber, FileType, FileMode, FileSize, Result, Timestamp // AccessMode removed, BlockNumber removed
};
use crate::shared::constants::VEXFS_ROOT_INO;
use crate::fs_core::{
    file::{File, FileHandle},
    directory::{Directory, DirectoryEntry},
    inode::{Inode, InodeManager, create_inode, get_inode, put_inode, delete_inode},
    path::PathResolver,
    permissions::{UserContext, AccessMode, check_read_permission, check_write_permission,
                 check_create_permission, check_delete_permission},
    locking::{acquire_inode_lock, release_inode_lock, LockType, LockManager}
};
use crate::fs_core::path::Path;
use crate::storage::StorageManager;

#[cfg(not(feature = "kernel"))]
use std::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::sync::Arc;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, collections::BTreeMap};
#[cfg(feature = "std")]
#[cfg(feature = "kernel")]
use alloc::{vec::Vec, string::String, collections::BTreeMap};
#[cfg(not(feature = "kernel"))]
use std::{vec::Vec, string::String, collections::BTreeMap};

#[cfg(feature = "kernel")]
use alloc::string::ToString;
#[cfg(not(feature = "kernel"))]
use std::string::ToString;

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
/// Context for path resolution operations
#[derive(Debug)]
pub struct ResolutionContext<'a> { // Added lifetime 'a
    /// Starting inode for relative path resolution
    pub current_dir_inode: InodeNumber,
    /// User context for permission checks during resolution (e.g., symlink traversal)
    pub user_context: UserContext,
    /// Whether to follow symbolic links during resolution
    pub follow_symlinks: bool,
    /// Reference to InodeManager
    pub inode_manager: &'a mut InodeManager, // Added field
}

impl<'a> ResolutionContext<'a> { // Added lifetime 'a
    /// Create a new resolution context
    pub fn new(current_dir_inode: InodeNumber, user_context: UserContext, inode_manager: &'a mut InodeManager) -> Self { // Added inode_manager
        Self {
            current_dir_inode,
            user_context,
            follow_symlinks: true, // Default behavior, can be modified by OperationContext
            inode_manager, // Added field
        }
    }
}
pub struct OperationContext<'a> { // Removed Debug since LockManager doesn't implement it
    /// User context for permission checking
    pub user: UserContext,
    /// Current working directory
    pub cwd_inode: InodeNumber,
    /// Default umask for new files
    pub umask: u32,
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
    /// Mutable reference to InodeManager
    pub inode_manager: &'a mut InodeManager,
    /// Mutable reference to LockManager
    pub lock_manager: &'a mut LockManager,
}

impl<'a> OperationContext<'a> { // Added lifetime 'a
    /// Create a new operation context
    pub fn new(user: UserContext, cwd_inode: InodeNumber, inode_manager: &'a mut InodeManager, lock_manager: &'a mut LockManager) -> Self {
        Self {
            user,
            cwd_inode,
            umask: 0o022,
            follow_symlinks: true,
            inode_manager,
            lock_manager,
        }
    }
    
    // /// Create a root operation context
    // // This will need to be adjusted as InodeManager needs to be instantiated and passed.
    // // Commenting out for now as it will cause an error.
    // pub fn root() -> Self {
    //     // Self::new(UserContext::root(), VEXFS_ROOT_INODE, /* ??? inode_manager ??? */)
    //     todo!("Root context creation needs InodeManager instance")
    // }
    
    /// Get resolution context for path operations
    pub fn resolution_context(&mut self) -> ResolutionContext<'_> { // Changed to &mut self and added lifetime
        ResolutionContext {
            current_dir_inode: self.cwd_inode,
            user_context: self.user.clone(),
            follow_symlinks: self.follow_symlinks,
            inode_manager: self.inode_manager,
        }
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
    pub fn create_file(path: &str, mode: u32, context: &mut OperationContext) -> Result<InodeNumber> { // context is now &mut
        // Resolve parent directory and filename
        let (parent_inode_num, filename) = PathResolver::resolve_parent(context.inode_manager, context.cwd_inode, &Path::from_str(path)?)?;
        
        // Lock parent directory
        let _parent_lock = acquire_inode_lock(context.lock_manager, parent_inode_num, LockType::Write, 0)?;
        
        // Check permissions to create in parent directory
        let parent_inode = get_inode(context.inode_manager, parent_inode_num)?;
        check_create_permission(&parent_inode, &context.user)?;
        
        // Check if file already exists
        if PathResolver::resolve_path(context.inode_manager, context.cwd_inode, &Path::from_str(path)?).is_ok() {
            return Err(VexfsError::FileExists);
        }
        
        // Create new inode
        let file_inode_arc = create_inode(context.inode_manager, FileType::Regular, FileMode::new(mode), context.user.uid, context.user.gid)?;
        let mut file_inode_mut_guard = Arc::try_unwrap(file_inode_arc).map_err(|_| VexfsError::LockConflict("Failed to get mutable inode".to_string()))?;
        // If Arc::try_unwrap fails, it means there are other Arcs. This is a design issue if we need to mutate here.
        // For now, assuming it succeeds or we need a different pattern like clone-modify-put.
        // Let's assume create_inode returns an Inode that we can make mutable or it's already what we need.
        // The create_inode in inode.rs now returns Arc<Inode>. We need to modify it.
        // A simpler way: get the number, then get mutable inode.
        // However, create_inode in inode.rs is designed to allocate and return the Arc<Inode>.
        // Let's get an Arc, then clone its content for modification if necessary, then put.

        let new_inode_arc = { // Shadowing to avoid confusion with file_inode_mut_guard
            let mut temp_inode = Inode::new(0, FileType::Regular, FileMode::new(mode), context.user.uid, context.user.gid); // ino will be set by allocate_inode
            temp_inode.mode = crate::fs_core::permissions::apply_umask(FileMode(mode), context.umask as u16);
            // uid and gid are already set
            temp_inode.nlink = 1;
            // create_inode in InodeManager now handles allocation and returns Arc<Inode>
            // The public helper create_inode also takes uid and gid.
            context.inode_manager.create_inode(FileType::Regular, FileMode::new(mode), context.user.uid, context.user.gid)?
        };
        
        let inode_number = new_inode_arc.ino; // Access ino from Arc<Inode>
        
        // Save the inode (put_inode now takes Arc<Inode>)
        put_inode(context.inode_manager, new_inode_arc.clone())?; // Clone Arc for put_inode
        
        // Create directory entry
        let entry = DirectoryEntry {
            inode_number,
            name: filename.clone(),
            file_type: FileType::Regular,
            name_len: filename.len() as u8,
        };
        
        // Add entry to parent directory
        crate::fs_core::directory::add_entry(context.inode_manager, parent_inode_num, entry, &context.user)?;
        
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
    pub fn open_file(path: &str, flags: u32, context: &mut OperationContext) -> Result<FileHandle> { // context is now &mut
        // Resolve the path
        let inode_number = PathResolver::resolve_path(context.inode_manager, context.cwd_inode, &Path::from_str(path)?)?;
        
        // Get and check the inode
        let inode_arc = get_inode(context.inode_manager, inode_number)?; // Returns Arc<Inode>
        let inode = (*inode_arc).clone(); // Clone the Inode data
        
        if !inode.is_file() && !inode.is_vector_file() {
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
    pub fn close_file(mut handle: FileHandle) -> Result<()> {
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
    pub fn delete_file(path: &str, context: &mut OperationContext) -> Result<()> { // context is now &mut
        // Resolve parent directory and filename
        let (parent_inode_num, filename) = PathResolver::resolve_parent(context.inode_manager, context.cwd_inode, &Path::from_str(path)?)?;
        
        // Resolve the file itself
        let file_inode_number = PathResolver::resolve_path(context.inode_manager, context.cwd_inode, &Path::from_str(path)?)?;
        
        // Lock both parent and file
        let _parent_lock = acquire_inode_lock(context.lock_manager, parent_inode_num, LockType::Write, 0)?;
        let _file_lock = acquire_inode_lock(context.lock_manager, file_inode_number, LockType::Write, 0)?;
        
        // Get inodes
        let parent_inode = get_inode(context.inode_manager, parent_inode_num)?;
        let file_inode_arc = get_inode(context.inode_manager, file_inode_number)?;
        let file_inode_data = (*file_inode_arc).clone(); // Clone Inode data for checks
        
        // Check permissions
        check_delete_permission(&parent_inode, &context.user)?;
        
        // Can't delete directories with this function
        if file_inode_data.is_dir() {
            return Err(VexfsError::IsDirectory);
        }
        
        // Remove directory entry
        crate::fs_core::directory::remove_entry(context.inode_manager, parent_inode_num, &filename, &context.user)?;
        
        // Decrease link count
        // To modify, we need a mutable Inode. Get Arc, clone, modify, then put.
        let mut updated_inode_data = file_inode_data; // Already a clone
        updated_inode_data.nlink -= 1;
        
        // If no more links, delete the inode and its data
        if updated_inode_data.nlink == 0 {
            // TODO: Deallocate file data blocks
            crate::fs_core::inode::deallocate_inode_blocks(file_inode_number)?;
            delete_inode(context.inode_manager, file_inode_number)?;
        } else {
            // Just update the link count
            // We need to create a new Arc for put_inode if it expects Arc<Inode>
            // Or if put_inode can take Inode, this is fine.
            // inode.rs put_inode takes Arc<Inode>.
            put_inode(context.inode_manager, Arc::new(updated_inode_data))?;
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
    pub fn create_directory(path: &str, mode: u32, context: &mut OperationContext) -> Result<InodeNumber> { // context is now &mut
        // Resolve parent directory and dirname
        let (parent_inode_num, dirname) = PathResolver::resolve_parent(context.inode_manager, context.cwd_inode, &Path::from_str(path)?)?;
        
        // Lock parent directory
        let _parent_lock = acquire_inode_lock(context.lock_manager, parent_inode_num, LockType::Write, 0)?;
        
        // Check permissions to create in parent directory
        let parent_inode = get_inode(context.inode_manager, parent_inode_num)?;
        check_create_permission(&parent_inode, &context.user)?;
        
        // Check if directory already exists
        if PathResolver::resolve_path(context.inode_manager, context.cwd_inode, &Path::from_str(path)?).is_ok() {
            return Err(VexfsError::FileExists);
        }
        
        // Create new inode for directory
        let dir_inode_arc = {
            let mut temp_inode = Inode::new(0, FileType::Directory, FileMode::new(mode), context.user.uid, context.user.gid);
            temp_inode.mode = crate::fs_core::permissions::apply_umask(FileMode(mode), context.umask as u16);
            // uid and gid are set
            temp_inode.nlink = 2; // . and entry from parent
            context.inode_manager.create_inode(FileType::Directory, FileMode::new(mode), context.user.uid, context.user.gid)?
        };

        let inode_number = dir_inode_arc.ino;
        
        // Save the inode
        put_inode(context.inode_manager, dir_inode_arc.clone())?;
        
        // Initialize directory with . and .. entries
        let current_entry = DirectoryEntry {
            inode_number,
            name: ".".to_string(),
            file_type: FileType::Directory,
            name_len: 1,
        };
        
        let parent_entry = DirectoryEntry {
            inode_number: parent_inode_num, // Use parent_inode_num
            name: "..".to_string(),
            file_type: FileType::Directory,
            name_len: 2,
        };
        
        crate::fs_core::directory::add_entry(context.inode_manager, inode_number, current_entry, &context.user)?;
        crate::fs_core::directory::add_entry(context.inode_manager, inode_number, parent_entry, &context.user)?;
        
        // Add entry to parent directory
        let entry = DirectoryEntry {
            inode_number,
            name: dirname.clone(),
            file_type: FileType::Directory,
            name_len: dirname.len() as u8,
        };
        
        crate::fs_core::directory::add_entry(context.inode_manager, parent_inode_num, entry, &context.user)?;
        
        // Update parent directory link count (for the .. entry we added)
        let parent_inode_arc = get_inode(context.inode_manager, parent_inode_num)?;
        let mut parent_updated_data = (*parent_inode_arc).clone();
        parent_updated_data.nlink += 1;
        put_inode(context.inode_manager, Arc::new(parent_updated_data))?;
        
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
    pub fn list_directory(path: &str, context: &mut OperationContext) -> Result<Vec<DirectoryEntry>> { // context is now &mut
        // Resolve the directory path
        let inode_number = PathResolver::resolve_path(context.inode_manager, context.cwd_inode, &Path::from_str(path)?)?;
        
        // Get and check the inode
        let inode_arc = get_inode(context.inode_manager, inode_number)?;
        let inode_data = (*inode_arc).clone();
        
        if !inode_data.is_dir() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Check read permission
        check_read_permission(&inode_arc, &context.user)?; // check_read_permission might need Arc<Inode> or Inode
        
        // Read directory entries
        crate::fs_core::directory::read_entries(context.inode_manager, inode_number, &context.user)
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
    pub fn remove_directory(path: &str, context: &mut OperationContext) -> Result<()> { // context is now &mut
        // Can't remove root directory
        if path == "/" {
            return Err(VexfsError::InvalidArgument("invalid argument".to_string()));
        }
        
        // Resolve parent directory and dirname
        let (parent_inode_num, dirname) = PathResolver::resolve_parent(context.inode_manager, context.cwd_inode, &Path::from_str(path)?)?;
        
        // Resolve the directory itself
        let dir_inode_number = PathResolver::resolve_path(context.inode_manager, context.cwd_inode, &Path::from_str(path)?)?;
        
        // Lock both parent and directory
        let _parent_lock = acquire_inode_lock(context.lock_manager, parent_inode_num, LockType::Write, 0)?;
        let _dir_lock = acquire_inode_lock(context.lock_manager, dir_inode_number, LockType::Write, 0)?;
        
        // Get inodes
        let parent_inode_arc = get_inode(context.inode_manager, parent_inode_num)?;
        let dir_inode_arc = get_inode(context.inode_manager, dir_inode_number)?;
        let dir_inode_data = (*dir_inode_arc).clone();
        
        // Check that it's actually a directory
        if !dir_inode_data.is_dir() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Check permissions
        check_delete_permission(&parent_inode_arc, &context.user)?;
        
        // Check that directory is empty (should only contain . and ..)
        let entries = crate::fs_core::directory::read_entries(context.inode_manager, dir_inode_number, &context.user)?;
        let non_special_entries: Vec<_> = entries.iter()
            .filter(|e| e.name != "." && e.name != "..")
            .collect();
        
        if !non_special_entries.is_empty() {
            return Err(VexfsError::DirectoryNotEmpty);
        }
        
        // Remove directory entry from parent
        crate::fs_core::directory::remove_entry(context.inode_manager, parent_inode_num, &dirname, &context.user)?;
        
        // Update parent directory link count (removing the .. reference)
        let parent_updated_arc = get_inode(context.inode_manager, parent_inode_num)?;
        let mut parent_updated = parent_updated_arc.as_ref().clone();
        parent_updated.nlink -= 1;
        put_inode(context.inode_manager, Arc::new(parent_updated))?;
        
        // Delete the directory inode
        delete_inode(context.inode_manager, dir_inode_number)?;
        
        Ok(())
    }
    
    pub fn rename<'a>(old_path: &str, new_path: &str, context: &mut OperationContext<'a>) -> Result<()> {
        let old_path_obj = Path::from_str(old_path)?;
        let new_path_obj = Path::from_str(new_path)?;

        // Resolve old file and its parent
        let (old_parent_inode_num, old_filename) = PathResolver::resolve_parent(context.inode_manager, context.cwd_inode, &old_path_obj)?;
        let file_inode_number = PathResolver::resolve_path(context.inode_manager, context.cwd_inode, &old_path_obj)?;
        
        // Resolve new parent
        let (new_parent_inode_num, new_filename) = PathResolver::resolve_parent(context.inode_manager, context.cwd_inode, &new_path_obj)?;
        
        // Lock all involved inodes
        let _old_parent_lock = acquire_inode_lock(context.lock_manager, old_parent_inode_num, LockType::Write, 0)?;
        let _new_parent_lock = if new_parent_inode_num != old_parent_inode_num {
            Some(acquire_inode_lock(context.lock_manager, new_parent_inode_num, LockType::Write, 0)?)
        } else {
            None
        };
        let _file_lock = acquire_inode_lock(context.lock_manager, file_inode_number, LockType::Write, 0)?;
        
        // Get inodes
        let old_parent_arc = get_inode(context.inode_manager, old_parent_inode_num)?;
        let new_parent_arc = if new_parent_inode_num != old_parent_inode_num {
            get_inode(context.inode_manager, new_parent_inode_num)?
        } else {
            old_parent_arc.clone()
        };
        let file_inode_arc = get_inode(context.inode_manager, file_inode_number)?;
        let file_inode_data = (*file_inode_arc).clone();
        
        // Check permissions
        crate::fs_core::permissions::PermissionChecker::check_delete(context.inode_manager, old_parent_inode_num, file_inode_number, &context.user)?.to_result()?;
        crate::fs_core::permissions::PermissionChecker::check_create(context.inode_manager, new_parent_inode_num, &context.user)?.to_result()?;
        
        // Check if destination exists
        if PathResolver::resolve_path(context.inode_manager, context.cwd_inode, &new_path_obj).is_ok() {
            return Err(VexfsError::FileExists);
        }
        
        // Prevent renaming "." or ".."
        if old_filename == "." || old_filename == ".." || new_filename == "." || new_filename == ".." {
            return Err(VexfsError::InvalidArgument("invalid argument".to_string()));
        }
        
        if file_inode_data.is_dir() {
            if PathResolver::is_descendant(context.inode_manager, new_parent_inode_num, file_inode_number)? {
                 return Err(VexfsError::InvalidArgument("invalid argument".to_string()));
            }
        }
        
        crate::fs_core::directory::remove_entry(context.inode_manager, old_parent_inode_num, &old_filename, &context.user)?;
        
        let new_entry = DirectoryEntry {
            inode_number: file_inode_number,
            name: new_filename.clone(),
            file_type: file_inode_data.file_type,
            name_len: new_filename.len() as u8,
        };
        crate::fs_core::directory::add_entry(context.inode_manager, new_parent_inode_num, new_entry, &context.user)?;
        
        let mut file_inode_data_to_put = file_inode_data;
        file_inode_data_to_put.touch_ctime();

        if file_inode_data_to_put.is_dir() && old_parent_inode_num != new_parent_inode_num {
            let old_parent_update_arc = get_inode(context.inode_manager, old_parent_inode_num)?;
            let mut old_parent_updated_data = (*old_parent_update_arc).clone();
            if old_parent_updated_data.nlink > 0 { old_parent_updated_data.nlink -= 1; }
            old_parent_updated_data.touch_mtime();
            old_parent_updated_data.touch_ctime();
            put_inode(context.inode_manager, Arc::new(old_parent_updated_data))?;
            
            let new_parent_update_arc = get_inode(context.inode_manager, new_parent_inode_num)?;
            let mut new_parent_updated_data = (*new_parent_update_arc).clone();
            new_parent_updated_data.nlink += 1;
            new_parent_updated_data.touch_mtime();
            new_parent_updated_data.touch_ctime();
            put_inode(context.inode_manager, Arc::new(new_parent_updated_data))?;
            
            crate::fs_core::directory::update_dotdot_entry(context.inode_manager, file_inode_number, new_parent_inode_num)?;
        }
        
        put_inode(context.inode_manager, Arc::new(file_inode_data_to_put))?;
        
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
    pub fn get_metadata<'a>(path: &str, context: &mut OperationContext<'a>) -> Result<OperationResult> {
        // Resolve the path
        let result = PathResolver::resolve_path(context.inode_manager, context.cwd_inode, &Path::from_str(path)?)?;
        let inode_arc = get_inode(context.inode_manager, result)?;
        let inode_data = &*inode_arc;
        
        // No special permission check needed for metadata (just needs to be able to access the path)
        
        Ok(OperationResult::FileMetadata {
            size: inode_data.size,
            file_type: inode_data.file_type,
            mtime: inode_data.mtime,
            permissions: inode_data.mode.0,
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
    pub fn change_permissions(path: &str, mode: u32, context: &mut OperationContext) -> Result<()> {
        // Resolve the path
        let inode_number = PathResolver::resolve_path(context.inode_manager, context.cwd_inode, &Path::from_str(path)?)?;
        
        // Lock the inode
        let _lock = acquire_inode_lock(context.lock_manager, inode_number, LockType::Write, context.user.uid)?;
        
        // Get and check the inode
        let inode_arc = get_inode(context.inode_manager, inode_number)?;
        let mut inode = (*inode_arc).clone();
        
        // Check permissions to change mode (owner or root can change permissions)
        if context.user.uid != 0 && context.user.uid != inode.uid {
            return Err(VexfsError::PermissionDenied("Permission denied".to_string()));
        }
        
        // Update permissions
        inode.mode = FileMode((inode.mode.0 & !0o777) | (mode & 0o777));
        inode.touch_ctime();
        
        put_inode(context.inode_manager, Arc::new(inode))?;
        
        Ok(())
    }
    
    /// Sync filesystem changes to storage
    /// 
    /// Forces synchronization of all pending changes to storage.
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn sync_filesystem<'a>(context: &mut OperationContext<'a>) -> Result<()> {
        context.inode_manager.sync()?;
        // Assuming InodeManager::sync() also handles syncing its underlying StorageManager if necessary.
        // If StorageManager needs to be synced directly and is not part of InodeManager's sync:
        // context.inode_manager.storage.sync()?; // If storage is public and has sync
        Ok(())
    }
}

// Public API functions

/// Create a new file
pub fn create_file<'a>(path: &str, mode: u32, context: &mut OperationContext<'a>) -> Result<InodeNumber> {
    FilesystemOperations::create_file(path, mode, context)
}

/// Open a file
pub fn open_file<'a>(path: &str, flags: u32, context: &mut OperationContext<'a>) -> Result<FileHandle> {
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
pub fn delete_file<'a>(path: &str, context: &mut OperationContext<'a>) -> Result<()> {
    FilesystemOperations::delete_file(path, context)
}

/// Create a directory
pub fn create_directory<'a>(path: &str, mode: u32, context: &mut OperationContext<'a>) -> Result<InodeNumber> {
    FilesystemOperations::create_directory(path, mode, context)
}

/// List directory contents
pub fn list_directory<'a>(path: &str, context: &mut OperationContext<'a>) -> Result<Vec<DirectoryEntry>> {
    FilesystemOperations::list_directory(path, context)
}

/// Remove a directory
pub fn remove_directory<'a>(path: &str, context: &mut OperationContext<'a>) -> Result<()> {
    FilesystemOperations::remove_directory(path, context)
}

/// Rename a file or directory
pub fn rename<'a>(old_path: &str, new_path: &str, context: &mut OperationContext<'a>) -> Result<()> {
    FilesystemOperations::rename(old_path, new_path, context)
}

/// Get metadata
pub fn get_metadata<'a>(path: &str, context: &mut OperationContext<'a>) -> Result<OperationResult> {
    FilesystemOperations::get_metadata(path, context)
}

/// Change permissions
pub fn change_permissions<'a>(path: &str, mode: u32, context: &mut OperationContext<'a>) -> Result<()> {
    FilesystemOperations::change_permissions(path, mode, context)
}

/// Sync filesystem
pub fn sync_filesystem<'a>(context: &mut OperationContext<'a>) -> Result<()> {
    FilesystemOperations::sync_filesystem(context)
}