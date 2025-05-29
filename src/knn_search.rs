//! k-NN search algorithm with metadata filtering for VexFS
//!
//! This module implements efficient k-NN search with metadata-based filtering support,
//! integrating with the ANNS module and vector storage system.



extern crate alloc;
use alloc::{vec::Vec, collections::BinaryHeap};
use core::cmp::Ordering;

use crate::anns::{DistanceMetric, AnnsIndex, SearchResult};
use crate::vector_metrics::{VectorMetrics, MetricsError, ApproximateMetrics};
use crate::vector_handlers::{VectorStorage, VectorEmbedding};
use crate::vector_storage::{VectorHeader, VectorDataType};
use crate::ondisk::VexfsInode;
use crate::fs_core::operations::OperationContext;
use crate::shared::errors::VexfsResult;

/// Maximum number of results that can be returned
pub const MAX_KNN_RESULTS: usize = 10000;

/// Maximum number of candidate vectors to consider during search
pub const MAX_CANDIDATES: usize = 100000;

/// Threshold for switching to exact search
pub const EXACT_SEARCH_THRESHOLD: usize = 1000;

/// k-NN search error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KnnError {
    /// Invalid k value
    InvalidK,
    /// Invalid query vector
    InvalidQuery,
    /// Storage error
    StorageError,
    /// Metrics error
    MetricsError(MetricsError),
    /// Index error
    IndexError,
    /// Memory allocation error
    AllocationError,
    /// Filter error
    FilterError,
}

impl From<MetricsError> for KnnError {
    fn from(error: MetricsError) -> Self {
        KnnError::MetricsError(error)
    }
}

impl From<KnnError> for crate::shared::errors::VexfsError {
    fn from(err: KnnError) -> Self {
        match err {
            KnnError::InvalidK => crate::shared::errors::VexfsError::InvalidArgument("Invalid k value".to_string()),
            KnnError::InvalidQuery => crate::shared::errors::VexfsError::InvalidArgument("Invalid query vector".to_string()),
            KnnError::StorageError => crate::shared::errors::VexfsError::IoError(crate::shared::errors::IoErrorKind::ReadError),
            KnnError::MetricsError(_) => crate::shared::errors::VexfsError::VectorError(crate::shared::errors::VectorErrorKind::SearchError),
            KnnError::IndexError => crate::shared::errors::VexfsError::IndexError(crate::shared::errors::IndexErrorKind::IndexCorrupted),
            KnnError::AllocationError => crate::shared::errors::VexfsError::OutOfMemory,
            KnnError::FilterError => crate::shared::errors::VexfsError::InvalidArgument("Filter error".to_string()),
        }
    }
}

/// Search result with distance and metadata
#[derive(Debug, Clone)]
pub struct KnnResult {
    /// Vector ID
    pub vector_id: u64,
    /// File inode number
    pub file_inode: u64,
    /// Distance to query vector
    pub distance: f32,
    /// Vector dimensions
    pub dimensions: u32,
    /// Vector data type
    pub data_type: VectorDataType,
    /// File size in bytes
    pub file_size: u64,
    /// File creation timestamp
    pub created_timestamp: u64,
    /// File modification timestamp
    pub modified_timestamp: u64,
}

impl PartialEq for KnnResult {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for KnnResult {}

impl PartialOrd for KnnResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // For max-heap, we want larger distances to have higher priority
        other.distance.partial_cmp(&self.distance)
    }
}

impl Ord for KnnResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// Metadata filter for search results
#[derive(Debug, Clone)]
pub struct MetadataFilter {
    /// Minimum file size filter
    pub min_file_size: Option<u64>,
    /// Maximum file size filter
    pub max_file_size: Option<u64>,
    /// Minimum creation timestamp
    pub min_created_timestamp: Option<u64>,
    /// Maximum creation timestamp
    pub max_created_timestamp: Option<u64>,
    /// Minimum modification timestamp
    pub min_modified_timestamp: Option<u64>,
    /// Maximum modification timestamp
    pub max_modified_timestamp: Option<u64>,
    /// Required vector dimensions
    pub required_dimensions: Option<u32>,
    /// Required data type
    pub required_data_type: Option<VectorDataType>,
    /// Maximum distance threshold
    pub max_distance: Option<f32>,
}

