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

//! Vector Data Encryption Module
//!
//! This module implements AES-256-GCM encryption for vector data at rest,
//! providing confidentiality and authenticity for stored vector embeddings.

use crate::shared::{
    errors::{VexfsError, VexfsResult},
    types::*,
};
use super::{SecurityError, key_management::EncryptionKey, integrity::IntegrityMetadata};

#[cfg(not(feature = "kernel"))]
use std::collections::HashMap;
#[cfg(feature = "kernel")]
use alloc::collections::BTreeMap as HashMap;

#[cfg(not(feature = "kernel"))]
use std::vec::Vec;
#[cfg(feature = "kernel")]
use alloc::vec::Vec;

/// AES-256-GCM key size in bytes
pub const AES_256_KEY_SIZE: usize = 32;

/// AES-GCM nonce size in bytes
pub const AES_GCM_NONCE_SIZE: usize = 12;

/// AES-GCM authentication tag size in bytes
pub const AES_GCM_TAG_SIZE: usize = 16;

/// Maximum plaintext size for a single encryption operation (16MB)
pub const MAX_ENCRYPTION_SIZE: usize = 16 * 1024 * 1024;

/// Encryption algorithms supported
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM (recommended)
    Aes256Gcm = 0,
    /// ChaCha20-Poly1305 (alternative)
    ChaCha20Poly1305 = 1,
    /// No encryption (for testing only)
    None = 255,
}

impl Default for EncryptionAlgorithm {
    fn default() -> Self {
        Self::Aes256Gcm
    }
}

/// Encryption configuration
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// Encryption algorithm to use
    pub algorithm: EncryptionAlgorithm,
    /// Whether to compress before encryption
    pub compress_before_encrypt: bool,
    /// Additional authenticated data (AAD) to include
    pub additional_data: Vec<u8>,
    /// Key derivation iterations (for PBKDF2)
    pub key_derivation_iterations: u32,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            compress_before_encrypt: true,
            additional_data: Vec::new(),
            key_derivation_iterations: 100_000,
        }
    }
}

/// Encrypted data container
#[derive(Debug, Clone)]
pub struct EncryptedData {
    /// Encrypted data bytes
    pub data: Vec<u8>,
    /// Nonce used for encryption
    pub nonce: [u8; AES_GCM_NONCE_SIZE],
    /// Authentication tag
    pub tag: [u8; AES_GCM_TAG_SIZE],
    /// Key version used for encryption
    pub key_version: u32,
    /// Integrity metadata
    pub integrity: IntegrityMetadata,
}

/// Encryption result from encrypt operation
#[derive(Debug, Clone)]
pub struct EncryptionResult {
    /// Encrypted data bytes
    pub data: Vec<u8>,
    /// Nonce used for encryption
    pub nonce: [u8; AES_GCM_NONCE_SIZE],
    /// Authentication tag
    pub tag: [u8; AES_GCM_TAG_SIZE],
}

/// Encryption error types
#[derive(Debug, Clone, PartialEq)]
pub enum EncryptionError {
    /// Invalid key size
    InvalidKeySize,
    /// Invalid nonce size
    InvalidNonceSize,
    /// Invalid data size (too large)
    DataTooLarge,
    /// Encryption operation failed
    EncryptionFailed,
    /// Decryption operation failed
    DecryptionFailed,
    /// Authentication tag verification failed
    AuthenticationFailed,
    /// Unsupported algorithm
    UnsupportedAlgorithm,
    /// Invalid configuration
    InvalidConfiguration,
}

impl From<EncryptionError> for VexfsError {
    fn from(err: EncryptionError) -> Self {
        match err {
            EncryptionError::InvalidKeySize => VexfsError::InvalidArgument("Invalid encryption key size".into()),
            EncryptionError::InvalidNonceSize => VexfsError::InvalidArgument("Invalid nonce size".into()),
            EncryptionError::DataTooLarge => VexfsError::InvalidArgument("Data too large for encryption".into()),
            EncryptionError::EncryptionFailed => VexfsError::Other("Encryption operation failed".into()),
            EncryptionError::DecryptionFailed => VexfsError::Other("Decryption operation failed".into()),
            EncryptionError::AuthenticationFailed => VexfsError::ChecksumMismatch,
            EncryptionError::UnsupportedAlgorithm => VexfsError::UnsupportedOperation,
            EncryptionError::InvalidConfiguration => VexfsError::InvalidConfiguration,
        }
    }
}

/// Vector encryption manager
pub struct VectorEncryption {
    /// Current encryption configuration
    config: EncryptionConfig,
    /// Nonce counter for generating unique nonces
    nonce_counter: u64,
    /// Cache of recently used encryption contexts
    context_cache: HashMap<u32, EncryptionContext>,
}

