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

//! Types for VexFS Shared Domain
//!
//! This module contains common type definitions used throughout the VexFS codebase.
//! Types are designed to be compatible with both kernel and userspace environments.

use crate::shared::constants::*;

// =======================
// Primitive Type Aliases
// =======================

/// Block number type for addressing filesystem blocks
pub type BlockNumber = u64;

/// Inode number type for addressing filesystem inodes
pub type InodeNumber = u64;

/// File offset type for byte addressing within files
pub type FileOffset = u64;

/// Size type for measuring data sizes
pub type Size = u64;

/// Timestamp type for filesystem timestamps (nanoseconds since epoch)
pub type Timestamp = u64;

/// Checksum type for data integrity verification
pub type Checksum = u32;

/// Transaction ID type for journaling
pub type TransactionId = u64;

/// Vector dimension type
pub type VectorDimension = u16;

/// Vector component type (f32 for compatibility)
pub type VectorComponent = f32;

/// Distance type for vector similarity measurements
pub type Distance = f32;

/// Score type for search result ranking
pub type Score = f32;

/// User ID type for filesystem ownership
pub type UserId = u32;

/// Group ID type for filesystem ownership
pub type GroupId = u32;

/// File size type (alias for Size for clarity)
pub type FileSize = u64;


/// Device ID type
pub type DeviceId = u64;

/// Link count type
pub type LinkCount = u32;

// =======================
// Filesystem Core Types
// =======================

/// File type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FileType {
    Unknown = 0,
    Regular = 1,
    Directory = 2,
    CharDevice = 3,
    BlockDevice = 4,
    Fifo = 5,
    Socket = 6,
    Symlink = 7,
    VectorFile = 8, // VexFS-specific: file with embedded vectors
}

impl From<u8> for FileType {
    fn from(value: u8) -> Self {
        match value {
            1 => FileType::Regular,
            2 => FileType::Directory,
            3 => FileType::CharDevice,
            4 => FileType::BlockDevice,
            5 => FileType::Fifo,
            6 => FileType::Socket,
            7 => FileType::Symlink,
            8 => FileType::VectorFile,
            _ => FileType::Unknown,
        }
    }
}

/// File permissions and mode bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMode(pub u32);

impl FileMode {
    pub fn new(mode: u32) -> Self {
        FileMode(mode & VEXFS_MAX_MODE)
    }

    pub fn is_readable(&self) -> bool {
        self.0 & 0o444 != 0
    }

    pub fn is_writable(&self) -> bool {
        self.0 & 0o222 != 0
    }

    pub fn is_executable(&self) -> bool {
        self.0 & 0o111 != 0
    }

    pub fn permissions(&self) -> u32 {
        self.0 & 0o777
    }
}

// Implement bitwise operations for FileMode
impl std::ops::BitAnd<u32> for FileMode {
    type Output = FileMode;
    
    fn bitand(self, rhs: u32) -> Self::Output {
        FileMode(self.0 & rhs)
    }
}

impl std::ops::BitOr<u32> for FileMode {
    type Output = FileMode;
    
    fn bitor(self, rhs: u32) -> Self::Output {
        FileMode(self.0 | rhs)
    }
}

impl std::ops::BitAnd<FileMode> for FileMode {
    type Output = FileMode;
    
    fn bitand(self, rhs: FileMode) -> Self::Output {
        FileMode(self.0 & rhs.0)
    }
}

impl std::ops::BitOr<FileMode> for FileMode {
    type Output = FileMode;
    
    fn bitor(self, rhs: FileMode) -> Self::Output {
        FileMode(self.0 | rhs.0)
    }
}

impl std::ops::Not for FileMode {
    type Output = u32;
    
    fn not(self) -> Self::Output {
        !self.0
    }
}

/// Block address structure for efficient block management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockAddress {
    pub block_number: BlockNumber,
    pub offset: u32,
    pub size: u32,
}

