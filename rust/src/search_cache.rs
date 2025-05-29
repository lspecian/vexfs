//! Advanced Search Result Caching for VexFS Vector Search
//!
//! This module implements a comprehensive enterprise-grade caching system for vector search results,
//! providing intelligent cache management, advanced caching strategies, distributed caching support,
//! cache warming, compression, persistence, and seamless integration with monitoring and analytics.

extern crate alloc;
use alloc::{vec::Vec, collections::BTreeMap, string::String, format, sync::Arc};
use core::{hash::{Hash, Hasher}, mem, cmp::Ordering as CmpOrdering};

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::fs_core::operations::OperationContext;
use crate::fs_core::enhanced_operation_context::{EnhancedOperationContext, OperationType, MemoryAllocationType};
use crate::vector_search::{SearchQuery, SearchOptions};
use crate::result_scoring::ScoredResult;
use crate::anns::DistanceMetric;
use crate::knn_search::MetadataFilter;
use crate::query_planner::{QueryCharacteristics, IndexRecommendation, QueryComplexity};
use crate::query_monitor::{QueryPerformanceMonitor, CacheEffectiveness, MonitoringConfig};

/// Maximum cache size in bytes (128MB default for enhanced caching)
pub const DEFAULT_MAX_CACHE_SIZE: usize = 128 * 1024 * 1024;

/// Maximum number of cache entries
pub const DEFAULT_MAX_CACHE_ENTRIES: usize = 50000;

/// Default cache entry TTL in seconds
pub const DEFAULT_CACHE_TTL_SECONDS: u64 = 600; // 10 minutes

/// Default cache warming batch size
pub const DEFAULT_CACHE_WARMING_BATCH_SIZE: usize = 100;

/// Default cache compression threshold (bytes)
pub const DEFAULT_COMPRESSION_THRESHOLD: usize = 8192; // 8KB

/// Default cache persistence interval (seconds)
pub const DEFAULT_PERSISTENCE_INTERVAL_SECONDS: u64 = 300; // 5 minutes

/// Default distributed cache shard count
pub const DEFAULT_SHARD_COUNT: usize = 16;

/// Default cache preload similarity threshold
pub const DEFAULT_PRELOAD_SIMILARITY_THRESHOLD: f32 = 0.85;

/// Cache key for search results
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CacheKey {
    /// Query vector hash (first 8 bytes of hash)
    vector_hash: u64,
    /// Number of results requested
    k: usize,
    /// Distance metric
    metric: DistanceMetric,
    /// Approximation flag
    approximate: bool,
    /// Expansion factor (quantized to avoid float precision issues)
    expansion_factor_q: u32,
    /// Filter hash (if present)
    filter_hash: Option<u64>,
    /// SIMD usage flag
    use_simd: bool,
    /// Exact distances flag
    exact_distances: bool,
}

impl CacheKey {
    /// Create cache key from search query
    pub fn from_query(query: &SearchQuery) -> Self {
        let vector_hash = Self::hash_vector(&query.vector);
        let filter_hash = query.filter.as_ref().map(|f| Self::hash_filter(f));
        let expansion_factor_q = (query.expansion_factor * 1000.0) as u32; // Quantize to avoid float issues
        
        Self {
            vector_hash,
            k: query.k,
            metric: query.metric,
            approximate: query.approximate,
            expansion_factor_q,
            filter_hash,
            use_simd: query.use_simd,
            exact_distances: query.exact_distances,
        }
    }
    
    /// Hash vector data for cache key
    fn hash_vector(vector: &[f32]) -> u64 {
        let mut hasher = SimpleHasher::new();
        
        // Hash vector length first
        vector.len().hash(&mut hasher);
        
        // Hash vector components (quantized to avoid float precision issues)
        for &value in vector {
            let quantized = (value * 10000.0) as i32; // Quantize to 4 decimal places
            quantized.hash(&mut hasher);
        }
        
        hasher.finish()
    }
    
    /// Hash metadata filter for cache key
    fn hash_filter(filter: &MetadataFilter) -> u64 {
        let mut hasher = SimpleHasher::new();
        
        // Hash all filter components
        filter.min_file_size.hash(&mut hasher);
        filter.max_file_size.hash(&mut hasher);
        filter.min_created_timestamp.hash(&mut hasher);
        filter.max_created_timestamp.hash(&mut hasher);
        filter.required_dimensions.hash(&mut hasher);
        filter.required_data_type.hash(&mut hasher);
        filter.max_distance.map(|d| (d * 10000.0) as i32).hash(&mut hasher);
        
        hasher.finish()
    }
}

