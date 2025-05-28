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

//! POSIX Access Control Lists (ACL) Implementation
//!
//! This module implements POSIX ACLs for fine-grained access control,
//! extending beyond traditional UNIX permissions to support complex
//! permission scenarios with multiple users and groups.

use crate::shared::{
    errors::{VexfsError, VexfsResult},
    types::*,
};
use crate::fs_core::permissions::{UserContext, AccessMode};
use super::SecurityError;

#[cfg(not(feature = "kernel"))]
use std::collections::HashMap;
#[cfg(feature = "kernel")]
use alloc::collections::BTreeMap as HashMap;

#[cfg(not(feature = "kernel"))]
use std::vec::Vec;
#[cfg(feature = "kernel")]
use alloc::vec::Vec;

/// Maximum number of ACL entries per inode
pub const MAX_ACL_ENTRIES: usize = 32;

/// Maximum size of extended attribute value
pub const MAX_XATTR_VALUE_SIZE: usize = 64 * 1024; // 64KB

/// ACL entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AclType {
    /// Owner permissions
    User = 0,
    /// Named user permissions
    NamedUser = 1,
    /// Group permissions
    Group = 2,
    /// Named group permissions
    NamedGroup = 3,
    /// Other permissions
    Other = 4,
    /// Mask permissions (maximum effective permissions)
    Mask = 5,
}

/// ACL permissions (compatible with POSIX ACL)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AclPermission {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
}

impl AclPermission {
    /// Create new ACL permission
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Self { read, write, execute }
    }

    /// Create read-only permission
    pub fn read_only() -> Self {
        Self::new(true, false, false)
    }

    /// Create read-write permission
    pub fn read_write() -> Self {
        Self::new(true, true, false)
    }

    /// Create full permission
    pub fn full() -> Self {
        Self::new(true, true, true)
    }

    /// Create no permission
    pub fn none() -> Self {
        Self::new(false, false, false)
    }

    /// Convert to octal representation
    pub fn to_octal(&self) -> u8 {
        let mut octal = 0;
        if self.read { octal |= 4; }
        if self.write { octal |= 2; }
        if self.execute { octal |= 1; }
        octal
    }

    /// Create from octal representation
    pub fn from_octal(octal: u8) -> Self {
        Self {
            read: (octal & 4) != 0,
            write: (octal & 2) != 0,
            execute: (octal & 1) != 0,
        }
    }

    /// Apply mask to permissions
    pub fn apply_mask(&self, mask: &AclPermission) -> AclPermission {
        AclPermission {
            read: self.read && mask.read,
            write: self.write && mask.write,
            execute: self.execute && mask.execute,
        }
    }

    /// Check if this permission allows the requested access
    pub fn allows(&self, requested: &AccessMode) -> bool {
        (!requested.read || self.read) &&
        (!requested.write || self.write) &&
        (!requested.execute || self.execute)
    }
}

impl From<AccessMode> for AclPermission {
    fn from(mode: AccessMode) -> Self {
        Self {
            read: mode.read,
            write: mode.write,
            execute: mode.execute,
        }
    }
}

/// Individual ACL entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AclEntry {
    /// Type of ACL entry
    pub entry_type: AclType,
    /// User or group ID (for named entries)
    pub id: Option<u32>,
    /// Permissions for this entry
    pub permissions: AclPermission,
}

impl AclEntry {
    /// Create a new ACL entry
    pub fn new(entry_type: AclType, id: Option<u32>, permissions: AclPermission) -> Self {
        Self {
            entry_type,
            id,
            permissions,
        }
    }

    /// Create owner entry
    pub fn owner(permissions: AclPermission) -> Self {
        Self::new(AclType::User, None, permissions)
    }

    /// Create named user entry
    pub fn named_user(uid: UserId, permissions: AclPermission) -> Self {
        Self::new(AclType::NamedUser, Some(uid), permissions)
    }

    /// Create group entry
    pub fn group(permissions: AclPermission) -> Self {
        Self::new(AclType::Group, None, permissions)
    }

    /// Create named group entry
    pub fn named_group(gid: GroupId, permissions: AclPermission) -> Self {
        Self::new(AclType::NamedGroup, Some(gid), permissions)
    }

    /// Create other entry
    pub fn other(permissions: AclPermission) -> Self {
        Self::new(AclType::Other, None, permissions)
    }

