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

//! Utilities for VexFS Shared Domain
//!
//! This module provides common utility functions used throughout the VexFS codebase.
//! Functions are designed to work in both kernel and userspace environments.

use crate::shared::constants::*;
use crate::shared::types::*;
use crate::shared::errors::*;

// =======================
// Path Utilities
// =======================

/// Normalize a path by removing redundant components
pub fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        return "/".to_string();
    }

    let mut components = Vec::new();
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    for part in parts {
        match part {
            "." => continue, // Current directory, skip
            ".." => {
                // Parent directory, pop if possible
                if !components.is_empty() && components.last() != Some(&"..") {
                    components.pop();
                } else if !path.starts_with('/') {
                    // For relative paths, keep ".." if at root level
                    components.push("..");
                }
            }
            _ => components.push(part),
        }
    }

    let normalized = if path.starts_with('/') {
        format!("/{}", components.join("/"))
    } else {
        components.join("/")
    };

    if normalized.is_empty() {
        if path.starts_with('/') {
            "/".to_string()
        } else {
            ".".to_string()
        }
    } else {
        normalized
    }
}

/// Get the parent directory of a path
pub fn parent_path(path: &str) -> Option<String> {
    if path == "/" || path.is_empty() {
        return None;
    }

    let normalized = normalize_path(path);
    if let Some(pos) = normalized.rfind('/') {
        if pos == 0 {
            Some("/".to_string())
        } else {
            Some(normalized[..pos].to_string())
        }
    } else {
        Some(".".to_string())
    }
}

/// Get the filename component of a path
pub fn filename(path: &str) -> Option<&str> {
    if path == "/" || path.is_empty() {
        return None;
    }

    let path = path.trim_end_matches('/');
    if let Some(pos) = path.rfind('/') {
        let name = &path[pos + 1..];
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    } else {
        Some(path)
    }
}

/// Join two path components
pub fn join_paths(base: &str, component: &str) -> String {
    if component.starts_with('/') {
        component.to_string()
    } else if base.ends_with('/') {
        format!("{}{}", base, component)
    } else {
        format!("{}/{}", base, component)
    }
}

/// Validate a filename for VexFS requirements
pub fn validate_filename(name: &str) -> VexfsResult<()> {
    if name.is_empty() {
        return Err(VexfsError::InvalidArgument("filename cannot be empty".to_string()));
    }

    if name.len() > VEXFS_MAX_NAME_LEN {
        return Err(VexfsError::InvalidArgument(format!(
            "filename too long: {} > {}",
            name.len(),
            VEXFS_MAX_NAME_LEN
        )));
    }

    if name == "." || name == ".." {
        return Err(VexfsError::InvalidArgument("invalid filename".to_string()));
    }

    if name.contains('\0') {
        return Err(VexfsError::InvalidArgument("filename contains null byte".to_string()));
    }

    if name.contains('/') {
        return Err(VexfsError::InvalidArgument("filename contains path separator".to_string()));
    }

    Ok(())
}

// =======================
// Alignment Utilities
// =======================

/// Align a value up to the next boundary
pub fn align_up(value: u64, alignment: u64) -> u64 {
    if alignment == 0 || (alignment & (alignment - 1)) != 0 {
        return value; // Invalid alignment, return unchanged
    }
    (value + alignment - 1) & !(alignment - 1)
}

/// Align a value down to the previous boundary
pub fn align_down(value: u64, alignment: u64) -> u64 {
    if alignment == 0 || (alignment & (alignment - 1)) != 0 {
        return value; // Invalid alignment, return unchanged
    }
    value & !(alignment - 1)
}

/// Check if a value is aligned to a boundary
pub fn is_aligned(value: u64, alignment: u64) -> bool {
    if alignment == 0 || (alignment & (alignment - 1)) != 0 {
        return false; // Invalid alignment
    }
    value & (alignment - 1) == 0
}

/// Round up to the next power of 2
pub fn next_power_of_2(mut value: u64) -> u64 {
    if value == 0 {
        return 1;
    }
    
    value -= 1;
    value |= value >> 1;
    value |= value >> 2;
    value |= value >> 4;
    value |= value >> 8;
    value |= value >> 16;
    value |= value >> 32;
    value + 1
}