/// Simple hasher implementation for kernel compatibility
struct SimpleHasher {
    state: u64,
}

impl SimpleHasher {
    fn new() -> Self {
        Self { state: 0xcbf29ce484222325 } // FNV offset basis
    }
}

impl Hasher for SimpleHasher {
    fn finish(&self) -> u64 {
        self.state
    }
    
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= byte as u64;
            self.state = self.state.wrapping_mul(0x100000001b3); // FNV prime
        }
    }
}

/// Enhanced cached search result entry with advanced features
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Cache key
    key: CacheKey,
    /// Cached search results (potentially compressed)
    results: CachedResults,
    /// Entry creation timestamp (microseconds)
    created_at: u64,
    /// Entry last access timestamp (microseconds)
    last_accessed: u64,
    /// Entry size in bytes (uncompressed)
    size_bytes: usize,
    /// Compressed size in bytes (if compressed)
    compressed_size_bytes: Option<usize>,
    /// Access count for statistics
    access_count: u64,
    /// Access frequency (accesses per hour)
    access_frequency: f32,
    /// Query characteristics when cached
    query_characteristics: QueryCharacteristics,
    /// Index recommendation when cached
    index_recommendation: IndexRecommendation,
    /// Cache entry priority for eviction
    priority: CachePriority,
    /// Entry tags for categorization
    tags: BTreeMap<String, String>,
    /// Compression algorithm used (if any)
    compression_algorithm: Option<CompressionAlgorithm>,
    /// Entry validation checksum
    checksum: u64,
    /// Shard ID for distributed caching
    shard_id: Option<u16>,
    /// Persistence status
    persistence_status: PersistenceStatus,
    /// Related cache keys for invalidation
    related_keys: Vec<CacheKey>,
}

/// Cached results with compression support
#[derive(Debug, Clone)]
pub enum CachedResults {
    /// Uncompressed results
    Uncompressed(Vec<ScoredResult>),
    /// Compressed results with algorithm info
    Compressed {
        data: Vec<u8>,
        algorithm: CompressionAlgorithm,
        original_size: usize,
    },
}

/// Cache entry priority for intelligent eviction
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum CachePriority {
    /// Low priority (rarely accessed)
    Low = 1,
    /// Normal priority (default)
    Normal = 2,
    /// High priority (frequently accessed)
    High = 3,
    /// Critical priority (system queries)
    Critical = 4,
    /// Pinned (never evict)
    Pinned = 5,
}

/// Compression algorithms supported
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// LZ4 compression (fast)
    Lz4,
    /// Zstd compression (balanced)
    Zstd,
    /// Custom vector compression
    VectorQuantization,
}

/// Persistence status for cache entries
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PersistenceStatus {
    /// Not persisted
    NotPersisted,
    /// Pending persistence
    PendingPersistence,
    /// Successfully persisted
    Persisted,
    /// Persistence failed
    PersistenceFailed,
}

impl CacheEntry {
    /// Create new cache entry with enhanced features
    pub fn new(
        key: CacheKey,
        results: Vec<ScoredResult>,
        timestamp: u64,
        query_characteristics: QueryCharacteristics,
        index_recommendation: IndexRecommendation,
    ) -> Self {
        let size_bytes = Self::calculate_size(&results);
        let checksum = Self::calculate_checksum(&results);
        
        Self {
            key,
            results: CachedResults::Uncompressed(results),
            created_at: timestamp,
            last_accessed: timestamp,
            size_bytes,
            compressed_size_bytes: None,
            access_count: 1,
            access_frequency: 0.0,
            query_characteristics,
            index_recommendation,
            priority: CachePriority::Normal,
            tags: BTreeMap::new(),
            compression_algorithm: None,
            checksum,
            shard_id: None,
            persistence_status: PersistenceStatus::NotPersisted,
            related_keys: Vec::new(),
        }
    }
    
    /// Create new cache entry with compression
    pub fn new_compressed(
        key: CacheKey,
        results: Vec<ScoredResult>,
        timestamp: u64,
        query_characteristics: QueryCharacteristics,
        index_recommendation: IndexRecommendation,
        compression_algorithm: CompressionAlgorithm,
    ) -> VexfsResult<Self> {
        let original_size = Self::calculate_size(&results);
        let checksum = Self::calculate_checksum(&results);
        
        let (cached_results, compressed_size) = match compression_algorithm {
            CompressionAlgorithm::None => {
                (CachedResults::Uncompressed(results), None)
            }
            _ => {
                let compressed_data = Self::compress_results(&results, compression_algorithm)?;
                let compressed_size = compressed_data.len();
                (
                    CachedResults::Compressed {
                        data: compressed_data,
                        algorithm: compression_algorithm,
                        original_size,
                    },
                    Some(compressed_size)
                )
            }
        };
        
        Ok(Self {
            key,
            results: cached_results,
            created_at: timestamp,
            last_accessed: timestamp,
            size_bytes: original_size,
            compressed_size_bytes: compressed_size,
            access_count: 1,
            access_frequency: 0.0,
            query_characteristics,
            index_recommendation,
            priority: CachePriority::Normal,
            tags: BTreeMap::new(),
            compression_algorithm: Some(compression_algorithm),
            checksum,
            shard_id: None,
            persistence_status: PersistenceStatus::NotPersisted,
            related_keys: Vec::new(),
        })
    }
    
