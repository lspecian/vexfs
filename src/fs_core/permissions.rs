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

//! Permissions and Security Module
//!
//! This module implements UNIX-style permission checking and access control
//! with support for special permissions and security validations.

use crate::shared::{
    errors::VexfsError,
    types::*,
    constants::*,
};
use super::{FsResult, InodeManager};
use super::inode::Inode;

/// Access mode flags for permission checking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessMode {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl AccessMode {
    /// Create new access mode
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Self { read, write, execute }
    }

    /// Create read-only access mode
    pub fn read_only() -> Self {
        Self::new(true, false, false)
    }

    /// Create write-only access mode
    pub fn write_only() -> Self {
        Self::new(false, true, false)
    }

    /// Create read-write access mode
    pub fn read_write() -> Self {
        Self::new(true, true, false)
    }

    /// Create execute-only access mode
    pub fn execute_only() -> Self {
        Self::new(false, false, true)
    }

    /// Create full access mode
    pub fn full() -> Self {
        Self::new(true, true, true)
    }

    /// Convert to octal representation
    pub fn to_octal(&self) -> u16 {
        let mut octal = 0;
        if self.read { octal |= 4; }
        if self.write { octal |= 2; }
        if self.execute { octal |= 1; }
        octal
    }

    /// Create from octal representation
    pub fn from_octal(octal: u16) -> Self {
        Self {
            read: (octal & 4) != 0,
            write: (octal & 2) != 0,
            execute: (octal & 1) != 0,
        }
    }
    
    /// Create from file open flags
    pub fn from_flags(flags: u32) -> Self {
        let read_flag = 0x01;
        let write_flag = 0x02;
        Self {
            read: (flags & read_flag) != 0,
            write: (flags & write_flag) != 0,
            execute: false, // Files don't need execute for open
        }
    }
}

/// User context for permission checking
#[derive(Debug, Clone, Copy)]
pub struct UserContext {
    pub uid: UserId,
    pub gid: GroupId,
    pub groups: &'static [GroupId], // Additional groups
    pub is_superuser: bool,
}

impl UserContext {
    /// Create new user context
    pub fn new(uid: UserId, gid: GroupId, groups: &'static [GroupId]) -> Self {
        Self {
            uid,
            gid,
            groups,
            is_superuser: uid == 0,
        }
    }

    /// Create root user context
    pub fn root() -> Self {
        Self {
            uid: 0,
            gid: 0,
            groups: &[],
            is_superuser: true,
        }
    }

    /// Check if user is in a specific group
    pub fn is_in_group(&self, gid: GroupId) -> bool {
        self.gid == gid || self.groups.contains(&gid)
    }
}

/// Access check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessCheck {
    /// Access granted
    Granted,
    /// Access denied due to insufficient permissions
    Denied,
    /// Access denied due to ownership mismatch
    NotOwner,
    /// Access denied due to group mismatch
    NotInGroup,
}

impl AccessCheck {
    /// Check if access was granted
    pub fn is_granted(&self) -> bool {
        matches!(self, AccessCheck::Granted)
    }

    /// Convert to Result type
    pub fn to_result(self) -> FsResult<()> {
        match self {
            AccessCheck::Granted => Ok(()),
            _ => Err(VexfsError::PermissionDenied("Access denied".into())),
        }
    }
}

/// Permission checker for filesystem access control
pub struct PermissionChecker;

impl PermissionChecker {
    /// Check if user has requested access to an inode
    pub fn check_access(
        inode_manager: &mut InodeManager,
        inode_num: InodeNumber,
        user: &UserContext,
        requested: AccessMode,
    ) -> FsResult<AccessCheck> {
        let inode = inode_manager.get_inode(inode_num)?;

        // Superuser bypasses most permission checks
        if user.is_superuser {
            // Even superuser can't execute non-executable files
            if requested.execute && !inode.is_dir() {
                let mode = inode.mode;
                if !Self::has_any_execute_permission(mode) {
                    return Ok(AccessCheck::Denied);
                }
            }
            return Ok(AccessCheck::Granted);
        }

        // Check ownership and group membership
        let is_owner = user.uid == inode.uid;
        let is_group_member = user.is_in_group(inode.gid);

        // Determine which permission bits to check
        let effective_mode = if is_owner {
            // Owner permissions (bits 8-6)
            AccessMode::from_octal(((inode.mode.permissions() >> 6) & 7) as u16)
        } else if is_group_member {
            // Group permissions (bits 5-3)
            AccessMode::from_octal(((inode.mode.permissions() >> 3) & 7) as u16)
        } else {
            // Other permissions (bits 2-0)
            AccessMode::from_octal((inode.mode.permissions() & 7) as u16)
        };

        // Check each requested permission
        if requested.read && !effective_mode.read {
            return Ok(AccessCheck::Denied);
        }
        if requested.write && !effective_mode.write {
            return Ok(AccessCheck::Denied);
        }
        if requested.execute && !effective_mode.execute {
            return Ok(AccessCheck::Denied);
        }

        Ok(AccessCheck::Granted)
    }

