//! Approximate Nearest Neighbor Search (ANNS) Module for VexFS
//! 
//! This module implements a kernel-optimized ANNS system based on Hierarchical Navigable Small World (HNSW)
//! algorithm with on-disk persistence, memory management, and write-ahead logging for crash consistency.
//! 
//! Key Features:
//! - HNSW algorithm with SIMD-optimized distance calculations
//! - Memory-mapped on-disk serialization with partial loading
//! - Incremental index updates with minimal graph restructuring
//! - LRU cache for memory management
//! - Write-ahead logging for crash recovery
//! - Tunable parameters for performance optimization

#![no_std]

use core::{mem, slice, ptr};
// Re-export submodules
pub mod hnsw;
pub mod serialization;
pub mod indexing;
pub mod memory_mgmt;
pub mod wal;

// Re-export commonly used types from submodules
pub use self::hnsw::{HnswIndex, HnswBuilder, HnswConfig, SearchParams, IndexStats};
pub use self::serialization::{IndexFormat, SerializationConfig, IndexSerializer, IndexDeserializer};
pub use self::indexing::{IndexBuilder, BatchBuilder, IncrementalBuilder, BuildConfig, BuildStats};
pub use self::memory_mgmt::{PartialLoader, LruCache, MemoryBudget, MemoryUsage};
pub use self::wal::{WalWriter, WalReader, Transaction, WalStats};
use crate::vector_storage::{VectorStorageError, VectorDataType, VectorHeader};

/// ANNS module errors
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnsError {
    /// Invalid index parameters
    InvalidParameters,
    /// Index not found
    IndexNotFound,
    /// Vector not found in index
    VectorNotFound,
    /// Index is corrupted
    CorruptedIndex,
    /// Memory allocation failed
    OutOfMemory,
    /// I/O error during index operations
    IoError,
    /// Invalid index format or version
    InvalidFormat,
    /// Cache miss
    CacheMiss,
    /// WAL error
    WalError,
    /// Index is read-only
    ReadOnly,
    /// Concurrent access error
    ConcurrentAccess,
}

/// ANNS algorithm types
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum AnnsAlgorithm {
    HNSW = 0,
    LSH = 1,
    IVF = 2,
    Graph = 3,
}

/// Distance metrics for similarity search
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum DistanceMetric {
    Euclidean = 0,
    Cosine = 1,
    Manhattan = 2,
    Hamming = 3,
    Dot = 4,
}

/// HNSW index parameters
#[derive(Debug, Clone, Copy)]
pub struct HnswParams {
    /// Maximum number of connections per node (M parameter)
    pub max_connections: u16,
    /// Maximum number of connections for layer 0 (MMax parameter)
    pub max_connections_layer0: u16,
    /// Level generation multiplier (mL parameter)
    pub level_multiplier: f32,
    /// Search depth during construction (efConstruction)
    pub ef_construction: u16,
    /// Search depth during search (ef)
    pub ef_search: u16,
    /// Distance metric to use
    pub distance_metric: DistanceMetric,
    /// Enable SIMD optimizations
    pub use_simd: bool,
    /// Memory budget for partial loading (in MB)
    pub memory_budget_mb: u32,
}

impl Default for HnswParams {
    fn default() -> Self {
        Self {
            max_connections: 16,
            max_connections_layer0: 32,
            level_multiplier: 1.0 / 2.0f32.ln(),
            ef_construction: 200,
            ef_search: 50,
            distance_metric: DistanceMetric::Euclidean,
            use_simd: true,
            memory_budget_mb: 64,
        }
    }
}

/// HNSW node representation
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct HnswNode {
    /// Vector ID this node represents
    pub vector_id: u64,
    /// Level of this node (0 = base layer)
    pub level: u8,
    /// Number of connections at this level
    pub connection_count: u16,
    /// Offset to connections array
    pub connections_offset: u32,
    /// Flags for node state
    pub flags: u8,
    /// Reserved for alignment
    pub reserved: [u8; 2],
}

/// HNSW layer information
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct HnswLayer {
    /// Layer number (0 = base layer)
    pub level: u8,
    /// Number of nodes in this layer
    pub node_count: u32,
    /// Offset to nodes array
    pub nodes_offset: u64,
    /// Entry point for this layer
    pub entry_point: u64,
    /// Reserved for future use
    pub reserved: [u8; 7],
}

/// ANNS index header for on-disk format
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct AnnsIndexHeader {
    /// Magic number for validation
    pub magic: u32,
    /// Index format version
    pub version: u32,
    /// Algorithm type
    pub algorithm: AnnsAlgorithm,
    /// Distance metric
    pub distance_metric: DistanceMetric,
    /// Vector data type
    pub data_type: VectorDataType,
    /// Number of dimensions
    pub dimensions: u32,
    /// Total number of vectors in index
    pub vector_count: u64,
    /// Number of layers in HNSW
    pub layer_count: u8,
    /// HNSW parameters
    pub max_connections: u16,
    pub max_connections_layer0: u16,
    pub ef_construction: u16,
    pub level_multiplier: f32,
    /// Entry point vector ID
    pub entry_point: u64,
    /// Timestamp when index was created
    pub created_timestamp: u64,
    /// Timestamp when index was last modified
    pub modified_timestamp: u64,
    /// Index flags
    pub flags: u32,
    /// Checksum for integrity
    pub checksum: u32,
    /// Offset to layer data
    pub layers_offset: u64,
    /// Offset to node data
    pub nodes_offset: u64,
    /// Offset to connections data
    pub connections_offset: u64,
    /// Offset to vector data
    pub vectors_offset: u64,
    /// Reserved for future expansion
    pub reserved: [u8; 32],
}