impl BlockAddress {
    pub fn new(block_number: BlockNumber, offset: u32, size: u32) -> Self {
        Self {
            block_number,
            offset,
            size,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.offset < VEXFS_DEFAULT_BLOCK_SIZE as u32 &&
        self.size > 0 &&
        (self.offset + self.size) <= VEXFS_DEFAULT_BLOCK_SIZE as u32
    }
}

/// Inode statistics structure
#[derive(Debug, Clone, Copy)]
pub struct InodeStat {
    pub ino: InodeNumber,
    pub size: Size,
    pub blocks: u64,
    pub atime: Timestamp,
    pub mtime: Timestamp,
    pub ctime: Timestamp,
    pub mode: FileMode,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub blksize: u32,
}

impl Default for InodeStat {
    fn default() -> Self {
        Self {
            ino: VEXFS_INVALID_INO,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            mode: FileMode::new(VEXFS_DEFAULT_FILE_MODE),
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: VEXFS_DEFAULT_BLOCK_SIZE as u32,
        }
    }
}

// =======================
// Result Type Aliases
// =======================

/// Standard Result type for VexFS operations
pub type Result<T> = core::result::Result<T, crate::shared::errors::VexfsError>;

/// File seeking behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekWhence {
    /// Seek from beginning of file
    Set,
    /// Seek from current position
    Current,
    /// Seek from end of file
    End,
}

// =======================
// Vector Types
// =======================

/// Vector data structure
#[derive(Debug, Clone)]
pub struct Vector {
    pub dimensions: VectorDimension,
    pub data: Vec<VectorComponent>,
    pub metadata: Option<VectorMetadata>,
}

impl Vector {
    pub fn new(dimensions: VectorDimension, data: Vec<VectorComponent>) -> core::result::Result<Self, &'static str> {
        if data.len() != dimensions as usize {
            return Err("Vector data length does not match dimensions");
        }
        if dimensions > VEXFS_MAX_VECTOR_DIMS as VectorDimension {
            return Err("Vector dimensions exceed maximum");
        }
        Ok(Self {
            dimensions,
            data,
            metadata: None,
        })
    }

    pub fn with_metadata(mut self, metadata: VectorMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn magnitude(&self) -> f32 {
        self.data.iter().map(|x| x * x).sum::<f32>().sqrt()
    }

    pub fn normalize(&mut self) {
        let mag = self.magnitude();
        if mag > 0.0 {
            for component in &mut self.data {
                *component /= mag;
            }
        }
    }

    pub fn normalized(&self) -> Self {
        let mut normalized = self.clone();
        normalized.normalize();
        normalized
    }
}

/// Vector metadata for additional information
#[derive(Debug, Clone)]
pub struct VectorMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub checksum: Checksum,
}

impl Default for VectorMetadata {
    fn default() -> Self {
        let now = 0; // TODO: Get current timestamp
        Self {
            name: None,
            description: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            checksum: 0,
        }
    }
}

/// Vector similarity metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SimilarityMetric {
    Euclidean = 0,
    Cosine = 1,
    DotProduct = 2,
    Manhattan = 3,
    Hamming = 4,
}

/// Distance metric type alias for compatibility
pub type DistanceMetric = SimilarityMetric;

impl From<u8> for SimilarityMetric {
    fn from(value: u8) -> Self {
        match value {
            1 => SimilarityMetric::Cosine,
            2 => SimilarityMetric::DotProduct,
            3 => SimilarityMetric::Manhattan,
            4 => SimilarityMetric::Hamming,
            _ => SimilarityMetric::Euclidean,
        }
    }
}

/// Search result entry
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub inode: InodeNumber,
    pub distance: Distance,
    pub score: Score,
    pub vector_index: usize,
    pub metadata: Option<VectorMetadata>,
}

