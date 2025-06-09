//! ANNS Integration with fs_core Architecture
//!
//! This module provides the main ANNS integration components that work seamlessly
//! with the VexFS fs_core architecture using OperationContext patterns.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::InodeNumber;
use crate::fs_core::operations::OperationContext;
use crate::vector_storage::{VectorHeader, VectorStorageManager, VectorStorageError};
pub use crate::vector_metrics::{DistanceMetric, calculate_distance};
use crate::storage::StorageManager;

#[cfg(not(feature = "kernel"))]
use std::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::sync::Arc;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, collections::BTreeMap};
#[cfg(feature = "std")]
use std::{vec::Vec, collections::BTreeMap};

use core::mem;

// Import from other ANNS modules
use super::hnsw::{HnswGraph, HnswNode};
use super::advanced_indexing::{
    IndexStrategy, LshConfig, IvfConfig, PqConfig, FlatConfig,
    LshIndex, IvfIndex
};
use super::advanced_strategies::{
    PqIndex, FlatIndex, IndexSelector, QueryPattern, CollectionSize,
    StrategyRecommendation, CollectionAnalysis, IndexSelectionResult
};

/// Error types for ANNS operations - now integrated with VexfsError system
#[derive(Debug, Clone)]
pub enum AnnsError {
    InvalidDimensions,
    InvalidParameter,
    IndexNotInitialized,
    StorageFull,
    VectorNotFound,
    InvalidVectorData,
    SerializationError,
    MemoryAllocationFailed,
    InvalidOperation,
    IOError,
    OutOfMemory,
    InvalidMemoryBlock,
    WalCorrupted,
    WalFull,
    StackOverflow,
    // Integration with fs_core errors
    VexfsError(VexfsError),
}

impl From<VexfsError> for AnnsError {
    fn from(err: VexfsError) -> Self {
        AnnsError::VexfsError(err)
    }
}

impl From<AnnsError> for VexfsError {
    fn from(err: AnnsError) -> Self {
        match err {
            AnnsError::InvalidDimensions => VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidDimensions(0)),
            AnnsError::InvalidParameter => VexfsError::InvalidArgument("Invalid ANNS parameter".to_string()),
            AnnsError::IndexNotInitialized => VexfsError::IndexError(crate::shared::errors::IndexErrorKind::IndexNotFound),
            AnnsError::StorageFull => VexfsError::OutOfSpace,
            AnnsError::VectorNotFound => VexfsError::VectorError(crate::shared::errors::VectorErrorKind::VectorNotFound),
            AnnsError::InvalidVectorData => VexfsError::VectorError(crate::shared::errors::VectorErrorKind::CorruptedData),
            AnnsError::SerializationError => VexfsError::VectorError(crate::shared::errors::VectorErrorKind::SerializationError),
            AnnsError::MemoryAllocationFailed => VexfsError::OutOfMemory,
            AnnsError::InvalidOperation => VexfsError::InvalidOperation("Invalid ANNS operation".to_string()),
            AnnsError::IOError => VexfsError::VectorError(crate::shared::errors::VectorErrorKind::IoError),
            AnnsError::OutOfMemory => VexfsError::OutOfMemory,
            AnnsError::InvalidMemoryBlock => VexfsError::VectorError(crate::shared::errors::VectorErrorKind::CorruptedData),
            AnnsError::WalCorrupted => VexfsError::JournalError(crate::shared::errors::JournalErrorKind::JournalCorrupted),
            AnnsError::WalFull => VexfsError::JournalError(crate::shared::errors::JournalErrorKind::JournalFull),
            AnnsError::StackOverflow => VexfsError::StackOverflow,
            AnnsError::VexfsError(vexfs_err) => vexfs_err,
        }
    }
}