/// Check if a value is a power of 2
pub fn is_power_of_2(value: u64) -> bool {
    value != 0 && (value & (value - 1)) == 0
}

/// Alias for is_power_of_2 for backwards compatibility
pub fn is_power_of_two(value: u64) -> bool {
    is_power_of_2(value)
}

// =======================
// Block Utilities
// =======================

/// Calculate the block number for a given offset
pub fn offset_to_block(offset: u64, block_size: u32) -> BlockNumber {
    offset / block_size as u64
}

/// Calculate the offset within a block
pub fn offset_in_block(offset: u64, block_size: u32) -> u32 {
    (offset % block_size as u64) as u32
}

/// Calculate the number of blocks needed for a given size
pub fn blocks_for_size(size: u64, block_size: u32) -> u64 {
    (size + block_size as u64 - 1) / block_size as u64
}

/// Convert block number and offset to absolute file offset
pub fn block_to_offset(block: BlockNumber, offset: u32, block_size: u32) -> u64 {
    block * block_size as u64 + offset as u64
}

// =======================
// Checksum Utilities
// =======================

/// Calculate CRC32 checksum
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = !VEXFS_CHECKSUM_SEED;
    
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ VEXFS_CRC32_POLYNOMIAL;
            } else {
                crc >>= 1;
            }
        }
    }
    
    !crc
}

/// Verify checksum matches expected value
pub fn verify_checksum(data: &[u8], expected: u32) -> bool {
    crc32(data) == expected
}

/// Calculate checksum for a structure
pub fn checksum_struct<T>(data: &T) -> u32 {
    let bytes = unsafe {
        core::slice::from_raw_parts(
            data as *const T as *const u8,
            core::mem::size_of::<T>(),
        )
    };
    crc32(bytes)
}

// =======================
// Time Utilities
// =======================

/// Get current timestamp (implementation depends on context)
pub fn current_timestamp() -> Timestamp {
    #[cfg(feature = "kernel")]
    {
        // In kernel mode, use kernel time functions
        // TODO: Use actual kernel time functions
        0 // Placeholder
    }
    #[cfg(not(feature = "kernel"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
}

/// Get current time (alias for current_timestamp for compatibility)
pub fn current_time() -> Timestamp {
    current_timestamp()
}

/// Get current time as u32 (for compatibility with some modules)
pub fn get_current_time() -> u32 {
    (current_timestamp() / VEXFS_TIMESTAMP_PRECISION) as u32
}

/// Convert timestamp to seconds since epoch
pub fn timestamp_to_secs(timestamp: Timestamp) -> u64 {
    timestamp / VEXFS_TIMESTAMP_PRECISION
}

/// Convert timestamp to nanoseconds within second
pub fn timestamp_to_nsecs(timestamp: Timestamp) -> u32 {
    (timestamp % VEXFS_TIMESTAMP_PRECISION) as u32
}

/// Check if enough time has passed for atime update
pub fn should_update_atime(last_atime: Timestamp, current_time: Timestamp) -> bool {
    let threshold = VEXFS_ATIME_UPDATE_THRESHOLD * VEXFS_TIMESTAMP_PRECISION;
    current_time >= last_atime + threshold
}

// =======================
// Math Utilities
// =======================

/// Calculate minimum of two values
pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

/// Calculate maximum of two values
pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

/// Clamp a value between min and max
pub fn clamp<T: PartialOrd + Copy>(value: T, min_val: T, max_val: T) -> T {
    if value < min_val {
        min_val
    } else if value > max_val {
        max_val
    } else {
        value
    }
}

/// Calculate the greatest common divisor
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

/// Calculate the least common multiple
pub fn lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        0
    } else {
        (a / gcd(a, b)) * b
    }
}

// =======================
// Vector Utilities
// =======================

/// Calculate Euclidean distance between two vectors
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> VexfsResult<f32> {
    if a.len() != b.len() {
        return Err(VexfsError::VectorError(VectorErrorKind::DimensionMismatch {
            expected: a.len() as u16,
            found: b.len() as u16,
        }));
    }

    let sum: f32 = a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum();
    
    Ok(sum.sqrt())
}