impl SearchResult {
    pub fn new(inode: InodeNumber, distance: Distance, vector_index: usize) -> Self {
        Self {
            inode,
            distance,
            score: 1.0 / (1.0 + distance), // Convert distance to similarity score
            vector_index,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: VectorMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Search query parameters
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub vector: Vector,
    pub k: usize,
    pub metric: SimilarityMetric,
    pub ef: Option<usize>, // HNSW-specific parameter
    pub filter: Option<SearchFilter>,
}

impl SearchQuery {
    pub fn new(vector: Vector, k: usize) -> Self {
        Self {
            vector,
            k: k.min(VEXFS_MAX_K),
            metric: SimilarityMetric::Euclidean,
            ef: None,
            filter: None,
        }
    }

    pub fn with_metric(mut self, metric: SimilarityMetric) -> Self {
        self.metric = metric;
        self
    }

    pub fn with_ef(mut self, ef: usize) -> Self {
        self.ef = Some(ef.min(VEXFS_MAX_EF));
        self
    }

    pub fn with_filter(mut self, filter: SearchFilter) -> Self {
        self.filter = Some(filter);
        self
    }
}

/// Search filter for restricting results
#[derive(Debug, Clone)]
pub struct SearchFilter {
    pub file_type: Option<FileType>,
    pub size_range: Option<(Size, Size)>,
    pub time_range: Option<(Timestamp, Timestamp)>,
    pub tags: Vec<String>,
    pub path_prefix: Option<String>,
}

impl SearchFilter {
    pub fn new() -> Self {
        Self {
            file_type: None,
            size_range: None,
            time_range: None,
            tags: Vec::new(),
            path_prefix: None,
        }
    }

    pub fn with_file_type(mut self, file_type: FileType) -> Self {
        self.file_type = Some(file_type);
        self
    }

    pub fn with_size_range(mut self, min_size: Size, max_size: Size) -> Self {
        self.size_range = Some((min_size, max_size));
        self
    }

    pub fn with_time_range(mut self, start_time: Timestamp, end_time: Timestamp) -> Self {
        self.time_range = Some((start_time, end_time));
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn with_path_prefix(mut self, prefix: String) -> Self {
        self.path_prefix = Some(prefix);
        self
    }
}

// =======================
// Memory Management Types
// =======================

/// Memory pool identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoolId(pub u32);

/// Memory allocation statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_size: Size,
    pub used_size: Size,
    pub free_size: Size,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub peak_usage: Size,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            total_size: 0,
            used_size: 0,
            free_size: 0,
            allocation_count: 0,
            deallocation_count: 0,
            peak_usage: 0,
        }
    }
}

// =======================
// I/O and Storage Types
// =======================

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IoOperation {
    Read = 0,
    Write = 1,
    Sync = 2,
    Trim = 3,
}

/// I/O request structure
#[derive(Debug, Clone)]
pub struct IoRequest {
    pub operation: IoOperation,
    pub block: BlockNumber,
    pub offset: u64,
    pub size: u32,
    pub buffer: *mut u8, // Raw pointer for kernel compatibility
    pub callback: Option<fn(result: core::result::Result<u32, i32>)>,
}

impl IoRequest {
    pub fn new_read(block: BlockNumber, offset: u64, size: u32, buffer: *mut u8) -> Self {
        Self {
            operation: IoOperation::Read,
            block,
            offset,
            size,
            buffer,
            callback: None,
        }
    }

    pub fn new_write(block: BlockNumber, offset: u64, size: u32, buffer: *mut u8) -> Self {
        Self {
            operation: IoOperation::Write,
            block,
            offset,
            size,
            buffer,
            callback: None,
        }
    }

    pub fn with_callback(mut self, callback: fn(result: core::result::Result<u32, i32>)) -> Self {
        self.callback = Some(callback);
        self
    }
}

// =======================
// Cache Types
// =======================

/// Cache entry identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub inode: InodeNumber,
    pub block: BlockNumber,
    pub version: u32,
}

impl CacheKey {
    pub fn new(inode: InodeNumber, block: BlockNumber) -> Self {
        Self {
            inode,
            block,
            version: 0,
        }
    }

    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }
}

/// Cache statistics
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub total_entries: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
    pub memory_usage: Size,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            total_entries: 0,
            hit_count: 0,
            miss_count: 0,
            eviction_count: 0,
            memory_usage: 0,
        }
    }
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            0.0
        } else {
            self.hit_count as f64 / total as f64
        }
    }

    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }
}

// =======================
// Configuration Types
// =======================