impl MetadataFilter {
    /// Create new empty filter
    pub fn new() -> Self {
        Self {
            min_file_size: None,
            max_file_size: None,
            min_created_timestamp: None,
            max_created_timestamp: None,
            min_modified_timestamp: None,
            max_modified_timestamp: None,
            required_dimensions: None,
            required_data_type: None,
            max_distance: None,
        }
    }
    
    /// Check if a result passes the metadata filter
    pub fn matches(&self, result: &KnnResult) -> bool {
        if let Some(min_size) = self.min_file_size {
            if result.file_size < min_size {
                return false;
            }
        }
        
        if let Some(max_size) = self.max_file_size {
            if result.file_size > max_size {
                return false;
            }
        }
        
        if let Some(min_created) = self.min_created_timestamp {
            if result.created_timestamp < min_created {
                return false;
            }
        }
        
        if let Some(max_created) = self.max_created_timestamp {
            if result.created_timestamp > max_created {
                return false;
            }
        }
        
        if let Some(min_modified) = self.min_modified_timestamp {
            if result.modified_timestamp < min_modified {
                return false;
            }
        }
        
        if let Some(max_modified) = self.max_modified_timestamp {
            if result.modified_timestamp > max_modified {
                return false;
            }
        }
        
        if let Some(req_dims) = self.required_dimensions {
            if result.dimensions != req_dims {
                return false;
            }
        }
        
        if let Some(req_type) = self.required_data_type {
            if core::mem::discriminant(&result.data_type) != core::mem::discriminant(&req_type) {
                return false;
            }
        }
        
        if let Some(max_dist) = self.max_distance {
            if result.distance > max_dist {
                return false;
            }
        }
        
        true
    }
}

/// Search parameters for k-NN queries
#[derive(Debug, Clone)]
pub struct SearchParams {
    /// Number of nearest neighbors to find
    pub k: usize,
    /// Distance metric to use
    pub metric: DistanceMetric,
    /// Search expansion factor (how many candidates to consider)
    pub expansion_factor: f32,
    /// Whether to use approximate search
    pub approximate: bool,
    /// Whether to use SIMD optimizations
    pub use_simd: bool,
    /// Metadata filter
    pub filter: Option<MetadataFilter>,
    /// Whether to include exact distances (vs approximate)
    pub exact_distances: bool,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            k: 10,
            metric: DistanceMetric::Euclidean,
            expansion_factor: 2.0,
            approximate: true,
            use_simd: true,
            filter: None,
            exact_distances: true,
        }
    }
}

/// k-NN search engine with metadata filtering
pub struct KnnSearchEngine {
    /// Vector metrics calculator
    metrics: VectorMetrics,
    /// Vector storage system
    storage: Box<dyn VectorStorage<Error = String>>,
    /// HNSW index for approximate search
    hnsw_index: Option<AnnsIndex>,
    /// Search result buffer
    result_buffer: Vec<KnnResult>,
    /// Candidate buffer for filtering
    candidate_buffer: Vec<SearchResult>,
}

impl KnnSearchEngine {
    /// Create new k-NN search engine
    pub fn new(storage: Box<dyn VectorStorage<Error = String>>) -> Result<Self, KnnError> {
        Ok(Self {
            metrics: VectorMetrics::new(true),
            storage,
            hnsw_index: None,
            result_buffer: Vec::new(),
            candidate_buffer: Vec::new(),
        })
    }
    
    /// Set HNSW index for approximate search
    pub fn set_hnsw_index(&mut self, index: AnnsIndex) {
        self.hnsw_index = Some(index);
    }
    
    /// Perform k-NN search with metadata filtering
    pub fn search(
        &mut self,
        context: &mut OperationContext,
        query: &[f32],
        params: &SearchParams,
    ) -> Result<Vec<KnnResult>, KnnError> {
        if params.k == 0 || params.k > MAX_KNN_RESULTS {
            return Err(KnnError::InvalidK);
        }
        
        if query.is_empty() {
            return Err(KnnError::InvalidQuery);
        }
        
        // Clear previous results
        self.result_buffer.clear();
        self.candidate_buffer.clear();
        
        // Determine search strategy
        if params.approximate && self.hnsw_index.is_some() {
            self.approximate_search(context, query, params)
        } else {
            self.exact_search(context, query, params)
        }
    }
    
