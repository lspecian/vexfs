//! Index building and management for ANNS operations
//! 
//! This module provides functionality for building, updating, and managing
//! ANNS indices with batching and incremental update capabilities.

use std::vec::Vec;
use crate::anns::{
    AnnsError, HnswParams, HnswNode, SearchResult,
};
use crate::vector_storage::{VectorHeader, VectorStorageManager};
use crate::anns::hnsw::HnswGraph;

/// Configuration for batch processing
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub batch_size: u32,
    pub max_memory_mb: u32,
    pub parallel_workers: u32,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            max_memory_mb: 512,
            parallel_workers: 1, // Single-threaded for kernel compatibility
        }
    }
}

/// Statistics for index building operations
#[derive(Debug, Clone, Default)]
pub struct BuildingStats {
    pub vectors_processed: u64,
    pub batches_completed: u32,
    pub total_build_time_ms: u64,
    pub memory_used_bytes: u64,
    pub errors_encountered: u32,
}

impl BuildingStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Update memory usage
    pub fn update_memory_usage(&mut self, bytes: u64) {
        self.memory_used_bytes = bytes;
    }

    /// Increment vector count
    pub fn add_vector(&mut self) {
        self.vectors_processed += 1;
    }

    /// Increment batch count
    pub fn complete_batch(&mut self) {
        self.batches_completed += 1;
    }

    /// Record an error
    pub fn record_error(&mut self) {
        self.errors_encountered += 1;
    }
}

/// Index builder for creating ANNS indices
pub struct IndexBuilder {
    dimensions: u32,
    distance_metric: crate::vector_metrics::DistanceMetric,
    params: HnswParams,
    batch_config: BatchConfig,
    stats: BuildingStats,
    graph: Option<HnswGraph>,
}

impl IndexBuilder {
    /// Create a new index builder
    pub fn new(
        dimensions: u32,
        distance_metric: crate::vector_metrics::DistanceMetric,
        params: HnswParams,
        batch_config: BatchConfig,
    ) -> Self {
        Self {
            dimensions,
            distance_metric,
            params,
            batch_config,
            stats: BuildingStats::new(),
            graph: None,
        }
    }

    /// Initialize the graph
    pub fn initialize(&mut self) -> Result<(), AnnsError> {
        let graph = HnswGraph::new(self.dimensions, self.params.clone())?;
        self.graph = Some(graph);
        Ok(())
    }

    /// Build index from storage
    pub fn build_from_storage(&mut self, _storage: &VectorStorageManager) -> Result<(), AnnsError> {
        // Initialize if not already done
        if self.graph.is_none() {
            self.initialize()?;
        }

        // In a full implementation, this would:
        // 1. Iterate through vectors in storage
        // 2. Process them in batches
        // 3. Build the HNSW graph
        // 4. Update statistics

        self.stats.complete_batch();
        Ok(())
    }

    /// Add a single vector to the index
    pub fn add_vector(&mut self, vector_id: u64, vector_data: &[f32]) -> Result<(), AnnsError> {
        if vector_data.len() != self.dimensions as usize {
            return Err(AnnsError::InvalidDimensions);
        }

        let graph = self.graph.as_mut().ok_or(AnnsError::IndexNotInitialized)?;
        
        // Generate layer for the new node
        let layer = graph.generate_layer();
        let node = HnswNode::new(vector_id, layer);
        
        graph.add_node(node)?;
        self.stats.add_vector();
        self.stats.update_memory_usage(graph.memory_usage());
        
        Ok(())
    }

    /// Get current building statistics
    pub fn stats(&self) -> &BuildingStats {
        &self.stats
    }

    /// Get mutable reference to stats
    pub fn stats_mut(&mut self) -> &mut BuildingStats {
        &mut self.stats
    }

    /// Check if the builder is initialized
    pub fn is_initialized(&self) -> bool {
        self.graph.is_some()
    }

    /// Get the current graph (if built)
    pub fn graph(&self) -> Option<&HnswGraph> {
        self.graph.as_ref()
    }

    /// Take ownership of the built graph
    pub fn take_graph(self) -> Option<HnswGraph> {
        self.graph
    }
}

/// Incremental updater for existing indices
pub struct IncrementalUpdater {
    dimensions: u32,
    params: HnswParams,
    pending_operations: Vec<UpdateOperation>,
    stats: BuildingStats,
}

impl IncrementalUpdater {
    /// Create a new incremental updater
    pub fn new(dimensions: u32, params: HnswParams) -> Self {
        Self {
            dimensions,
            params,
            pending_operations: Vec::new(),
            stats: BuildingStats::new(),
        }
    }

    /// Add an insert operation
    pub fn queue_insert(&mut self, vector_id: u64, data_offset: u64, level: u8) {
        self.pending_operations.push(UpdateOperation::Insert {
            vector_id,
            data_offset,
            level,
        });
    }