/// Filesystem mount options
#[derive(Debug, Clone)]
pub struct MountOptions {
    pub read_only: bool,
    pub sync_mode: bool,
    pub cache_enabled: bool,
    pub vector_indexing: bool,
    pub compression: bool,
    pub encryption: bool,
    pub journal_enabled: bool,
    pub max_file_size: Size,
    pub block_size: u32,
    pub cache_size: Size,
}

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            read_only: false,
            sync_mode: false,
            cache_enabled: true,
            vector_indexing: true,
            compression: false,
            encryption: false,
            journal_enabled: true,
            max_file_size: VEXFS_MAX_FILE_SIZE,
            block_size: VEXFS_DEFAULT_BLOCK_SIZE as u32,
            cache_size: VEXFS_DEFAULT_PAGE_CACHE_SIZE as Size,
        }
    }
}

// =======================
// Additional Type Aliases for Compatibility
// =======================

/// Vector ID type
pub type VectorId = u64;

/// Vector dimensions type for compatibility 
pub type VectorDimensions = u16;

/// Vector data type
pub type VectorData = Vec<f32>;


/// File permissions type
pub type FilePermissions = u32;

/// Node ID type for graph structures
pub type NodeId = u64;

/// Sequence number type
pub type SequenceNumber = u64;

/// Directory entry structure
#[derive(Debug, Clone, PartialEq)]
pub struct DirectoryEntry {
    pub name: [u8; 256],
    pub inode: InodeNumber,
    pub file_type: FileType,
}

/// Inode flags for special file attributes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InodeFlags(pub u32);

impl InodeFlags {
    pub const IMMUTABLE: Self = Self(0x00000010);
    pub const APPEND_ONLY: Self = Self(0x00000020);
    pub const NO_DUMP: Self = Self(0x00000040);
    pub const NO_ATIME: Self = Self(0x00000080);
    pub const VECTOR_ENABLED: Self = Self(0x80000000);
}

/// Vector dimension type
pub type VectorDim = u32;

/// File pointer type for compatibility
pub type FilePtr = *mut u8;

/// Generic vector element type
pub type VectorElement = f32;

/// Vector search result entry for compatibility
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub inode: InodeNumber,
    pub score: f32,
    pub vector: Option<Vector>,
}

/// Memory pool configuration
#[derive(Debug, Clone)]
pub struct MemoryPoolConfig {
    pub small_pool_size: usize,
    pub medium_pool_size: usize,
    pub large_pool_size: usize,
    pub pressure_threshold: f32,
}

impl Default for MemoryPoolConfig {
    fn default() -> Self {
        Self {
            small_pool_size: crate::shared::constants::VEXFS_DEFAULT_SMALL_POOL_SIZE,
            medium_pool_size: crate::shared::constants::VEXFS_DEFAULT_MEDIUM_POOL_SIZE,
            large_pool_size: crate::shared::constants::VEXFS_DEFAULT_LARGE_POOL_SIZE,
            pressure_threshold: crate::shared::constants::VEXFS_DEFAULT_MEMORY_PRESSURE_THRESHOLD,
        }
    }
}

/// Transaction configuration
#[derive(Debug, Clone)]
pub struct TransactionConfig {
    pub max_size: usize,
    pub commit_interval: u64,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            max_size: crate::shared::constants::VEXFS_DEFAULT_MAX_TRANSACTION_SIZE,
            commit_interval: crate::shared::constants::VEXFS_DEFAULT_JOURNAL_COMMIT_INTERVAL,
        }
    }
}

/// Debug configuration
#[derive(Debug, Clone)]
pub struct DebugConfig {
    pub buffer_size: usize,
    pub enabled: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            buffer_size: crate::shared::constants::VEXFS_DEFAULT_DEBUG_BUFFER_SIZE,
            enabled: false,
        }
    }
}

/// Scoring method enum for compatibility
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoringMethod {
    Cosine,
    Euclidean,
    Manhattan,
    Dot,
}