    /// Calculate entry size in bytes
    fn calculate_size(results: &[ScoredResult]) -> usize {
        mem::size_of::<CacheEntry>() +
        results.len() * mem::size_of::<ScoredResult>() +
        2048 // Enhanced overhead estimate for new fields
    }
    
    /// Calculate checksum for results validation
    fn calculate_checksum(results: &[ScoredResult]) -> u64 {
        let mut hasher = SimpleHasher::new();
        results.len().hash(&mut hasher);
        for result in results {
            result.result.file_inode.hash(&mut hasher);
            ((result.score * 10000.0) as u64).hash(&mut hasher);
        }
        hasher.finish()
    }
    
    /// Compress results using specified algorithm
    fn compress_results(results: &[ScoredResult], algorithm: CompressionAlgorithm) -> VexfsResult<Vec<u8>> {
        match algorithm {
            CompressionAlgorithm::None => {
                Err(VexfsError::InvalidOperation("Cannot compress with None algorithm".to_string()))
            }
            CompressionAlgorithm::Lz4 => {
                // Placeholder for LZ4 compression
                let serialized = Self::serialize_results(results)?;
                Ok(serialized) // For now, return uncompressed
            }
            CompressionAlgorithm::Zstd => {
                // Placeholder for Zstd compression
                let serialized = Self::serialize_results(results)?;
                Ok(serialized) // For now, return uncompressed
            }
            CompressionAlgorithm::VectorQuantization => {
                // Placeholder for vector quantization
                let serialized = Self::serialize_results(results)?;
                Ok(serialized) // For now, return uncompressed
            }
        }
    }
    
    /// Serialize results for compression/persistence
    fn serialize_results(results: &[ScoredResult]) -> VexfsResult<Vec<u8>> {
        // Simple serialization - in practice, would use a proper serialization format
        let mut data = Vec::new();
        
        // Write count
        let count = results.len() as u32;
        data.extend_from_slice(&count.to_le_bytes());
        
        // Write each result
        for result in results {
            data.extend_from_slice(&result.result.file_inode.to_le_bytes());
            data.extend_from_slice(&result.score.to_le_bytes());
        }
        
        Ok(data)
    }
    
    /// Deserialize results from compressed/persisted data
    fn deserialize_results(data: &[u8]) -> VexfsResult<Vec<ScoredResult>> {
        if data.len() < 4 {
            return Err(VexfsError::InvalidData("Insufficient data for deserialization".to_string()));
        }
        
        let mut offset = 0;
        
        // Read count
        let count_bytes: [u8; 4] = data[offset..offset + 4].try_into()
            .map_err(|_| VexfsError::InvalidData("Invalid count data".to_string()))?;
        let count = u32::from_le_bytes(count_bytes) as usize;
        offset += 4;
        
        let mut results = Vec::with_capacity(count);
        
        // Read each result
        for _ in 0..count {
            if offset + 12 > data.len() {
                return Err(VexfsError::InvalidData("Insufficient data for result".to_string()));
            }
            
            let inode_bytes: [u8; 8] = data[offset..offset + 8].try_into()
                .map_err(|_| VexfsError::InvalidData("Invalid inode data".to_string()))?;
            let inode = u64::from_le_bytes(inode_bytes);
            offset += 8;
            
            let score_bytes: [u8; 4] = data[offset..offset + 4].try_into()
                .map_err(|_| VexfsError::InvalidData("Invalid score data".to_string()))?;
            let score = f32::from_le_bytes(score_bytes);
            offset += 4;
            
            // Create a minimal KnnResult for deserialization
            let knn_result = crate::knn_search::KnnResult {
                vector_id: inode,
                file_inode: inode,
                distance: score,
                dimensions: 128, // Default
                data_type: crate::vector_storage::VectorDataType::Float32,
                file_size: 0,
                created_timestamp: 0,
                modified_timestamp: 0,
            };
            
            results.push(ScoredResult {
                result: knn_result,
                score,
                confidence: 0.8, // Default
                rank: 0,
                normalized_score: score,
                quality_flags: 0,
            });
        }
        
        Ok(results)
    }
    