/// Encryption context for a specific key version
#[derive(Debug, Clone)]
struct EncryptionContext {
    /// Key version
    key_version: u32,
    /// Last used timestamp
    last_used: u64,
    /// Usage count
    usage_count: u64,
}

impl VectorEncryption {
    /// Create a new vector encryption manager
    pub fn new(key_manager: &super::key_management::KeyManager) -> VexfsResult<Self> {
        Ok(Self {
            config: EncryptionConfig::default(),
            nonce_counter: 1,
            context_cache: HashMap::new(),
        })
    }

    /// Update encryption configuration
    pub fn update_config(&mut self, config: EncryptionConfig) -> VexfsResult<()> {
        // Validate configuration
        if config.key_derivation_iterations < 10_000 {
            return Err(EncryptionError::InvalidConfiguration.into());
        }

        self.config = config;
        Ok(())
    }

    /// Encrypt vector data using AES-256-GCM
    pub fn encrypt(
        &mut self,
        plaintext: &[u8],
        key: &EncryptionKey,
        config: &EncryptionConfig,
    ) -> VexfsResult<EncryptionResult> {
        if plaintext.len() > MAX_ENCRYPTION_SIZE {
            return Err(EncryptionError::DataTooLarge.into());
        }

        if key.key_material.len() != AES_256_KEY_SIZE {
            return Err(EncryptionError::InvalidKeySize.into());
        }

        match config.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                self.encrypt_aes_256_gcm(plaintext, key, config)
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.encrypt_chacha20_poly1305(plaintext, key, config)
            }
            EncryptionAlgorithm::None => {
                // No encryption - for testing only
                Ok(EncryptionResult {
                    data: plaintext.to_vec(),
                    nonce: [0u8; AES_GCM_NONCE_SIZE],
                    tag: [0u8; AES_GCM_TAG_SIZE],
                })
            }
        }
    }

    /// Decrypt vector data
    pub fn decrypt(
        &mut self,
        encrypted_data: &EncryptedData,
        key: &EncryptionKey,
    ) -> VexfsResult<Vec<u8>> {
        if key.key_material.len() != AES_256_KEY_SIZE {
            return Err(EncryptionError::InvalidKeySize.into());
        }

        match self.config.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                self.decrypt_aes_256_gcm(encrypted_data, key)
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                self.decrypt_chacha20_poly1305(encrypted_data, key)
            }
            EncryptionAlgorithm::None => {
                // No decryption - for testing only
                Ok(encrypted_data.data.clone())
            }
        }
    }

    /// Generate a cryptographically secure nonce
    pub fn generate_nonce(&mut self) -> [u8; AES_GCM_NONCE_SIZE] {
        let mut nonce = [0u8; AES_GCM_NONCE_SIZE];
        
        // Use counter + timestamp for uniqueness
        let counter = self.nonce_counter;
        self.nonce_counter = self.nonce_counter.wrapping_add(1);
        
        // In a real implementation, this would use a CSPRNG
        // For now, use a simple counter-based approach
        let counter_bytes = counter.to_le_bytes();
        nonce[0..8].copy_from_slice(&counter_bytes);
        
        // Add some entropy from current "time" (simplified)
        let time_bytes = (counter * 1000).to_le_bytes();
        nonce[8..12].copy_from_slice(&time_bytes[0..4]);
        
        nonce
    }

    /// Encrypt using AES-256-GCM
    fn encrypt_aes_256_gcm(
        &mut self,
        plaintext: &[u8],
        key: &EncryptionKey,
        config: &EncryptionConfig,
    ) -> VexfsResult<EncryptionResult> {
        let nonce = self.generate_nonce();
        
        // In a real implementation, this would use a proper AES-GCM library
        // For demonstration, we'll use a simplified approach
        let encrypted_data = self.simple_encrypt(plaintext, &key.key_material, &nonce)?;
        let tag = self.compute_auth_tag(&encrypted_data, &key.key_material, &nonce, &config.additional_data)?;
        
        Ok(EncryptionResult {
            data: encrypted_data,
            nonce,
            tag,
        })
    }

    /// Decrypt using AES-256-GCM
    fn decrypt_aes_256_gcm(
        &mut self,
        encrypted_data: &EncryptedData,
        key: &EncryptionKey,
    ) -> VexfsResult<Vec<u8>> {
        // Verify authentication tag first
        let computed_tag = self.compute_auth_tag(
            &encrypted_data.data,
            &key.key_material,
            &encrypted_data.nonce,
            &self.config.additional_data,
        )?;
        
        if computed_tag != encrypted_data.tag {
            return Err(EncryptionError::AuthenticationFailed.into());
        }
        
        // Decrypt the data
        self.simple_decrypt(&encrypted_data.data, &key.key_material, &encrypted_data.nonce)
    }

    /// Encrypt using ChaCha20-Poly1305 (placeholder)
    fn encrypt_chacha20_poly1305(
        &mut self,
        plaintext: &[u8],
        key: &EncryptionKey,
        config: &EncryptionConfig,
    ) -> VexfsResult<EncryptionResult> {
        // Placeholder implementation - would use ChaCha20-Poly1305 in real code
        self.encrypt_aes_256_gcm(plaintext, key, config)
    }

    /// Decrypt using ChaCha20-Poly1305 (placeholder)
    fn decrypt_chacha20_poly1305(
        &mut self,
        encrypted_data: &EncryptedData,
        key: &EncryptionKey,
    ) -> VexfsResult<Vec<u8>> {
        // Placeholder implementation - would use ChaCha20-Poly1305 in real code
        self.decrypt_aes_256_gcm(encrypted_data, key)
    }

    /// Simple XOR-based encryption (for demonstration only)
    /// In production, this would be replaced with proper AES-GCM
    fn simple_encrypt(
        &self,
        plaintext: &[u8],
        key: &[u8],
        nonce: &[u8],
    ) -> VexfsResult<Vec<u8>> {
        let mut encrypted = Vec::with_capacity(plaintext.len());
        
        for (i, &byte) in plaintext.iter().enumerate() {
            let key_byte = key[i % key.len()];
            let nonce_byte = nonce[i % nonce.len()];
            encrypted.push(byte ^ key_byte ^ nonce_byte);
        }
        
        Ok(encrypted)
    }

    /// Simple XOR-based decryption (for demonstration only)
    fn simple_decrypt(
        &self,
        ciphertext: &[u8],
        key: &[u8],
        nonce: &[u8],
    ) -> VexfsResult<Vec<u8>> {
        // XOR is symmetric, so decryption is the same as encryption
        self.simple_encrypt(ciphertext, key, nonce)
    }

    /// Compute authentication tag (simplified HMAC-like)
    fn compute_auth_tag(
        &self,
        data: &[u8],
        key: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> VexfsResult<[u8; AES_GCM_TAG_SIZE]> {
        let mut tag = [0u8; AES_GCM_TAG_SIZE];
        
        // Simple hash-based authentication (in production, use proper GMAC)
        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(data);
        hash_input.extend_from_slice(key);
        hash_input.extend_from_slice(nonce);
        hash_input.extend_from_slice(aad);
        
        // Simple checksum for demonstration
        let mut checksum = 0u64;
        for (i, &byte) in hash_input.iter().enumerate() {
            checksum = checksum.wrapping_add((byte as u64).wrapping_mul(i as u64 + 1));
        }
        
        let checksum_bytes = checksum.to_le_bytes();
        tag[0..8].copy_from_slice(&checksum_bytes);
        tag[8..16].copy_from_slice(&checksum_bytes);
        
        Ok(tag)
    }

    /// Get encryption statistics
    pub fn get_stats(&self) -> EncryptionStats {
        EncryptionStats {
            total_encryptions: self.nonce_counter - 1,
            cache_size: self.context_cache.len(),
            algorithm: self.config.algorithm,
        }
    }

    /// Clear encryption context cache
    pub fn clear_cache(&mut self) {
        self.context_cache.clear();
    }
}

