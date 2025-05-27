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

//! Configuration for VexFS Shared Domain
//!
//! This module provides configuration management for VexFS components.
//! Supports both compile-time and runtime configuration options.

use crate::shared::constants::*;
use crate::shared::types::*;
use crate::shared::errors::*;

// =======================
// Global Configuration
// =======================

/// Global VexFS configuration structure
#[derive(Debug, Clone)]
pub struct VexfsConfig {
    /// Filesystem configuration
    pub filesystem: FilesystemConfig,
    
    /// Vector indexing configuration
    pub vector: VectorConfig,
    
    /// Cache configuration
    pub cache: CacheConfig,
    
    /// I/O configuration
    pub io: IoConfig,
    
    /// Journal configuration
    pub journal: JournalConfig,
    
    /// Memory management configuration
    pub memory: MemoryConfig,
    
    /// Debug and logging configuration
    pub debug: DebugConfig,
}

impl Default for VexfsConfig {
    fn default() -> Self {
        Self {
            filesystem: FilesystemConfig::default(),
            vector: VectorConfig::default(),
            cache: CacheConfig::default(),
            io: IoConfig::default(),
            journal: JournalConfig::default(),
            memory: MemoryConfig::default(),
            debug: DebugConfig::default(),
        }
    }
}

// =======================
// Filesystem Configuration
// =======================

/// Filesystem-specific configuration
#[derive(Debug, Clone)]
pub struct FilesystemConfig {
    /// Block size for the filesystem
    pub block_size: u32,
    
    /// Maximum file size allowed
    pub max_file_size: u64,
    
    /// Maximum number of inodes
    pub max_inodes: u64,
    
    /// Enable compression
    pub compression_enabled: bool,
    
    /// Enable encryption
    pub encryption_enabled: bool,
    
    /// Default file mode for new files
    pub default_file_mode: u32,
    
    /// Default directory mode for new directories
    pub default_dir_mode: u32,
    
    /// Access time update threshold (seconds)
    pub atime_threshold: u64,
    
    /// Enable strict validation
    pub strict_validation: bool,
}

impl Default for FilesystemConfig {
    fn default() -> Self {
        Self {
            block_size: VEXFS_DEFAULT_BLOCK_SIZE as u32,
            max_file_size: VEXFS_MAX_FILE_SIZE,
            max_inodes: VEXFS_MAX_INODES,
            compression_enabled: false,
            encryption_enabled: false,
            default_file_mode: VEXFS_DEFAULT_FILE_MODE,
            default_dir_mode: VEXFS_DEFAULT_DIR_MODE,
            atime_threshold: VEXFS_ATIME_UPDATE_THRESHOLD,
            strict_validation: true,
        }
    }
    
}

// =======================
// Additional Configuration Types
// =======================

/// Vector pointer type for FFI compatibility
pub type VectorPtr = *mut crate::shared::types::Vector;

impl FilesystemConfig {
    /// Validate configuration parameters
    pub fn validate(&self) -> VexfsResult<()> {
        if !crate::shared::utils::is_power_of_2(self.block_size as u64) {
            return Err(VexfsError::InvalidConfiguration);
        }
        
        if self.block_size < VEXFS_MIN_BLOCK_SIZE as u32 || 
           self.block_size > VEXFS_MAX_BLOCK_SIZE as u32 {
            return Err(VexfsError::InvalidConfiguration);
        }
        
        if self.max_file_size > VEXFS_MAX_FILE_SIZE {
            return Err(VexfsError::InvalidConfiguration);
        }
        
        if self.max_inodes == 0 || self.max_inodes > VEXFS_MAX_INODES {
            return Err(VexfsError::InvalidConfiguration);
        }
        
        Ok(())
    }
}

// =======================
// Vector Configuration
// =======================

/// Vector indexing configuration
#[derive(Debug, Clone)]
pub struct VectorConfig {
    /// Enable vector indexing
    pub indexing_enabled: bool,
    
    /// Maximum vector dimensions
    pub max_dimensions: u16,
    
    /// Default similarity metric
    pub default_metric: SimilarityMetric,
    
    /// HNSW configuration
    pub hnsw: HnswConfig,
    
