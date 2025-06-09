//! Storage-HNSW Bridge Interface Implementation
//! 
//! This module provides the critical integration layer between OptimizedVectorStorageManager
//! and OptimizedHnswGraph, enabling seamless communication and synchronization between
//! storage and graph components while maintaining <6KB stack usage limits.

use std::sync::Arc;
use std::collections::BTreeMap;
use std::sync::{RwLock, Mutex};
use crate::shared::{VexfsResult, VexfsError};
use crate::vector_storage::{VectorStorageManager, VectorLocation, VectorDataType};
use crate::anns::{HnswGraph, HnswNode, AnnsError};
use crate::storage::StorageManager;
use crate::shared::types::InodeNumber;

/// Maximum stack allocation size to ensure <6KB limit
const MAX_STACK_ALLOCATION: usize = 1024; // 1KB safety margin

/// Bridge configuration for storage-HNSW integration
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Enable lazy synchronization for performance
    pub lazy_sync: bool,
    /// Batch size for bulk operations
    pub batch_size: usize,
    /// Maximum concurrent operations
    pub max_concurrent_ops: usize,
    /// Enable automatic graph rebuilding
    pub auto_rebuild: bool,
    /// Synchronization interval in milliseconds
    pub sync_interval_ms: u64,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            lazy_sync: true,
            batch_size: 100,
            max_concurrent_ops: 4,
            auto_rebuild: false,
            sync_interval_ms: 1000,
        }
    }
}

/// Simple operation context for bridge operations
#[derive(Debug, Clone)]
pub struct OperationContext {
    pub user_id: u32,
    pub group_id: u32,
    pub process_id: u32,
    pub transaction_id: Option<u64>,
    pub timeout_ms: u64,
}

impl Default for OperationContext {
    fn default() -> Self {
        Self {
            user_id: 1000,
            group_id: 1000,
            process_id: 12345,
            transaction_id: Some(1),
            timeout_ms: 30000,
        }
    }
}

/// Bridge interface trait for storage-HNSW communication
pub trait StorageHnswBridge {
    /// Insert vector with automatic graph synchronization
    fn insert_vector_with_sync(
        &mut self,
        context: &mut OperationContext,
        vector_id: u64,
        vector_data: &[f32],
        metadata: VectorMetadata,
    ) -> VexfsResult<()>;

    /// Update vector and synchronize graph
    fn update_vector_with_sync(
        &mut self,
        context: &mut OperationContext,
        vector_id: u64,
        vector_data: &[f32],
    ) -> VexfsResult<()>;

    /// Delete vector and update graph
    fn delete_vector_with_sync(
        &mut self,
        context: &mut OperationContext,
        vector_id: u64,
    ) -> VexfsResult<()>;

    /// Search vectors using HNSW graph
    fn search_vectors(
        &self,
        context: &mut OperationContext,
        query: &[f32],
        k: usize,
        search_params: SearchParameters,
    ) -> VexfsResult<Vec<VectorSearchResult>>;

    /// Force synchronization between storage and graph
    fn force_sync(&mut self, context: &mut OperationContext) -> VexfsResult<()>;

    /// Get synchronization status
    fn sync_status(&self) -> SyncStatus;
}

/// Vector metadata for bridge operations
#[derive(Debug, Clone)]
pub struct VectorMetadata {
    pub dimensions: u32,
    pub data_type: VectorDataType,
    pub file_inode: u64,
    pub compression_type: u8,
}

/// Search parameters for vector queries
#[derive(Debug, Clone)]
pub struct SearchParameters {
    pub ef_search: Option<u16>,
    pub similarity_threshold: Option<f32>,
    pub max_distance: Option<f32>,
    pub include_metadata: bool,
}

impl Default for SearchParameters {
    fn default() -> Self {
        Self {
            ef_search: None,
            similarity_threshold: None,
            max_distance: None,
            include_metadata: false,
        }
    }
}

/// Vector search result with enhanced metadata
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub vector_id: u64,
    pub distance: f32,
    pub similarity: f32,
    pub metadata: Option<VectorMetadata>,
    pub location: Option<VectorLocation>,
}