    /// Get results, decompressing if necessary
    pub fn get_results(&self) -> VexfsResult<Vec<ScoredResult>> {
        match &self.results {
            CachedResults::Uncompressed(results) => Ok(results.clone()),
            CachedResults::Compressed { data, algorithm, .. } => {
                match algorithm {
                    CompressionAlgorithm::None => {
                        Err(VexfsError::InvalidOperation("Invalid compression state".to_string()))
                    }
                    _ => {
                        // For now, assume data is serialized but not actually compressed
                        Self::deserialize_results(data)
                    }
                }
            }
        }
    }
    
    /// Get effective size (compressed if available, otherwise uncompressed)
    pub fn get_effective_size(&self) -> usize {
        self.compressed_size_bytes.unwrap_or(self.size_bytes)
    }
    
    /// Update access frequency based on time since last access
    pub fn update_access_frequency(&mut self, current_time: u64) {
        let time_since_creation = current_time - self.created_at;
        if time_since_creation > 0 {
            let hours = time_since_creation as f32 / 3_600_000_000.0; // Convert microseconds to hours
            self.access_frequency = self.access_count as f32 / hours.max(1.0);
        }
    }
    
    /// Check if entry is expired
    pub fn is_expired(&self, current_time: u64, ttl_seconds: u64) -> bool {
        let ttl_us = ttl_seconds * 1_000_000;
        current_time - self.created_at > ttl_us
    }
    
    /// Update access statistics
    pub fn mark_accessed(&mut self, timestamp: u64) {
        self.last_accessed = timestamp;
        self.access_count += 1;
        self.update_access_frequency(timestamp);
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total cache insertions
    pub insertions: u64,
    /// Total cache evictions
    pub evictions: u64,
    /// Current cache size in bytes
    pub current_size_bytes: usize,
    /// Current number of entries
    pub current_entries: usize,
    /// Peak cache size in bytes
    pub peak_size_bytes: usize,
    /// Peak number of entries
    pub peak_entries: usize,
    /// Average entry size
    pub avg_entry_size_bytes: usize,
    /// Cache hit rate (0.0 - 1.0)
    pub hit_rate: f32,
    /// Memory efficiency (useful data / total memory)
    pub memory_efficiency: f32,
}

impl CacheStatistics {
    /// Update hit rate calculation
    pub fn update_hit_rate(&mut self) {
        let total_requests = self.hits + self.misses;
        if total_requests > 0 {
            self.hit_rate = self.hits as f32 / total_requests as f32;
        }
    }
    
    /// Update memory efficiency calculation
    pub fn update_memory_efficiency(&mut self) {
        if self.current_size_bytes > 0 && self.current_entries > 0 {
            self.avg_entry_size_bytes = self.current_size_bytes / self.current_entries;
            // Estimate useful data vs overhead
            let estimated_useful_data = self.current_entries * mem::size_of::<ScoredResult>() * 10; // Estimate
            self.memory_efficiency = estimated_useful_data as f32 / self.current_size_bytes as f32;
        }
    }
}

/// Cache operation metadata for OperationContext integration
#[derive(Debug, Clone)]
struct CacheOperationMetadata {
    /// Operation ID
    operation_id: u64,
    /// Operation start time (microseconds)
    start_time_us: u64,
    /// Cache operation type
    operation_type: CacheOperationType,
    /// Cache key involved
    cache_key: CacheKey,
    /// Estimated memory impact
    memory_impact: isize, // Positive for additions, negative for removals
    /// Operation status
    status: CacheOperationStatus,
    /// User ID for permission tracking
    user_id: u32,
}

/// Cache operation types
#[derive(Debug, Clone, Copy, PartialEq)]
enum CacheOperationType {
    /// Cache lookup operation
    Lookup,
    /// Cache insertion operation
    Insert,
    /// Cache eviction operation
    Evict,
    /// Cache warming operation
    Warm,
    /// Cache cleanup operation
    Cleanup,
}

/// Cache operation status
#[derive(Debug, Clone, Copy, PartialEq)]
enum CacheOperationStatus {
    /// Operation is starting
    Starting,
    /// Operation is processing
    Processing,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
}

/// Cache eviction policy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// Time-based expiration
    TTL,
    /// Hybrid LRU + TTL
    Hybrid,
}