impl From<SimilarityMetric> for ScoringMethod {
    fn from(metric: SimilarityMetric) -> Self {
        match metric {
            SimilarityMetric::Cosine => ScoringMethod::Cosine,
            SimilarityMetric::Euclidean => ScoringMethod::Euclidean,
            SimilarityMetric::Manhattan => ScoringMethod::Manhattan,
            SimilarityMetric::DotProduct => ScoringMethod::Dot,
            SimilarityMetric::Hamming => ScoringMethod::Euclidean, // Fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_conversion() {
        assert_eq!(FileType::from(1), FileType::Regular);
        assert_eq!(FileType::from(2), FileType::Directory);
        assert_eq!(FileType::from(8), FileType::VectorFile);
        assert_eq!(FileType::from(99), FileType::Unknown);
    }

    #[test]
    fn test_file_mode() {
        let mode = FileMode::new(0o644);
        assert!(mode.is_readable());
        assert!(mode.is_writable());
        assert!(!mode.is_executable());
        assert_eq!(mode.permissions(), 0o644);
    }

    #[test]
    fn test_block_address() {
        let addr = BlockAddress::new(100, 0, 512);
        assert!(addr.is_valid());
        
        let invalid_addr = BlockAddress::new(100, 4096, 512);
        assert!(!invalid_addr.is_valid());
    }

    #[test]
    fn test_vector_creation() {
        let data = vec![1.0, 2.0, 3.0];
        let vector = Vector::new(3, data).unwrap();
        assert_eq!(vector.dimensions, 3);
        assert_eq!(vector.data.len(), 3);
        
        let invalid_vector = Vector::new(2, vec![1.0, 2.0, 3.0]);
        assert!(invalid_vector.is_err());
    }

    #[test]
    fn test_vector_magnitude() {
        let vector = Vector::new(3, vec![3.0, 4.0, 0.0]).unwrap();
        assert_eq!(vector.magnitude(), 5.0);
    }

    #[test]
    fn test_search_result() {
        let result = SearchResult::new(123, 0.5, 0);
        assert_eq!(result.inode, 123);
        assert_eq!(result.distance, 0.5);
        assert!(result.score > 0.0);
    }

    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::default();
        stats.hit_count = 80;
        stats.miss_count = 20;
        
        assert_eq!(stats.hit_rate(), 0.8);
        assert_eq!(stats.miss_rate(), 0.2);
    }

    #[test]
    fn test_similarity_metric_conversion() {
        assert_eq!(SimilarityMetric::from(0), SimilarityMetric::Euclidean);
        assert_eq!(SimilarityMetric::from(1), SimilarityMetric::Cosine);
        assert_eq!(SimilarityMetric::from(99), SimilarityMetric::Euclidean);
    }
}
// Additional missing types for configuration

/// Quantization configuration for vector compression
#[derive(Debug, Clone)]
pub struct QuantizationConfig {
    /// Enable vector quantization
    pub enabled: bool,
    /// Number of centroids for quantization
    pub num_centroids: usize,
    /// Quantization bits per dimension
    pub bits_per_dim: u8,
    /// Training sample size
    pub training_samples: usize,
}

impl Default for QuantizationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            num_centroids: 256,
            bits_per_dim: 8,
            training_samples: 10000,
        }
    }
}

/// Search configuration parameters
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Default search algorithm
    pub algorithm: SearchAlgorithm,
    /// Maximum search results
    pub max_results: usize,
    /// Search timeout in milliseconds
    pub timeout_ms: u32,
    /// Enable result caching
    pub cache_results: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            algorithm: SearchAlgorithm::Approximate,
            max_results: 100,
            timeout_ms: 5000,
            cache_results: true,
        }
    }
}

/// Search algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAlgorithm {
    /// Exact nearest neighbor search
    Exact,
    /// Approximate nearest neighbor (HNSW)
    Approximate,
    /// Hybrid search combining exact and approximate
    Hybrid,
}

/// Kernel configuration parameters
#[derive(Debug, Clone)]
pub struct KernelConfig {
    /// Enable kernel debugging
    pub debug_enabled: bool,
    /// Kernel log level
    pub log_level: LogLevel,
    /// Memory safety checks
    pub memory_safety: bool,
    /// Maximum kernel stack size
    pub max_stack_size: usize,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            debug_enabled: false,
            log_level: LogLevel::Info,
            memory_safety: true,
            max_stack_size: 8192,
        }
    }
}

