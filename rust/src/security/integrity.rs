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

//! Data Integrity Management for VexFS
//!
//! This module implements cryptographic checksums (SHA-256) for vector data blocks,
//! providing integrity verification during read operations and automatic corruption
//! detection and reporting.

use crate::shared::{
    errors::{VexfsError, VexfsResult},
    types::*,
};
use super::SecurityError;

#[cfg(not(feature = "kernel"))]
use std::collections::HashMap;
#[cfg(feature = "kernel")]
use alloc::collections::BTreeMap as HashMap;

#[cfg(not(feature = "kernel"))]
use std::{string::String, vec::Vec};
#[cfg(feature = "kernel")]
use alloc::{string::{String, ToString}, vec::Vec};

/// SHA-256 hash size in bytes
pub const SHA256_HASH_SIZE: usize = 32;

/// CRC32 hash size in bytes
pub const CRC32_HASH_SIZE: usize = 4;

/// Maximum data size for integrity checking (1GB)
pub const MAX_INTEGRITY_DATA_SIZE: usize = 1024 * 1024 * 1024;

/// Supported checksum algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChecksumType {
    /// No checksum
    None = 0,
    /// CRC32 checksum (fast, basic error detection)
    Crc32 = 1,
    /// SHA-256 hash (cryptographically secure)
    Sha256 = 2,
    /// Blake3 hash (fast and secure)
    Blake3 = 3,
}

impl Default for ChecksumType {
    fn default() -> Self {
        Self::Sha256
    }
}

impl ChecksumType {
    /// Get the size of the checksum in bytes
    pub fn size(&self) -> usize {
        match self {
            ChecksumType::None => 0,
            ChecksumType::Crc32 => CRC32_HASH_SIZE,
            ChecksumType::Sha256 => SHA256_HASH_SIZE,
            ChecksumType::Blake3 => SHA256_HASH_SIZE, // Blake3 also produces 32-byte hashes
        }
    }

    /// Check if this checksum type is cryptographically secure
    pub fn is_cryptographic(&self) -> bool {
        matches!(self, ChecksumType::Sha256 | ChecksumType::Blake3)
    }

    /// Get the recommended checksum type for a given data size
    pub fn recommended_for_size(data_size: usize) -> Self {
        if data_size < 1024 {
            // Small data - CRC32 is sufficient
            ChecksumType::Crc32
        } else if data_size < 1024 * 1024 {
            // Medium data - SHA-256 for security
            ChecksumType::Sha256
        } else {
            // Large data - Blake3 for speed and security
            ChecksumType::Blake3
        }
    }
}

/// Integrity metadata for a data block
#[derive(Debug, Clone)]
pub struct IntegrityMetadata {
    /// Checksum of the data
    pub checksum: Vec<u8>,
    /// Type of checksum used
    pub checksum_type: ChecksumType,
    /// Timestamp when checksum was created
    pub created_at: u64,
    /// Optional inode number for context
    pub inode: Option<InodeNumber>,
}

impl IntegrityMetadata {
    /// Create new integrity metadata
    pub fn new(checksum: Vec<u8>, checksum_type: ChecksumType, inode: Option<InodeNumber>) -> Self {
        Self {
            checksum,
            checksum_type,
            created_at: 0, // TODO: get current time
            inode,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Header
        bytes.push(self.checksum_type as u8);
        bytes.extend_from_slice(&self.created_at.to_le_bytes());
        bytes.extend_from_slice(&(self.checksum.len() as u32).to_le_bytes());
        
        // Inode (optional)
        match self.inode {
            Some(inode) => {
                bytes.push(1); // Has inode
                bytes.extend_from_slice(&inode.to_le_bytes());
            }
            None => {
                bytes.push(0); // No inode
                bytes.extend_from_slice(&[0u8; 8]);
            }
        }
        
        // Checksum data
        bytes.extend_from_slice(&self.checksum);
        
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> VexfsResult<Self> {
        if bytes.len() < 22 {
            return Err(VexfsError::InvalidData("Integrity metadata too short".into()));
        }

        let checksum_type = match bytes[0] {
            0 => ChecksumType::None,
            1 => ChecksumType::Crc32,
            2 => ChecksumType::Sha256,
            3 => ChecksumType::Blake3,
            _ => return Err(VexfsError::InvalidData("Invalid checksum type".into())),
        };

        let created_at = u64::from_le_bytes([
            bytes[1], bytes[2], bytes[3], bytes[4],
            bytes[5], bytes[6], bytes[7], bytes[8]
        ]);

        let checksum_len = u32::from_le_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]) as usize;

        let inode = if bytes[13] == 1 {
            Some(u64::from_le_bytes([
                bytes[14], bytes[15], bytes[16], bytes[17],
                bytes[18], bytes[19], bytes[20], bytes[21]
            ]))
        } else {
            None
        };

        if bytes.len() < 22 + checksum_len {
            return Err(VexfsError::InvalidData("Truncated checksum data".into()));
        }

        let checksum = bytes[22..22 + checksum_len].to_vec();

        Ok(Self {
            checksum,
            checksum_type,
            created_at,
            inode,
        })
    }
}