    /// Perform approximate k-NN search using HNSW index
    fn approximate_search(
        &mut self,
        context: &mut OperationContext,
        query: &[f32],
        params: &SearchParams,
    ) -> Result<Vec<KnnResult>, KnnError> {
        let hnsw = self.hnsw_index.as_ref().ok_or(KnnError::IndexError)?;
        
        // Calculate expanded search size
        let search_k = ((params.k as f32) * params.expansion_factor) as usize;
        let search_k = search_k.min(MAX_CANDIDATES);
        
        // Perform HNSW search
        let candidates = hnsw.search(context, query, search_k, None)
            .map_err(|_| KnnError::IndexError)?;
        
        self.candidate_buffer.extend(candidates);
        
        // Convert candidates to k-NN results with metadata
        self.process_candidates(context, query, params)
    }
    
    /// Perform exact k-NN search by scanning all vectors
    fn exact_search(
        &mut self,
        context: &mut OperationContext,
        query: &[f32],
        params: &SearchParams,
    ) -> Result<Vec<KnnResult>, KnnError> {
        // Get all vector IDs from storage
        let vector_ids = self.storage.get_all_vector_ids()
            .map_err(|_| KnnError::StorageError)?;
        
        if vector_ids.len() <= EXACT_SEARCH_THRESHOLD {
            self.exact_linear_scan(context, query, params, &vector_ids)
        } else {
            // For large datasets, use approximate methods for candidate generation
            // then exact distances for final ranking
            self.hybrid_search(context, query, params, &vector_ids)
        }
    }
    