    /// Check if user can read from an inode
    pub fn check_read(
        inode_manager: &mut InodeManager,
        inode_num: InodeNumber,
        user: &UserContext,
    ) -> FsResult<AccessCheck> {
        Self::check_access(inode_manager, inode_num, user, AccessMode::read_only())
    }

    /// Check if user can write to an inode
    pub fn check_write(
        inode_manager: &mut InodeManager,
        inode_num: InodeNumber,
        user: &UserContext,
    ) -> FsResult<AccessCheck> {
        Self::check_access(inode_manager, inode_num, user, AccessMode::write_only())
    }

    /// Check if user can execute an inode
    pub fn check_execute(
        inode_manager: &mut InodeManager,
        inode_num: InodeNumber,
        user: &UserContext,
    ) -> FsResult<AccessCheck> {
        Self::check_access(inode_manager, inode_num, user, AccessMode::execute_only())
    }

    /// Check if user can create entries in a directory
    pub fn check_create(
        inode_manager: &mut InodeManager,
        dir_inode: InodeNumber,
        user: &UserContext,
    ) -> FsResult<AccessCheck> {
        // Need write + execute permissions on directory
        let access_mode = AccessMode::new(false, true, true);
        Self::check_access(inode_manager, dir_inode, user, access_mode)
    }

    /// Check if user can delete entries from a directory
    pub fn check_delete(
        inode_manager: &mut InodeManager,
        dir_inode: InodeNumber,
        file_inode: InodeNumber,
        user: &UserContext,
    ) -> FsResult<AccessCheck> {
        // Check directory write permission
        let dir_check = Self::check_create(inode_manager, dir_inode, user)?;
        if !dir_check.is_granted() {
            return Ok(dir_check);
        }

        // Check for sticky bit on directory
        let dir = inode_manager.get_inode(dir_inode)?;
        if Self::has_sticky_bit(dir.mode) {
            // With sticky bit, only owner of file or directory can delete
            if !user.is_superuser && user.uid != dir.uid {
                let file = inode_manager.get_inode(file_inode)?;
                if user.uid != file.uid {
                    return Ok(AccessCheck::NotOwner);
                }
            }
        }

        Ok(AccessCheck::Granted)
    }

    /// Check if user can change ownership of an inode
    pub fn check_chown(
        inode_manager: &mut InodeManager,
        inode_num: InodeNumber,
        user: &UserContext,
        new_uid: UserId,
        new_gid: GroupId,
    ) -> FsResult<AccessCheck> {
        // Only superuser or owner can change ownership
        if user.is_superuser {
            return Ok(AccessCheck::Granted);
        }

        let inode = inode_manager.get_inode(inode_num)?;
        
        // Owner can only change group to a group they belong to
        if user.uid == inode.uid {
            if new_uid != inode.uid {
                return Ok(AccessCheck::Denied); // Can't change owner
            }
            if !user.is_in_group(new_gid) {
                return Ok(AccessCheck::NotInGroup);
            }
            return Ok(AccessCheck::Granted);
        }

        Ok(AccessCheck::NotOwner)
    }

    /// Check if user can change permissions of an inode
    pub fn check_chmod(
        inode_manager: &mut InodeManager,
        inode_num: InodeNumber,
        user: &UserContext,
    ) -> FsResult<AccessCheck> {
        // Only superuser or owner can change permissions
        if user.is_superuser {
            return Ok(AccessCheck::Granted);
        }

        let inode = inode_manager.get_inode(inode_num)?;
        if user.uid == inode.uid {
            Ok(AccessCheck::Granted)
        } else {
            Ok(AccessCheck::NotOwner)
        }
    }