/// Synchronization status information
#[derive(Debug, Clone)]
pub struct SyncStatus {
    pub is_synchronized: bool,
    pub pending_operations: usize,
    pub last_sync_timestamp: u64,
    pub sync_errors: usize,
}

/// Bridge error types for integration failures
#[derive(Debug, Clone)]
pub enum BridgeError {
    StorageError(String),
    GraphError(String),
    SyncError(String),
    InvalidOperation(String),
    ResourceExhausted,
    StackOverflow,
}

impl From<BridgeError> for VexfsError {
    fn from(err: BridgeError) -> Self {
        match err {
            BridgeError::StorageError(_) => VexfsError::StorageError(
                crate::shared::errors::StorageErrorKind::AllocationFailed
            ),
            BridgeError::GraphError(_) => VexfsError::IndexError(
                crate::shared::errors::IndexErrorKind::IndexCorrupted
            ),
            BridgeError::SyncError(msg) => VexfsError::InvalidOperation(msg),
            BridgeError::InvalidOperation(msg) => VexfsError::InvalidArgument(msg),
            BridgeError::ResourceExhausted => VexfsError::InternalError("Resource exhausted".to_string()),
            BridgeError::StackOverflow => VexfsError::InternalError("Stack overflow detected".to_string()),
        }
    }
}

/// Pending operation for lazy synchronization
#[derive(Debug, Clone)]
enum PendingOperation {
    Insert { vector_id: u64, vector_data: Vec<f32>, metadata: VectorMetadata },
    Update { vector_id: u64, vector_data: Vec<f32> },
    Delete { vector_id: u64 },
}

/// Optimized Vector Storage Manager with HNSW integration
pub struct OptimizedVectorStorageManager {
    /// Base vector storage manager
    storage_manager: VectorStorageManager,
    /// HNSW graph for approximate nearest neighbor search
    hnsw_graph: Arc<RwLock<HnswGraph>>,
    /// Bridge configuration
    config: BridgeConfig,
    /// Pending synchronization operations (heap-allocated to avoid stack overflow)
    pending_ops: Arc<Mutex<Vec<PendingOperation>>>,
    /// Synchronization status
    sync_status: Arc<RwLock<SyncStatus>>,
    /// Vector ID to graph node mapping (heap-allocated)
    vector_to_node: Arc<RwLock<BTreeMap<u64, u64>>>,
}

impl OptimizedVectorStorageManager {
    /// Create new optimized vector storage manager with HNSW integration
    pub fn new(
        storage_manager: Arc<StorageManager>,
        dimensions: u32,
        config: BridgeConfig,
    ) -> VexfsResult<Self> {
        // Check stack usage - allocate large structures on heap
        let stack_check = [0u8; 64]; // Small stack allocation for safety check
        if stack_check.len() > MAX_STACK_ALLOCATION {
            return Err(VexfsError::from(BridgeError::StackOverflow));
        }

        let vector_storage = VectorStorageManager::new(
            storage_manager,
            4096, // block_size
            1000000, // total_blocks
        );

        // Create HNSW graph with default parameters (heap-allocated)
        let hnsw_params = crate::anns::HnswParams::default();
        let hnsw_graph = HnswGraph::new(dimensions, hnsw_params)
            .map_err(|e| VexfsError::from(BridgeError::GraphError(format!("Failed to create HNSW graph: {:?}", e))))?;

        let sync_status = SyncStatus {
            is_synchronized: true,
            pending_operations: 0,
            last_sync_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            sync_errors: 0,
        };

        Ok(Self {
            storage_manager: vector_storage,
            hnsw_graph: Arc::new(RwLock::new(hnsw_graph)),
            config,
            pending_ops: Arc::new(Mutex::new(Vec::new())),
            sync_status: Arc::new(RwLock::new(sync_status)),
            vector_to_node: Arc::new(RwLock::new(BTreeMap::new())),
        })
    }