    /// Perform exact linear scan for small datasets
    fn exact_linear_scan(
        &mut self,
        context: &mut OperationContext,
        query: &[f32],
        params: &SearchParams,
        vector_ids: &[u64],
    ) -> Result<Vec<KnnResult>, KnnError> {
        let mut heap = BinaryHeap::with_capacity(params.k);
        
        for &vector_id in vector_ids {
            // Load vector header and data
            let header = self.storage.get_vector_header(vector_id)
                .map_err(|_| KnnError::StorageError)?;
            
            // Skip if dimensions don't match query
            if header.dimensions as usize != query.len() {
                continue;
            }
            
            let vector_data = self.storage.get_vector_data(vector_id)
                .map_err(|_| KnnError::StorageError)?;
            
            // Calculate exact distance
            let distance = self.metrics.calculate_distance(
                query,
                &vector_data,
                params.metric,
            )?;
            
            // Create result with metadata
            let result = self.create_knn_result(vector_id, &header, distance)?;
            
            // Apply metadata filter
            if let Some(ref filter) = params.filter {
                if !filter.matches(&result) {
                    continue;
                }
            }
            
            // Add to heap
            if heap.len() < params.k {
                heap.push(result);
            } else if result.distance < heap.peek().unwrap().distance {
                heap.pop();
                heap.push(result);
            }
        }
        
        // Convert heap to sorted vector
        let mut results: Vec<_> = heap.into_iter().collect();
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal));
        
        Ok(results)
    }
    
    /// Perform hybrid search combining approximate and exact methods
    fn hybrid_search(
        &mut self,
        context: &mut OperationContext,
        query: &[f32],
        params: &SearchParams,
        vector_ids: &[u64],
    ) -> Result<Vec<KnnResult>, KnnError> {
        // First pass: approximate distances for candidate generation
        let mut candidates = Vec::with_capacity(vector_ids.len().min(MAX_CANDIDATES));
        
        for &vector_id in vector_ids {
            let header = self.storage.get_vector_header(vector_id)
                .map_err(|_| KnnError::StorageError)?;
            
            if header.dimensions as usize != query.len() {
                continue;
            }
            
            let vector_data = self.storage.get_vector_data(vector_id)
                .map_err(|_| KnnError::StorageError)?;
            
            // Use approximate distance for fast filtering
            let approx_distance = match params.metric {
                DistanceMetric::Euclidean => {
                    ApproximateMetrics::approx_euclidean_distance(query, &vector_data)
                }
                DistanceMetric::Cosine => {
                    ApproximateMetrics::approx_cosine_distance(query, &vector_data)
                }
                _ => {
                    // Fallback to exact distance for other metrics
                    self.metrics.calculate_distance(query, &vector_data, params.metric)?
                }
            };
            
            candidates.push((vector_id, header, approx_distance));
        }
        
        // Sort by approximate distance and take top candidates
        candidates.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));
        let top_candidates = (params.k * 10).min(candidates.len()); // 10x oversampling
        
        // Second pass: exact distances for top candidates
        let mut heap = BinaryHeap::with_capacity(params.k);
        
        for (vector_id, header, _) in candidates.into_iter().take(top_candidates) {
            let vector_data = self.storage.get_vector_data(vector_id)
                .map_err(|_| KnnError::StorageError)?;
            
            // Calculate exact distance
            let exact_distance = if params.exact_distances {
                self.metrics.calculate_distance(query, &vector_data, params.metric)?
            } else {
                // Use the approximate distance if exact is not required
                self.metrics.calculate_distance(query, &vector_data, params.metric)?
            };
            
            let result = self.create_knn_result(vector_id, &header, exact_distance)?;
            
            // Apply metadata filter
            if let Some(ref filter) = params.filter {
                if !filter.matches(&result) {
                    continue;
                }
            }
            
            // Add to heap
            if heap.len() < params.k {
                heap.push(result);
            } else if result.distance < heap.peek().unwrap().distance {
                heap.pop();
                heap.push(result);
            }
        }
        
        // Convert heap to sorted vector
        let mut results: Vec<_> = heap.into_iter().collect();
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal));
        
        Ok(results)
    }
    
    /// Process search candidates and convert to k-NN results
    fn process_candidates(
        &mut self,
        context: &mut OperationContext,
        query: &[f32],
        params: &SearchParams,
    ) -> Result<Vec<KnnResult>, KnnError> {
        let mut heap = BinaryHeap::with_capacity(params.k);
        
        for candidate in &self.candidate_buffer {
            // Load vector header for metadata
            let header = self.storage.get_vector_header(candidate.vector_id)
                .map_err(|_| KnnError::StorageError)?;
            
            let distance = if params.exact_distances {
                // Recalculate exact distance
                let vector_data = self.storage.get_vector_data(candidate.vector_id)
                    .map_err(|_| KnnError::StorageError)?;
                
                self.metrics.calculate_distance(query, &vector_data, params.metric)?
            } else {
                // Use distance from HNSW search
                candidate.distance
            };
            
            let result = self.create_knn_result(candidate.vector_id, &header, distance)?;
            
            // Apply metadata filter
            if let Some(ref filter) = params.filter {
                if !filter.matches(&result) {
                    continue;
                }
            }
            
            // Add to heap
            if heap.len() < params.k {
                heap.push(result);
            } else if result.distance < heap.peek().unwrap().distance {
                heap.pop();
                heap.push(result);
            }
        }
        
        // Convert heap to sorted vector
        let mut results: Vec<_> = heap.into_iter().collect();
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal));
        
        Ok(results)
    }
    
    /// Create k-NN result from vector header and distance
    fn create_knn_result(
        &self,
        vector_id: u64,
        header: &VectorHeader,
        distance: f32,
    ) -> Result<KnnResult, KnnError> {
        // Get file metadata from inode
        let file_size = match self.get_file_size(header.file_inode) {
            Ok(size) => size,
            Err(_) => 0, // Default to 0 if unable to get file size
        };
        
        Ok(KnnResult {
            vector_id,
            file_inode: header.file_inode,
            distance,
            dimensions: header.dimensions,
            data_type: header.data_type,
            file_size,
            created_timestamp: header.created_timestamp,
            modified_timestamp: header.modified_timestamp,
        })
    }
    
    /// Get file size from inode (placeholder implementation)
    fn get_file_size(&self, inode_num: u64) -> Result<u64, KnnError> {
        // In a real implementation, this would query the inode management system
        // For now, return a placeholder value
        Ok(1024) // Default file size
    }
    
    /// Perform batch k-NN search for multiple queries
    pub fn batch_search(
        &mut self,
        context: &mut OperationContext,
        queries: &[&[f32]],
        params: &SearchParams,
    ) -> Result<Vec<Vec<KnnResult>>, KnnError> {
        let mut batch_results = Vec::with_capacity(queries.len());
        
        for query in queries {
            let results = self.search(context, query, params)?;
            batch_results.push(results);
        }
        
        Ok(batch_results)
    }
    
    /// Get search statistics
    pub fn get_search_stats(&self) -> SearchStats {
        SearchStats {
            total_vectors: self.storage.get_vector_count().unwrap_or(0) as u64,
            index_available: self.hnsw_index.is_some(),
            last_search_candidates: self.candidate_buffer.len(),
            last_search_results: self.result_buffer.len(),
        }
    }
}