/// System log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Error messages only
    Error,
    /// Warnings and errors
    Warn,
    /// Informational messages
    Info,
    /// Debug messages
    Debug,
    /// Verbose debugging
    Trace,
}

/// Metrics configuration for performance monitoring
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    /// Metrics collection interval in seconds
    pub collection_interval: u32,
    /// Maximum metrics history entries
    pub max_history: usize,
    /// Enable latency tracking
    pub track_latency: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: 60,
            max_history: 1000,
            track_latency: true,
        }
    }
}

// ================================
// Kernel-Compatible Synchronization
// ================================

// Conditional imports for different environments
#[cfg(not(feature = "kernel"))]
use std::sync::{RwLock, Mutex};

#[cfg(feature = "kernel")]
use core::sync::atomic::{AtomicBool, Ordering};

// Kernel-compatible synchronization primitives
#[cfg(feature = "kernel")]
pub struct KernelRwLock<T> {
    data: core::cell::UnsafeCell<T>,
    locked: AtomicBool,
}

#[cfg(feature = "kernel")]
unsafe impl<T: Send> Send for KernelRwLock<T> {}
#[cfg(feature = "kernel")]
unsafe impl<T: Send + Sync> Sync for KernelRwLock<T> {}

#[cfg(feature = "kernel")]
impl<T> KernelRwLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: core::cell::UnsafeCell::new(data),
            locked: AtomicBool::new(false),
        }
    }

    pub fn read(&self) -> &T {
        unsafe { &*self.data.get() }
    }

    pub fn write(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

#[cfg(feature = "kernel")]
pub struct KernelMutex<T> {
    data: core::cell::UnsafeCell<T>,
    locked: AtomicBool,
}

#[cfg(feature = "kernel")]
unsafe impl<T: Send> Send for KernelMutex<T> {}
#[cfg(feature = "kernel")]
unsafe impl<T: Send + Sync> Sync for KernelMutex<T> {}

#[cfg(feature = "kernel")]
impl<T> KernelMutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: core::cell::UnsafeCell::new(data),
            locked: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

// Type aliases for easy switching between environments
#[cfg(not(feature = "kernel"))]
pub type VexfsRwLock<T> = RwLock<T>;
#[cfg(not(feature = "kernel"))]
pub type VexfsMutex<T> = Mutex<T>;

#[cfg(feature = "kernel")]
pub type VexfsRwLock<T> = KernelRwLock<T>;
#[cfg(feature = "kernel")]
pub type VexfsMutex<T> = KernelMutex<T>;
// =======================
// Filesystem Operation Types
// =======================

/// Directory entry information for filesystem operations
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Inode number of the entry
    pub inode: InodeNumber,
    /// Entry type (file, directory, symlink, etc.)
    pub entry_type: FileType,
    /// Name of the entry
    pub name: String,
    /// Size of the entry
    pub size: FileSize,
    /// Last modification time
    pub mtime: Timestamp,
}

/// File handle for open files
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileHandle(pub u64);

impl FileHandle {
    pub const INVALID: FileHandle = FileHandle(0);
    
    pub fn new(id: u64) -> Self {
        FileHandle(id)
    }
    
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for FileHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Access mode for file operations
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    ReadOnly = 0,
    WriteOnly = 1,
    ReadWrite = 2,
    Append = 3,
}

impl Default for AccessMode {
    fn default() -> Self {
        AccessMode::ReadOnly
    }
}

/// File creation flags
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateFlags {
    /// Create file if it doesn't exist
    pub create: bool,
    /// Fail if file already exists (with create)
    pub exclusive: bool,
    /// Truncate file to zero length
    pub truncate: bool,
    /// Open for append
    pub append: bool,
}

impl Default for CreateFlags {
    fn default() -> Self {
        CreateFlags {
            create: false,
            exclusive: false,
            truncate: false,
            append: false,
        }
    }
}

/// Seek position for file operations
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekPosition {
    /// From beginning of file
    Start(FileOffset),
    /// From current position
    Current(i64),
    /// From end of file
    End(i64),
}