impl AnnsIndexHeader {
    pub const MAGIC: u32 = 0x414E4E53; // "ANNS"
    pub const VERSION: u32 = 1;
    pub const SIZE: usize = mem::size_of::<AnnsIndexHeader>();
}

/// Search result with distance
#[derive(Debug, Clone, Copy)]
pub struct SearchResult {
    /// Vector ID
    pub vector_id: u64,
    /// Distance to query vector
    pub distance: f32,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
}

/// Search context for maintaining state across operations
pub struct SearchContext {
    /// Visited nodes bitmap (fixed size for no_std)
    pub visited: [u64; 64], // 4096 bits
    /// Current search depth
    pub depth: u16,
    /// Found candidates
    pub candidates: [SearchResult; 256],
    /// Number of candidates found
    pub candidate_count: u16,
}

impl SearchContext {
    pub fn new() -> Self {
        Self {
            visited: [0; 64],
            depth: 0,
            candidates: [SearchResult { vector_id: 0, distance: f32::INFINITY, confidence: 0.0 }; 256],
            candidate_count: 0,
        }
    }

    pub fn reset(&mut self) {
        self.visited.fill(0);
        self.depth = 0;
        self.candidate_count = 0;
    }

    pub fn is_visited(&self, node_id: u64) -> bool {
        let word_idx = (node_id / 64) as usize;
        let bit_idx = (node_id % 64) as usize;
        if word_idx >= self.visited.len() {
            return false;
        }
        (self.visited[word_idx] & (1u64 << bit_idx)) != 0
    }

    pub fn mark_visited(&mut self, node_id: u64) {
        let word_idx = (node_id / 64) as usize;
        let bit_idx = (node_id % 64) as usize;
        if word_idx < self.visited.len() {
            self.visited[word_idx] |= 1u64 << bit_idx;
        }
    }
}

/// LRU cache for partial index loading
pub struct IndexCache {
    /// Cache entries (vector_id -> data_offset)
    pub entries: [(u64, u64, u64); 1024], // (vector_id, offset, timestamp)
    /// Next timestamp for LRU
    pub next_timestamp: u64,
    /// Current cache size
    pub current_size: u32,
    /// Maximum cache size in bytes
    pub max_size: u32,
}

impl IndexCache {
    pub fn new(max_size_mb: u32) -> Self {
        Self {
            entries: [(0, 0, 0); 1024],
            next_timestamp: 1,
            current_size: 0,
            max_size: max_size_mb * 1024 * 1024,
        }
    }

    pub fn get(&mut self, vector_id: u64) -> Option<u64> {
        for i in 0..self.entries.len() {
            if self.entries[i].0 == vector_id && self.entries[i].0 != 0 {
                // Update timestamp for LRU
                self.entries[i].2 = self.next_timestamp;
                self.next_timestamp = self.next_timestamp.wrapping_add(1);
                return Some(self.entries[i].1);
            }
        }
        None
    }

    pub fn put(&mut self, vector_id: u64, offset: u64) -> Result<(), AnnsError> {
        // Find empty slot or LRU victim
        let mut victim_idx = 0;
        let mut oldest_timestamp = u64::MAX;
        let mut empty_slot = None;

        for i in 0..self.entries.len() {
            if self.entries[i].0 == 0 {
                empty_slot = Some(i);
                break;
            }
            if self.entries[i].2 < oldest_timestamp {
                oldest_timestamp = self.entries[i].2;
                victim_idx = i;
            }
        }

        let slot_idx = empty_slot.unwrap_or(victim_idx);
        self.entries[slot_idx] = (vector_id, offset, self.next_timestamp);
        self.next_timestamp = self.next_timestamp.wrapping_add(1);

        Ok(())
    }

    pub fn evict_lru(&mut self) {
        let mut oldest_idx = 0;
        let mut oldest_timestamp = u64::MAX;

        for i in 0..self.entries.len() {
            if self.entries[i].0 != 0 && self.entries[i].2 < oldest_timestamp {
                oldest_timestamp = self.entries[i].2;
                oldest_idx = i;
            }
        }

        self.entries[oldest_idx] = (0, 0, 0);
    }
}

/// Write-Ahead Log entry types
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum WalEntryType {
    Insert = 0,
    Delete = 1,
    Update = 2,
    Checkpoint = 3,
    Commit = 4,
}