    /// Add a delete operation
    pub fn queue_delete(&mut self, vector_id: u64) {
        self.pending_operations.push(UpdateOperation::Delete { vector_id });
    }

    /// Add an update operation
    pub fn queue_update(&mut self, vector_id: u64, new_data_offset: u64) {
        self.pending_operations.push(UpdateOperation::Update {
            vector_id,
            new_data_offset,
        });
    }

    /// Apply all pending operations to a graph
    pub fn apply_operations(&mut self, graph: &mut HnswGraph) -> Result<(), AnnsError> {
        for operation in self.pending_operations.drain(..) {
            match operation {
                UpdateOperation::Insert { vector_id, data_offset: _, level } => {
                    let node = HnswNode::new(vector_id, level);
                    graph.add_node(node)?;
                    self.stats.add_vector();
                }
                UpdateOperation::Delete { vector_id: _ } => {
                    // In a full implementation, this would remove the node from the graph
                    self.stats.record_error(); // Placeholder for unimplemented
                }
                UpdateOperation::Update { vector_id: _, new_data_offset: _ } => {
                    // In a full implementation, this would update the node's vector data
                    self.stats.record_error(); // Placeholder for unimplemented
                }
            }
        }

        self.stats.update_memory_usage(graph.memory_usage());
        Ok(())
    }

    /// Get the number of pending operations
    pub fn pending_count(&self) -> usize {
        self.pending_operations.len()
    }

    /// Clear all pending operations
    pub fn clear_pending(&mut self) {
        self.pending_operations.clear();
    }

    /// Get current statistics
    pub fn stats(&self) -> &BuildingStats {
        &self.stats
    }
}

/// Types of update operations
#[derive(Debug, Clone)]
enum UpdateOperation {
    Insert {
        vector_id: u64,
        data_offset: u64,
        level: u8,
    },
    Delete {
        vector_id: u64,
    },
    Update {
        vector_id: u64,
        new_data_offset: u64,
    },
}

/// Builder for creating batch processing configurations
pub struct BatchBuilder {
    batch_size: u32,
    vectors: Vec<(u64, Vec<f32>)>,
}

impl BatchBuilder {
    /// Create a new batch builder
    pub fn new(batch_size: u32) -> Self {
        Self {
            batch_size,
            vectors: Vec::new(),
        }
    }

    /// Add a vector to the current batch
    pub fn add_vector(&mut self, vector_id: u64, vector_data: Vec<f32>) -> Result<(), AnnsError> {
        self.vectors.push((vector_id, vector_data));
        Ok(())
    }

    /// Check if the batch is full
    pub fn is_full(&self) -> bool {
        self.vectors.len() >= self.batch_size as usize
    }

    /// Get the current batch and clear it
    pub fn take_batch(&mut self) -> Vec<(u64, Vec<f32>)> {
        let batch = self.vectors.clone();
        self.vectors.clear();
        batch
    }

    /// Get the number of vectors in the current batch
    pub fn current_size(&self) -> usize {
        self.vectors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_metrics::DistanceMetric;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.max_memory_mb, 512);
        assert_eq!(config.parallel_workers, 1);
    }

    #[test]
    fn test_building_stats() {
        let mut stats = BuildingStats::new();
        assert_eq!(stats.vectors_processed, 0);
        
        stats.add_vector();
        assert_eq!(stats.vectors_processed, 1);
        
        stats.complete_batch();
        assert_eq!(stats.batches_completed, 1);
    }

    #[test]
    fn test_index_builder() {
        let params = HnswParams::default();
        let config = BatchConfig::default();
        let mut builder = IndexBuilder::new(128, DistanceMetric::Euclidean, params, config);
        
        assert!(!builder.is_initialized());
        builder.initialize().unwrap();
        assert!(builder.is_initialized());
    }

    #[test]
    fn test_incremental_updater() {
        let params = HnswParams::default();
        let mut updater = IncrementalUpdater::new(128, params);
        
        assert_eq!(updater.pending_count(), 0);
        
        updater.queue_insert(1, 0, 0);
        assert_eq!(updater.pending_count(), 1);
        
        updater.clear_pending();
        assert_eq!(updater.pending_count(), 0);
    }

    #[test]
    fn test_batch_builder() {
        let mut builder = BatchBuilder::new(2);
        assert!(!builder.is_full());
        
        builder.add_vector(1, vec![1.0, 2.0, 3.0]).unwrap();
        assert!(!builder.is_full());
        
        builder.add_vector(2, vec![4.0, 5.0, 6.0]).unwrap();
        assert!(builder.is_full());
        
        let batch = builder.take_batch();
        assert_eq!(batch.len(), 2);
        assert_eq!(builder.current_size(), 0);
    }
}