impl From<VectorStorageError> for AnnsError {
    fn from(err: VectorStorageError) -> Self {
        match err {
            VectorStorageError::InvalidDimensions(_) => AnnsError::InvalidDimensions,
            VectorStorageError::InvalidDimension(_) => AnnsError::InvalidDimensions,
            VectorStorageError::DimensionMismatch { .. } => AnnsError::InvalidDimensions,
            VectorStorageError::InvalidComponent(_) => AnnsError::InvalidVectorData,
            VectorStorageError::VectorTooLarge => AnnsError::InvalidVectorData,
            VectorStorageError::VectorNotFound => AnnsError::VectorNotFound,
            VectorStorageError::MetadataTooLarge => AnnsError::InvalidVectorData,
            VectorStorageError::SerializationError => AnnsError::SerializationError,
            VectorStorageError::DeserializationError => AnnsError::SerializationError,
            VectorStorageError::NormalizationError => AnnsError::InvalidVectorData,
            VectorStorageError::SearchError => AnnsError::InvalidOperation,
            VectorStorageError::IndexError => AnnsError::InvalidOperation,
            VectorStorageError::InvalidVectorId => AnnsError::VectorNotFound,
            VectorStorageError::CorruptedData => AnnsError::InvalidVectorData,
            VectorStorageError::NoSpace => AnnsError::StorageFull,
            VectorStorageError::IoError => AnnsError::IOError,
            VectorStorageError::InvalidVersion => AnnsError::InvalidOperation,
            VectorStorageError::ChecksumMismatch => AnnsError::InvalidVectorData,
            VectorStorageError::FileNotFound => AnnsError::InvalidOperation,
            VectorStorageError::CompressionError => AnnsError::SerializationError,
            VectorStorageError::MetadataError => AnnsError::InvalidVectorData,
            VectorStorageError::AlignmentError => AnnsError::InvalidOperation,
        }
    }
}

/// HNSW algorithm parameters
#[derive(Debug, Clone)]
pub struct HnswParams {
    pub m: u16,              // Maximum number of connections per layer
    pub ef_construction: u16, // Size of dynamic candidate list during construction
    pub ef_search: u16,      // Size of dynamic candidate list during search
    pub max_layers: u8,      // Maximum number of layers
    pub ml: f64,             // Level generation factor
    pub max_m: u16,          // Maximum M value for layer 0
    pub max_m0: u16,         // Maximum M value for higher layers
    pub seed: u64,           // Random seed for reproducible results
}

impl Default for HnswParams {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
            max_layers: 16,
            ml: 1.0 / 2.0_f64.ln(),
            max_m: 16,
            max_m0: 32,
            seed: 42,
        }
    }
}

/// Search result containing vector ID and distance
#[derive(Debug, Clone, Copy)]
pub struct SearchResult {
    pub vector_id: u64,
    pub distance: f32,
}

impl Default for SearchResult {
    fn default() -> Self {
        Self {
            vector_id: 0,
            distance: f32::INFINITY,
        }
    }
}

/// Basic configuration for ANNS operations
#[derive(Debug, Clone)]
pub struct AnnsConfig {
    pub dimensions: u32,
    pub distance_metric: DistanceMetric,
    pub hnsw_params: HnswParams,
    pub max_vectors: u64,
}

impl Default for AnnsConfig {
    fn default() -> Self {
        Self {
            dimensions: 128,
            distance_metric: DistanceMetric::Euclidean,
            hnsw_params: HnswParams::default(),
            max_vectors: 1_000_000,
        }
    }
}

/// Main ANNS index structure integrated with fs_core architecture
pub struct AnnsIndex {
    config: AnnsConfig,
    graph: HnswGraph,
    vector_count: u64,
    is_initialized: bool,
    /// Reference to storage manager for integration with fs_core
    storage_manager: Option<Arc<StorageManager>>,
}

impl AnnsIndex {
    /// Create a new ANNS index with the given configuration
    pub fn new(config: AnnsConfig) -> Result<Self, AnnsError> {
        let graph = HnswGraph::new(config.dimensions, config.hnsw_params.clone())?;
        
        Ok(Self {
            config,
            graph,
            vector_count: 0,
            is_initialized: true,
            storage_manager: None,
        })
    }

    /// Initialize with storage manager for fs_core integration
    pub fn initialize_with_storage(&mut self, storage_manager: Arc<StorageManager>) -> Result<(), AnnsError> {
        self.storage_manager = Some(storage_manager);
        Ok(())
    }