/// Calculate cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> VexfsResult<f32> {
    if a.len() != b.len() {
        return Err(VexfsError::VectorError(VectorErrorKind::DimensionMismatch {
            expected: a.len() as u16,
            found: b.len() as u16,
        }));
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return Ok(0.0);
    }

    Ok(dot_product / (magnitude_a * magnitude_b))
}

/// Calculate dot product of two vectors
pub fn dot_product(a: &[f32], b: &[f32]) -> VexfsResult<f32> {
    if a.len() != b.len() {
        return Err(VexfsError::VectorError(VectorErrorKind::DimensionMismatch {
            expected: a.len() as u16,
            found: b.len() as u16,
        }));
    }

    Ok(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum())
}

/// Calculate Manhattan distance between two vectors
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> VexfsResult<f32> {
    if a.len() != b.len() {
        return Err(VexfsError::VectorError(VectorErrorKind::DimensionMismatch {
            expected: a.len() as u16,
            found: b.len() as u16,
        }));
    }

    Ok(a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum())
}

/// Normalize a vector to unit length
pub fn normalize_vector(vector: &mut [f32]) -> VexfsResult<()> {
    let magnitude: f32 = vector.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    
    if magnitude == 0.0 {
        return Err(VexfsError::VectorError(VectorErrorKind::NormalizationError));
    }

    for component in vector {
        *component /= magnitude;
    }

    Ok(())
}

// =======================
// Memory Utilities
// =======================

/// Safe memory copy that works in both kernel and userspace
pub fn safe_copy_memory(dst: *mut u8, src: *const u8, len: usize) -> VexfsResult<()> {
    if dst.is_null() || src.is_null() {
        return Err(VexfsError::InvalidArgument("null pointer".to_string()));
    }

    if len == 0 {
        return Ok(());
    }

    unsafe {
        #[cfg(feature = "kernel")]
        {
            core::ptr::copy_nonoverlapping(src, dst, len);
        }
        #[cfg(not(feature = "kernel"))]
        {
            std::ptr::copy_nonoverlapping(src, dst, len);
        }
    }

    Ok(())
}

/// Safe memory zero that works in both kernel and userspace
pub fn safe_zero_memory(ptr: *mut u8, len: usize) -> VexfsResult<()> {
    if ptr.is_null() {
        return Err(VexfsError::InvalidArgument("null pointer".to_string()));
    }

    if len == 0 {
        return Ok(());
    }

    unsafe {
        #[cfg(feature = "kernel")]
        {
            core::ptr::write_bytes(ptr, 0, len);
        }
        #[cfg(not(feature = "kernel"))]
        {
            std::ptr::write_bytes(ptr, 0, len);
        }
    }

    Ok(())
}

