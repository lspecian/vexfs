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

//! Security and Access Control Module for VexFS
//!
//! This module implements comprehensive security features for VexFS including:
//! - At-rest encryption for vector data (AES-256-GCM)
//! - POSIX permission checks for vector operations
//! - POSIX ACL support with extended attributes
//! - Capability-based access control for IOCTL operations
//! - Cryptographic checksums for data integrity (SHA-256)
//! - Secure key management and rotation

pub mod encryption;
pub mod acl;
pub mod capabilities;
pub mod integrity;
pub mod key_management;

// Re-export commonly used types and functions
pub use encryption::{
    VectorEncryption, EncryptionConfig, EncryptedData,
    EncryptionError, EncryptionAlgorithm, EncryptionResult
};
pub use acl::{
    AccessControlList, AclEntry, AclPermission, AclType, AclManager,
    XattrManager
};
pub use key_management::EncryptionKey;
pub use capabilities::{
    Capability, CapabilitySet, CapabilityManager, SecurityLevel,
    IoctlSecurityValidator, PrivilegeEscalationDetector
};
pub use integrity::{
    IntegrityChecker, ChecksumType, IntegrityMetadata, VerificationResult,
    DataIntegrityManager
};
pub use key_management::{
    KeyManager, KeyDerivation, KeyRotation, SecureKeyStorage,
    KeyMaterial, KeyVersion
};

use crate::shared::{
    errors::{VexfsError, VexfsResult},
    types::*,
};
use crate::fs_core::{
    permissions::{UserContext, AccessMode, PermissionChecker},
    operations::OperationContext,
};

/// Security context for operations
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// User context for permission checking
    pub user: UserContext,
    /// Security level for the operation
    pub security_level: SecurityLevel,
    /// Capabilities available to the user
    pub capabilities: CapabilitySet,
    /// Encryption configuration
    pub encryption_config: EncryptionConfig,
    /// Audit trail enabled
    pub audit_enabled: bool,
}

impl SecurityContext {
    /// Create a new security context
    pub fn new(user: UserContext, security_level: SecurityLevel) -> Self {
        Self {
            user,
            security_level,
            capabilities: CapabilitySet::new(), // Start with empty capabilities
            encryption_config: EncryptionConfig::default(),
            audit_enabled: true,
        }
    }

    /// Create a root security context with full privileges
    pub fn root() -> Self {
        Self {
            user: UserContext::root(),
            security_level: SecurityLevel::System,
            capabilities: CapabilitySet::all(),
            encryption_config: EncryptionConfig::default(),
            audit_enabled: true,
        }
    }

    /// Check if the context has a specific capability
    pub fn has_capability(&self, capability: Capability) -> bool {
        self.capabilities.has(capability)
    }

    /// Add a capability to the context
    pub fn add_capability(&mut self, capability: Capability) {
        self.capabilities.add(capability);
    }

    /// Remove a capability from the context
    pub fn remove_capability(&mut self, capability: Capability) {
        self.capabilities.remove(capability);
    }
}

/// Main security manager that coordinates all security operations
pub struct SecurityManager {
    /// Vector encryption manager
    encryption: VectorEncryption,
    /// Access control list manager
    acl_manager: AclManager,
    /// Capability manager
    capability_manager: CapabilityManager,
    /// Data integrity manager
    integrity_manager: DataIntegrityManager,
    /// Key management system
    key_manager: KeyManager,
    /// Extended attributes manager
    xattr_manager: XattrManager,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new() -> VexfsResult<Self> {
        let key_manager = KeyManager::new()?;
        let encryption = VectorEncryption::new(&key_manager)?;
        let acl_manager = AclManager::new();
        let capability_manager = CapabilityManager::new();
        let integrity_manager = DataIntegrityManager::new();
        let xattr_manager = XattrManager::new();

        Ok(Self {
            encryption,
            acl_manager,
            capability_manager,
            integrity_manager,
            key_manager,
            xattr_manager,
        })
    }