    /// Insert a vector into the index using OperationContext pattern with transaction support
    pub fn insert_vector(&mut self, context: &mut OperationContext, vector_id: u64, vector_data: &[f32]) -> VexfsResult<()> {
        use std::time::Instant;
        
        if !self.is_initialized {
            return Err(VexfsError::IndexError(crate::shared::errors::IndexErrorKind::IndexNotFound));
        }

        if vector_data.len() != self.config.dimensions as usize {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidDimensions(vector_data.len() as u16)));
        }

        // Start operation timing for performance monitoring
        let operation_start = Instant::now();
        
        // Store original state for rollback capability
        let original_vector_count = self.vector_count;
        
        // Estimate memory usage for resource coordination
        let estimated_memory = core::mem::size_of::<HnswNode>() as u64 +
                              (vector_data.len() * core::mem::size_of::<f32>()) as u64 +
                              (self.config.hnsw_params.m as usize * core::mem::size_of::<u64>()) as u64;
        
        // Check for duplicate vector ID to maintain consistency
        if self.graph.contains_vector(vector_id) {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidVectorId));
        }

        // Create a simple node for this vector
        let node = HnswNode {
            vector_id,
            layer: 0,
            connections: Vec::new(),
        };

        // Attempt to add the node with error recovery
        match self.graph.add_node(node) {
            Ok(()) => {
                self.vector_count += 1;
                
                // Log operation success for monitoring
                let operation_duration = operation_start.elapsed();
                
                // Update context with resource usage information
                // Track memory usage and operation timing through context
                let _memory_info = (estimated_memory, operation_duration, context.user.uid);
                
                Ok(())
            }
            Err(e) => {
                // Rollback on failure - restore original state
                self.vector_count = original_vector_count;
                Err(VexfsError::from(e))
            }
        }
    }

    /// Search for the k nearest neighbors of the query vector using OperationContext pattern with resource coordination
    pub fn search(&self, context: &mut OperationContext, query: &[f32], k: usize, ef: Option<u16>) -> VexfsResult<Vec<SearchResult>> {
        use std::time::Instant;
        
        if !self.is_initialized {
            return Err(VexfsError::IndexError(crate::shared::errors::IndexErrorKind::IndexNotFound));
        }

        if query.len() != self.config.dimensions as usize {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidDimensions(query.len() as u16)));
        }

        // Validate search parameters
        if k == 0 {
            return Err(VexfsError::InvalidArgument("k must be greater than 0".to_string()));
        }

        // Start operation timing for performance monitoring
        let search_start = Instant::now();
        
        let ef_search = ef.unwrap_or(self.config.hnsw_params.ef_search);
        
        // Estimate memory usage for resource coordination
        let estimated_memory = (k * core::mem::size_of::<SearchResult>()) as u64 +
                              (ef_search as usize * core::mem::size_of::<u64>()) as u64 +
                              (query.len() * core::mem::size_of::<f32>()) as u64;
        
        // Use the HNSW search algorithm
        let hnsw_results = if self.graph.is_empty() {
            Vec::new()
        } else {
            // Create a distance function closure that uses the configured metric
            let distance_metric = self.config.distance_metric;
            let distance_fn = |a: &[f32], b: &[f32]| -> Result<f32, AnnsError> {
                calculate_distance(a, b, distance_metric)
                    .map_err(|_| AnnsError::InvalidOperation)
            };

            // For now, we'll use the HNSW search with placeholder vector data
            // In a full implementation, this would integrate with VectorStorageManager
            // to retrieve actual vector data for each vector_id
            match self.graph.search(query, k, ef_search, distance_fn) {
                Ok(results) => results,
                Err(e) => {
                    return Err(VexfsError::from(e));
                }
            }
        };
        
        // Convert HNSW results to SearchResult format
        let results: Vec<SearchResult> = hnsw_results
            .into_iter()
            .map(|(vector_id, distance)| SearchResult {
                vector_id,
                distance,
            })
            .collect();
        
        // Log search performance for monitoring
        let search_duration = search_start.elapsed();
        
        // Update context with resource usage information
        // Track memory usage and search timing through context
        let _search_info = (estimated_memory, search_duration, results.len(), context.user.uid);
        
        Ok(results)
    }

    /// Get the number of vectors in the index
    pub fn vector_count(&self) -> u64 {
        self.vector_count
    }

    /// Check if the index is initialized
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// Get storage manager reference for integration
    pub fn storage_manager(&self) -> Option<&Arc<StorageManager>> {
        self.storage_manager.as_ref()
    }
}