    /// Maximum vectors per file
    pub max_vectors_per_file: u32,
    
    /// Enable vector compression
    pub compression_enabled: bool,
    
    /// Vector cache size (number of vectors)
    pub cache_size: u32,
    
    /// Enable async indexing
    pub async_indexing: bool,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            indexing_enabled: true,
            max_dimensions: VEXFS_MAX_VECTOR_DIMS as u16,
            default_metric: SimilarityMetric::Euclidean,
            hnsw: HnswConfig::default(),
            max_vectors_per_file: VEXFS_MAX_VECTORS_PER_FILE,
            compression_enabled: false,
            cache_size: VEXFS_DEFAULT_VECTOR_CACHE_SIZE,
            async_indexing: true,
        }
    }
}

/// HNSW (Hierarchical Navigable Small World) configuration
#[derive(Debug, Clone)]
pub struct HnswConfig {
    /// Maximum connections per node (M parameter)
    pub max_connections: u16,
    
    /// Maximum connections for layer 0 (M_L parameter)
    pub max_connections_layer0: u16,
    
    /// Level generation factor (mL parameter)
    pub level_factor: f64,
    
    /// Search parameter (ef)
    pub ef_search: u16,
    
    /// Construction parameter (ef_construction)
    pub ef_construction: u16,
    
    /// Seed for random number generation
    pub random_seed: u64,
    
    /// Enable pruning during construction
    pub enable_pruning: bool,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            max_connections: VEXFS_DEFAULT_HNSW_M,
            max_connections_layer0: VEXFS_DEFAULT_HNSW_M_L,
            level_factor: VEXFS_DEFAULT_HNSW_ML,
            ef_search: VEXFS_DEFAULT_EF,
            ef_construction: VEXFS_DEFAULT_EF_CONSTRUCTION,
            random_seed: VEXFS_DEFAULT_RANDOM_SEED,
            enable_pruning: true,
        }
    }
}

impl HnswConfig {
    /// Validate HNSW configuration
    pub fn validate(&self) -> VexfsResult<()> {
        if self.max_connections == 0 || self.max_connections > VEXFS_MAX_HNSW_M {
            return Err(VexfsError::InvalidConfiguration);
        }
        
        if self.max_connections_layer0 == 0 || self.max_connections_layer0 > VEXFS_MAX_HNSW_M_L {
            return Err(VexfsError::InvalidConfiguration);
        }
        
        if self.ef_search == 0 || self.ef_search > VEXFS_MAX_EF {
            return Err(VexfsError::InvalidConfiguration);
        }
        
        if self.ef_construction == 0 || self.ef_construction > VEXFS_MAX_EF_CONSTRUCTION {
            return Err(VexfsError::InvalidConfiguration);
        }
        
        if self.level_factor <= 0.0 || self.level_factor > 2.0 {
            return Err(VexfsError::InvalidConfiguration);
        }
        
        Ok(())
    }
}

// =======================
// Cache Configuration
// =======================

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Enable page cache
    pub page_cache_enabled: bool,
    
    /// Page cache size in bytes
    pub page_cache_size: u64,
    
    /// Enable inode cache
    pub inode_cache_enabled: bool,
    
    /// Inode cache size (number of inodes)
    pub inode_cache_size: u32,
    
    /// Enable vector cache
    pub vector_cache_enabled: bool,
    
    /// Vector cache size (number of vectors)
    pub vector_cache_size: u32,
    
    /// Cache replacement policy
    pub replacement_policy: CacheReplacementPolicy,
    
    /// Write-through vs write-back policy
    pub write_policy: CacheWritePolicy,
    
    /// Cache line size
    pub cache_line_size: u32,
}

/// Cache replacement policies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheReplacementPolicy {
    Lru,     // Least Recently Used
    Lfu,     // Least Frequently Used
    Clock,   // Clock algorithm
    Random,  // Random replacement
}

