//! Vector search and retrieval for VexFS
//! 
//! This module implements the user-facing vector search interface leveraging the ANNS infrastructure,
//! providing similarity metrics, query processing, and result filtering.

#![no_std]

extern crate alloc;
use alloc::{vec::Vec, collections::BTreeMap};
use core::cmp::Ordering;

use crate::anns::{DistanceMetric, HnswIndex, SearchResult};
use crate::vector_metrics::{VectorMetrics, MetricsError};
use crate::vector_storage::{VectorStorage, VectorHeader, VectorDataType};
use crate::knn_search::{KnnSearchEngine, KnnResult, SearchParams, MetadataFilter, KnnError};
use crate::result_scoring::{ResultScorer, ScoringParams, ScoredResult, ScoringError};
use crate::inode_mgmt::VexfsInode;

/// Maximum number of results that can be returned from a search
pub const MAX_SEARCH_RESULTS: usize = 10000;

/// Maximum batch size for search requests
pub const MAX_BATCH_SIZE: usize = 100;

/// Vector search error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchError {
    /// Invalid query parameters
    InvalidQuery,
    /// Storage access error
    StorageError,
    /// k-NN search error
    KnnError(KnnError),
    /// Metrics calculation error
    MetricsError(MetricsError),
    /// Scoring error
    ScoringError(ScoringError),
    /// Index not available
    IndexNotAvailable,
    /// Result processing error
    ResultProcessingError,
    /// Memory allocation error
    AllocationError,
    /// Invalid batch size
    InvalidBatchSize,
}

impl From<KnnError> for SearchError {
    fn from(error: KnnError) -> Self {
        SearchError::KnnError(error)
    }
}

impl From<MetricsError> for SearchError {
    fn from(error: MetricsError) -> Self {
        SearchError::MetricsError(error)
    }
}

impl From<ScoringError> for SearchError {
    fn from(error: ScoringError) -> Self {
        SearchError::ScoringError(error)
    }
}

/// Search query with all parameters
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Query vector
    pub vector: Vec<f32>,
    /// Number of results to return
    pub k: usize,
    /// Distance metric to use
    pub metric: DistanceMetric,
    /// Whether to use approximate search
    pub approximate: bool,
    /// Search expansion factor for approximate search
    pub expansion_factor: f32,
    /// Metadata filter
    pub filter: Option<MetadataFilter>,
    /// Whether to include exact distances
    pub exact_distances: bool,
    /// Whether to use SIMD optimizations
    pub use_simd: bool,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            vector: Vec::new(),
            k: 10,
            metric: DistanceMetric::Euclidean,
            approximate: true,
            expansion_factor: 2.0,
            filter: None,
            exact_distances: true,
            use_simd: true,
        }
    }
}

/// Batch search request
#[derive(Debug, Clone)]
pub struct BatchSearchRequest {
    /// Multiple queries to execute
    pub queries: Vec<SearchQuery>,
    /// Whether to merge results across queries
    pub merge_results: bool,
    /// Maximum total results to return if merging
    pub max_total_results: usize,
}

/// Advanced search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Enable result caching
    pub enable_caching: bool,
    /// Cache expiration time in seconds
    pub cache_expiration: u64,
    /// Enable search result validation
    pub validate_results: bool,
    /// Enable search analytics
    pub enable_analytics: bool,
    /// Custom scoring parameters
    pub scoring_params: Option<ScoringParams>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            enable_caching: false,
            cache_expiration: 300, // 5 minutes
            validate_results: true,
            enable_analytics: false,
            scoring_params: None,
        }
    }
}

/// Search analytics and statistics
#[derive(Debug, Clone, Default)]
pub struct SearchAnalytics {
    /// Total search time in microseconds
    pub search_time_us: u64,
    /// Number of vectors examined
    pub vectors_examined: usize,
    /// Cache hit rate (if caching enabled)
    pub cache_hit_rate: f32,
    /// Index utilization rate
    pub index_utilization: f32,
    /// Average result confidence
    pub avg_confidence: f32,
    /// Result distribution statistics
    pub result_distribution: ResultDistribution,
}

/// Result distribution statistics
#[derive(Debug, Clone, Default)]
pub struct ResultDistribution {
    /// Distance statistics
    pub distance_stats: DistanceStats,
    /// File type distribution
    pub file_type_distribution: BTreeMap<String, usize>,
    /// Size distribution buckets
    pub size_distribution: [usize; 5], // Small, Medium, Large, XLarge, XXLarge
}

/// Distance statistics
#[derive(Debug, Clone, Default)]
pub struct DistanceStats {
    /// Minimum distance
    pub min: f32,
    /// Maximum distance
    pub max: f32,
    /// Mean distance
    pub mean: f32,
    /// Standard deviation
    pub std_dev: f32,
    /// Median distance
    pub median: f32,
}

/// Main vector search engine
pub struct VectorSearchEngine {
    /// k-NN search engine
    knn_engine: KnnSearchEngine,
    /// Result scorer
    result_scorer: ResultScorer,
    /// Search options
    options: SearchOptions,
    /// Search analytics
    analytics: SearchAnalytics,
}