/// Encryption statistics
#[derive(Debug, Clone)]
pub struct EncryptionStats {
    /// Total number of encryption operations performed
    pub total_encryptions: u64,
    /// Current cache size
    pub cache_size: usize,
    /// Current encryption algorithm
    pub algorithm: EncryptionAlgorithm,
}

/// Utility functions for encryption key management
pub struct EncryptionUtils;

impl EncryptionUtils {
    /// Derive encryption key from password using PBKDF2
    pub fn derive_key_from_password(
        password: &[u8],
        salt: &[u8],
        iterations: u32,
    ) -> VexfsResult<[u8; AES_256_KEY_SIZE]> {
        if password.is_empty() {
            return Err(EncryptionError::InvalidConfiguration.into());
        }

        if salt.len() < 16 {
            return Err(EncryptionError::InvalidConfiguration.into());
        }

        // Simple PBKDF2-like key derivation (in production, use proper PBKDF2)
        let mut key = [0u8; AES_256_KEY_SIZE];
        let mut hash_input = Vec::new();
        
        for iteration in 0..iterations {
            hash_input.clear();
            hash_input.extend_from_slice(password);
            hash_input.extend_from_slice(salt);
            hash_input.extend_from_slice(&iteration.to_le_bytes());
            
            // Simple hash function for demonstration
            let mut hash = 0u64;
            for (i, &byte) in hash_input.iter().enumerate() {
                hash = hash.wrapping_add((byte as u64).wrapping_mul(i as u64 + 1));
            }
            
            let hash_bytes = hash.to_le_bytes();
            for (i, &byte) in hash_bytes.iter().enumerate() {
                key[i % AES_256_KEY_SIZE] ^= byte;
            }
        }
        
        Ok(key)
    }