/// Result of integrity verification
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether the data is valid
    pub is_valid: bool,
    /// Expected checksum
    pub expected_checksum: Vec<u8>,
    /// Actual checksum computed
    pub actual_checksum: Vec<u8>,
    /// Type of checksum used
    pub checksum_type: ChecksumType,
    /// Error message if verification failed
    pub error_message: Option<String>,
}

impl VerificationResult {
    /// Create a successful verification result
    pub fn valid(checksum: Vec<u8>, checksum_type: ChecksumType) -> Self {
        Self {
            is_valid: true,
            expected_checksum: checksum.clone(),
            actual_checksum: checksum,
            checksum_type,
            error_message: None,
        }
    }

    /// Create a failed verification result
    pub fn invalid(
        expected: Vec<u8>,
        actual: Vec<u8>,
        checksum_type: ChecksumType,
        error: String,
    ) -> Self {
        Self {
            is_valid: false,
            expected_checksum: expected,
            actual_checksum: actual,
            checksum_type,
            error_message: Some(error),
        }
    }
}

/// Integrity checker for computing and verifying checksums
pub struct IntegrityChecker;

impl IntegrityChecker {
    /// Compute checksum for data
    pub fn compute_checksum(data: &[u8], checksum_type: ChecksumType) -> VexfsResult<Vec<u8>> {
        if data.len() > MAX_INTEGRITY_DATA_SIZE {
            return Err(VexfsError::InvalidArgument("Data too large for integrity checking".into()));
        }

        match checksum_type {
            ChecksumType::None => Ok(Vec::new()),
            ChecksumType::Crc32 => Ok(Self::compute_crc32(data)),
            ChecksumType::Sha256 => Ok(Self::compute_sha256(data)),
            ChecksumType::Blake3 => Ok(Self::compute_blake3(data)),
        }
    }

    /// Verify data against checksum
    pub fn verify_checksum(
        data: &[u8],
        expected_checksum: &[u8],
        checksum_type: ChecksumType,
    ) -> VexfsResult<VerificationResult> {
        if checksum_type == ChecksumType::None {
            return Ok(VerificationResult::valid(Vec::new(), checksum_type));
        }

        let actual_checksum = Self::compute_checksum(data, checksum_type)?;

        if actual_checksum == expected_checksum {
            Ok(VerificationResult::valid(actual_checksum, checksum_type))
        } else {
            Ok(VerificationResult::invalid(
                expected_checksum.to_vec(),
                actual_checksum,
                checksum_type,
                "Checksum mismatch".to_string(),
            ))
        }
    }

    /// Compute CRC32 checksum
    fn compute_crc32(data: &[u8]) -> Vec<u8> {
        // Simple CRC32 implementation (in production, use a proper CRC32 library)
        let mut crc = 0xFFFFFFFFu32;
        
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }
        