impl VectorSearchEngine {
    /// Create new vector search engine
    pub fn new(storage: VectorStorage, options: SearchOptions) -> Result<Self, SearchError> {
        let knn_engine = KnnSearchEngine::new(storage)?;
        let scoring_params = options.scoring_params.clone().unwrap_or_default();
        let result_scorer = ResultScorer::new(scoring_params);
        
        Ok(Self {
            knn_engine,
            result_scorer,
            options,
            analytics: SearchAnalytics::default(),
        })
    }
    
    /// Set HNSW index for approximate search
    pub fn set_hnsw_index(&mut self, index: HnswIndex) {
        self.knn_engine.set_hnsw_index(index);
    }
    
    /// Perform vector search
    pub fn search(&mut self, query: SearchQuery) -> Result<Vec<ScoredResult>, SearchError> {
        let start_time = self.get_current_time_us();
        
        // Validate query
        if query.vector.is_empty() || query.k == 0 || query.k > MAX_SEARCH_RESULTS {
            return Err(SearchError::InvalidQuery);
        }
        
        // Convert to k-NN search parameters
        let search_params = SearchParams {
            k: query.k,
            metric: query.metric,
            expansion_factor: query.expansion_factor,
            approximate: query.approximate,
            use_simd: query.use_simd,
            filter: query.filter.clone(),
            exact_distances: query.exact_distances,
        };
        
        // Perform k-NN search
        let knn_results = self.knn_engine.search(&query.vector, &search_params)?;
        
        // Score and rank results
        let results = self.result_scorer.score_and_rank(&knn_results, &search_params)?;
        
        // Update analytics
        let end_time = self.get_current_time_us();
        self.analytics.search_time_us = end_time - start_time;
        self.analytics.vectors_examined = results.len();
        
        // Validate results if enabled
        let final_results = if self.options.validate_results {
            self.validate_results(results)?
        } else {
            results
        };
        
        Ok(final_results)
    }
    
    /// Perform batch search
    pub fn batch_search(&mut self, batch: BatchSearchRequest) -> Result<Vec<Vec<ScoredResult>>, SearchError> {
        if batch.queries.is_empty() || batch.queries.len() > MAX_BATCH_SIZE {
            return Err(SearchError::InvalidBatchSize);
        }
        
        let mut all_results = Vec::with_capacity(batch.queries.len());
        
        for query in batch.queries {
            let results = self.search(query)?;
            all_results.push(results);
        }
        
        if batch.merge_results {
            // Merge and re-rank results
            let merged = self.merge_batch_results(all_results, batch.max_total_results)?;
            Ok(vec![merged])
        } else {
            Ok(all_results)
        }
    }
    
    /// Merge results from multiple searches
    fn merge_batch_results(
        &mut self,
        result_sets: Vec<Vec<ScoredResult>>,
        max_results: usize,
    ) -> Result<Vec<ScoredResult>, SearchError> {
        // Simple merge by taking the best results from each set
        let mut all_results = Vec::new();
        
        for results in result_sets {
            all_results.extend(results);
        }
        
        // Sort by relevance score
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        
        // Deduplicate by vector ID
        let mut seen_ids = BTreeMap::new();
        let mut deduped = Vec::new();
        
        for result in all_results {
            if !seen_ids.contains_key(&result.result.vector_id) && deduped.len() < max_results {
                seen_ids.insert(result.result.vector_id, true);
                deduped.push(result);
            }
        }
        
        Ok(deduped)
    }
    
    /// Validate search results
    fn validate_results(&self, mut results: Vec<ScoredResult>) -> Result<Vec<ScoredResult>, SearchError> {
        // Remove invalid results
        results.retain(|r| {
            r.confidence > 0.1 // Minimum confidence threshold
                && r.result.distance.is_finite()
                && r.score.is_finite()
        });
        
        Ok(results)
    }
    
    /// Get current time in microseconds (placeholder implementation)
    fn get_current_time_us(&self) -> u64 {
        // In a real kernel implementation, this would use kernel time functions
        1640995200_000_000 // Placeholder timestamp in microseconds
    }
    
    /// Get search analytics
    pub fn get_analytics(&self) -> &SearchAnalytics {
        &self.analytics
    }
    
    /// Reset analytics
    pub fn reset_analytics(&mut self) {
        self.analytics = SearchAnalytics::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_storage::VectorStorage;

    #[test]
    fn test_search_query_default() {
        let query = SearchQuery::default();
        assert_eq!(query.k, 10);
        assert_eq!(query.metric, DistanceMetric::Euclidean);
        assert!(query.approximate);
        assert!(query.use_simd);
    }

    #[test]
    fn test_search_options_default() {
        let options = SearchOptions::default();
        assert!(!options.enable_caching);
        assert!(options.validate_results);
        assert!(!options.enable_analytics);
    }

    #[test]
    fn test_batch_search_request() {
        let query = SearchQuery::default();
        let batch = BatchSearchRequest {
            queries: vec![query],
            merge_results: false,
            max_total_results: 100,
        };
        
        assert_eq!(batch.queries.len(), 1);
        assert!(!batch.merge_results);
    }
}