    /// Create mask entry
    pub fn mask(permissions: AclPermission) -> Self {
        Self::new(AclType::Mask, None, permissions)
    }

    /// Check if this entry matches the given user context
    pub fn matches_user(&self, user: &UserContext, file_owner: UserId, file_group: GroupId) -> bool {
        match self.entry_type {
            AclType::User => user.uid == file_owner,
            AclType::NamedUser => self.id == Some(user.uid),
            AclType::Group => user.gid == file_group || user.is_in_group(file_group),
            AclType::NamedGroup => {
                if let Some(gid) = self.id {
                    user.is_in_group(gid)
                } else {
                    false
                }
            }
            AclType::Other => true, // Other always matches
            AclType::Mask => false, // Mask doesn't match users directly
        }
    }

    /// Serialize ACL entry to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.entry_type as u8);
        
        match self.id {
            Some(id) => {
                bytes.push(1); // Has ID
                bytes.extend_from_slice(&id.to_le_bytes());
            }
            None => {
                bytes.push(0); // No ID
                bytes.extend_from_slice(&[0u8; 4]);
            }
        }
        
        bytes.push(self.permissions.to_octal());
        bytes
    }

    /// Deserialize ACL entry from bytes
    pub fn from_bytes(bytes: &[u8]) -> VexfsResult<Self> {
        if bytes.len() < 7 {
            return Err(VexfsError::InvalidData("ACL entry too short".into()));
        }

        let entry_type = match bytes[0] {
            0 => AclType::User,
            1 => AclType::NamedUser,
            2 => AclType::Group,
            3 => AclType::NamedGroup,
            4 => AclType::Other,
            5 => AclType::Mask,
            _ => return Err(VexfsError::InvalidData("Invalid ACL entry type".into())),
        };

        let id = if bytes[1] == 1 {
            Some(u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]))
        } else {
            None
        };

        let permissions = AclPermission::from_octal(bytes[6]);

        Ok(Self {
            entry_type,
            id,
            permissions,
        })
    }
}

/// Complete Access Control List
#[derive(Debug, Clone)]
pub struct AccessControlList {
    /// List of ACL entries
    pub entries: Vec<AclEntry>,
    /// Default ACL for directories (inherited by new files)
    pub default_entries: Vec<AclEntry>,
    /// Version of the ACL format
    pub version: u32,
    /// Creation timestamp
    pub created_at: u64,
    /// Last modification timestamp
    pub modified_at: u64,
}