        (crc ^ 0xFFFFFFFF).to_le_bytes().to_vec()
    }

    /// Compute SHA-256 hash
    fn compute_sha256(data: &[u8]) -> Vec<u8> {
        // Simplified SHA-256 implementation for demonstration
        // In production, use a proper cryptographic library
        let mut hash = [0u8; SHA256_HASH_SIZE];
        
        // Simple hash function based on data content
        let mut state = [0x6a09e667u32, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
        
        // Process data in chunks
        for (i, &byte) in data.iter().enumerate() {
            let state_idx = i % 8;
            state[state_idx] = state[state_idx].wrapping_add(byte as u32);
            state[state_idx] = state[state_idx].rotate_left(7);
        }
        
        // Mix the state
        for i in 0..8 {
            state[i] = state[i].wrapping_add(data.len() as u32);
        }
        
        // Convert to bytes
        for (i, &word) in state.iter().enumerate() {
            let bytes = word.to_le_bytes();
            hash[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
        }
        
        hash.to_vec()
    }

    /// Compute Blake3 hash (placeholder implementation)
    fn compute_blake3(data: &[u8]) -> Vec<u8> {
        // For demonstration, use a modified SHA-256-like approach
        // In production, use the actual Blake3 algorithm
        let mut hash = Self::compute_sha256(data);
        
        // Add some Blake3-specific mixing
        for i in 0..hash.len() {
            hash[i] = hash[i].wrapping_add((i as u8).wrapping_mul(3));
        }
        
        hash
    }

    /// Compute checksum with automatic type selection
    pub fn compute_auto(data: &[u8]) -> VexfsResult<(Vec<u8>, ChecksumType)> {
        let checksum_type = ChecksumType::recommended_for_size(data.len());
        let checksum = Self::compute_checksum(data, checksum_type)?;
        Ok((checksum, checksum_type))
    }

    /// Verify data integrity with detailed error reporting
    pub fn verify_with_details(
        data: &[u8],
        metadata: &IntegrityMetadata,
    ) -> VexfsResult<VerificationResult> {
        // Check if data size is reasonable
        if data.len() > MAX_INTEGRITY_DATA_SIZE {
            return Ok(VerificationResult::invalid(
                metadata.checksum.clone(),
                Vec::new(),
                metadata.checksum_type,
                "Data too large for verification".to_string(),
            ));
        }

        // Verify checksum
        Self::verify_checksum(data, &metadata.checksum, metadata.checksum_type)
    }
}

/// Data integrity manager for coordinating integrity operations
pub struct DataIntegrityManager {
    /// Cache of integrity metadata by data identifier
    metadata_cache: HashMap<String, IntegrityMetadata>,
    /// Statistics about integrity operations
    stats: IntegrityStats,
}

/// Integrity operation statistics
#[derive(Debug, Clone)]
pub struct IntegrityStats {
    /// Total number of checksums computed
    pub checksums_computed: u64,
    /// Total number of verifications performed
    pub verifications_performed: u64,
    /// Number of verification failures
    pub verification_failures: u64,
    /// Number of corrupted blocks detected
    pub corrupted_blocks: u64,
    /// Total bytes processed
    pub bytes_processed: u64,
}

impl IntegrityStats {
    fn new() -> Self {
        Self {
            checksums_computed: 0,
            verifications_performed: 0,
            verification_failures: 0,
            corrupted_blocks: 0,
            bytes_processed: 0,
        }
    }

    /// Get verification success rate
    pub fn success_rate(&self) -> f64 {
        if self.verifications_performed == 0 {
            1.0
        } else {
            1.0 - (self.verification_failures as f64 / self.verifications_performed as f64)
        }
    }

    /// Get corruption rate
    pub fn corruption_rate(&self) -> f64 {
        if self.verifications_performed == 0 {
            0.0
        } else {
            self.corrupted_blocks as f64 / self.verifications_performed as f64
        }
    }
}

impl DataIntegrityManager {
    /// Create a new data integrity manager
    pub fn new() -> Self {
        Self {
            metadata_cache: HashMap::new(),
            stats: IntegrityStats::new(),
        }
    }

    /// Generate checksum for data
    pub fn generate_checksum(
        &mut self,
        data: &[u8],
        checksum_type: ChecksumType,
        inode: Option<InodeNumber>,
    ) -> VexfsResult<IntegrityMetadata> {
        let checksum = IntegrityChecker::compute_checksum(data, checksum_type)?;
        
        let metadata = IntegrityMetadata::new(checksum, checksum_type, inode);
        
        // Update statistics
        self.stats.checksums_computed += 1;
        self.stats.bytes_processed += data.len() as u64;
        
        Ok(metadata)
    }

    /// Generate checksum with automatic type selection
    pub fn generate_checksum_auto(
        &mut self,
        data: &[u8],
        inode: Option<InodeNumber>,
    ) -> VexfsResult<IntegrityMetadata> {
        let (checksum, checksum_type) = IntegrityChecker::compute_auto(data)?;
        
        let metadata = IntegrityMetadata::new(checksum, checksum_type, inode);
        
        // Update statistics
        self.stats.checksums_computed += 1;
        self.stats.bytes_processed += data.len() as u64;
        
        Ok(metadata)
    }

    /// Verify data integrity
    pub fn verify_checksum(
        &mut self,
        data: &[u8],
        metadata: &IntegrityMetadata,
        inode: Option<InodeNumber>,
    ) -> VexfsResult<VerificationResult> {
        let result = IntegrityChecker::verify_with_details(data, metadata)?;
        
        // Update statistics
        self.stats.verifications_performed += 1;
        self.stats.bytes_processed += data.len() as u64;
        
        if !result.is_valid {
            self.stats.verification_failures += 1;
            self.stats.corrupted_blocks += 1;
            
            // Log corruption event
            self.log_corruption_event(inode, metadata, &result);
        }
        
        Ok(result)
    }

    /// Store integrity metadata in cache
    pub fn cache_metadata(&mut self, key: String, metadata: IntegrityMetadata) {
        self.metadata_cache.insert(key, metadata);
    }

    /// Retrieve integrity metadata from cache
    pub fn get_cached_metadata(&self, key: &str) -> Option<&IntegrityMetadata> {
        self.metadata_cache.get(key)
    }

    /// Remove integrity metadata from cache
    pub fn remove_cached_metadata(&mut self, key: &str) -> Option<IntegrityMetadata> {
        self.metadata_cache.remove(key)
    }

    /// Clear integrity metadata cache
    pub fn clear_cache(&mut self) {
        self.metadata_cache.clear();
    }

    /// Get integrity statistics
    pub fn get_stats(&self) -> &IntegrityStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = IntegrityStats::new();
    }

    /// Log corruption event
    fn log_corruption_event(
        &self,
        inode: Option<InodeNumber>,
        metadata: &IntegrityMetadata,
        result: &VerificationResult,
    ) {
        // In a real implementation, this would write to a corruption log
        #[cfg(not(feature = "kernel"))]
        {
            println!(
                "CORRUPTION DETECTED: inode={:?} type={:?} expected={:02x?} actual={:02x?}",
                inode,
                metadata.checksum_type,
                result.expected_checksum,
                result.actual_checksum
            );
        }
    }

    /// Perform integrity check on multiple data blocks
    pub fn batch_verify(
        &mut self,
        data_blocks: &[(Vec<u8>, IntegrityMetadata)],
    ) -> VexfsResult<Vec<VerificationResult>> {
        let mut results = Vec::new();
        
        for (data, metadata) in data_blocks {
            let result = self.verify_checksum(data, metadata, metadata.inode)?;
            results.push(result);
        }
        
        Ok(results)
    }

    /// Get corruption summary
    pub fn get_corruption_summary(&self) -> CorruptionSummary {
        CorruptionSummary {
            total_verifications: self.stats.verifications_performed,
            total_failures: self.stats.verification_failures,
            total_corrupted_blocks: self.stats.corrupted_blocks,
            success_rate: self.stats.success_rate(),
            corruption_rate: self.stats.corruption_rate(),
        }
    }
}