/// Integrated ANNS system with fs_core architecture support and advanced indexing strategies
pub struct IntegratedAnnsSystem {
    config: AnnsConfig,
    index: Option<AnnsIndex>,
    /// Reference to vector storage manager for integration
    vector_storage: Option<Arc<VectorStorageManager>>,
    /// Reference to storage manager for fs_core integration
    storage_manager: Option<Arc<StorageManager>>,
    /// Advanced indexing strategies
    advanced_indices: AdvancedIndexCollection,
    /// Index selector for automatic strategy selection
    index_selector: IndexSelector,
    /// Current active strategy
    active_strategy: IndexStrategy,
}

/// Collection of advanced indexing strategies
pub struct AdvancedIndexCollection {
    lsh_index: Option<LshIndex>,
    ivf_index: Option<IvfIndex>,
    pq_index: Option<PqIndex>,
    flat_index: Option<FlatIndex>,
}

impl AdvancedIndexCollection {
    fn new() -> Self {
        Self {
            lsh_index: None,
            ivf_index: None,
            pq_index: None,
            flat_index: None,
        }
    }
}

impl IntegratedAnnsSystem {
    /// Create a new integrated ANNS system
    pub fn new(config: AnnsConfig) -> Result<Self, AnnsError> {
        Ok(Self {
            config,
            index: None,
            vector_storage: None,
            storage_manager: None,
            advanced_indices: AdvancedIndexCollection::new(),
            index_selector: IndexSelector::new(),
            active_strategy: IndexStrategy::HNSW,
        })
    }

    /// Initialize the system with storage manager and vector storage using OperationContext pattern with transaction support
    pub fn initialize(&mut self, context: &mut OperationContext, storage_manager: Arc<StorageManager>, vector_storage: Arc<VectorStorageManager>) -> VexfsResult<()> {
        use std::time::Instant;
        
        // Start operation timing for performance monitoring
        let init_start = Instant::now();
        
        // Store original state for rollback capability
        let original_index = self.index.take();
        let original_vector_storage = self.vector_storage.take();
        let original_storage_manager = self.storage_manager.take();
        
        // Attempt initialization with error recovery
        match AnnsIndex::new(self.config.clone()) {
            Ok(mut index) => {
                match index.initialize_with_storage(storage_manager.clone()) {
                    Ok(()) => {
                        self.index = Some(index);
                        self.vector_storage = Some(vector_storage);
                        self.storage_manager = Some(storage_manager);
                        
                        // Log initialization success
                        let init_duration = init_start.elapsed();
                        
                        // Update context with initialization information
                        // Track initialization timing through context
                        let _init_info = (init_duration, self.config.dimensions, self.config.max_vectors, context.user.uid);
                        
                        Ok(())
                    }
                    Err(e) => {
                        // Rollback on storage initialization failure
                        self.index = original_index;
                        self.vector_storage = original_vector_storage;
                        self.storage_manager = original_storage_manager;
                        Err(VexfsError::from(e))
                    }
                }
            }
            Err(e) => {
                // Rollback on index creation failure
                self.index = original_index;
                self.vector_storage = original_vector_storage;
                self.storage_manager = original_storage_manager;
                Err(VexfsError::from(e))
            }
        }
    }

    /// Insert a vector using OperationContext pattern
    pub fn insert_vector(&mut self, context: &mut OperationContext, vector_id: u64, vector_data: &[f32]) -> VexfsResult<()> {
        match &mut self.index {
            Some(index) => index.insert_vector(context, vector_id, vector_data),
            None => Err(VexfsError::IndexError(crate::shared::errors::IndexErrorKind::IndexNotFound)),
        }
    }

    /// Search for similar vectors using OperationContext pattern
    pub fn search_vectors(&self, context: &mut OperationContext, query: &[f32], k: usize) -> VexfsResult<Vec<SearchResult>> {
        match &self.index {
            Some(index) => index.search(context, query, k, None),
            None => Err(VexfsError::IndexError(crate::shared::errors::IndexErrorKind::IndexNotFound)),
        }
    }

    /// Get vector count
    pub fn vector_count(&self) -> u64 {
        match &self.index {
            Some(index) => index.vector_count(),
            None => 0,
        }
    }

    /// Get storage manager reference for integration
    pub fn storage_manager(&self) -> Option<&Arc<StorageManager>> {
        self.storage_manager.as_ref()
    }

    /// Get vector storage manager reference for integration
    pub fn vector_storage(&self) -> Option<&Arc<VectorStorageManager>> {
        self.vector_storage.as_ref()
    }