/// Multi-level search result cache with comprehensive management
pub struct SearchResultCache {
    /// Main cache storage (LRU ordered)
    cache: BTreeMap<CacheKey, CacheEntry>,
    /// LRU access order tracking
    lru_order: Vec<CacheKey>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache statistics
    stats: CacheStatistics,
    /// Active cache operations for lifecycle management
    active_operations: BTreeMap<u64, CacheOperationMetadata>,
    /// Operation counter for unique operation IDs
    operation_counter: u64,
    /// Cache warming candidates
    warming_candidates: Vec<(CacheKey, QueryCharacteristics, IndexRecommendation)>,
}

impl SearchResultCache {
    /// Create new search result cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: BTreeMap::new(),
            lru_order: Vec::new(),
            config,
            stats: CacheStatistics::default(),
            active_operations: BTreeMap::new(),
            operation_counter: 0,
            warming_candidates: Vec::new(),
        }
    }
    
    /// Lookup cached search results with OperationContext integration
    pub fn lookup(
        &mut self,
        context: &mut OperationContext,
        key: &CacheKey,
    ) -> VexfsResult<Option<Vec<ScoredResult>>> {
        let start_time = self.get_current_time_us();
        
        // Start cache operation tracking
        let operation_id = self.start_cache_operation(
            context,
            CacheOperationType::Lookup,
            key.clone(),
            0,
            start_time,
        )?;
        
        // Check if entry exists and is not expired
        let (result, should_update_lru) = if let Some(entry) = self.cache.get(key) {
            if entry.is_expired(start_time, self.config.ttl_seconds) {
                // Entry expired, remove it
                self.cache.remove(key);
                self.remove_from_lru_order(key);
                self.stats.evictions += 1;
                self.stats.current_entries = self.cache.len();
                self.stats.current_size_bytes = self.calculate_total_size();
                
                self.stats.misses += 1;
                (None, false)
            } else {
                // Cache hit - get results (decompressing if necessary)
                match entry.get_results() {
                    Ok(results) => {
                        self.stats.hits += 1;
                        (Some(results), true)
                    }
                    Err(_) => {
                        // Failed to decompress, treat as miss
                        self.stats.misses += 1;
                        (None, false)
                    }
                }
            }
        } else {
            // Cache miss
            self.stats.misses += 1;
            (None, false)
        };
        
        // Update entry access time and LRU order if needed (separate from the lookup)
        if should_update_lru {
            if let Some(entry_mut) = self.cache.get_mut(key) {
                entry_mut.mark_accessed(start_time);
            }
            self.update_lru_order(key);
        }
        
        let result = result;
        
        // Update statistics
        self.stats.update_hit_rate();
        
        // Complete operation
        let end_time = self.get_current_time_us();
        self.complete_cache_operation(operation_id, end_time - start_time)?;
        
        Ok(result)
    }
    
    /// Insert search results into cache with OperationContext integration
    pub fn insert(
        &mut self,
        context: &mut OperationContext,
        key: CacheKey,
        results: Vec<ScoredResult>,
        query_characteristics: QueryCharacteristics,
        index_recommendation: IndexRecommendation,
    ) -> VexfsResult<()> {
        let start_time = self.get_current_time_us();
        
        // Calculate entry size
        let entry_size = CacheEntry::calculate_size(&results);
        
        // Start cache operation tracking
        let operation_id = self.start_cache_operation(
            context,
            CacheOperationType::Insert,
            key.clone(),
            entry_size as isize,
            start_time,
        )?;
        
        // Check if we should cache this entry
        if !self.should_cache(&query_characteristics, entry_size) {
            self.complete_cache_operation(operation_id, 0)?;
            return Ok(());
        }
        
        // Ensure we have space for the new entry
        self.ensure_cache_space(entry_size)?;
        
        // Create and insert cache entry
        let entry = CacheEntry::new(
            key.clone(),
            results,
            start_time,
            query_characteristics,
            index_recommendation,
        );
        
        // Remove existing entry if present
        if self.cache.contains_key(&key) {
            self.remove_from_lru_order(&key);
        }
        
        // Insert new entry
        self.cache.insert(key.clone(), entry);
        self.lru_order.push(key);
        
        // Update statistics
        self.stats.insertions += 1;
        self.stats.current_entries = self.cache.len();
        self.stats.current_size_bytes = self.calculate_total_size();
        
        if self.stats.current_size_bytes > self.stats.peak_size_bytes {
            self.stats.peak_size_bytes = self.stats.current_size_bytes;
        }
        if self.stats.current_entries > self.stats.peak_entries {
            self.stats.peak_entries = self.stats.current_entries;
        }
        
        self.stats.update_memory_efficiency();
        
        // Complete operation
        let end_time = self.get_current_time_us();
        self.complete_cache_operation(operation_id, end_time - start_time)?;
        
        Ok(())
    }
    
    /// Get cache statistics
    pub fn get_statistics(&self) -> &CacheStatistics {
        &self.stats
    }
    
    /// Clear entire cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru_order.clear();
        self.stats.current_entries = 0;
        self.stats.current_size_bytes = 0;
        self.warming_candidates.clear();
    }
    
    /// Get cache configuration
    pub fn get_config(&self) -> &CacheConfig {
        &self.config
    }
    
    /// Check if query should be cached
    fn should_cache(&self, characteristics: &QueryCharacteristics, entry_size: usize) -> bool {
        // Don't cache if entry is too large
        if entry_size > self.config.max_size_bytes / 10 {
            return false;
        }
        
        // Cache complex queries more aggressively
        match characteristics.complexity {
            crate::query_planner::QueryComplexity::Simple => characteristics.k >= 50,
            crate::query_planner::QueryComplexity::Moderate => characteristics.k >= 20,
            crate::query_planner::QueryComplexity::Complex => characteristics.k >= 10,
            crate::query_planner::QueryComplexity::HighlyComplex => true,
        }
    }
    
    /// Ensure cache has space for new entry
    fn ensure_cache_space(&mut self, required_size: usize) -> VexfsResult<()> {
        // Check size constraint
        while self.stats.current_size_bytes + required_size > self.config.max_size_bytes {
            if !self.evict_lru_entry()? {
                break; // No more entries to evict
            }
        }
        
        // Check entry count constraint
        while self.cache.len() >= self.config.max_entries {
            if !self.evict_lru_entry()? {
                break; // No more entries to evict
            }
        }
        
        Ok(())
    }
    
    /// Evict least recently used entry
    fn evict_lru_entry(&mut self) -> VexfsResult<bool> {
        if let Some(key) = self.lru_order.first().cloned() {
            self.cache.remove(&key);
            self.lru_order.remove(0);
            self.stats.evictions += 1;
            self.stats.current_entries = self.cache.len();
            self.stats.current_size_bytes = self.calculate_total_size();
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Update LRU order for accessed key
    fn update_lru_order(&mut self, key: &CacheKey) {
        // Remove key from current position
        self.remove_from_lru_order(key);
        // Add to end (most recently used)
        self.lru_order.push(key.clone());
    }
    
    /// Remove key from LRU order
    fn remove_from_lru_order(&mut self, key: &CacheKey) {
        if let Some(pos) = self.lru_order.iter().position(|k| k == key) {
            self.lru_order.remove(pos);
        }
    }
    
    /// Calculate total cache size
    fn calculate_total_size(&self) -> usize {
        self.cache.values().map(|entry| entry.size_bytes).sum()
    }
    
    /// Start cache operation tracking
    fn start_cache_operation(
        &mut self,
        context: &OperationContext,
        operation_type: CacheOperationType,
        cache_key: CacheKey,
        memory_impact: isize,
        start_time: u64,
    ) -> VexfsResult<u64> {
        self.operation_counter += 1;
        let operation_id = self.operation_counter;
        
        let metadata = CacheOperationMetadata {
            operation_id,
            start_time_us: start_time,
            operation_type,
            cache_key,
            memory_impact,
            status: CacheOperationStatus::Starting,
            user_id: context.user.uid,
        };
        
        self.active_operations.insert(operation_id, metadata);
        Ok(operation_id)
    }
    
    /// Complete cache operation
    fn complete_cache_operation(&mut self, operation_id: u64, _execution_time: u64) -> VexfsResult<()> {
        if let Some(mut metadata) = self.active_operations.remove(&operation_id) {
            metadata.status = CacheOperationStatus::Completed;
            Ok(())
        } else {
            Err(VexfsError::InvalidOperation("Cache operation not found".to_string()))
        }
    }
    
    /// Get current time in microseconds (placeholder)
    fn get_current_time_us(&self) -> u64 {
        1640995200_000_000 // Placeholder timestamp
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum cache size in bytes
    pub max_size_bytes: usize,
    /// Maximum number of entries
    pub max_entries: usize,
    /// Entry TTL in seconds
    pub ttl_seconds: u64,
    /// Eviction policy
    pub eviction_policy: EvictionPolicy,
    /// Enable cache warming
    pub enable_warming: bool,
    /// Cache warming threshold (cache when query complexity >= threshold)
    pub warming_complexity_threshold: f32,
    /// Enable cache compression
    pub enable_compression: bool,
    /// Memory pressure threshold (0.0 - 1.0)
    pub memory_pressure_threshold: f32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: DEFAULT_MAX_CACHE_SIZE,
            max_entries: DEFAULT_MAX_CACHE_ENTRIES,
            ttl_seconds: DEFAULT_CACHE_TTL_SECONDS,
            eviction_policy: EvictionPolicy::Hybrid,
            enable_warming: true,
            warming_complexity_threshold: 0.7,
            enable_compression: false,
            memory_pressure_threshold: 0.8,
        }
    }
}
/// Enhanced cache configuration with advanced features
#[derive(Debug, Clone)]
pub struct EnhancedCacheConfig {
    /// Base cache configuration
    pub base_config: CacheConfig,
    /// Enable distributed caching
    pub enable_distributed_caching: bool,
    /// Number of cache shards for distributed caching
    pub shard_count: usize,
    /// Enable cache persistence
    pub enable_persistence: bool,
    /// Cache persistence interval (seconds)
    pub persistence_interval_seconds: u64,
    /// Enable cache preloading
    pub enable_preloading: bool,
    /// Preload similarity threshold
    pub preload_similarity_threshold: f32,
    /// Cache warming batch size
    pub warming_batch_size: usize,
    /// Compression threshold (bytes)
    pub compression_threshold: usize,
    /// Enable cache analytics
    pub enable_analytics: bool,
    /// Enable cache invalidation tracking
    pub enable_invalidation_tracking: bool,
    /// Maximum cache age for warming (seconds)
    pub max_warming_age_seconds: u64,
}