/// Search statistics
#[derive(Debug, Clone)]
pub struct SearchStats {
    /// Total number of vectors in storage
    pub total_vectors: u64,
    /// Whether HNSW index is available
    pub index_available: bool,
    /// Number of candidates in last search
    pub last_search_candidates: usize,
    /// Number of results in last search
    pub last_search_results: usize,
}

/// Query-aware pruning for efficient search
pub struct QueryPruner {
    /// Distance threshold for pruning
    distance_threshold: f32,
    /// Dimension subset for fast filtering
    dimension_subset: Vec<usize>,
}

impl QueryPruner {
    /// Create new query pruner
    pub fn new(distance_threshold: f32, subset_size: usize, total_dims: usize) -> Self {
        // Select evenly spaced dimensions for subset
        let step = total_dims / subset_size.max(1);
        let dimension_subset: Vec<usize> = (0..total_dims).step_by(step.max(1)).take(subset_size).collect();
        
        Self {
            distance_threshold,
            dimension_subset,
        }
    }
    
    /// Check if a vector should be pruned (skipped) during search
    pub fn should_prune(&self, query: &[f32], candidate: &[f32]) -> bool {
        if self.dimension_subset.is_empty() {
            return false;
        }
        
        // Calculate partial distance using subset of dimensions
        let mut partial_distance_sq = 0.0f32;
        
        for &dim_idx in &self.dimension_subset {
            if dim_idx < query.len() && dim_idx < candidate.len() {
                let diff = query[dim_idx] - candidate[dim_idx];
                partial_distance_sq += diff * diff;
            }
        }
        
        // Scale up the partial distance to estimate full distance
        let scale_factor = query.len() as f32 / self.dimension_subset.len() as f32;
        let estimated_distance = (partial_distance_sq * scale_factor).sqrt();
        
        estimated_distance > self.distance_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metadata_filter() {
        let mut filter = MetadataFilter::new();
        filter.min_file_size = Some(1000);
        filter.max_distance = Some(0.5);
        
        let result = KnnResult {
            vector_id: 1,
            file_inode: 100,
            distance: 0.3,
            dimensions: 128,
            data_type: VectorDataType::Float32,
            file_size: 2000,
            created_timestamp: 1000000,
            modified_timestamp: 1000000,
        };
        
        assert!(filter.matches(&result));
        
        let result2 = KnnResult {
            vector_id: 2,
            file_inode: 101,
            distance: 0.8, // Exceeds max_distance
            dimensions: 128,
            data_type: VectorDataType::Float32,
            file_size: 2000,
            created_timestamp: 1000000,
            modified_timestamp: 1000000,
        };
        
        assert!(!filter.matches(&result2));
    }
    
    #[test]
    fn test_query_pruner() {
        let pruner = QueryPruner::new(1.0, 4, 8);
        
        let query = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let close_candidate = [1.1, 2.1, 3.1, 4.1, 5.1, 6.1, 7.1, 8.1];
        let far_candidate = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        
        assert!(!pruner.should_prune(&query, &close_candidate));
        assert!(pruner.should_prune(&query, &far_candidate));
    }
}