/// Cache write policies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheWritePolicy {
    WriteThrough,  // Write to cache and storage immediately
    WriteBack,     // Write to cache, defer storage write
    WriteAround,   // Write to storage, bypass cache
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            page_cache_enabled: true,
            page_cache_size: VEXFS_DEFAULT_PAGE_CACHE_SIZE as u64,
            inode_cache_enabled: true,
            inode_cache_size: VEXFS_DEFAULT_INODE_CACHE_SIZE,
            vector_cache_enabled: true,
            vector_cache_size: VEXFS_DEFAULT_VECTOR_CACHE_SIZE,
            replacement_policy: CacheReplacementPolicy::Lru,
            write_policy: CacheWritePolicy::WriteBack,
            cache_line_size: VEXFS_DEFAULT_CACHE_LINE_SIZE,
        }
    }
}

// =======================
// I/O Configuration
// =======================

/// I/O configuration
#[derive(Debug, Clone)]
pub struct IoConfig {
    /// Enable asynchronous I/O
    pub async_io_enabled: bool,
    
    /// I/O queue depth
    pub queue_depth: u32,
    
    /// Read-ahead size in blocks
    pub readahead_size: u32,
    
    /// Enable I/O batching
    pub batching_enabled: bool,
    
    /// Maximum batch size
    pub max_batch_size: u32,
    
    /// I/O timeout in milliseconds
    pub timeout_ms: u32,
    
    /// Number of I/O threads
    pub io_threads: u32,
    
    /// Enable direct I/O
    pub direct_io: bool,
}

impl Default for IoConfig {
    fn default() -> Self {
        Self {
            async_io_enabled: true,
            queue_depth: VEXFS_DEFAULT_IO_QUEUE_DEPTH,
            readahead_size: VEXFS_DEFAULT_READAHEAD_SIZE,
            batching_enabled: true,
            max_batch_size: VEXFS_DEFAULT_MAX_BATCH_SIZE,
            timeout_ms: VEXFS_DEFAULT_IO_TIMEOUT_MS,
            io_threads: VEXFS_DEFAULT_IO_THREADS,
            direct_io: false,
        }
    }
}

// =======================
// Journal Configuration
// =======================

/// Journal configuration
#[derive(Debug, Clone)]
pub struct JournalConfig {
    /// Enable journaling
    pub enabled: bool,
    
    /// Journal size in blocks
    pub size_blocks: u32,
    
    /// Journal commit interval in milliseconds
    pub commit_interval_ms: u32,
    
    /// Enable write-ahead logging
    pub wal_enabled: bool,
    
    /// Maximum transaction size
    pub max_transaction_size: u32,
    
    /// Enable compression for journal entries
    pub compression_enabled: bool,
    
    /// Enable checksums for journal entries
    pub checksums_enabled: bool,
    
    /// Journal flush policy
    pub flush_policy: JournalFlushPolicy,
}

/// Journal flush policies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalFlushPolicy {
    Immediate,  // Flush after every transaction
    Periodic,   // Flush at regular intervals
    OnSync,     // Flush only on explicit sync
    Lazy,       // Flush when convenient
}

impl Default for JournalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            size_blocks: VEXFS_DEFAULT_JOURNAL_SIZE,
            commit_interval_ms: VEXFS_DEFAULT_JOURNAL_COMMIT_INTERVAL,
            wal_enabled: true,
            max_transaction_size: VEXFS_DEFAULT_MAX_TRANSACTION_SIZE,
            compression_enabled: false,
            checksums_enabled: true,
            flush_policy: JournalFlushPolicy::Periodic,
        }
    }
}

// =======================
// Memory Configuration
// =======================

/// Memory management configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Enable memory pools
    pub pools_enabled: bool,
    
    /// Small allocation pool size
    pub small_pool_size: u64,
    
    /// Medium allocation pool size
    pub medium_pool_size: u64,
    
    /// Large allocation pool size
    pub large_pool_size: u64,
    
    /// Enable memory statistics
    pub stats_enabled: bool,
    
    /// Memory allocation alignment
    pub alignment: u32,
    
    /// Enable memory debugging
    pub debug_enabled: bool,
    
    /// Memory pressure threshold (percentage)
    pub pressure_threshold: u8,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            pools_enabled: true,
            small_pool_size: VEXFS_DEFAULT_SMALL_POOL_SIZE,
            medium_pool_size: VEXFS_DEFAULT_MEDIUM_POOL_SIZE,
            large_pool_size: VEXFS_DEFAULT_LARGE_POOL_SIZE,
            stats_enabled: true,
            alignment: VEXFS_DEFAULT_ALIGNMENT,
            debug_enabled: false,
            pressure_threshold: VEXFS_DEFAULT_MEMORY_PRESSURE_THRESHOLD,
        }
    }
}