impl Default for EnhancedCacheConfig {
    fn default() -> Self {
        Self {
            base_config: CacheConfig::default(),
            enable_distributed_caching: false,
            shard_count: DEFAULT_SHARD_COUNT,
            enable_persistence: false,
            persistence_interval_seconds: DEFAULT_PERSISTENCE_INTERVAL_SECONDS,
            enable_preloading: true,
            preload_similarity_threshold: DEFAULT_PRELOAD_SIMILARITY_THRESHOLD,
            warming_batch_size: DEFAULT_CACHE_WARMING_BATCH_SIZE,
            compression_threshold: DEFAULT_COMPRESSION_THRESHOLD,
            enable_analytics: true,
            enable_invalidation_tracking: true,
            max_warming_age_seconds: 3600, // 1 hour
        }
    }
}

/// Advanced cache analytics for monitoring and optimization
#[derive(Debug, Clone, Default)]
pub struct CacheAnalytics {
    /// Cache hit rate by query pattern
    pub hit_rate_by_pattern: BTreeMap<String, f32>,
    /// Cache miss reasons
    pub miss_reasons: BTreeMap<String, u64>,
    /// Compression effectiveness
    pub compression_stats: CompressionStats,
    /// Cache warming effectiveness
    pub warming_stats: WarmingStats,
    /// Eviction patterns
    pub eviction_patterns: EvictionPatterns,
    /// Performance impact metrics
    pub performance_impact: PerformanceImpact,
    /// Cache utilization over time
    pub utilization_history: Vec<UtilizationSnapshot>,
}