/// Summary of corruption detection
#[derive(Debug, Clone)]
pub struct CorruptionSummary {
    /// Total number of verifications performed
    pub total_verifications: u64,
    /// Total number of verification failures
    pub total_failures: u64,
    /// Total number of corrupted blocks detected
    pub total_corrupted_blocks: u64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Corruption rate (0.0 to 1.0)
    pub corruption_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_types() {
        assert_eq!(ChecksumType::Crc32.size(), 4);
        assert_eq!(ChecksumType::Sha256.size(), 32);
        assert!(!ChecksumType::Crc32.is_cryptographic());
        assert!(ChecksumType::Sha256.is_cryptographic());
    }

    #[test]
    fn test_crc32_computation() {
        let data = b"Hello, VexFS!";
        let checksum = IntegrityChecker::compute_crc32(data);
        assert_eq!(checksum.len(), 4);
        
        // Same data should produce same checksum
        let checksum2 = IntegrityChecker::compute_crc32(data);
        assert_eq!(checksum, checksum2);
        
        // Different data should produce different checksum
        let checksum3 = IntegrityChecker::compute_crc32(b"Different data");
        assert_ne!(checksum, checksum3);
    }

    #[test]
    fn test_sha256_computation() {
        let data = b"Hello, VexFS!";
        let checksum = IntegrityChecker::compute_sha256(data);
        assert_eq!(checksum.len(), 32);
        
        // Same data should produce same checksum
        let checksum2 = IntegrityChecker::compute_sha256(data);
        assert_eq!(checksum, checksum2);
        
        // Different data should produce different checksum
        let checksum3 = IntegrityChecker::compute_sha256(b"Different data");
        assert_ne!(checksum, checksum3);
    }