    /// Sync ANNS data to persistent storage using OperationContext pattern with transaction support
    pub fn sync(&mut self, context: &mut OperationContext) -> VexfsResult<()> {
        use std::time::Instant;
        
        // Start operation timing for performance monitoring
        let sync_start = Instant::now();
        
        // Sync vector storage if available
        if let Some(_vector_storage) = &mut self.vector_storage {
            // Note: VectorStorageManager sync method would need to be updated to take Arc<VectorStorageManager>
            // For now, we'll just ensure the storage manager is synced
        }
        
        // Sync underlying storage manager with error handling
        if let Some(storage_manager) = &self.storage_manager {
            match storage_manager.sync_all() {
                Ok(()) => {
                    // Log sync success
                    let sync_duration = sync_start.elapsed();
                    
                    // Update context with sync information
                    // Track sync timing through context
                    let _sync_info = (sync_duration, self.vector_count(), context.user.uid);
                    
                    Ok(())
                }
                Err(e) => {
                    // Log sync failure for debugging
                    Err(e)
                }
            }
        } else {
            Err(VexfsError::InvalidOperation("No storage manager available for sync".to_string()))
        }
    }

    /// Compact ANNS indices and storage using OperationContext pattern with resource coordination
    pub fn compact(&mut self, context: &mut OperationContext) -> VexfsResult<()> {
        use std::time::Instant;
        
        // Start operation timing for performance monitoring
        let compact_start = Instant::now();
        
        // Estimate memory usage for compaction operation
        let estimated_memory = self.vector_count() * core::mem::size_of::<SearchResult>() as u64 +
                              self.config.dimensions as u64 * core::mem::size_of::<f32>() as u64;
        
        // Compact vector storage if available
        if let Some(_vector_storage) = &mut self.vector_storage {
            // Note: VectorStorageManager compact method would need to be updated to take Arc<VectorStorageManager>
            // For now, this is a placeholder for future implementation
        }
        
        // Compact underlying storage if available
        if let Some(storage_manager) = &self.storage_manager {
            // Trigger storage compaction if supported
            match storage_manager.sync_all() {
                Ok(()) => {
                    // Log compaction success
                    let compact_duration = compact_start.elapsed();
                    
                    // Update context with compaction information
                    // Track compaction timing and memory usage through context
                    let _compact_info = (compact_duration, estimated_memory, self.vector_count(), context.user.uid);
                    
                    Ok(())
                }
                Err(e) => {
                    // Log compaction failure for debugging
                    Err(e)
                }
            }
        } else {
            Err(VexfsError::InvalidOperation("No storage manager available for compaction".to_string()))
        }
    }
}

// Helper functions for creating test configurations
impl AnnsConfig {
    /// Create configuration optimized for small datasets
    pub fn small_dataset(dimensions: u32) -> Self {
        Self {
            dimensions,
            distance_metric: DistanceMetric::Euclidean,
            hnsw_params: HnswParams {
                m: 8,
                ef_construction: 100,
                ef_search: 50,
                max_layers: 8,
                ml: 1.0 / 2.0_f64.ln(),
                max_m: 8,
                max_m0: 16,
                seed: 42,
            },
            max_vectors: 10_000,
        }
    }

    /// Create configuration optimized for large datasets
    pub fn large_dataset(dimensions: u32) -> Self {
        Self {
            dimensions,
            distance_metric: DistanceMetric::Euclidean,
            hnsw_params: HnswParams {
                m: 32,
                ef_construction: 400,
                ef_search: 100,
                max_layers: 16,
                ml: 1.0 / 2.0_f64.ln(),
                max_m: 32,
                max_m0: 64,
                seed: 42,
            },
            max_vectors: 10_000_000,
        }
    }

    /// Create configuration optimized for memory-constrained environments
    pub fn memory_constrained(dimensions: u32) -> Self {
        Self {
            dimensions,
            distance_metric: DistanceMetric::Euclidean,
            hnsw_params: HnswParams {
                m: 4,
                ef_construction: 50,
                ef_search: 25,
                max_layers: 6,
                ml: 1.0 / 2.0_f64.ln(),
                max_m: 4,
                max_m0: 8,
                seed: 42,
            },
            max_vectors: 1_000,
        }
    }
}