/// WAL entry header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct WalEntryHeader {
    /// Entry type
    pub entry_type: WalEntryType,
    /// Entry size in bytes
    pub size: u32,
    /// Transaction ID
    pub txn_id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Checksum
    pub checksum: u32,
    /// Flags
    pub flags: u8,
    /// Reserved
    pub reserved: [u8; 2],
}

/// WAL insert entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct WalInsertEntry {
    /// Header
    pub header: WalEntryHeader,
    /// Vector ID being inserted
    pub vector_id: u64,
    /// Level assigned to this vector
    pub level: u8,
    /// Number of connections
    pub connection_count: u16,
    /// Reserved
    pub reserved: [u8; 5],
    // Followed by connection data
}

/// Main ANNS index structure
pub struct AnnsIndex {
    /// Index header
    pub header: AnnsIndexHeader,
    /// HNSW parameters
    pub params: HnswParams,
    /// Memory cache for partial loading
    pub cache: IndexCache,
    /// Search context (reused across searches)
    pub search_context: SearchContext,
    /// WAL transaction ID
    pub next_txn_id: u64,
    /// Index is loaded
    pub is_loaded: bool,
    /// Read-only mode
    pub read_only: bool,
}

impl AnnsIndex {
    /// Create a new ANNS index with specified parameters
    pub fn new(params: HnswParams, dimensions: u32, data_type: VectorDataType) -> Self {
        let header = AnnsIndexHeader {
            magic: AnnsIndexHeader::MAGIC,
            version: AnnsIndexHeader::VERSION,
            algorithm: AnnsAlgorithm::HNSW,
            distance_metric: params.distance_metric,
            data_type,
            dimensions,
            vector_count: 0,
            layer_count: 0,
            max_connections: params.max_connections,
            max_connections_layer0: params.max_connections_layer0,
            ef_construction: params.ef_construction,
            level_multiplier: params.level_multiplier,
            entry_point: 0,
            created_timestamp: 0, // TODO: get kernel time
            modified_timestamp: 0,
            flags: 0,
            checksum: 0,
            layers_offset: 0,
            nodes_offset: 0,
            connections_offset: 0,
            vectors_offset: 0,
            reserved: [0; 32],
        };

        Self {
            header,
            params,
            cache: IndexCache::new(params.memory_budget_mb),
            search_context: SearchContext::new(),
            next_txn_id: 1,
            is_loaded: false,
            read_only: false,
        }
    }

    /// Add a vector to the index
    pub fn insert_vector(&mut self, vector_id: u64, vector_data: &[f32]) -> Result<(), AnnsError> {
        if self.read_only {
            return Err(AnnsError::ReadOnly);
        }

        if vector_data.len() != self.header.dimensions as usize {
            return Err(AnnsError::InvalidParameters);
        }

        // Generate random level for HNSW
        let level = self.generate_random_level();
        
        // Create new node
        let node = HnswNode {
            vector_id,
            level,
            connection_count: 0,
            connections_offset: 0,
            flags: 0,
            reserved: [0; 2],
        };

        // WAL logging
        self.log_insert(vector_id, level)?;

        // Update header
        self.header.vector_count += 1;
        if level as u8 >= self.header.layer_count {
            self.header.layer_count = level + 1;
        }

        // If this is the first vector, make it the entry point
        if self.header.entry_point == 0 {
            self.header.entry_point = vector_id;
        }

        // TODO: Implement actual graph construction
        // This would involve:
        // 1. Finding nearest neighbors at each level
        // 2. Creating bidirectional connections
        // 3. Maintaining max_connections constraints
        // 4. Updating existing connections if needed

        Ok(())
    }

    /// Search for k nearest neighbors
    pub fn search_knn(&mut self, query: &[f32], k: usize, ef: Option<u16>) -> Result<[SearchResult; 256], AnnsError> {
        if query.len() != self.header.dimensions as usize {
            return Err(AnnsError::InvalidParameters);
        }

        if self.header.vector_count == 0 {
            return Err(AnnsError::VectorNotFound);
        }

        let ef_search = ef.unwrap_or(self.params.ef_search);
        self.search_context.reset();

        // Start from entry point
        let mut current_best = SearchResult {
            vector_id: self.header.entry_point,
            distance: f32::INFINITY,
            confidence: 0.0,
        };

        // TODO: Implement HNSW search algorithm
        // This would involve:
        // 1. Starting from top layer and working down
        // 2. Greedy search at each layer
        // 3. Maintaining candidate list
        // 4. Using distance calculations with SIMD optimization

        // For now, return empty result
        Ok(self.search_context.candidates)
    }

    /// Remove a vector from the index
    pub fn delete_vector(&mut self, vector_id: u64) -> Result<(), AnnsError> {
        if self.read_only {
            return Err(AnnsError::ReadOnly);
        }

        // WAL logging
        self.log_delete(vector_id)?;

        // TODO: Implement vector deletion
        // This would involve:
        // 1. Finding the node in the graph
        // 2. Removing all connections to this node
        // 3. Reconnecting neighbors to maintain graph connectivity
        // 4. Updating entry point if necessary

        self.header.vector_count = self.header.vector_count.saturating_sub(1);
        Ok(())
    }