impl AccessControlList {
    /// Create a new empty ACL
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            default_entries: Vec::new(),
            version: 1,
            created_at: 0, // TODO: get current time
            modified_at: 0,
        }
    }

    /// Create ACL from POSIX file mode
    pub fn from_mode(mode: FileMode, owner: UserId, group: GroupId) -> Self {
        let mut acl = Self::new();
        
        let owner_perms = AclPermission::from_octal(((mode.permissions() >> 6) & 7) as u8);
        let group_perms = AclPermission::from_octal(((mode.permissions() >> 3) & 7) as u8);
        let other_perms = AclPermission::from_octal((mode.permissions() & 7) as u8);
        
        acl.entries.push(AclEntry::owner(owner_perms));
        acl.entries.push(AclEntry::group(group_perms));
        acl.entries.push(AclEntry::other(other_perms));
        
        acl
    }

    /// Add an ACL entry
    pub fn add_entry(&mut self, entry: AclEntry) -> VexfsResult<()> {
        if self.entries.len() >= MAX_ACL_ENTRIES {
            return Err(VexfsError::OutOfRange("Too many ACL entries".into()));
        }

        // Check for duplicate entries
        for existing in &self.entries {
            if existing.entry_type == entry.entry_type && existing.id == entry.id {
                return Err(VexfsError::InvalidArgument("Duplicate ACL entry".into()));
            }
        }

        self.entries.push(entry);
        self.modified_at = 0; // TODO: get current time
        Ok(())
    }

    /// Remove an ACL entry
    pub fn remove_entry(&mut self, entry_type: AclType, id: Option<u32>) -> VexfsResult<()> {
        let initial_len = self.entries.len();
        self.entries.retain(|entry| {
            !(entry.entry_type == entry_type && entry.id == id)
        });

        if self.entries.len() == initial_len {
            return Err(VexfsError::NotFound);
        }

        self.modified_at = 0; // TODO: get current time
        Ok(())
    }

    /// Get ACL entry by type and ID
    pub fn get_entry(&self, entry_type: AclType, id: Option<u32>) -> Option<&AclEntry> {
        self.entries.iter().find(|entry| {
            entry.entry_type == entry_type && entry.id == id
        })
    }

    /// Update an existing ACL entry
    pub fn update_entry(&mut self, entry_type: AclType, id: Option<u32>, permissions: AclPermission) -> VexfsResult<()> {
        for entry in &mut self.entries {
            if entry.entry_type == entry_type && entry.id == id {
                entry.permissions = permissions;
                self.modified_at = 0; // TODO: get current time
                return Ok(());
            }
        }
        Err(VexfsError::NotFound)
    }

    /// Check if ACL is valid
    pub fn validate(&self) -> VexfsResult<()> {
        let mut has_user = false;
        let mut has_group = false;
        let mut has_other = false;
        let mut has_mask = false;
        let mut has_named_entries = false;

        for entry in &self.entries {
            match entry.entry_type {
                AclType::User => {
                    if has_user {
                        return Err(VexfsError::InvalidData("Multiple user entries".into()));
                    }
                    has_user = true;
                }
                AclType::NamedUser => {
                    has_named_entries = true;
                }
                AclType::Group => {
                    if has_group {
                        return Err(VexfsError::InvalidData("Multiple group entries".into()));
                    }
                    has_group = true;
                }
                AclType::NamedGroup => {
                    has_named_entries = true;
                }
                AclType::Other => {
                    if has_other {
                        return Err(VexfsError::InvalidData("Multiple other entries".into()));
                    }
                    has_other = true;
                }
                AclType::Mask => {
                    if has_mask {
                        return Err(VexfsError::InvalidData("Multiple mask entries".into()));
                    }
                    has_mask = true;
                }
            }
        }

        // Required entries
        if !has_user || !has_group || !has_other {
            return Err(VexfsError::InvalidData("Missing required ACL entries".into()));
        }

        // Mask is required if there are named entries
        if has_named_entries && !has_mask {
            return Err(VexfsError::InvalidData("Mask required with named entries".into()));
        }

        Ok(())
    }

    /// Serialize ACL to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Header
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.created_at.to_le_bytes());
        bytes.extend_from_slice(&self.modified_at.to_le_bytes());
        bytes.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(self.default_entries.len() as u32).to_le_bytes());
        
        // Entries
        for entry in &self.entries {
            let entry_bytes = entry.to_bytes();
            bytes.extend_from_slice(&entry_bytes);
        }
        
        // Default entries
        for entry in &self.default_entries {
            let entry_bytes = entry.to_bytes();
            bytes.extend_from_slice(&entry_bytes);
        }
        
        bytes
    }

    /// Deserialize ACL from bytes
    pub fn from_bytes(bytes: &[u8]) -> VexfsResult<Self> {
        if bytes.len() < 24 {
            return Err(VexfsError::InvalidData("ACL data too short".into()));
        }

        let version = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let created_at = u64::from_le_bytes([
            bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11]
        ]);
        let modified_at = u64::from_le_bytes([
            bytes[12], bytes[13], bytes[14], bytes[15],
            bytes[16], bytes[17], bytes[18], bytes[19]
        ]);
        let entry_count = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as usize;
        let default_count = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]) as usize;

        if entry_count > MAX_ACL_ENTRIES || default_count > MAX_ACL_ENTRIES {
            return Err(VexfsError::InvalidData("Too many ACL entries".into()));
        }

        let mut offset = 28;
        let mut entries = Vec::new();
        
        // Parse entries
        for _ in 0..entry_count {
            if offset + 7 > bytes.len() {
                return Err(VexfsError::InvalidData("Truncated ACL entry".into()));
            }
            
            let entry = AclEntry::from_bytes(&bytes[offset..offset + 7])?;
            entries.push(entry);
            offset += 7;
        }
        
        // Parse default entries
        let mut default_entries = Vec::new();
        for _ in 0..default_count {
            if offset + 7 > bytes.len() {
                return Err(VexfsError::InvalidData("Truncated default ACL entry".into()));
            }
            
            let entry = AclEntry::from_bytes(&bytes[offset..offset + 7])?;
            default_entries.push(entry);
            offset += 7;
        }

        let acl = Self {
            entries,
            default_entries,
            version,
            created_at,
            modified_at,
        };

        acl.validate()?;
        Ok(acl)
    }
}