/// Compression statistics
#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    /// Total bytes before compression
    pub total_uncompressed_bytes: u64,
    /// Total bytes after compression
    pub total_compressed_bytes: u64,
    /// Compression ratio (compressed/uncompressed)
    pub compression_ratio: f32,
    /// Compression time (microseconds)
    pub total_compression_time_us: u64,
    /// Decompression time (microseconds)
    pub total_decompression_time_us: u64,
    /// Number of compressed entries
    pub compressed_entries: u64,
}

/// Cache warming statistics
#[derive(Debug, Clone, Default)]
pub struct WarmingStats {
    /// Total warming operations
    pub total_warming_operations: u64,
    /// Successful warming operations
    pub successful_warming_operations: u64,
    /// Failed warming operations
    pub failed_warming_operations: u64,
    /// Average warming time (microseconds)
    pub avg_warming_time_us: u64,
    /// Warming hit rate (warmed entries that were accessed)
    pub warming_hit_rate: f32,
    /// Warming effectiveness score
    pub warming_effectiveness: f32,
}

/// Eviction pattern analysis
#[derive(Debug, Clone, Default)]
pub struct EvictionPatterns {
    /// Evictions by priority level
    pub evictions_by_priority: BTreeMap<String, u64>,
    /// Evictions by access frequency
    pub evictions_by_frequency: BTreeMap<String, u64>,
    /// Evictions by age
    pub evictions_by_age: BTreeMap<String, u64>,
    /// Average time between creation and eviction
    pub avg_lifetime_seconds: f32,
}