    /// Serialize index to byte array for persistence
    pub fn serialize(&self) -> Result<[u8; 65536], AnnsError> {
        let mut buffer = [0u8; 65536];
        let mut offset = 0;

        // Write header
        unsafe {
            let header_bytes = core::slice::from_raw_parts(
                &self.header as *const _ as *const u8,
                mem::size_of::<AnnsIndexHeader>()
            );
            if offset + header_bytes.len() > buffer.len() {
                return Err(AnnsError::OutOfMemory);
            }
            buffer[offset..offset + header_bytes.len()].copy_from_slice(header_bytes);
            offset += header_bytes.len();
        }

        // TODO: Write layer data, nodes, connections, and vectors
        // This would involve writing each component to the buffer
        // with proper alignment and checksums

        Ok(buffer)
    }

    /// Deserialize index from byte array
    pub fn deserialize(data: &[u8]) -> Result<Self, AnnsError> {
        if data.len() < AnnsIndexHeader::SIZE {
            return Err(AnnsError::InvalidFormat);
        }

        // Read header
        let header = unsafe {
            ptr::read_unaligned(data.as_ptr() as *const AnnsIndexHeader)
        };

        // Validate header
        if header.magic != AnnsIndexHeader::MAGIC {
            return Err(AnnsError::InvalidFormat);
        }

        if header.version != AnnsIndexHeader::VERSION {
            return Err(AnnsError::InvalidFormat);
        }

        let params = HnswParams {
            max_connections: header.max_connections,
            max_connections_layer0: header.max_connections_layer0,
            level_multiplier: header.level_multiplier,
            ef_construction: header.ef_construction,
            ef_search: 50, // Default search parameter
            distance_metric: header.distance_metric,
            use_simd: true,
            memory_budget_mb: 64,
        };

        let mut index = Self::new(params, header.dimensions, header.data_type);
        index.header = header;
        index.is_loaded = true;

        // TODO: Load layer data, nodes, and connections
        // This would involve reading the serialized graph structure

        Ok(index)
    }

    /// Generate random level for HNSW node
    fn generate_random_level(&self) -> u8 {
        // Simple deterministic level generation for kernel space
        // In practice, this would use proper random number generation
        let mut level = 0u8;
        let mut rng = self.header.vector_count.wrapping_mul(1103515245).wrapping_add(12345);
        
        while level < 16 && (rng & 1) == 1 {
            level += 1;
            rng >>= 1;
        }
        
        level
    }

    /// Calculate distance between vectors with SIMD optimization
    fn calculate_distance(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        if vec1.len() != vec2.len() {
            return f32::INFINITY;
        }

        match self.params.distance_metric {
            DistanceMetric::Euclidean => {
                if self.params.use_simd {
                    self.euclidean_distance_simd(vec1, vec2)
                } else {
                    self.euclidean_distance_scalar(vec1, vec2)
                }
            }
            DistanceMetric::Cosine => self.cosine_distance(vec1, vec2),
            DistanceMetric::Manhattan => self.manhattan_distance(vec1, vec2),
            DistanceMetric::Dot => self.dot_product(vec1, vec2),
            _ => f32::INFINITY,
        }
    }

    /// SIMD-optimized Euclidean distance
    fn euclidean_distance_simd(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        // TODO: Implement SIMD optimization using kernel SIMD intrinsics
        // For now, fall back to scalar implementation
        self.euclidean_distance_scalar(vec1, vec2)
    }

    /// Scalar Euclidean distance calculation
    fn euclidean_distance_scalar(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        let mut sum = 0.0f32;
        for i in 0..vec1.len() {
            let diff = vec1[i] - vec2[i];
            sum += diff * diff;
        }
        sum.sqrt()
    }

    /// Cosine distance calculation
    fn cosine_distance(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        let mut dot = 0.0f32;
        let mut norm1 = 0.0f32;
        let mut norm2 = 0.0f32;

        for i in 0..vec1.len() {
            dot += vec1[i] * vec2[i];
            norm1 += vec1[i] * vec1[i];
            norm2 += vec2[i] * vec2[i];
        }

        let norm_product = (norm1 * norm2).sqrt();
        if norm_product == 0.0 {
            return f32::INFINITY;
        }

        1.0 - (dot / norm_product)
    }

    /// Manhattan distance calculation
    fn manhattan_distance(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        let mut sum = 0.0f32;
        for i in 0..vec1.len() {
            sum += (vec1[i] - vec2[i]).abs();
        }
        sum
    }

    /// Dot product calculation (for similarity)
    fn dot_product(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        let mut sum = 0.0f32;
        for i in 0..vec1.len() {
            sum += vec1[i] * vec2[i];
        }
        -sum // Negate for distance (smaller is better)
    }

    /// Log insert operation to WAL
    fn log_insert(&mut self, vector_id: u64, level: u8) -> Result<(), AnnsError> {
        let entry = WalInsertEntry {
            header: WalEntryHeader {
                entry_type: WalEntryType::Insert,
                size: mem::size_of::<WalInsertEntry>() as u32,
                txn_id: self.next_txn_id,
                timestamp: 0, // TODO: get kernel time
                checksum: 0,
                flags: 0,
                reserved: [0; 2],
            },
            vector_id,
            level,
            connection_count: 0,
            reserved: [0; 5],
        };

        self.next_txn_id = self.next_txn_id.wrapping_add(1);

        // TODO: Write to WAL file
        // This would involve writing the entry to the persistent WAL

        Ok(())
    }