// =======================
// Debug Configuration
// =======================

/// Debug and logging configuration
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Enable debug logging
    pub logging_enabled: bool,
    
    /// Log level
    pub log_level: LogLevel,
    
    /// Enable performance tracing
    pub tracing_enabled: bool,
    
    /// Enable statistics collection
    pub stats_enabled: bool,
    
    /// Enable assertions in release builds
    pub assertions_enabled: bool,
    
    /// Debug output buffer size
    pub buffer_size: u32,
    
    /// Enable verbose error reporting
    pub verbose_errors: bool,
}

/// Debug log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            logging_enabled: cfg!(debug_assertions),
            log_level: if cfg!(debug_assertions) { LogLevel::Debug } else { LogLevel::Warn },
            tracing_enabled: cfg!(debug_assertions),
            stats_enabled: true,
            assertions_enabled: cfg!(debug_assertions),
            buffer_size: VEXFS_DEFAULT_DEBUG_BUFFER_SIZE,
            verbose_errors: cfg!(debug_assertions),
        }
    }
}

// =======================
// Configuration Builder
// =======================

/// Builder for creating VexFS configurations
pub struct VexfsConfigBuilder {
    config: VexfsConfig,
}

impl VexfsConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            config: VexfsConfig::default(),
        }
    }

    /// Set filesystem configuration
    pub fn filesystem(mut self, filesystem: FilesystemConfig) -> Self {
        self.config.filesystem = filesystem;
        self
    }

    /// Set vector configuration
    pub fn vector(mut self, vector: VectorConfig) -> Self {
        self.config.vector = vector;
        self
    }

    /// Set cache configuration
    pub fn cache(mut self, cache: CacheConfig) -> Self {
        self.config.cache = cache;
        self
    }

    /// Set I/O configuration
    pub fn io(mut self, io: IoConfig) -> Self {
        self.config.io = io;
        self
    }

    /// Set journal configuration
    pub fn journal(mut self, journal: JournalConfig) -> Self {
        self.config.journal = journal;
        self
    }

    /// Set memory configuration
    pub fn memory(mut self, memory: MemoryConfig) -> Self {
        self.config.memory = memory;
        self
    }

    /// Set debug configuration
    pub fn debug(mut self, debug: DebugConfig) -> Self {
        self.config.debug = debug;
        self
    }

    /// Build the final configuration
    pub fn build(self) -> VexfsResult<VexfsConfig> {
        // Validate all configuration components
        self.config.filesystem.validate()?;
        self.config.vector.hnsw.validate()?;
        
        Ok(self.config)
    }
}

impl Default for VexfsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =======================
// Configuration Access
// =======================

/// Global configuration instance (for userspace)
#[cfg(not(feature = "kernel"))]
static mut GLOBAL_CONFIG: Option<VexfsConfig> = None;

/// Initialize global configuration
#[cfg(not(feature = "kernel"))]
pub fn init_config(config: VexfsConfig) -> VexfsResult<()> {
    unsafe {
        GLOBAL_CONFIG = Some(config);
    }
    Ok(())
}

