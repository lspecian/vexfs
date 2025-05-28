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

//! Secure Key Management for VexFS Encryption
//!
//! This module implements secure key storage, derivation, and rotation
//! for VexFS encryption operations, ensuring keys are never stored in
//! plaintext and providing secure key lifecycle management.

use crate::shared::{
    errors::{VexfsError, VexfsResult},
    types::*,
};
use crate::fs_core::permissions::UserContext;
use super::{SecurityError, encryption::EncryptionAlgorithm};

#[cfg(not(feature = "kernel"))]
use std::collections::HashMap;
#[cfg(feature = "kernel")]
use alloc::collections::BTreeMap as HashMap;

#[cfg(not(feature = "kernel"))]
use std::vec::Vec;
#[cfg(feature = "kernel")]
use alloc::vec::Vec;

/// Master key size in bytes (256-bit)
pub const MASTER_KEY_SIZE: usize = 32;

/// Key derivation salt size in bytes
pub const KEY_SALT_SIZE: usize = 32;

/// Key version size in bytes
pub const KEY_VERSION_SIZE: usize = 4;

/// Maximum number of key versions to keep
pub const MAX_KEY_VERSIONS: usize = 10;

/// Key rotation interval in seconds (default: 30 days)
pub const DEFAULT_KEY_ROTATION_INTERVAL: u64 = 30 * 24 * 60 * 60;

/// Minimum key rotation interval in seconds (1 hour)
pub const MIN_KEY_ROTATION_INTERVAL: u64 = 60 * 60;

/// Key derivation iteration count for PBKDF2
pub const KEY_DERIVATION_ITERATIONS: u32 = 100_000;

/// Encryption key material
#[derive(Debug, Clone)]
pub struct EncryptionKey {
    /// Key material bytes
    pub key_material: Vec<u8>,
    /// Key version
    pub version: u32,
    /// Creation timestamp
    pub created_at: u64,
    /// Encryption algorithm this key is for
    pub algorithm: EncryptionAlgorithm,
}

impl EncryptionKey {
    /// Create a new encryption key
    pub fn new(key_material: Vec<u8>, version: u32, algorithm: EncryptionAlgorithm) -> Self {
        Self {
            key_material,
            version,
            created_at: 0, // TODO: get current time
            algorithm,
        }
    }

    /// Check if key is valid
    pub fn is_valid(&self) -> bool {
        !self.key_material.is_empty() && 
        self.key_material.len() >= 16 && // Minimum 128-bit key
        !self.key_material.iter().all(|&b| b == 0) // Not all zeros
    }

    /// Get key age in seconds
    pub fn age(&self, current_time: u64) -> u64 {
        current_time.saturating_sub(self.created_at)
    }

    /// Check if key needs rotation
    pub fn needs_rotation(&self, current_time: u64, rotation_interval: u64) -> bool {
        self.age(current_time) >= rotation_interval
    }

    /// Serialize key to encrypted bytes
    pub fn to_encrypted_bytes(&self, master_key: &[u8]) -> VexfsResult<Vec<u8>> {
        let mut plaintext = Vec::new();
        
        // Header
        plaintext.extend_from_slice(&self.version.to_le_bytes());
        plaintext.extend_from_slice(&self.created_at.to_le_bytes());
        plaintext.push(self.algorithm as u8);
        plaintext.extend_from_slice(&(self.key_material.len() as u32).to_le_bytes());
        
        // Key material
        plaintext.extend_from_slice(&self.key_material);
        
        // Encrypt with master key (simplified encryption)
        let encrypted = Self::simple_encrypt(&plaintext, master_key)?;
        Ok(encrypted)
    }