    /// Check special permission bits
    fn has_setuid_bit(mode: FileMode) -> bool {
        (mode.permissions() & 0o4000) != 0
    }

    fn has_setgid_bit(mode: FileMode) -> bool {
        (mode.permissions() & 0o2000) != 0
    }

    fn has_sticky_bit(mode: FileMode) -> bool {
        (mode.permissions() & 0o1000) != 0
    }

    /// Check if any execute permission is set
    fn has_any_execute_permission(mode: FileMode) -> bool {
        (mode.permissions() & 0o111) != 0
    }

    /// Validate permission mode for setting
    pub fn validate_mode(mode: u16, file_type: FileType, user: &UserContext) -> FsResult<()> {
        // Check for invalid permission bits
        if mode & !0o7777 != 0 {
            return Err(VexfsError::InvalidArgument("Invalid permission bits".into()));
        }

        // Non-superuser restrictions
        if !user.is_superuser {
            // Can't set setuid/setgid on directories
            if file_type == FileType::Directory {
                if mode & 0o4000 != 0 {
                    return Err(VexfsError::PermissionDenied("Cannot set setuid on directory".into()));
                }
            }

            // Special restrictions for setgid
            if mode & 0o2000 != 0 && file_type == FileType::Regular {
                // Additional checks could go here
            }
        }

        Ok(())
    }

    /// Calculate effective user ID after setuid/setgid
    pub fn calculate_effective_uid(
        inode_manager: &mut InodeManager,
        executable_inode: InodeNumber,
        user: &UserContext,
    ) -> FsResult<UserId> {
        let inode = inode_manager.get_inode(executable_inode)?;
        
        if Self::has_setuid_bit(inode.mode) && inode.file_type == FileType::Regular {
            Ok(inode.uid)
        } else {
            Ok(user.uid)
        }
    }

    /// Calculate effective group ID after setuid/setgid
    pub fn calculate_effective_gid(
        inode_manager: &mut InodeManager,
        executable_inode: InodeNumber,
        user: &UserContext,
    ) -> FsResult<GroupId> {
        let inode = inode_manager.get_inode(executable_inode)?;
        
        if Self::has_setgid_bit(inode.mode) {
            Ok(inode.gid)
        } else {
            Ok(user.gid)
        }
    }
}

// Public helper functions that were being imported
pub fn can_read(inode: &Inode, user: &UserContext) -> bool {
    // Simplified permission check without InodeManager dependency
    let is_owner = user.uid == inode.uid;
    let is_group_member = user.is_in_group(inode.gid);
    
    if user.is_superuser {
        return true;
    }
    
    let mode = inode.mode.permissions();
    if is_owner {
        (mode >> 6) & 4 != 0
    } else if is_group_member {
        (mode >> 3) & 4 != 0
    } else {
        mode & 4 != 0
    }
}

pub fn can_write(inode: &Inode, user: &UserContext) -> bool {
    // Simplified permission check without InodeManager dependency
    let is_owner = user.uid == inode.uid;
    let is_group_member = user.is_in_group(inode.gid);
    
    if user.is_superuser {
        return true;
    }
    
    let mode = inode.mode.permissions();
    if is_owner {
        (mode >> 6) & 2 != 0
    } else if is_group_member {
        (mode >> 3) & 2 != 0
    } else {
        mode & 2 != 0
    }
}

pub fn can_access_directory(inode: &Inode, user: &UserContext) -> bool {
    // Simplified permission check without InodeManager dependency
    let is_owner = user.uid == inode.uid;
    let is_group_member = user.is_in_group(inode.gid);
    
    if user.is_superuser {
        return true;
    }
    
    let mode = inode.mode.permissions();
    if is_owner {
        (mode >> 6) & 1 != 0
    } else if is_group_member {
        (mode >> 3) & 1 != 0
    } else {
        mode & 1 != 0
    }
}

pub fn can_list_directory(inode: &Inode, user: &UserContext) -> bool {
    can_read(inode, user)
}