/// Default configuration instance
static DEFAULT_CONFIG: VexfsConfig = VexfsConfig {
    filesystem: FilesystemConfig {
        block_size: VEXFS_DEFAULT_BLOCK_SIZE as u32,
        max_file_size: VEXFS_MAX_FILE_SIZE,
        max_inodes: VEXFS_MAX_INODES,
        compression_enabled: false,
        encryption_enabled: false,
        default_file_mode: VEXFS_DEFAULT_FILE_MODE,
        default_dir_mode: VEXFS_DEFAULT_DIR_MODE,
        atime_threshold: VEXFS_ATIME_UPDATE_THRESHOLD,
        strict_validation: true,
    },
    vector: VectorConfig {
        indexing_enabled: true,
        max_dimensions: VEXFS_MAX_VECTOR_DIMENSIONS,
        default_metric: crate::shared::types::DistanceMetric::Euclidean,
        hnsw_params: HnswConfig {
            max_connections: VEXFS_DEFAULT_HNSW_M,
            max_connections_layer0: VEXFS_DEFAULT_HNSW_M_L,
            ef_search: VEXFS_DEFAULT_EF_SEARCH,
            ef_construction: VEXFS_DEFAULT_EF_CONSTRUCTION,
            random_seed: VEXFS_DEFAULT_RANDOM_SEED,
        },
        quantization: QuantizationConfig {
            enabled: false,
            bits_per_component: 8,
            training_threshold: 10000,
        },
        memory: MemoryConfig {
            cache_size: VEXFS_DEFAULT_INDEX_CACHE_SIZE,
            page_size: VEXFS_DEFAULT_PAGE_SIZE,
            cache_line_size: VEXFS_DEFAULT_CACHE_LINE_SIZE,
            prealloc_size: 64 * 1024 * 1024,
        },
        io: IoConfig {
            queue_depth: VEXFS_DEFAULT_IO_QUEUE_DEPTH,
            readahead_size: VEXFS_DEFAULT_READAHEAD_SIZE,
            batch_enabled: true,
            max_batch_size: VEXFS_DEFAULT_MAX_BATCH_SIZE,
            timeout_ms: VEXFS_DEFAULT_IO_TIMEOUT_MS,
            io_threads: VEXFS_DEFAULT_IO_THREADS as u32,
        },
        search: SearchConfig {
            parallel_enabled: true,
            max_threads: 4,
            result_cache_size: 1000,
            timeout_ms: (VEXFS_DEFAULT_TIMEOUT_SECS as u64 * 1000) as u32,
        },
    },
    debug: DebugConfig {
        enable_logging: false,
        log_level: crate::shared::types::LogLevel::Info,
        enable_metrics: false,
        enable_tracing: false,
        memory_tracking: false,
    },
    kernel: KernelConfig {
        module_name: "vexfs".to_string(),
        max_open_files: 65536,
        sync_interval: 30,
        readahead_enabled: true,
        writeback_enabled: true,
    },
};

/// Get global configuration
#[cfg(not(feature = "kernel"))]
pub fn get_config() -> &'static VexfsConfig {
    unsafe {
        GLOBAL_CONFIG.as_ref().unwrap_or(&DEFAULT_CONFIG)
    }
}

/// Update global configuration
#[cfg(not(feature = "kernel"))]
pub fn update_config<F>(f: F) -> VexfsResult<()>
where
    F: FnOnce(&mut VexfsConfig),
{
    unsafe {
        if let Some(ref mut config) = GLOBAL_CONFIG {
            f(config);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VexfsConfig::default();
        assert!(config.filesystem.validate().is_ok());
        assert!(config.vector.hnsw.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = VexfsConfigBuilder::new()
            .filesystem(FilesystemConfig {
                block_size: 8192,
                ..Default::default()
            })
            .build()
            .unwrap();
        
        assert_eq!(config.filesystem.block_size, 8192);
    }

    #[test]
    fn test_filesystem_config_validation() {
        let mut config = FilesystemConfig::default();
        assert!(config.validate().is_ok());
        
        config.block_size = 1023; // Not power of 2
        assert!(config.validate().is_err());
        
        config.block_size = 512; // Too small
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_hnsw_config_validation() {
        let mut config = HnswConfig::default();
        assert!(config.validate().is_ok());
        
        config.max_connections = 0;
        assert!(config.validate().is_err());
        
        config.max_connections = 16;
        config.level_factor = -1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cache_replacement_policy() {
        let policy = CacheReplacementPolicy::Lru;
        assert_eq!(policy, CacheReplacementPolicy::Lru);
    }

    #[test]
    fn test_journal_flush_policy() {
        let policy = JournalFlushPolicy::Periodic;
        assert_eq!(policy, JournalFlushPolicy::Periodic);
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }
}