/// ACL Manager for handling ACL operations
pub struct AclManager {
    /// Cache of ACLs by inode
    acl_cache: HashMap<InodeNumber, AccessControlList>,
    /// Extended attributes manager
    xattr_manager: XattrManager,
}

impl AclManager {
    /// Create a new ACL manager
    pub fn new() -> Self {
        Self {
            acl_cache: HashMap::new(),
            xattr_manager: XattrManager::new(),
        }
    }

    /// Set ACL for an inode
    pub fn set_acl(&mut self, inode: InodeNumber, acl: AccessControlList) -> VexfsResult<()> {
        // Validate ACL
        acl.validate()?;

        // Store in extended attributes
        let acl_bytes = acl.to_bytes();
        self.xattr_manager.set_xattr(inode, "system.posix_acl_access", &acl_bytes)?;

        // Cache the ACL
        self.acl_cache.insert(inode, acl);

        Ok(())
    }

    /// Get ACL for an inode
    pub fn get_acl(&mut self, inode: InodeNumber) -> VexfsResult<AccessControlList> {
        // Check cache first
        if let Some(acl) = self.acl_cache.get(&inode) {
            return Ok(acl.clone());
        }

        // Load from extended attributes
        match self.xattr_manager.get_xattr(inode, "system.posix_acl_access") {
            Ok(acl_bytes) => {
                let acl = AccessControlList::from_bytes(&acl_bytes)?;
                self.acl_cache.insert(inode, acl.clone());
                Ok(acl)
            }
            Err(VexfsError::NotFound) => {
                Err(VexfsError::NotFound)
            }
            Err(e) => Err(e),
        }
    }

    /// Remove ACL for an inode
    pub fn remove_acl(&mut self, inode: InodeNumber) -> VexfsResult<()> {
        self.acl_cache.remove(&inode);
        self.xattr_manager.remove_xattr(inode, "system.posix_acl_access")
    }

    /// Check ACL permission for a user
    pub fn check_acl_permission(
        &self,
        acl: &AccessControlList,
        user: &UserContext,
        requested: &AccessMode,
    ) -> VexfsResult<bool> {
        // Convert AccessMode to AclPermission for comparison
        let requested_perm = AclPermission::from(*requested);

        // Find the most specific matching entry
        let mut matching_entry: Option<&AclEntry> = None;
        let mut entry_priority = -1i32;

        // We need file owner and group info - for now, assume they're available
        // In real implementation, this would come from the inode
        let file_owner = 0; // TODO: get from inode
        let file_group = 0; // TODO: get from inode

        for entry in &acl.entries {
            if entry.matches_user(user, file_owner, file_group) {
                let priority = match entry.entry_type {
                    AclType::User => 5,
                    AclType::NamedUser => 4,
                    AclType::Group => 2,
                    AclType::NamedGroup => 3,
                    AclType::Other => 1,
                    AclType::Mask => 0, // Mask doesn't match directly
                };

                if priority > entry_priority {
                    matching_entry = Some(entry);
                    entry_priority = priority;
                }
            }
        }

        if let Some(entry) = matching_entry {
            let mut effective_perms = entry.permissions;

            // Apply mask if present and entry is not user owner or other
            if matches!(entry.entry_type, AclType::NamedUser | AclType::Group | AclType::NamedGroup) {
                if let Some(mask_entry) = acl.get_entry(AclType::Mask, None) {
                    effective_perms = effective_perms.apply_mask(&mask_entry.permissions);
                }
            }

            Ok(effective_perms.allows(&requested))
        } else {
            // No matching entry found
            Ok(false)
        }
    }

    /// Clear ACL cache
    pub fn clear_cache(&mut self) {
        self.acl_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.acl_cache.len(), 0) // (cached_acls, dirty_acls)
    }
}

/// Extended Attributes Manager
pub struct XattrManager {
    /// Cache of extended attributes by inode
    xattr_cache: HashMap<InodeNumber, HashMap<String, Vec<u8>>>,
}