    /// Log delete operation to WAL
    fn log_delete(&mut self, vector_id: u64) -> Result<(), AnnsError> {
        let entry_header = WalEntryHeader {
            entry_type: WalEntryType::Delete,
            size: (mem::size_of::<WalEntryHeader>() + 8) as u32, // header + vector_id
            txn_id: self.next_txn_id,
            timestamp: 0, // TODO: get kernel time
            checksum: 0,
            flags: 0,
            reserved: [0; 2],
        };

        self.next_txn_id = self.next_txn_id.wrapping_add(1);

        // TODO: Write to WAL file
        // This would involve writing the header and vector_id to the persistent WAL

        Ok(())
    }

    /// Replay WAL for crash recovery
    pub fn replay_wal(&mut self, wal_data: &[u8]) -> Result<(), AnnsError> {
        let mut offset = 0;

        while offset + mem::size_of::<WalEntryHeader>() <= wal_data.len() {
            let header = unsafe {
                ptr::read_unaligned(wal_data[offset..].as_ptr() as *const WalEntryHeader)
            };

            // Validate entry
            if offset + header.size as usize > wal_data.len() {
                break;
            }

            // Process entry based on type
            match header.entry_type {
                WalEntryType::Insert => {
                    if header.size >= mem::size_of::<WalInsertEntry>() as u32 {
                        let insert_entry = unsafe {
                            ptr::read_unaligned(wal_data[offset..].as_ptr() as *const WalInsertEntry)
                        };
                        // TODO: Replay insert operation
                    }
                }
                WalEntryType::Delete => {
                    if header.size >= (mem::size_of::<WalEntryHeader>() + 8) as u32 {
                        let vector_id = unsafe {
                            ptr::read_unaligned(wal_data[offset + mem::size_of::<WalEntryHeader>()..].as_ptr() as *const u64)
                        };
                        // TODO: Replay delete operation
                    }
                }
                WalEntryType::Checkpoint => {
                    // Checkpoint marker - can truncate WAL here
                    break;
                }
                _ => {
                    // Unknown entry type - skip
                }
            }

            offset += header.size as usize;
        }

        Ok(())
    }

    /// Get index statistics
    pub fn get_stats(&self) -> AnnsStats {
        AnnsStats {
            vector_count: self.header.vector_count,
            layer_count: self.header.layer_count,
            entry_point: self.header.entry_point,
            cache_hits: 0, // TODO: track cache statistics
            cache_misses: 0,
            search_operations: 0,
            insert_operations: 0,
            delete_operations: 0,
            avg_search_time_us: 0,
            memory_usage_bytes: 0,
        }
    }
}

/// ANNS index statistics
#[derive(Debug, Clone, Copy)]
pub struct AnnsStats {
    pub vector_count: u64,
    pub layer_count: u8,
    pub entry_point: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub search_operations: u64,
    pub insert_operations: u64,
    pub delete_operations: u64,
    pub avg_search_time_us: u32,
    pub memory_usage_bytes: u64,
}

/// Tests for ANNS functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anns_header_size() {
        // Ensure header is reasonably sized
        assert!(AnnsIndexHeader::SIZE <= 256);
    }

    #[test]
    fn test_hnsw_params_default() {
        let params = HnswParams::default();
        assert_eq!(params.max_connections, 16);
        assert_eq!(params.max_connections_layer0, 32);
        assert!(params.use_simd);
    }

    #[test]
    fn test_search_context() {
        let mut ctx = SearchContext::new();
        assert!(!ctx.is_visited(42));
        ctx.mark_visited(42);
        assert!(ctx.is_visited(42));
    }

    #[test]
    fn test_index_cache() {
        let mut cache = IndexCache::new(1);
        assert!(cache.get(123).is_none());
        cache.put(123, 456).unwrap();
        assert_eq!(cache.get(123), Some(456));
    }

    #[test]
    fn test_distance_calculations() {
        let params = HnswParams::default();
        let index = AnnsIndex::new(params, 3, VectorDataType::Float32);
        
        let vec1 = [1.0, 2.0, 3.0];
        let vec2 = [4.0, 5.0, 6.0];
        
        let dist = index.euclidean_distance_scalar(&vec1, &vec2);
        assert!((dist - 5.196152).abs() < 0.001); // sqrt(27)
    }
}
/// Integrated ANNS system that coordinates all components
pub struct IntegratedAnnsSystem {
    /// Core HNSW index
    pub hnsw_index: HnswIndex,
    /// Index serialization handler
    pub serializer: IndexSerializer,
    /// Batch and incremental index builder
    pub index_builder: IndexBuilder,
    /// Memory management system
    pub memory_manager: PartialLoader,
    /// Write-ahead log for crash consistency
    pub wal_writer: WalWriter,
    /// System configuration
    pub config: SystemConfig,
    /// Runtime statistics
    pub stats: SystemStats,
}