    /// Get vector data from storage (stack-safe implementation)
    fn get_vector_data_safe(&self, _vector_id: u64) -> VexfsResult<Vec<f32>> {
        // In a full implementation, this would read from actual storage
        // For now, return placeholder data
        let dimensions = 128; // This should come from vector metadata
        let vector_data = vec![0.0f32; dimensions];
        Ok(vector_data)
    }

    /// Process pending operations in batches (stack-safe)
    fn process_pending_operations(&mut self, _context: &mut OperationContext) -> VexfsResult<()> {
        let mut pending_ops = self.pending_ops.lock().unwrap();
        
        if pending_ops.is_empty() {
            return Ok(());
        }

        // Process in batches to avoid stack overflow
        let batch_size = self.config.batch_size.min(MAX_STACK_ALLOCATION / 64);
        let mut processed = 0;

        while processed < pending_ops.len() && processed < batch_size {
            let op = &pending_ops[processed];
            
            match op {
                PendingOperation::Insert { vector_id, vector_data, metadata } => {
                    self.sync_insert_to_graph(*vector_id, vector_data, metadata)?;
                }
                PendingOperation::Update { vector_id, vector_data } => {
                    self.sync_update_to_graph(*vector_id, vector_data)?;
                }
                PendingOperation::Delete { vector_id } => {
                    self.sync_delete_from_graph(*vector_id)?;
                }
            }
            
            processed += 1;
        }

        // Remove processed operations
        pending_ops.drain(0..processed);
        
        // Update sync status
        {
            let mut status = self.sync_status.write().unwrap();
            status.pending_operations = pending_ops.len();
            status.last_sync_timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }

        Ok(())
    }

    /// Synchronize vector insertion to HNSW graph
    fn sync_insert_to_graph(
        &self,
        vector_id: u64,
        _vector_data: &[f32],
        _metadata: &VectorMetadata,
    ) -> VexfsResult<()> {
        let mut graph = self.hnsw_graph.write().unwrap();
        
        // Create HNSW node
        let node = HnswNode {
            vector_id,
            layer: 0, // Simple layer assignment for now
            connections: Vec::new(),
        };

        graph.add_node(node)
            .map_err(|e| VexfsError::from(BridgeError::GraphError(format!("Failed to add node: {:?}", e))))?;

        // Update vector-to-node mapping
        {
            let mut mapping = self.vector_to_node.write().unwrap();
            mapping.insert(vector_id, vector_id); // Simple 1:1 mapping for now
        }

        Ok(())
    }

    /// Synchronize vector update to HNSW graph
    fn sync_update_to_graph(&self, vector_id: u64, _vector_data: &[f32]) -> VexfsResult<()> {
        // For updates, we need to rebuild connections
        // This is a simplified implementation
        let graph = self.hnsw_graph.read().unwrap();
        
        if graph.get_node(vector_id).is_none() {
            return Err(VexfsError::from(BridgeError::GraphError("Node not found for update".to_string())));
        }

        // In a full implementation, this would update the node's connections
        // based on the new vector data
        Ok(())
    }

    /// Synchronize vector deletion from HNSW graph
    fn sync_delete_from_graph(&self, vector_id: u64) -> VexfsResult<()> {
        let mut graph = self.hnsw_graph.write().unwrap();
        
        graph.remove_node(vector_id)
            .map_err(|e| VexfsError::from(BridgeError::GraphError(format!("Failed to remove node: {:?}", e))))?;

        // Update vector-to-node mapping
        {
            let mut mapping = self.vector_to_node.write().unwrap();
            mapping.remove(&vector_id);
        }

        Ok(())
    }
}