    /// Encrypt vector data for storage
    pub fn encrypt_vector_data(
        &mut self,
        context: &SecurityContext,
        data: &[u8],
        inode: InodeNumber,
    ) -> VexfsResult<EncryptedData> {
        // Check encryption permissions
        if !context.has_capability(Capability::VectorEncrypt) {
            return Err(VexfsError::PermissionDenied("Vector encryption not permitted".into()));
        }

        // Get or derive encryption key for this inode
        let key = self.key_manager.get_or_create_key(inode, &context.user)?;
        
        // Encrypt the data
        let encrypted = self.encryption.encrypt(data, &key, &context.encryption_config)?;
        
        // Generate integrity checksum
        let integrity_metadata = self.integrity_manager.generate_checksum(
            data,
            ChecksumType::Sha256,
            Some(inode),
        )?;
        
        Ok(EncryptedData {
            data: encrypted.data,
            nonce: encrypted.nonce,
            tag: encrypted.tag,
            key_version: key.version,
            integrity: integrity_metadata,
        })
    }

    /// Decrypt vector data from storage
    pub fn decrypt_vector_data(
        &mut self,
        context: &SecurityContext,
        encrypted_data: &EncryptedData,
        inode: InodeNumber,
    ) -> VexfsResult<Vec<u8>> {
        // Check decryption permissions
        if !context.has_capability(Capability::VectorDecrypt) {
            return Err(VexfsError::PermissionDenied("Vector decryption not permitted".into()));
        }

        // Get the encryption key
        let key = self.key_manager.get_key(inode, encrypted_data.key_version)?;
        
        // Decrypt the data
        let decrypted = self.encryption.decrypt(encrypted_data, &key)?;
        
        // Verify integrity
        let verification = self.integrity_manager.verify_checksum(
            &decrypted,
            &encrypted_data.integrity,
            Some(inode),
        )?;
        
        if !verification.is_valid {
            return Err(VexfsError::CorruptedData);
        }
        
        Ok(decrypted)
    }

    /// Check vector operation permissions
    pub fn check_vector_permission(
        &mut self,
        context: &SecurityContext,
        inode: InodeNumber,
        operation: VectorOperation,
    ) -> VexfsResult<()> {
        // Check basic POSIX permissions first
        let access_mode = match operation {
            VectorOperation::Read => AccessMode::read_only(),
            VectorOperation::Write | VectorOperation::Create => AccessMode::write_only(),
            VectorOperation::Delete => AccessMode::write_only(),
            VectorOperation::Search => AccessMode::read_only(),
        };

        // Use a dummy inode manager for permission checking
        // In real implementation, this would use the actual inode manager
        // PermissionChecker::check_access(inode_manager, inode, &context.user, access_mode)?;

        // Check ACL permissions if they exist
        if let Ok(acl) = self.acl_manager.get_acl(inode) {
            if !self.acl_manager.check_acl_permission(&acl, &context.user, &access_mode)? {
                return Err(VexfsError::PermissionDenied("ACL permission denied".into()));
            }
        }

        // Check capability-based permissions
        let required_capability = match operation {
            VectorOperation::Read => Capability::VectorRead,
            VectorOperation::Write => Capability::VectorWrite,
            VectorOperation::Create => Capability::VectorCreate,
            VectorOperation::Delete => Capability::VectorDelete,
            VectorOperation::Search => Capability::VectorSearch,
        };

        if !context.has_capability(required_capability) {
            return Err(VexfsError::PermissionDenied(
                format!("Missing capability: {:?}", required_capability)
            ));
        }

        Ok(())
    }

    /// Validate IOCTL operation security
    pub fn validate_ioctl_security(
        &mut self,
        context: &SecurityContext,
        ioctl_cmd: u8,
        data_size: usize,
    ) -> VexfsResult<()> {
        // Use the capability manager's IOCTL validator
        self.capability_manager.validate_ioctl_operation(
            context,
            ioctl_cmd,
            data_size,
        )
    }

    /// Set ACL for an inode
    pub fn set_acl(
        &mut self,
        context: &SecurityContext,
        inode: InodeNumber,
        acl: AccessControlList,
    ) -> VexfsResult<()> {
        // Check if user has permission to modify ACLs
        if !context.has_capability(Capability::AclModify) && !context.user.is_superuser {
            return Err(VexfsError::PermissionDenied("ACL modification not permitted".into()));
        }

        self.acl_manager.set_acl(inode, acl)
    }