/// Lock type for file locking
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// Shared read lock
    Shared = 0,
    /// Exclusive write lock
    Exclusive = 1,
    /// Unlock
    Unlock = 2,
}

/// Lock information
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FileLock {
    /// Type of lock
    pub lock_type: LockType,
    /// Start offset of lock
    pub start: FileOffset,
    /// Length of lock (0 = to EOF)
    pub length: FileSize,
    /// Process ID holding the lock
    pub pid: u32,
}

/// Range for file operations
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRange {
    /// Start offset
    pub start: FileOffset,
    /// Length of range
    pub length: FileSize,
}

impl FileRange {
    pub fn new(start: FileOffset, length: FileSize) -> Self {
        FileRange { start, length }
    }
    
    pub fn end(&self) -> FileOffset {
        self.start + self.length
    }
    
    pub fn contains(&self, offset: FileOffset) -> bool {
        offset >= self.start && offset < self.end()
    }
    
    pub fn overlaps(&self, other: &FileRange) -> bool {
        self.start < other.end() && other.start < self.end()
    }
}

/// Directory iterator state
#[derive(Debug)]
pub struct DirIterator {
    /// Directory inode being iterated
    pub dir_inode: InodeNumber,
    /// Current position in directory
    pub position: u64,
    /// Buffer for reading directory entries
    pub buffer: Vec<u8>,
    /// Current offset in buffer
    pub buffer_offset: usize,
}

impl DirIterator {
    pub fn new(dir_inode: InodeNumber) -> Self {
        DirIterator {
            dir_inode,
            position: 0,
            buffer: Vec::new(),
            buffer_offset: 0,
        }
    }
}

/// Path component for path resolution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathComponent {
    /// Root directory
    Root,
    /// Current directory
    Current,
    /// Parent directory  
    Parent,
    /// Normal name component
    Name(String),
}

/// Resolved path information
#[derive(Debug, Clone)]
pub struct ResolvedPath {
    /// Final inode number
    pub inode: InodeNumber,
    /// Parent directory inode
    pub parent_inode: InodeNumber,
    /// Final component name
    pub name: String,
    /// Full resolved path
    pub full_path: String,
}

/// Statistics about the filesystem
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VexfsStats {
    /// Total number of blocks in filesystem
    pub total_blocks: BlockNumber,
    /// Number of free blocks
    pub free_blocks: BlockNumber,
    /// Total number of inodes
    pub total_inodes: InodeNumber,
    /// Number of free inodes
    pub free_inodes: InodeNumber,
    /// Number of files
    pub file_count: u64,
    /// Number of directories
    pub dir_count: u64,
    /// Total size of all files
    pub total_file_size: FileSize,
    /// Fragmentation percentage (0-100)
    pub fragmentation: u8,
    /// Average file size
    pub avg_file_size: FileSize,
    /// Cache hit ratio (0-100)
    pub cache_hit_ratio: u8,
    /// Journal usage percentage (0-100)
    pub journal_usage: u8,
}

/// File attributes structure for metadata operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct FileAttributes {
    /// File size in bytes
    pub size: FileSize,
    /// File mode (type and permissions)
    pub mode: u32,
    /// User ID of owner
    pub uid: u32,
    /// Group ID of owner
    pub gid: u32,
    /// Access time (Unix timestamp)
    pub atime: Timestamp,
    /// Modification time (Unix timestamp)
    pub mtime: Timestamp,
    /// Status change time (Unix timestamp)
    pub ctime: Timestamp,
    /// Number of hard links
    pub nlink: u32,
    /// Device ID (if special file)
    pub dev: u32,
    /// File system block size
    pub blksize: u32,
    /// Number of blocks allocated
    pub blocks: u64,
}

impl Default for FileAttributes {
    fn default() -> Self {
        Self {
            size: 0,
            mode: 0o644, // Default file permissions
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            nlink: 1,
            dev: 0,
            blksize: VEXFS_DEFAULT_BLOCK_SIZE as u32,
            blocks: 0,
        }
    }
}