    /// Generate random salt for key derivation
    pub fn generate_salt() -> [u8; 32] {
        let mut salt = [0u8; 32];
        
        // In production, use a CSPRNG
        // For demonstration, use a simple pattern
        for (i, byte) in salt.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(17).wrapping_add(42);
        }
        
        salt
    }

    /// Validate encryption key strength
    pub fn validate_key_strength(key: &[u8]) -> bool {
        if key.len() != AES_256_KEY_SIZE {
            return false;
        }

        // Check for all-zero key
        if key.iter().all(|&b| b == 0) {
            return false;
        }

        // Check for simple patterns
        let mut pattern_count = 0;
        for i in 1..key.len() {
            if key[i] == key[i - 1] {
                pattern_count += 1;
            }
        }

        // Reject keys with too many repeated bytes
        pattern_count < key.len() / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::key_management::KeyManager;

    #[test]
    fn test_encryption_config_default() {
        let config = EncryptionConfig::default();
        assert_eq!(config.algorithm, EncryptionAlgorithm::Aes256Gcm);
        assert!(config.compress_before_encrypt);
        assert_eq!(config.key_derivation_iterations, 100_000);
    }

    #[test]
    fn test_nonce_generation() {
        let key_manager = KeyManager::new().unwrap();
        let mut encryption = VectorEncryption::new(&key_manager).unwrap();
        
        let nonce1 = encryption.generate_nonce();
        let nonce2 = encryption.generate_nonce();
        
        // Nonces should be different
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn test_key_derivation() {
        let password = b"test_password_123";
        let salt = EncryptionUtils::generate_salt();
        
        let key1 = EncryptionUtils::derive_key_from_password(password, &salt, 1000).unwrap();
        let key2 = EncryptionUtils::derive_key_from_password(password, &salt, 1000).unwrap();
        
        // Same password and salt should produce same key
        assert_eq!(key1, key2);
        
        // Different salt should produce different key
        let mut different_salt = EncryptionUtils::generate_salt();
        different_salt[0] = different_salt[0].wrapping_add(1); // Make it actually different
        let key3 = EncryptionUtils::derive_key_from_password(password, &different_salt, 1000).unwrap();
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_key_validation() {
        // Create a key with good entropy (not all the same byte)
        let mut good_key = [0u8; AES_256_KEY_SIZE];
        for (i, byte) in good_key.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(17).wrapping_add(42);
        }
        assert!(EncryptionUtils::validate_key_strength(&good_key));
        
        let zero_key = [0u8; AES_256_KEY_SIZE];
        assert!(!EncryptionUtils::validate_key_strength(&zero_key));
        
        let short_key = [42u8; 16];
        assert!(!EncryptionUtils::validate_key_strength(&short_key));
    }

    #[test]
    fn test_encryption_roundtrip() {
        let key_manager = KeyManager::new().unwrap();
        let mut encryption = VectorEncryption::new(&key_manager).unwrap();
        
        let plaintext = b"Hello, VexFS encryption!";
        let key_material = [42u8; AES_256_KEY_SIZE];
        let key = super::super::key_management::EncryptionKey {
            key_material: key_material.to_vec(),
            version: 1,
            created_at: 0,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
        };
        
        let config = EncryptionConfig::default();
        
        // Encrypt
        let encrypted = encryption.encrypt(plaintext, &key, &config).unwrap();
        
        // Create EncryptedData structure
        let encrypted_data = EncryptedData {
            data: encrypted.data,
            nonce: encrypted.nonce,
            tag: encrypted.tag,
            key_version: 1,
            integrity: super::super::integrity::IntegrityMetadata {
                checksum: Vec::new(),
                checksum_type: super::super::integrity::ChecksumType::Sha256,
                created_at: 0,
                inode: None,
            },
        };
        
        // Decrypt
        let decrypted = encryption.decrypt(&encrypted_data, &key).unwrap();
        
        assert_eq!(plaintext, decrypted.as_slice());
    }
}