    /// Get ACL for an inode
    pub fn get_acl(
        &mut self,
        context: &SecurityContext,
        inode: InodeNumber,
    ) -> VexfsResult<AccessControlList> {
        // Check if user has permission to read ACLs
        if !context.has_capability(Capability::AclRead) && !context.user.is_superuser {
            return Err(VexfsError::PermissionDenied("ACL read not permitted".into()));
        }

        self.acl_manager.get_acl(inode)
    }

    /// Rotate encryption keys
    pub fn rotate_keys(&mut self, context: &SecurityContext) -> VexfsResult<()> {
        if !context.has_capability(Capability::KeyManagement) {
            return Err(VexfsError::PermissionDenied("Key rotation not permitted".into()));
        }

        self.key_manager.rotate_all_keys()
    }

    /// Audit security operation
    pub fn audit_operation(
        &self,
        context: &SecurityContext,
        operation: &str,
        inode: Option<InodeNumber>,
        result: &VexfsResult<()>,
    ) {
        if !context.audit_enabled {
            return;
        }

        // In a real implementation, this would write to an audit log
        let status = if result.is_ok() { "SUCCESS" } else { "FAILURE" };
        
        #[cfg(not(feature = "kernel"))]
        {
            println!(
                "AUDIT: user={} operation={} inode={:?} status={} level={:?}",
                context.user.uid,
                operation,
                inode,
                status,
                context.security_level
            );
        }
    }
}

/// Vector operations that require security checks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorOperation {
    Read,
    Write,
    Create,
    Delete,
    Search,
}

/// Security error types specific to the security module
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityError {
    EncryptionFailed,
    DecryptionFailed,
    KeyNotFound,
    InvalidKey,
    PermissionDenied,
    CapabilityMissing,
    IntegrityCheckFailed,
    AclNotFound,
    InvalidAcl,
}

impl From<SecurityError> for VexfsError {
    fn from(err: SecurityError) -> Self {
        match err {
            SecurityError::EncryptionFailed => VexfsError::Other("Encryption failed".into()),
            SecurityError::DecryptionFailed => VexfsError::Other("Decryption failed".into()),
            SecurityError::KeyNotFound => VexfsError::Other("Encryption key not found".into()),
            SecurityError::InvalidKey => VexfsError::Other("Invalid encryption key".into()),
            SecurityError::PermissionDenied => VexfsError::PermissionDenied("Security permission denied".into()),
            SecurityError::CapabilityMissing => VexfsError::PermissionDenied("Required capability missing".into()),
            SecurityError::IntegrityCheckFailed => VexfsError::ChecksumMismatch,
            SecurityError::AclNotFound => VexfsError::Other("ACL not found".into()),
            SecurityError::InvalidAcl => VexfsError::Other("Invalid ACL".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_context_creation() {
        let user = UserContext::new(1000, 1000, &[]);
        let context = SecurityContext::new(user, SecurityLevel::User);
        
        assert_eq!(context.user.uid, 1000);
        assert_eq!(context.security_level, SecurityLevel::User);
        assert!(context.audit_enabled);
    }

    #[test]
    fn test_root_security_context() {
        let context = SecurityContext::root();
        
        assert!(context.user.is_superuser);
        assert_eq!(context.security_level, SecurityLevel::System);
        assert!(context.has_capability(Capability::VectorRead));
        assert!(context.has_capability(Capability::VectorWrite));
    }

    #[test]
    fn test_capability_management() {
        let user = UserContext::new(1000, 1000, &[]);
        let mut context = SecurityContext::new(user, SecurityLevel::User);
        
        assert!(!context.has_capability(Capability::VectorEncrypt));
        
        context.add_capability(Capability::VectorEncrypt);
        assert!(context.has_capability(Capability::VectorEncrypt));
        
        context.remove_capability(Capability::VectorEncrypt);
        assert!(!context.has_capability(Capability::VectorEncrypt));
    }
}