/// Safe memory comparison
pub fn safe_compare_memory(a: *const u8, b: *const u8, len: usize) -> VexfsResult<bool> {
    if a.is_null() || b.is_null() {
        return Err(VexfsError::InvalidArgument("null pointer".to_string()));
    }

    if len == 0 {
        return Ok(true);
    }

    unsafe {
        for i in 0..len {
            if *a.add(i) != *b.add(i) {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

// =======================
// Validation Utilities
// =======================

/// Validate block size is a power of 2 and within limits
pub fn validate_block_size(block_size: u32) -> VexfsResult<()> {
    if block_size == 0 {
        return Err(VexfsError::InvalidArgument("block size cannot be zero".to_string()));
    }
    
    if !is_power_of_2(block_size as u64) {
        return Err(VexfsError::InvalidArgument("block size must be a power of 2".to_string()));
    }
    
    if block_size < VEXFS_MIN_BLOCK_SIZE as u32 {
        return Err(VexfsError::InvalidArgument(format!(
            "block size {} too small, minimum is {}",
            block_size,
            VEXFS_MIN_BLOCK_SIZE
        )));
    }
    
    if block_size > VEXFS_MAX_BLOCK_SIZE as u32 {
        return Err(VexfsError::InvalidArgument(format!(
            "block size {} too large, maximum is {}",
            block_size,
            VEXFS_MAX_BLOCK_SIZE
        )));
    }
    
    Ok(())
}

/// Validate inode number is within valid range
pub fn validate_inode_number(ino: InodeNumber) -> VexfsResult<()> {
    if ino == 0 {
        return Err(VexfsError::InvalidArgument("inode number cannot be zero".to_string()));
    }
    
    if ino > VEXFS_MAX_INODE_NUM {
        return Err(VexfsError::InvalidArgument(format!(
            "inode number {} exceeds maximum {}",
            ino,
            VEXFS_MAX_INODE_NUM
        )));
    }
    
    Ok(())
}

/// Validate block number is within valid range
pub fn validate_block_number(block: BlockNumber) -> VexfsResult<()> {
    if block > VEXFS_MAX_BLOCK_NUM {
        return Err(VexfsError::InvalidArgument(format!(
            "block number {} exceeds maximum {}",
            block,
            VEXFS_MAX_BLOCK_NUM
        )));
    }
    
    Ok(())
}

/// Convert error code to string (for debugging)
pub fn error_to_string(error: &VexfsError) -> String {
    // Use the Display implementation which is already comprehensive
    format!("{}", error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/a/b/../c"), "/a/c");
        assert_eq!(normalize_path("/a/./b"), "/a/b");
        assert_eq!(normalize_path("a/b/../c"), "a/c");
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_parent_path() {
        assert_eq!(parent_path("/a/b/c"), Some("/a/b".to_string()));
        assert_eq!(parent_path("/a"), Some("/".to_string()));
        assert_eq!(parent_path("/"), None);
        assert_eq!(parent_path("a/b"), Some(".".to_string()));
    }

    #[test]
    fn test_filename() {
        assert_eq!(filename("/a/b/c"), Some("c"));
        assert_eq!(filename("/a"), Some("a"));
        assert_eq!(filename("/"), None);
        assert_eq!(filename("file.txt"), Some("file.txt"));
    }

    #[test]
    fn test_alignment() {
        assert_eq!(align_up(10, 8), 16);
        assert_eq!(align_down(10, 8), 8);
        assert!(is_aligned(16, 8));
        assert!(!is_aligned(10, 8));
    }

    #[test]
    fn test_power_of_2() {
        assert!(is_power_of_2(8));
        assert!(!is_power_of_2(10));
        assert_eq!(next_power_of_2(10), 16);
        assert_eq!(next_power_of_2(8), 8);
    }

    #[test]
    fn test_block_utilities() {
        assert_eq!(offset_to_block(4096, 4096), 1);
        assert_eq!(offset_in_block(4100, 4096), 4);
        assert_eq!(blocks_for_size(8192, 4096), 2);
        assert_eq!(blocks_for_size(8000, 4096), 2);
    }

    #[test]
    fn test_crc32() {
        let data = b"hello world";
        let checksum = crc32(data);
        assert!(verify_checksum(data, checksum));
        assert!(!verify_checksum(data, checksum + 1));
    }

    #[test]
    fn test_math_utilities() {
        assert_eq!(min(5, 3), 3);
        assert_eq!(max(5, 3), 5);
        assert_eq!(clamp(10, 0, 5), 5);
        assert_eq!(clamp(-5, 0, 5), 0);
        assert_eq!(clamp(3, 0, 5), 3);
        assert_eq!(gcd(48, 18), 6);
        assert_eq!(lcm(4, 6), 12);
    }

    #[test]
    fn test_vector_utilities() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        
        let distance = euclidean_distance(&a, &b).unwrap();
        assert!((distance - 5.196152).abs() < 0.001);
        
        let dot = dot_product(&a, &b).unwrap();
        assert_eq!(dot, 32.0);
        
        let manhattan = manhattan_distance(&a, &b).unwrap();
        assert_eq!(manhattan, 9.0);
    }

    #[test]
    fn test_validate_filename() {
        assert!(validate_filename("valid_name.txt").is_ok());
        assert!(validate_filename("").is_err());
        assert!(validate_filename("name/with/slash").is_err());
        assert!(validate_filename("name\0with\0null").is_err());
        assert!(validate_filename(".").is_err());
        assert!(validate_filename("..").is_err());
        
        let long_name = "a".repeat(VEXFS_MAX_NAME_LEN + 1);
        assert!(validate_filename(&long_name).is_err());
    }
}