pub fn can_create_in_directory(inode: &Inode, user: &UserContext) -> bool {
    can_write(inode, user) && can_access_directory(inode, user)
}

pub fn can_delete_from_directory(inode: &Inode, user: &UserContext) -> bool {
    can_write(inode, user) && can_access_directory(inode, user)
}

pub fn permission_bits(mode: FileMode) -> u16 {
    mode & 0o777
}

pub fn apply_umask(mode: FileMode, umask: u16) -> FileMode {
    mode & !(umask as u32)
}

pub fn check_read_permission(inode: &Inode, user: &UserContext) -> FsResult<()> {
    if can_read(inode, user) {
        Ok(())
    } else {
        Err(VexfsError::PermissionDenied("Read permission denied".into()))
    }
}

pub fn check_write_permission(inode: &Inode, user: &UserContext) -> FsResult<()> {
    if can_write(inode, user) {
        Ok(())
    } else {
        Err(VexfsError::PermissionDenied("Write permission denied".into()))
    }
}

pub fn check_create_permission(inode: &Inode, user: &UserContext) -> FsResult<()> {
    if can_create_in_directory(inode, user) {
        Ok(())
    } else {
        Err(VexfsError::PermissionDenied("Create permission denied".into()))
    }
}

pub fn check_delete_permission(inode: &Inode, user: &UserContext) -> FsResult<()> {
    if can_delete_from_directory(inode, user) {
        Ok(())
    } else {
        Err(VexfsError::PermissionDenied("Delete permission denied".into()))
    }
}

/// Security policy enforcement
pub struct SecurityPolicy;

impl SecurityPolicy {
    /// Check if operation should be audited
    pub fn should_audit(
        operation: &str,
        inode_num: InodeNumber,
        user: &UserContext,
    ) -> bool {
        // Audit superuser operations
        if user.is_superuser {
            return true;
        }

        // Audit sensitive operations
        matches!(operation, "chmod" | "chown" | "delete" | "setuid_exec")
    }

    /// Check for security violations
    pub fn check_security_violation(
        operation: &str,
        inode_num: InodeNumber,
        user: &UserContext,
    ) -> FsResult<()> {
        // Placeholder for security policy checks
        // Could include:
        // - MAC (Mandatory Access Control) checks
        // - SELinux-style policies
        // - Capability checks
        // - Rate limiting
        
        Ok(())
    }

    /// Validate file creation parameters
    pub fn validate_creation(
        name: &str,
        mode: FileMode,
        file_type: FileType,
        user: &UserContext,
    ) -> FsResult<()> {
        // Validate filename
        if name.len() > MAX_FILENAME_LENGTH {
            return Err(VexfsError::NameTooLong);
        }

        // Check for dangerous filenames
        if name.starts_with('.') && name.len() > 1 {
            // Allow normal dotfiles but be cautious about unusual patterns
        }

        // Validate permissions for file type
        PermissionChecker::validate_mode(mode.permissions(), file_type, user)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_mode() {
        let mode = AccessMode::new(true, false, true);
        assert_eq!(mode.to_octal(), 5); // r-x = 5
        
        let from_octal = AccessMode::from_octal(6);
        assert!(from_octal.read);
        assert!(from_octal.write);
        assert!(!from_octal.execute);
    }

    #[test]
    fn test_user_context() {
        let user = UserContext::new(1000, 1000, &[100, 200]);
        assert!(user.is_in_group(1000));
        assert!(user.is_in_group(100));
        assert!(!user.is_in_group(300));
        
        let root = UserContext::root();
        assert!(root.is_superuser);
    }

    #[test]
    fn test_permission_validation() {
        let user = UserContext::new(1000, 1000, &[]);
        
        // Valid mode
        assert!(PermissionChecker::validate_mode(0o644, FileType::Regular, &user).is_ok());
        
        // Invalid mode (too many bits)
        assert!(PermissionChecker::validate_mode(0o10000, FileType::Regular, &user).is_err());
    }

    #[test]
    fn test_access_check_result() {
        assert!(AccessCheck::Granted.is_granted());
        assert!(!AccessCheck::Denied.is_granted());
        
        assert!(AccessCheck::Granted.to_result().is_ok());
        assert!(AccessCheck::Denied.to_result().is_err());
    }
}