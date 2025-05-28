//! Approximate Nearest Neighbor Search (ANNS) implementation for VexFS
//!
//! This module provides efficient vector similarity search capabilities using
//! the Hierarchical Navigable Small World (HNSW) algorithm.
//!
//! This module has been integrated with the fs_core architecture to use OperationContext
//! patterns and work seamlessly with the established VexFS components.

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

// Re-export key components from submodules  
pub mod hnsw;
pub mod indexing;
pub mod memory_mgmt;
pub mod serialization;
pub mod wal;

// Only export what actually exists in the modules
pub use self::hnsw::{HnswGraph, HnswNode};
pub use self::serialization::{IndexSerializer, IndexDeserializer};
pub use self::indexing::{IndexBuilder, IncrementalUpdater, BuildingStats};

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
}

impl Default for HnswParams {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construction: 200,
            ef_search: 50,
            max_layers: 16,
            ml: 1.0 / 2.0_f64.ln(),
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

    /// Insert a vector into the index using OperationContext pattern
    pub fn insert_vector(&mut self, _context: &mut OperationContext, vector_id: u64, vector_data: &[f32]) -> VexfsResult<()> {
        if !self.is_initialized {
            return Err(VexfsError::IndexError(crate::shared::errors::IndexErrorKind::IndexNotFound));
        }

        if vector_data.len() != self.config.dimensions as usize {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidDimensions(vector_data.len() as u16)));
        }

        // Create a simple node for this vector
        let node = HnswNode {
            vector_id,
            layer: 0,
            connections: Vec::new(),
        };

        self.graph.add_node(node).map_err(|e| VexfsError::from(e))?;
        self.vector_count += 1;

        Ok(())
    }

    /// Search for the k nearest neighbors of the query vector using OperationContext pattern
    pub fn search(&self, _context: &mut OperationContext, query: &[f32], _k: usize, ef: Option<u16>) -> VexfsResult<Vec<SearchResult>> {
        if !self.is_initialized {
            return Err(VexfsError::IndexError(crate::shared::errors::IndexErrorKind::IndexNotFound));
        }

        if query.len() != self.config.dimensions as usize {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidDimensions(query.len() as u16)));
        }

        let _ef_search = ef.unwrap_or(self.config.hnsw_params.ef_search);
        
        // Simple linear search for now - will be replaced with HNSW traversal
        let _results = Vec::new();
        
        // For now, return empty results
        // In a full implementation, this would traverse the HNSW graph
        
        Ok(_results)
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

/// Integrated ANNS system with fs_core architecture support
pub struct IntegratedAnnsSystem {
    config: AnnsConfig,
    index: Option<AnnsIndex>,
    /// Reference to vector storage manager for integration
    vector_storage: Option<Arc<VectorStorageManager>>,
    /// Reference to storage manager for fs_core integration
    storage_manager: Option<Arc<StorageManager>>,
}

impl IntegratedAnnsSystem {
    /// Create a new integrated ANNS system
    pub fn new(config: AnnsConfig) -> Result<Self, AnnsError> {
        Ok(Self {
            config,
            index: None,
            vector_storage: None,
            storage_manager: None,
        })
    }

    /// Initialize the system with storage manager and vector storage using OperationContext pattern
    pub fn initialize(&mut self, _context: &mut OperationContext, storage_manager: Arc<StorageManager>, vector_storage: Arc<VectorStorageManager>) -> VexfsResult<()> {
        let mut index = AnnsIndex::new(self.config.clone()).map_err(|e| VexfsError::from(e))?;
        index.initialize_with_storage(storage_manager.clone()).map_err(|e| VexfsError::from(e))?;
        
        self.index = Some(index);
        self.vector_storage = Some(vector_storage);
        self.storage_manager = Some(storage_manager);
        
        Ok(())
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

    /// Sync ANNS data to persistent storage using OperationContext pattern
    pub fn sync(&mut self, _context: &mut OperationContext) -> VexfsResult<()> {
        // Sync vector storage if available
        if let Some(_vector_storage) = &mut self.vector_storage {
            // Note: VectorStorageManager sync method would need to be updated to take Arc<VectorStorageManager>
            // For now, we'll just ensure the storage manager is synced
        }
        
        // Sync underlying storage manager
        if let Some(storage_manager) = &self.storage_manager {
            storage_manager.sync_all()?;
        }
        
        Ok(())
    }

    /// Compact ANNS indices and storage using OperationContext pattern
    pub fn compact(&mut self, _context: &mut OperationContext) -> VexfsResult<()> {
        // Compact vector storage if available
        if let Some(_vector_storage) = &mut self.vector_storage {
            // Note: VectorStorageManager compact method would need to be updated to take Arc<VectorStorageManager>
            // For now, this is a placeholder for future implementation
        }
        
        Ok(())
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
            },
            max_vectors: 1_000,
        }
    }
}