/// System-wide configuration
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// HNSW configuration
    pub hnsw_config: HnswConfig,
    /// Serialization configuration
    pub serialization_config: SerializationConfig,
    /// Build configuration
    pub build_config: BuildConfig,
    /// Memory budget
    pub memory_budget: MemoryBudget,
    /// WAL configuration
    pub wal_config: WalConfig,
    /// Performance tuning parameters
    pub performance: PerformanceConfig,
}

/// Performance tuning configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Enable SIMD optimizations
    pub use_simd: bool,
    /// Batch size for operations
    pub batch_size: usize,
    /// Parallel processing threads (0 = auto-detect)
    pub num_threads: usize,
    /// Cache line prefetch distance
    pub prefetch_distance: usize,
    /// Memory access pattern optimization
    pub optimize_memory_layout: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            use_simd: true,
            batch_size: 1000,
            num_threads: 0,
            prefetch_distance: 64,
            optimize_memory_layout: true,
        }
    }
}

/// WAL configuration
#[derive(Debug, Clone)]
pub struct WalConfig {
    /// WAL file path
    pub wal_path: [u8; 256],
    /// Sync frequency (operations)
    pub sync_frequency: u32,
    /// Maximum WAL size before checkpoint
    pub max_wal_size: u64,
    /// Enable compression
    pub compress: bool,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            wal_path: [0; 256],
            sync_frequency: 1000,
            max_wal_size: 128 * 1024 * 1024, // 128MB
            compress: false,
        }
    }
}

/// System-wide statistics
#[derive(Debug, Clone, Copy)]
pub struct SystemStats {
    /// HNSW index stats
    pub index_stats: IndexStats,
    /// Build statistics
    pub build_stats: BuildStats,
    /// Memory usage
    pub memory_usage: MemoryUsage,
    /// WAL statistics
    pub wal_stats: WalStats,
    /// Performance metrics
    pub performance: PerformanceStats,
}

/// Performance statistics
#[derive(Debug, Clone, Copy)]
pub struct PerformanceStats {
    /// Total operations processed
    pub total_operations: u64,
    /// Average operation latency (microseconds)
    pub avg_latency_us: f32,
    /// Peak memory usage (bytes)
    pub peak_memory_bytes: u64,
    /// Cache hit rate (0.0-1.0)
    pub cache_hit_rate: f32,
    /// SIMD usage percentage
    pub simd_usage_percent: f32,
    /// Throughput (operations per second)
    pub throughput_ops: f32,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            total_operations: 0,
            avg_latency_us: 0.0,
            peak_memory_bytes: 0,
            cache_hit_rate: 0.0,
            simd_usage_percent: 0.0,
            throughput_ops: 0.0,
        }
    }
}

impl IntegratedAnnsSystem {
    /// Create a new integrated ANNS system
    pub fn new(config: SystemConfig) -> Result<Self, AnnsError> {
        // Initialize HNSW index
        let hnsw_index = HnswIndex::new(config.hnsw_config.clone())?;
        
        // Initialize serialization system
        let serializer = IndexSerializer::new(config.serialization_config.clone())?;
        
        // Initialize index builder
        let index_builder = IndexBuilder::new(config.build_config.clone())?;
        
        // Initialize memory manager
        let memory_manager = PartialLoader::new(config.memory_budget.clone())?;
        
        // Initialize WAL writer
        let wal_writer = WalWriter::new(config.wal_config.clone())?;
        
        let stats = SystemStats {
            index_stats: IndexStats::default(),
            build_stats: BuildStats::default(),
            memory_usage: MemoryUsage::default(),
            wal_stats: WalStats::default(),
            performance: PerformanceStats::default(),
        };

        Ok(Self {
            hnsw_index,
            serializer,
            index_builder,
            memory_manager,
            wal_writer,
            config,
            stats,
        })
    }

    /// Insert a vector with full system integration
    pub fn insert_vector(&mut self, vector_id: u64, vector_data: &[f32]) -> Result<(), AnnsError> {
        // Start transaction
        let mut transaction = self.wal_writer.begin_transaction()?;
        
        // Record operation in WAL
        transaction.log_insert(vector_id, vector_data)?;
        
        // Insert into HNSW index
        self.hnsw_index.insert(vector_id, vector_data)?;
        
        // Update memory management
        self.memory_manager.register_vector(vector_id, vector_data.len() * 4)?;
        
        // Commit transaction
        transaction.commit()?;
        
        // Update statistics
        self.stats.performance.total_operations += 1;
        self.stats.index_stats.vector_count += 1;

        Ok(())
    }

    /// Search with full system integration
    pub fn search_knn(&mut self, query: &[f32], k: usize) -> Result<Vec<SearchResult>, AnnsError> {
        // Use memory manager to ensure query data is accessible
        self.memory_manager.ensure_loaded_for_search()?;
        
        // Perform search using HNSW
        let search_params = SearchParams {
            k,
            ef: self.config.hnsw_config.ef_search,
            use_simd: self.config.performance.use_simd,
        };
        
        let results = self.hnsw_index.search(query, &search_params)?;
        
        // Update performance statistics
        self.stats.performance.total_operations += 1;
        
        Ok(results)
    }