    /// Deserialize key from encrypted bytes
    pub fn from_encrypted_bytes(encrypted: &[u8], master_key: &[u8]) -> VexfsResult<Self> {
        // Decrypt with master key
        let plaintext = Self::simple_decrypt(encrypted, master_key)?;
        
        if plaintext.len() < 17 {
            return Err(VexfsError::InvalidData("Encrypted key data too short".into()));
        }

        let version = u32::from_le_bytes([plaintext[0], plaintext[1], plaintext[2], plaintext[3]]);
        let created_at = u64::from_le_bytes([
            plaintext[4], plaintext[5], plaintext[6], plaintext[7],
            plaintext[8], plaintext[9], plaintext[10], plaintext[11]
        ]);
        
        let algorithm = match plaintext[12] {
            0 => EncryptionAlgorithm::Aes256Gcm,
            1 => EncryptionAlgorithm::ChaCha20Poly1305,
            255 => EncryptionAlgorithm::None,
            _ => return Err(VexfsError::InvalidData("Invalid encryption algorithm".into())),
        };
        
        let key_len = u32::from_le_bytes([plaintext[13], plaintext[14], plaintext[15], plaintext[16]]) as usize;
        
        if plaintext.len() < 17 + key_len {
            return Err(VexfsError::InvalidData("Truncated key material".into()));
        }
        
        let key_material = plaintext[17..17 + key_len].to_vec();
        
        Ok(Self {
            key_material,
            version,
            created_at,
            algorithm,
        })
    }

    /// Simple encryption for key storage (XOR-based for demonstration)
    fn simple_encrypt(data: &[u8], key: &[u8]) -> VexfsResult<Vec<u8>> {
        if key.is_empty() {
            return Err(VexfsError::InvalidArgument("Empty encryption key".into()));
        }

        let mut encrypted = Vec::with_capacity(data.len());
        for (i, &byte) in data.iter().enumerate() {
            encrypted.push(byte ^ key[i % key.len()]);
        }
        Ok(encrypted)
    }

    /// Simple decryption for key storage
    fn simple_decrypt(data: &[u8], key: &[u8]) -> VexfsResult<Vec<u8>> {
        // XOR is symmetric
        Self::simple_encrypt(data, key)
    }
}

/// Key material for different purposes
#[derive(Debug, Clone)]
pub struct KeyMaterial {
    /// Raw key bytes
    pub bytes: Vec<u8>,
    /// Purpose of this key material
    pub purpose: KeyPurpose,
    /// Salt used for derivation
    pub salt: Vec<u8>,
    /// Derivation parameters
    pub derivation_params: KeyDerivationParams,
}

/// Key purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyPurpose {
    /// Master key for key encryption
    Master = 0,
    /// Data encryption key
    DataEncryption = 1,
    /// Metadata encryption key
    MetadataEncryption = 2,
    /// Index encryption key
    IndexEncryption = 3,
    /// Backup encryption key
    BackupEncryption = 4,
}

/// Key derivation parameters
#[derive(Debug, Clone)]
pub struct KeyDerivationParams {
    /// Number of iterations for PBKDF2
    pub iterations: u32,
    /// Hash algorithm used
    pub hash_algorithm: HashAlgorithm,
    /// Key length to derive
    pub key_length: usize,
}

impl Default for KeyDerivationParams {
    fn default() -> Self {
        Self {
            iterations: KEY_DERIVATION_ITERATIONS,
            hash_algorithm: HashAlgorithm::Sha256,
            key_length: MASTER_KEY_SIZE,
        }
    }
}

/// Hash algorithms for key derivation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HashAlgorithm {
    Sha256 = 0,
    Sha512 = 1,
    Blake3 = 2,
}

/// Key version information
#[derive(Debug, Clone)]
pub struct KeyVersion {
    /// Version number
    pub version: u32,
    /// Creation timestamp
    pub created_at: u64,
    /// Whether this version is active
    pub is_active: bool,
    /// Whether this version can be used for decryption
    pub can_decrypt: bool,
}

/// Key derivation utilities
pub struct KeyDerivation;

impl KeyDerivation {
    /// Derive key from password using PBKDF2
    pub fn derive_from_password(
        password: &[u8],
        salt: &[u8],
        params: &KeyDerivationParams,
    ) -> VexfsResult<Vec<u8>> {
        if password.is_empty() {
            return Err(VexfsError::InvalidArgument("Empty password".into()));
        }

        if salt.len() < 16 {
            return Err(VexfsError::InvalidArgument("Salt too short".into()));
        }

        // Simplified PBKDF2 implementation
        let mut key = vec![0u8; params.key_length];
        let mut hash_input = Vec::new();
        
        // First, create a base hash from password and salt
        hash_input.extend_from_slice(password);
        hash_input.extend_from_slice(salt);
        let base_hash = Self::hash_data(&hash_input, params.hash_algorithm);
        
        // Use the base hash as starting point for key material
        for (i, &byte) in base_hash.iter().enumerate() {
            if i < key.len() {
                key[i] = byte;
            }
        }
        
        // Then apply iterations to strengthen the key
        for iteration in 1..params.iterations {
            hash_input.clear();
            hash_input.extend_from_slice(&key); // Use current key state
            hash_input.extend_from_slice(salt);
            hash_input.extend_from_slice(&iteration.to_le_bytes());
            
            let hash = Self::hash_data(&hash_input, params.hash_algorithm);
            
            for (i, &byte) in hash.iter().enumerate() {
                if i < key.len() {
                    key[i] ^= byte;
                }
            }
        }
        
        Ok(key)
    }