/// Performance impact metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceImpact {
    /// Average cache lookup time (microseconds)
    pub avg_lookup_time_us: u64,
    /// Average cache insertion time (microseconds)
    pub avg_insertion_time_us: u64,
    /// Cache overhead percentage
    pub cache_overhead_percent: f32,
    /// Memory efficiency score
    pub memory_efficiency_score: f32,
    /// Query acceleration factor (speedup from caching)
    pub query_acceleration_factor: f32,
}

/// Cache utilization snapshot
#[derive(Debug, Clone)]
pub struct UtilizationSnapshot {
    /// Timestamp (microseconds)
    pub timestamp_us: u64,
    /// Memory utilization (0.0 - 1.0)
    pub memory_utilization: f32,
    /// Entry count utilization (0.0 - 1.0)
    pub entry_utilization: f32,
    /// Hit rate at this time
    pub hit_rate: f32,
    /// Active operations count
    pub active_operations: usize,
}

/// Cache invalidation tracking
#[derive(Debug, Clone)]
pub struct InvalidationTracker {
    /// Invalidation rules
    pub invalidation_rules: Vec<InvalidationRule>,
    /// Invalidation history
    pub invalidation_history: Vec<InvalidationEvent>,
    /// Related key mappings
    pub related_key_mappings: BTreeMap<CacheKey, Vec<CacheKey>>,
}

/// Cache invalidation rule
#[derive(Debug, Clone)]
pub struct InvalidationRule {
    /// Rule ID
    pub rule_id: u64,
    /// Rule type
    pub rule_type: InvalidationRuleType,
    /// Rule condition
    pub condition: String,
    /// Affected key patterns
    pub key_patterns: Vec<String>,
    /// Rule priority
    pub priority: u32,
}

/// Types of invalidation rules
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InvalidationRuleType {
    /// Time-based invalidation
    TimeBased,
    /// Data change invalidation
    DataChange,
    /// Dependency invalidation
    Dependency,
    /// Manual invalidation
    Manual,
    /// Pattern-based invalidation
    Pattern,
}

/// Cache invalidation event
#[derive(Debug, Clone)]
pub struct InvalidationEvent {
    /// Event timestamp
    pub timestamp_us: u64,
    /// Invalidation reason
    pub reason: InvalidationReason,
    /// Number of entries invalidated
    pub entries_invalidated: usize,
    /// Invalidation rule ID (if applicable)
    pub rule_id: Option<u64>,
}

/// Reasons for cache invalidation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InvalidationReason {
    /// TTL expiration
    TtlExpiration,
    /// Manual invalidation
    Manual,
    /// Data update
    DataUpdate,
    /// Memory pressure
    MemoryPressure,
    /// Dependency change
    DependencyChange,
    /// Pattern match
    PatternMatch,
}

/// Cache warming manager
#[derive(Debug)]
pub struct CacheWarmingManager {
    /// Warming candidates queue
    warming_queue: Vec<WarmingCandidate>,
    /// Warming statistics
    warming_stats: WarmingStats,
    /// Warming configuration
    warming_config: WarmingConfig,
    /// Active warming operations
    active_warming_operations: BTreeMap<u64, WarmingOperation>,
    /// Warming operation counter
    warming_operation_counter: u64,
}

/// Cache warming candidate
#[derive(Debug, Clone)]
pub struct WarmingCandidate {
    /// Cache key to warm
    pub cache_key: CacheKey,
    /// Query characteristics
    pub query_characteristics: QueryCharacteristics,
    /// Index recommendation
    pub index_recommendation: IndexRecommendation,
    /// Warming priority
    pub priority: f32,
    /// Predicted access probability
    pub access_probability: f32,
    /// Warming timestamp
    pub warming_timestamp_us: u64,
}

/// Cache warming configuration
#[derive(Debug, Clone)]
pub struct WarmingConfig {
    /// Enable proactive warming
    pub enable_proactive_warming: bool,
    /// Maximum warming operations per batch
    pub max_warming_batch_size: usize,
    /// Warming operation timeout (microseconds)
    pub warming_timeout_us: u64,
    /// Minimum access probability for warming
    pub min_access_probability: f32,
    /// Warming priority threshold
    pub priority_threshold: f32,
}

/// Active warming operation
#[derive(Debug, Clone)]
pub struct WarmingOperation {
    /// Operation ID
    pub operation_id: u64,
    /// Cache key being warmed
    pub cache_key: CacheKey,
    /// Operation start time
    pub start_time_us: u64,
    /// Operation status
    pub status: WarmingOperationStatus,
    /// Progress (0.0 - 1.0)
    pub progress: f32,
}

/// Warming operation status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WarmingOperationStatus {
    /// Operation is queued
    Queued,
    /// Operation is in progress
    InProgress,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
}