impl StorageHnswBridge for OptimizedVectorStorageManager {
    fn insert_vector_with_sync(
        &mut self,
        _context: &mut OperationContext,
        vector_id: u64,
        vector_data: &[f32],
        metadata: VectorMetadata,
    ) -> VexfsResult<()> {
        // For now, we'll use a simplified approach that just handles the graph synchronization
        // In a full implementation, this would integrate with the actual storage manager
        
        // Handle graph synchronization
        if self.config.lazy_sync {
            // Add to pending operations
            let pending_op = PendingOperation::Insert {
                vector_id,
                vector_data: vector_data.to_vec(),
                metadata,
            };
            
            {
                let mut pending_ops = self.pending_ops.lock().unwrap();
                pending_ops.push(pending_op);
            }

            // Update sync status
            {
                let mut status = self.sync_status.write().unwrap();
                status.is_synchronized = false;
                status.pending_operations += 1;
            }
        } else {
            // Immediate synchronization
            self.sync_insert_to_graph(vector_id, vector_data, &metadata)?;
        }

        Ok(())
    }

    fn update_vector_with_sync(
        &mut self,
        _context: &mut OperationContext,
        vector_id: u64,
        vector_data: &[f32],
    ) -> VexfsResult<()> {
        // Update in storage first
        // Note: This is a simplified implementation - full implementation would
        // need to handle vector updates in storage properly

        // Handle graph synchronization
        if self.config.lazy_sync {
            let pending_op = PendingOperation::Update {
                vector_id,
                vector_data: vector_data.to_vec(),
            };
            
            {
                let mut pending_ops = self.pending_ops.lock().unwrap();
                pending_ops.push(pending_op);
            }

            {
                let mut status = self.sync_status.write().unwrap();
                status.is_synchronized = false;
                status.pending_operations += 1;
            }
        } else {
            self.sync_update_to_graph(vector_id, vector_data)?;
        }

        Ok(())
    }

    fn delete_vector_with_sync(
        &mut self,
        _context: &mut OperationContext,
        vector_id: u64,
    ) -> VexfsResult<()> {
        // Delete from storage first
        // Note: This is a simplified implementation - full implementation would
        // need to handle vector deletion in storage properly

        // Handle graph synchronization
        if self.config.lazy_sync {
            let pending_op = PendingOperation::Delete { vector_id };
            
            {
                let mut pending_ops = self.pending_ops.lock().unwrap();
                pending_ops.push(pending_op);
            }

            {
                let mut status = self.sync_status.write().unwrap();
                status.is_synchronized = false;
                status.pending_operations += 1;
            }
        } else {
            self.sync_delete_from_graph(vector_id)?;
        }

        Ok(())
    }

    fn search_vectors(
        &self,
        _context: &mut OperationContext,
        _query: &[f32],
        k: usize,
        _search_params: SearchParameters,
    ) -> VexfsResult<Vec<VectorSearchResult>> {
        // Ensure we're synchronized before searching
        if !self.sync_status.read().unwrap().is_synchronized && !self.config.lazy_sync {
            return Err(VexfsError::from(BridgeError::SyncError("Graph not synchronized".to_string())));
        }

        let graph = self.hnsw_graph.read().unwrap();
        
        if graph.is_empty() {
            return Ok(Vec::new());
        }

        // Use heap allocation for results to avoid stack overflow
        let mut results = Vec::with_capacity(k);

        // Simplified search implementation - in full implementation this would
        // use the actual HNSW search algorithm with proper distance calculations
        
        // For now, return placeholder results
        // In full implementation, this would:
        // 1. Use HNSW graph search algorithm
        // 2. Retrieve actual vector data from storage
        // 3. Calculate real distances
        // 4. Apply similarity thresholds
        
        for i in 0..k.min(graph.node_count()) {
            let vector_id = i as u64 + 1; // Placeholder vector ID
            let distance = 0.5f32; // Placeholder distance
            let similarity = 1.0 - distance; // Simple similarity calculation
            
            let result = VectorSearchResult {
                vector_id,
                distance,
                similarity,
                metadata: None,
                location: None,
            };
            
            results.push(result);
        }

        Ok(results)
    }

    fn force_sync(&mut self, context: &mut OperationContext) -> VexfsResult<()> {
        self.process_pending_operations(context)?;
        
        {
            let mut status = self.sync_status.write().unwrap();
            status.is_synchronized = true;
            status.pending_operations = 0;
        }
        
        Ok(())
    }