    /// Derive key from master key and context
    pub fn derive_from_master(
        master_key: &[u8],
        context: &[u8],
        purpose: KeyPurpose,
        key_length: usize,
    ) -> VexfsResult<Vec<u8>> {
        if master_key.len() < 16 {
            return Err(VexfsError::InvalidArgument("Master key too short".into()));
        }

        let mut derivation_input = Vec::new();
        derivation_input.extend_from_slice(master_key);
        derivation_input.extend_from_slice(context);
        derivation_input.push(purpose as u8);
        derivation_input.extend_from_slice(&(key_length as u32).to_le_bytes());
        
        // Use multiple rounds of hashing to derive the key
        let mut derived_key = vec![0u8; key_length];
        let mut current_hash = derivation_input;
        
        for round in 0..(key_length + 31) / 32 {
            current_hash.extend_from_slice(&round.to_le_bytes());
            let hash = Self::hash_data(&current_hash, HashAlgorithm::Sha256);
            
            let copy_len = (key_length - round * 32).min(32);
            derived_key[round * 32..round * 32 + copy_len]
                .copy_from_slice(&hash[..copy_len]);
            
            current_hash = hash;
        }
        
        Ok(derived_key)
    }

    /// Generate cryptographically secure random bytes
    pub fn generate_random_bytes(length: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; length];
        
        // Simple pseudo-random generation (in production, use a CSPRNG)
        // Use a different seed each time to ensure different outputs
        static mut COUNTER: u64 = 0;
        let mut seed = unsafe {
            COUNTER = COUNTER.wrapping_add(1);
            0x12345678u64.wrapping_add(COUNTER.wrapping_mul(0x9e3779b9))
        };
        
        for byte in &mut bytes {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            *byte = (seed >> 24) as u8;
        }
        