    /// Batch insert multiple vectors
    pub fn batch_insert(&mut self, vectors: &[(u64, &[f32])]) -> Result<(), AnnsError> {
        // Start batch transaction
        let mut transaction = self.wal_writer.begin_transaction()?;
        
        // Use batch builder for efficient insertion
        let mut batch_builder = self.index_builder.create_batch_builder()?;
        
        for (vector_id, vector_data) in vectors {
            // Log to WAL
            transaction.log_insert(*vector_id, vector_data)?;
            
            // Add to batch
            batch_builder.add_vector(*vector_id, vector_data)?;
        }
        
        // Build batch and integrate
        let batch_result = batch_builder.build()?;
        self.hnsw_index.integrate_batch(batch_result)?;
        
        // Commit transaction
        transaction.commit()?;
        
        // Update statistics
        self.stats.performance.total_operations += vectors.len() as u64;
        self.stats.index_stats.vector_count += vectors.len() as u64;

        Ok(())
    }

    /// Save index to disk with full system coordination
    pub fn save_to_disk(&mut self, path: &[u8]) -> Result<(), AnnsError> {
        // Checkpoint WAL first
        self.wal_writer.checkpoint()?;
        
        // Serialize index
        let serialized_data = self.serializer.serialize(&self.hnsw_index)?;
        
        // Write to disk using ondisk layer
        // TODO: Integrate with VexFS ondisk module
        
        // Update metadata
        self.stats.index_stats.last_save_timestamp = 0; // TODO: get kernel time
        
        Ok(())
    }

    /// Load index from disk with full system coordination
    pub fn load_from_disk(&mut self, path: &[u8]) -> Result<(), AnnsError> {
        // Load serialized data
        // TODO: Integrate with VexFS ondisk module to read data
        let serialized_data = &[]; // Placeholder
        
        // Deserialize index
        self.hnsw_index = self.serializer.deserialize(serialized_data)?;
        
        // Replay WAL for crash recovery
        let wal_data = self.wal_writer.read_uncommitted_entries()?;
        self.replay_wal_entries(&wal_data)?;
        
        // Initialize memory manager with loaded index
        self.memory_manager.initialize_from_index(&self.hnsw_index)?;
        
        Ok(())
    }

    /// Recover from crash using WAL replay
    pub fn recover_from_crash(&mut self) -> Result<(), AnnsError> {
        // Read WAL entries
        let wal_entries = self.wal_writer.read_all_entries()?;
        
        // Find last consistent checkpoint
        let checkpoint_pos = self.find_last_checkpoint(&wal_entries)?;
        
        // Replay entries from checkpoint
        self.replay_wal_entries(&wal_entries[checkpoint_pos..])?;
        
        // Verify index consistency
        self.verify_index_consistency()?;
        
        Ok(())
    }

    /// Optimize index performance
    pub fn optimize(&mut self) -> Result<(), AnnsError> {
        // Rebuild index with optimal parameters
        let rebuild_config = self.calculate_optimal_rebuild_config()?;
        
        // Create optimized index
        let mut optimized_index = HnswIndex::new(rebuild_config)?;
        
        // Transfer all vectors to optimized index
        self.transfer_vectors_to_optimized(&mut optimized_index)?;
        
        // Replace current index
        self.hnsw_index = optimized_index;
        
        // Update statistics
        self.stats.build_stats.optimization_count += 1;
        
        Ok(())
    }

    /// Get comprehensive system statistics
    pub fn get_system_stats(&self) -> SystemStats {
        self.stats
    }

    /// Update system configuration
    pub fn update_config(&mut self, new_config: SystemConfig) -> Result<(), AnnsError> {
        // Validate configuration compatibility
        if new_config.hnsw_config.dimensions != self.config.hnsw_config.dimensions {
            return Err(AnnsError::InvalidParameters);
        }
        
        // Apply new configuration
        self.config = new_config;
        
        // Update component configurations
        self.hnsw_index.update_config(self.config.hnsw_config.clone())?;
        self.memory_manager.update_budget(self.config.memory_budget.clone())?;
        
        Ok(())
    }

    // Private helper methods
    
    fn replay_wal_entries(&mut self, entries: &[u8]) -> Result<(), AnnsError> {
        // TODO: Implement WAL replay logic
        Ok(())
    }
    
    fn find_last_checkpoint(&self, entries: &[u8]) -> Result<usize, AnnsError> {
        // TODO: Find last checkpoint in WAL entries
        Ok(0)
    }
    
    fn verify_index_consistency(&self) -> Result<(), AnnsError> {
        // TODO: Verify index is in consistent state
        Ok(())
    }
    
    fn calculate_optimal_rebuild_config(&self) -> Result<HnswConfig, AnnsError> {
        // TODO: Calculate optimal configuration based on current stats
        Ok(self.config.hnsw_config.clone())
    }
    