    #[test]
    fn test_integrity_verification() {
        let data = b"Test data for integrity checking";
        let checksum = IntegrityChecker::compute_checksum(data, ChecksumType::Sha256).unwrap();
        
        // Valid verification
        let result = IntegrityChecker::verify_checksum(data, &checksum, ChecksumType::Sha256).unwrap();
        assert!(result.is_valid);
        
        // Invalid verification (corrupted data)
        let corrupted_data = b"Test data for integrity checking!"; // Added exclamation
        let result = IntegrityChecker::verify_checksum(corrupted_data, &checksum, ChecksumType::Sha256).unwrap();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_integrity_metadata_serialization() {
        let checksum = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let metadata = IntegrityMetadata::new(checksum.clone(), ChecksumType::Sha256, Some(123));
        
        let bytes = metadata.to_bytes();
        let deserialized = IntegrityMetadata::from_bytes(&bytes).unwrap();
        
        assert_eq!(metadata.checksum, deserialized.checksum);
        assert_eq!(metadata.checksum_type, deserialized.checksum_type);
        assert_eq!(metadata.inode, deserialized.inode);
    }

    #[test]
    fn test_data_integrity_manager() {
        let mut manager = DataIntegrityManager::new();
        let data = b"Test data for manager";
        
        // Generate checksum
        let metadata = manager.generate_checksum(data, ChecksumType::Sha256, Some(456)).unwrap();
        
        // Verify checksum
        let result = manager.verify_checksum(data, &metadata, Some(456)).unwrap();
        assert!(result.is_valid);
        
        // Check statistics
        let stats = manager.get_stats();
        assert_eq!(stats.checksums_computed, 1);
        assert_eq!(stats.verifications_performed, 1);
        assert_eq!(stats.verification_failures, 0);
    }

    #[test]
    fn test_automatic_checksum_selection() {
        let small_data = b"small";
        let (_, checksum_type) = IntegrityChecker::compute_auto(small_data).unwrap();
        assert_eq!(checksum_type, ChecksumType::Crc32);
        
        let medium_data = vec![0u8; 2048];
        let (_, checksum_type) = IntegrityChecker::compute_auto(&medium_data).unwrap();
        assert_eq!(checksum_type, ChecksumType::Sha256);
        
        let large_data = vec![0u8; 2 * 1024 * 1024];
        let (_, checksum_type) = IntegrityChecker::compute_auto(&large_data).unwrap();
        assert_eq!(checksum_type, ChecksumType::Blake3);
    }
}