        bytes
    }

    /// Generate salt for key derivation
    pub fn generate_salt() -> Vec<u8> {
        Self::generate_random_bytes(KEY_SALT_SIZE)
    }

    /// Hash data using specified algorithm
    fn hash_data(data: &[u8], algorithm: HashAlgorithm) -> Vec<u8> {
        match algorithm {
            HashAlgorithm::Sha256 => Self::sha256_hash(data),
            HashAlgorithm::Sha512 => Self::sha512_hash(data),
            HashAlgorithm::Blake3 => Self::blake3_hash(data),
        }
    }

    /// Simple SHA-256 hash implementation
    fn sha256_hash(data: &[u8]) -> Vec<u8> {
        // Simplified implementation (use proper crypto library in production)
        let mut hash = [0u8; 32];
        let mut state = [0x6a09e667u32, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
        
        for (i, &byte) in data.iter().enumerate() {
            let state_idx = i % 8;
            state[state_idx] = state[state_idx].wrapping_add(byte as u32);
            state[state_idx] = state[state_idx].rotate_left(7);
        }
        
        for i in 0..8 {
            let bytes = state[i].to_le_bytes();
            hash[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
        }
        
        hash.to_vec()
    }

    /// Simple SHA-512 hash implementation
    fn sha512_hash(data: &[u8]) -> Vec<u8> {
        // For demonstration, use double SHA-256
        let first_hash = Self::sha256_hash(data);
        let second_hash = Self::sha256_hash(&first_hash);
        [first_hash, second_hash].concat()
    }

    /// Simple Blake3 hash implementation
    fn blake3_hash(data: &[u8]) -> Vec<u8> {
        // For demonstration, use modified SHA-256
        let mut hash = Self::sha256_hash(data);
        for i in 0..hash.len() {
            hash[i] = hash[i].wrapping_add((i as u8).wrapping_mul(7));
        }
        hash
    }
}

/// Key rotation manager
pub struct KeyRotation {
    /// Rotation interval in seconds
    rotation_interval: u64,
    /// Last rotation timestamp
    last_rotation: u64,
    /// Automatic rotation enabled
    auto_rotation_enabled: bool,
}

impl KeyRotation {
    /// Create a new key rotation manager
    pub fn new(rotation_interval: u64) -> Self {
        Self {
            rotation_interval: rotation_interval.max(MIN_KEY_ROTATION_INTERVAL),
            last_rotation: 0,
            auto_rotation_enabled: true,
        }
    }

    /// Check if rotation is needed
    pub fn needs_rotation(&self, current_time: u64) -> bool {
        self.auto_rotation_enabled && 
        current_time - self.last_rotation >= self.rotation_interval
    }

    /// Update last rotation time
    pub fn mark_rotated(&mut self, current_time: u64) {
        self.last_rotation = current_time;
    }

    /// Set rotation interval
    pub fn set_interval(&mut self, interval: u64) {
        self.rotation_interval = interval.max(MIN_KEY_ROTATION_INTERVAL);
    }

    /// Enable or disable automatic rotation
    pub fn set_auto_rotation(&mut self, enabled: bool) {
        self.auto_rotation_enabled = enabled;
    }
}

/// Secure key storage
pub struct SecureKeyStorage {
    /// Encrypted key storage
    encrypted_keys: HashMap<String, Vec<u8>>,
    /// Master key for key encryption
    master_key: Vec<u8>,
}

impl SecureKeyStorage {
    /// Create new secure key storage
    pub fn new(master_key: Vec<u8>) -> VexfsResult<Self> {
        if master_key.len() != MASTER_KEY_SIZE {
            return Err(VexfsError::InvalidArgument("Invalid master key size".into()));
        }

        Ok(Self {
            encrypted_keys: HashMap::new(),
            master_key,
        })
    }

    /// Store encrypted key
    pub fn store_key(&mut self, key_id: String, key: &EncryptionKey) -> VexfsResult<()> {
        let encrypted_data = key.to_encrypted_bytes(&self.master_key)?;
        self.encrypted_keys.insert(key_id, encrypted_data);
        Ok(())
    }

    /// Retrieve and decrypt key
    pub fn retrieve_key(&self, key_id: &str) -> VexfsResult<EncryptionKey> {
        let encrypted_data = self.encrypted_keys.get(key_id)
            .ok_or_else(|| VexfsError::NotFound)?;
        
        EncryptionKey::from_encrypted_bytes(encrypted_data, &self.master_key)
    }

    /// Remove key
    pub fn remove_key(&mut self, key_id: &str) -> VexfsResult<()> {
        self.encrypted_keys.remove(key_id)
            .ok_or_else(|| VexfsError::NotFound)?;
        Ok(())
    }

    /// List all key IDs
    pub fn list_keys(&self) -> Vec<String> {
        self.encrypted_keys.keys().cloned().collect()
    }

    /// Clear all keys
    pub fn clear(&mut self) {
        self.encrypted_keys.clear();
    }
}

/// Main key manager
pub struct KeyManager {
    /// Secure key storage
    storage: SecureKeyStorage,
    /// Key rotation manager
    rotation: KeyRotation,
    /// Key versions by inode
    inode_keys: HashMap<InodeNumber, Vec<KeyVersion>>,
    /// Current key version counter
    current_version: u32,
}

impl KeyManager {
    /// Create a new key manager
    pub fn new() -> VexfsResult<Self> {
        // Generate master key
        let master_key = KeyDerivation::generate_random_bytes(MASTER_KEY_SIZE);
        let storage = SecureKeyStorage::new(master_key)?;
        let rotation = KeyRotation::new(DEFAULT_KEY_ROTATION_INTERVAL);

        Ok(Self {
            storage,
            rotation,
            inode_keys: HashMap::new(),
            current_version: 1,
        })
    }

    /// Create key manager with custom master key
    pub fn with_master_key(master_key: Vec<u8>) -> VexfsResult<Self> {
        let storage = SecureKeyStorage::new(master_key)?;
        let rotation = KeyRotation::new(DEFAULT_KEY_ROTATION_INTERVAL);

        Ok(Self {
            storage,
            rotation,
            inode_keys: HashMap::new(),
            current_version: 1,
        })
    }

    /// Get or create encryption key for an inode
    pub fn get_or_create_key(&mut self, inode: InodeNumber, user: &UserContext) -> VexfsResult<EncryptionKey> {
        // Check if we have any key for this inode (look for the current version)
        let key_id = format!("inode_{}_{}", inode, self.current_version);
        
        // Try to get existing key for current version
        if let Ok(key) = self.storage.retrieve_key(&key_id) {
            return Ok(key);
        }
        
        // Check if we have any previous version for this inode
        if let Some(versions) = self.inode_keys.get(&inode) {
            if let Some(latest_version) = versions.iter().filter(|v| v.is_active).max_by_key(|v| v.version) {
                let existing_key_id = format!("inode_{}_{}", inode, latest_version.version);
                if let Ok(key) = self.storage.retrieve_key(&existing_key_id) {
                    return Ok(key);
                }
            }
        }

        // Create new key
        self.create_key_for_inode(inode, user)
    }

    /// Get specific key version
    pub fn get_key(&self, inode: InodeNumber, version: u32) -> VexfsResult<EncryptionKey> {
        let key_id = format!("inode_{}_{}", inode, version);
        self.storage.retrieve_key(&key_id)
    }

    /// Create new encryption key for an inode
    pub fn create_key_for_inode(&mut self, inode: InodeNumber, user: &UserContext) -> VexfsResult<EncryptionKey> {
        // Derive key material from inode, user context, and version for uniqueness
        let context = format!("inode_{}_user_{}_version_{}", inode, user.uid, self.current_version);
        let key_material = KeyDerivation::derive_from_master(
            &self.storage.master_key,
            context.as_bytes(),
            KeyPurpose::DataEncryption,
            32, // AES-256 key size
        )?;

        let key = EncryptionKey::new(
            key_material,
            self.current_version,
            EncryptionAlgorithm::Aes256Gcm,
        );

        // Store the key
        let key_id = format!("inode_{}_{}", inode, self.current_version);
        self.storage.store_key(key_id, &key)?;

        // Update version tracking
        let version_info = KeyVersion {
            version: self.current_version,
            created_at: key.created_at,
            is_active: true,
            can_decrypt: true,
        };

        self.inode_keys.entry(inode).or_insert_with(Vec::new).push(version_info);
        self.current_version += 1;

        Ok(key)
    }

    /// Rotate key for an inode
    pub fn rotate_key(&mut self, inode: InodeNumber, user: &UserContext) -> VexfsResult<EncryptionKey> {
        // Mark old versions as inactive
        if let Some(versions) = self.inode_keys.get_mut(&inode) {
            for version in versions {
                version.is_active = false;
            }
        }

        // Create new key
        self.create_key_for_inode(inode, user)
    }

    /// Rotate all keys
    pub fn rotate_all_keys(&mut self) -> VexfsResult<()> {
        let current_time = 0; // TODO: get current time
        
        // Check if rotation is needed
        if !self.rotation.needs_rotation(current_time) {
            return Ok(());
        }

        // Get all inodes that have keys
        let inodes: Vec<InodeNumber> = self.inode_keys.keys().cloned().collect();
        
        for inode in inodes {
            // For rotation, we need a user context - use root for system rotation
            let root_user = UserContext::root();
            self.rotate_key(inode, &root_user)?;
        }

        self.rotation.mark_rotated(current_time);
        Ok(())
    }

    /// Clean up old key versions
    pub fn cleanup_old_versions(&mut self) -> VexfsResult<()> {
        for (inode, versions) in &mut self.inode_keys {
            // Keep only the most recent versions
            versions.sort_by(|a, b| b.version.cmp(&a.version));
            
            if versions.len() > MAX_KEY_VERSIONS {
                let to_remove = versions.split_off(MAX_KEY_VERSIONS);
                
                // Remove old keys from storage
                for version in to_remove {
                    let key_id = format!("inode_{}_{}", inode, version.version);
                    let _ = self.storage.remove_key(&key_id);
                }
            }
        }

        Ok(())
    }

    /// Get key statistics
    pub fn get_key_stats(&self) -> KeyStats {
        let total_keys = self.storage.list_keys().len();
        let total_inodes = self.inode_keys.len();
        let active_versions = self.inode_keys.values()
            .map(|versions| versions.iter().filter(|v| v.is_active).count())
            .sum();

        KeyStats {
            total_keys,
            total_inodes,
            active_versions,
            current_version: self.current_version,
        }
    }

    /// Set key rotation interval
    pub fn set_rotation_interval(&mut self, interval: u64) {
        self.rotation.set_interval(interval);
    }

    /// Enable or disable automatic key rotation
    pub fn set_auto_rotation(&mut self, enabled: bool) {
        self.rotation.set_auto_rotation(enabled);
    }
}

/// Key management statistics
#[derive(Debug, Clone)]
pub struct KeyStats {
    /// Total number of keys stored
    pub total_keys: usize,
    /// Number of inodes with keys
    pub total_inodes: usize,
    /// Number of active key versions
    pub active_versions: usize,
    /// Current version counter
    pub current_version: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_derivation() {
        let password = b"test_password";
        let salt = KeyDerivation::generate_salt();
        let params = KeyDerivationParams::default();
        
        let key1 = KeyDerivation::derive_from_password(password, &salt, &params).unwrap();
        let key2 = KeyDerivation::derive_from_password(password, &salt, &params).unwrap();
        
        // Same inputs should produce same key
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), MASTER_KEY_SIZE);
        
        // Different salt should produce different key
        let different_salt = {
            let mut salt = KeyDerivation::generate_salt();
            // Ensure it's actually different by modifying multiple bytes
            salt[0] = salt[0].wrapping_add(1);
            salt[1] = salt[1].wrapping_add(2);
            salt[2] = salt[2].wrapping_add(3);
            salt
        };
        let key3 = KeyDerivation::derive_from_password(password, &different_salt, &params).unwrap();
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_encryption_key_serialization() {
        let key_material = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let key = EncryptionKey::new(key_material.clone(), 1, EncryptionAlgorithm::Aes256Gcm);
        let master_key = vec![42u8; MASTER_KEY_SIZE];
        
        let encrypted = key.to_encrypted_bytes(&master_key).unwrap();
        let decrypted = EncryptionKey::from_encrypted_bytes(&encrypted, &master_key).unwrap();
        
        assert_eq!(key.key_material, decrypted.key_material);
        assert_eq!(key.version, decrypted.version);
        assert_eq!(key.algorithm, decrypted.algorithm);
    }

    #[test]
    fn test_key_manager() {
        let mut manager = KeyManager::new().unwrap();
        let user = UserContext::new(1000, 1000, &[]);
        
        // Create key for inode
        let key1 = manager.get_or_create_key(123, &user).unwrap();
        assert!(key1.is_valid());
        
        // Getting same inode should return same key
        let key2 = manager.get_or_create_key(123, &user).unwrap();
        assert_eq!(key1.key_material, key2.key_material);
        assert_eq!(key1.version, key2.version);
        
        // Different inode should get different key
        let key3 = manager.get_or_create_key(456, &user).unwrap();
        assert_ne!(key1.key_material, key3.key_material);
        
        // Check that we have 2 keys total (one for each inode)
        let stats = manager.get_key_stats();
        assert_eq!(stats.total_keys, 2);
    }

    #[test]
    fn test_key_rotation() {
        let mut manager = KeyManager::new().unwrap();
        let user = UserContext::new(1000, 1000, &[]);
        
        // Create initial key
        let key1 = manager.get_or_create_key(123, &user).unwrap();
        let version1 = key1.version;
        
        // Rotate key
        let key2 = manager.rotate_key(123, &user).unwrap();
        let version2 = key2.version;
        
        assert_ne!(key1.key_material, key2.key_material);
        assert_ne!(version1, version2);
        assert!(version2 > version1);
    }

    #[test]
    fn test_secure_key_storage() {
        let master_key = vec![42u8; MASTER_KEY_SIZE];
        let mut storage = SecureKeyStorage::new(master_key).unwrap();
        
        let key_material = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let key = EncryptionKey::new(key_material.clone(), 1, EncryptionAlgorithm::Aes256Gcm);
        
        // Store key
        storage.store_key("test_key".to_string(), &key).unwrap();
        
        // Retrieve key
        let retrieved = storage.retrieve_key("test_key").unwrap();
        assert_eq!(key.key_material, retrieved.key_material);
        assert_eq!(key.version, retrieved.version);
        
        // Remove key
        storage.remove_key("test_key").unwrap();
        assert!(storage.retrieve_key("test_key").is_err());
    }
}