    fn transfer_vectors_to_optimized(&mut self, target_index: &mut HnswIndex) -> Result<(), AnnsError> {
        // TODO: Transfer all vectors to new optimized index
        Ok(())
    }
}

/// Factory for creating pre-configured ANNS systems
pub struct AnnsSystemFactory;

impl AnnsSystemFactory {
    /// Create system optimized for high-throughput batch operations
    pub fn create_batch_optimized(dimensions: u32, memory_budget_mb: u32) -> Result<IntegratedAnnsSystem, AnnsError> {
        let config = SystemConfig {
            hnsw_config: HnswConfig {
                dimensions,
                max_connections: 32,
                max_connections_layer0: 64,
                ef_construction: 400,
                ef_search: 100,
                level_multiplier: 1.0 / 2.0f32.ln(),
                distance_metric: DistanceMetric::Euclidean,
            },
            serialization_config: SerializationConfig {
                format: IndexFormat::Binary,
                compression_level: 6,
                enable_mmap: true,
                chunk_size: 64 * 1024,
            },
            build_config: BuildConfig {
                batch_size: 10000,
                enable_parallel: true,
                num_threads: 0, // auto-detect
                memory_limit: memory_budget_mb * 1024 * 1024,
            },
            memory_budget: MemoryBudget {
                total_bytes: memory_budget_mb * 1024 * 1024,
                cache_ratio: 0.7,
                preload_ratio: 0.2,
            },
            wal_config: WalConfig {
                wal_path: [0; 256], // TODO: set proper path
                sync_frequency: 10000,
                max_wal_size: 256 * 1024 * 1024,
                compress: true,
            },
            performance: PerformanceConfig {
                use_simd: true,
                batch_size: 10000,
                num_threads: 0,
                prefetch_distance: 128,
                optimize_memory_layout: true,
            },
        };
        
        IntegratedAnnsSystem::new(config)
    }
    
    /// Create system optimized for low-latency real-time searches
    pub fn create_realtime_optimized(dimensions: u32, memory_budget_mb: u32) -> Result<IntegratedAnnsSystem, AnnsError> {
        let config = SystemConfig {
            hnsw_config: HnswConfig {
                dimensions,
                max_connections: 16,
                max_connections_layer0: 32,
                ef_construction: 200,
                ef_search: 32,
                level_multiplier: 1.0 / 2.0f32.ln(),
                distance_metric: DistanceMetric::Euclidean,
            },
            serialization_config: SerializationConfig {
                format: IndexFormat::Binary,
                compression_level: 1, // Low compression for speed
                enable_mmap: true,
                chunk_size: 4 * 1024, // Smaller chunks for faster access
            },
            build_config: BuildConfig {
                batch_size: 100,
                enable_parallel: false, // Single-threaded for consistency
                num_threads: 1,
                memory_limit: memory_budget_mb * 1024 * 1024,
            },
            memory_budget: MemoryBudget {
                total_bytes: memory_budget_mb * 1024 * 1024,
                cache_ratio: 0.9, // High cache ratio
                preload_ratio: 0.05,
            },
            wal_config: WalConfig {
                wal_path: [0; 256],
                sync_frequency: 1, // Immediate sync
                max_wal_size: 32 * 1024 * 1024,
                compress: false,
            },
            performance: PerformanceConfig {
                use_simd: true,
                batch_size: 1,
                num_threads: 1,
                prefetch_distance: 32,
                optimize_memory_layout: true,
            },
        };
        
        IntegratedAnnsSystem::new(config)
    }
    
    /// Create system optimized for memory-constrained environments
    pub fn create_memory_optimized(dimensions: u32, memory_budget_mb: u32) -> Result<IntegratedAnnsSystem, AnnsError> {
        let config = SystemConfig {
            hnsw_config: HnswConfig {
                dimensions,
                max_connections: 8,
                max_connections_layer0: 16,
                ef_construction: 100,
                ef_search: 16,
                level_multiplier: 1.0 / 2.0f32.ln(),
                distance_metric: DistanceMetric::Euclidean,
            },
            serialization_config: SerializationConfig {
                format: IndexFormat::Compressed,
                compression_level: 9, // Maximum compression
                enable_mmap: true,
                chunk_size: 1024,
            },
            build_config: BuildConfig {
                batch_size: 100,
                enable_parallel: false,
                num_threads: 1,
                memory_limit: memory_budget_mb * 1024 * 1024 / 2, // Conservative limit
            },
            memory_budget: MemoryBudget {
                total_bytes: memory_budget_mb * 1024 * 1024,
                cache_ratio: 0.3, // Low cache ratio
                preload_ratio: 0.1,
            },
            wal_config: WalConfig {
                wal_path: [0; 256],
                sync_frequency: 100,
                max_wal_size: 16 * 1024 * 1024,
                compress: true,
            },
            performance: PerformanceConfig {
                use_simd: false, // Disable for memory savings
                batch_size: 10,
                num_threads: 1,
                prefetch_distance: 8,
                optimize_memory_layout: false,
            },
        };
        
        IntegratedAnnsSystem::new(config)
    }
}