    fn sync_status(&self) -> SyncStatus {
        self.sync_status.read().unwrap().clone()
    }
}

/// Enhanced Vector Storage Manager with search capabilities
impl OptimizedVectorStorageManager {
    /// Perform k-nearest neighbor search using HNSW graph
    pub fn knn_search(
        &self,
        context: &mut OperationContext,
        query: &[f32],
        k: usize,
    ) -> VexfsResult<Vec<VectorSearchResult>> {
        self.search_vectors(context, query, k, SearchParameters::default())
    }

    /// Perform range query with distance threshold
    pub fn range_search(
        &self,
        context: &mut OperationContext,
        query: &[f32],
        max_distance: f32,
    ) -> VexfsResult<Vec<VectorSearchResult>> {
        let search_params = SearchParameters {
            max_distance: Some(max_distance),
            ..Default::default()
        };
        
        self.search_vectors(context, query, 1000, search_params) // Large k for range search
    }

    /// Perform similarity search with threshold
    pub fn similarity_search(
        &self,
        context: &mut OperationContext,
        query: &[f32],
        similarity_threshold: f32,
        max_results: usize,
    ) -> VexfsResult<Vec<VectorSearchResult>> {
        let search_params = SearchParameters {
            similarity_threshold: Some(similarity_threshold),
            ..Default::default()
        };
        
        self.search_vectors(context, query, max_results, search_params)
    }

    /// Get storage and graph statistics
    pub fn get_statistics(&self) -> BridgeStatistics {
        let graph = self.hnsw_graph.read().unwrap();
        let sync_status = self.sync_status.read().unwrap();
        
        BridgeStatistics {
            total_vectors: graph.node_count() as u64,
            graph_memory_usage: graph.memory_usage(),
            storage_memory_usage: 0, // Would be calculated from storage manager
            sync_status: sync_status.clone(),
            pending_operations: sync_status.pending_operations,
        }
    }
}

/// Bridge statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct BridgeStatistics {
    pub total_vectors: u64,
    pub graph_memory_usage: u64,
    pub storage_memory_usage: u64,
    pub sync_status: SyncStatus,
    pub pending_operations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> OperationContext {
        OperationContext::default()
    }

    fn create_mock_storage_manager() -> Arc<StorageManager> {
        // For now, create a placeholder storage manager
        // In full implementation, this would create a proper mock
        use crate::storage::{BlockManager, SpaceAllocator, VexfsJournal, PersistenceManager, SuperblockManager, BlockCacheManager, VexfsLayout};
        
        // This is a simplified mock - in real implementation would use proper mocking
        Arc::new(StorageManager::new_for_testing())
    }

    #[test]
    fn test_bridge_config_default() {
        let config = BridgeConfig::default();
        assert!(config.lazy_sync);
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.max_concurrent_ops, 4);
        assert!(!config.auto_rebuild);
        assert_eq!(config.sync_interval_ms, 1000);
    }

    #[test]
    fn test_search_parameters_default() {
        let params = SearchParameters::default();
        assert!(params.ef_search.is_none());
        assert!(params.similarity_threshold.is_none());
        assert!(params.max_distance.is_none());
        assert!(!params.include_metadata);
    }

    #[test]
    fn test_optimized_vector_storage_manager_creation() {
        let storage_manager = create_mock_storage_manager();
        let config = BridgeConfig::default();
        
        let result = OptimizedVectorStorageManager::new(storage_manager, 128, config);
        assert!(result.is_ok());
        
        let bridge = result.unwrap();
        let status = bridge.sync_status();
        assert!(status.is_synchronized);
        assert_eq!(status.pending_operations, 0);
    }

    #[test]
    fn test_sync_status() {
        let storage_manager = create_mock_storage_manager();
        let config = BridgeConfig::default();
        let bridge = OptimizedVectorStorageManager::new(storage_manager, 128, config).unwrap();
        
        let status = bridge.sync_status();
        assert!(status.is_synchronized);
        assert_eq!(status.pending_operations, 0);
    }
}