impl XattrManager {
    /// Create a new extended attributes manager
    pub fn new() -> Self {
        Self {
            xattr_cache: HashMap::new(),
        }
    }

    /// Set extended attribute
    pub fn set_xattr(&mut self, inode: InodeNumber, name: &str, value: &[u8]) -> VexfsResult<()> {
        if value.len() > MAX_XATTR_VALUE_SIZE {
            return Err(VexfsError::InvalidArgument("Extended attribute value too large".into()));
        }

        let inode_xattrs = self.xattr_cache.entry(inode).or_insert_with(HashMap::new);
        inode_xattrs.insert(name.to_string(), value.to_vec());

        // TODO: Persist to storage
        Ok(())
    }

    /// Get extended attribute
    pub fn get_xattr(&self, inode: InodeNumber, name: &str) -> VexfsResult<Vec<u8>> {
        if let Some(inode_xattrs) = self.xattr_cache.get(&inode) {
            if let Some(value) = inode_xattrs.get(name) {
                return Ok(value.clone());
            }
        }

        // TODO: Load from storage
        Err(VexfsError::NotFound)
    }

    /// Remove extended attribute
    pub fn remove_xattr(&mut self, inode: InodeNumber, name: &str) -> VexfsResult<()> {
        if let Some(inode_xattrs) = self.xattr_cache.get_mut(&inode) {
            if inode_xattrs.remove(name).is_some() {
                // TODO: Remove from storage
                return Ok(());
            }
        }

        Err(VexfsError::NotFound)
    }

    /// List extended attributes for an inode
    pub fn list_xattrs(&self, inode: InodeNumber) -> VexfsResult<Vec<String>> {
        if let Some(inode_xattrs) = self.xattr_cache.get(&inode) {
            Ok(inode_xattrs.keys().cloned().collect())
        } else {
            // TODO: Load from storage
            Ok(Vec::new())
        }
    }

    /// Clear extended attributes cache
    pub fn clear_cache(&mut self) {
        self.xattr_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acl_permission() {
        let perm = AclPermission::new(true, false, true);
        assert_eq!(perm.to_octal(), 5); // r-x = 5

        let from_octal = AclPermission::from_octal(6);
        assert!(from_octal.read);
        assert!(from_octal.write);
        assert!(!from_octal.execute);
    }

    #[test]
    fn test_acl_entry() {
        let entry = AclEntry::named_user(1000, AclPermission::read_write());
        assert_eq!(entry.entry_type, AclType::NamedUser);
        assert_eq!(entry.id, Some(1000));
        assert!(entry.permissions.read);
        assert!(entry.permissions.write);
        assert!(!entry.permissions.execute);
    }

    #[test]
    fn test_acl_serialization() {
        let mut acl = AccessControlList::new();
        acl.add_entry(AclEntry::owner(AclPermission::full())).unwrap();
        acl.add_entry(AclEntry::group(AclPermission::read_only())).unwrap();
        acl.add_entry(AclEntry::other(AclPermission::none())).unwrap();

        let bytes = acl.to_bytes();
        let deserialized = AccessControlList::from_bytes(&bytes).unwrap();

        assert_eq!(acl.entries.len(), deserialized.entries.len());
        assert_eq!(acl.version, deserialized.version);
    }

    #[test]
    fn test_acl_validation() {
        let mut acl = AccessControlList::new();
        
        // Invalid ACL (missing required entries)
        assert!(acl.validate().is_err());

        // Valid minimal ACL
        acl.add_entry(AclEntry::owner(AclPermission::full())).unwrap();
        acl.add_entry(AclEntry::group(AclPermission::read_only())).unwrap();
        acl.add_entry(AclEntry::other(AclPermission::none())).unwrap();
        assert!(acl.validate().is_ok());

        // Add named user (requires mask)
        acl.add_entry(AclEntry::named_user(1000, AclPermission::read_write())).unwrap();
        assert!(acl.validate().is_err()); // Missing mask

        // Add mask
        acl.add_entry(AclEntry::mask(AclPermission::read_write())).unwrap();
        assert!(acl.validate().is_ok());
    }

    #[test]
    fn test_mask_application() {
        let perm = AclPermission::full();
        let mask = AclPermission::read_only();
        let masked = perm.apply_mask(&mask);

        assert!(masked.read);
        assert!(!masked.write);
        assert!(!masked.execute);
    }
}